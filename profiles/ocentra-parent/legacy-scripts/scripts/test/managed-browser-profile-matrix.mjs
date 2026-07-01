import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdir, stat, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';

import { removeDirectoryWithRetry } from './agent-service-process.mjs';

const defaultUrls = ['https://example.com/', 'https://www.wikipedia.org/', 'https://www.youtube.com/'];
const defaultProfiles = ['managed-browser-profile-a', 'managed-browser-profile-b', 'managed-browser-profile-c'];
const runId = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
const evidenceDirectory = join(process.cwd(), 'test-results', 'managed-browser-profile-matrix');
const screenshotDirectory = join(evidenceDirectory, `${runId}-screenshots`);
const probeRoot = join(tmpdir(), `ocentra-parent-managed-browser-profile-matrix-${process.pid}`);

const urls = envList('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_URLS', defaultUrls);
const profiles = envList('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_PROFILES', defaultProfiles);
const browserFilters = envList('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_BROWSERS', []);
const timeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_TIMEOUT_MS', 45_000);
const commandTimeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_COMMAND_TIMEOUT_MS', 15_000);
const readyTimeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_READY_TIMEOUT_MS', 15_000);

async function main() {
  const supportedBrowsers = await installedSupportedBrowsers();
  const unsupportedBrowsers = await installedUnsupportedBrowsers();

  if (supportedBrowsers.length === 0) {
    throw new Error('No installed Chrome, Edge, or Firefox executable found for managed browser matrix proof.');
  }

  await mkdir(evidenceDirectory, { recursive: true });
  await mkdir(screenshotDirectory, { recursive: true });
  await mkdir(probeRoot, { recursive: true });

  const browserResults = [];
  try {
    for (const browser of supportedBrowsers) {
      browserResults.push(await runBrowserMatrix(browser));
    }
  } finally {
    await stopWindowsProcessesByCommandLineFragment(probeRoot);
    await removeDirectoryWithRetry(probeRoot, { attempts: 20, delayMs: 250 });
  }

  const evidence = {
    schemaVersion: 2,
    generatedAt: new Date().toISOString(),
    urls,
    historyProbeUrls: urls.map((url, index) => historyProbeUrl(url, index)),
    profiles,
    supportedBrowsers: browserResults,
    unsupportedInstalledBrowsers: unsupportedBrowsers,
    summary: summarize(browserResults, unsupportedBrowsers),
  };

  const evidencePath = join(evidenceDirectory, `${runId}.json`);
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);

  printSummary(evidence, evidencePath);

  if (evidence.summary.failures.length > 0) {
    process.exitCode = 1;
  }
}

async function runBrowserMatrix(browser) {
  if (browser.bridge === 'webdriver-bidi') {
    return runFirefoxBrowserMatrix(browser);
  }
  return runChromiumBrowserMatrix(browser);
}

async function runChromiumBrowserMatrix(browser) {
  const observations = [];
  for (const profileName of profiles) {
    const profileRun = await launchChromiumProfile(browser, profileName);
    try {
      observations.push(await observeChromiumProfile(browser, profileRun));
    } finally {
      await cleanupProfileRun(profileRun);
    }
  }
  return {
    browser,
    profiles: observations,
  };
}

async function runFirefoxBrowserMatrix(browser) {
  const observations = [];
  for (const profileName of profiles) {
    const profileRun = await launchFirefoxProfile(browser, profileName);
    try {
      observations.push(await observeFirefoxProfile(browser, profileRun));
    } finally {
      await cleanupProfileRun(profileRun);
    }
  }
  return {
    browser,
    profiles: observations,
  };
}

async function waitForChromiumReady(client, expectedFragment = null) {
  const deadline = Date.now() + readyTimeoutMs;
  while (Date.now() < deadline) {
    const state = await evaluateString(client, 'document.readyState').catch(() => '');
    const href = await evaluateString(client, 'location.href').catch(() => '');
    if (state === 'complete' && (expectedFragment === null || href.includes(expectedFragment))) {
      return { readyState: state, url: href };
    }
    await delay(500);
  }
  return {
    readyState: await evaluateString(client, 'document.readyState').catch(() => 'unknown'),
    url: await evaluateString(client, 'location.href').catch(() => ''),
  };
}

async function waitForFirefoxReady(bidi, context, expectedFragment = null) {
  const deadline = Date.now() + readyTimeoutMs;
  while (Date.now() < deadline) {
    const state = await evaluateFirefoxString(bidi, context, 'document.readyState').catch(() => '');
    const href = await evaluateFirefoxString(bidi, context, 'location.href').catch(() => '');
    if (state === 'complete' && (expectedFragment === null || href.includes(expectedFragment))) {
      return { readyState: state, url: href };
    }
    await delay(500);
  }
  return {
    readyState: await evaluateFirefoxString(bidi, context, 'document.readyState').catch(() => 'unknown'),
    url: await evaluateFirefoxString(bidi, context, 'location.href').catch(() => ''),
  };
}

async function launchChromiumProfile(browser, profileName) {
  const port = await freePort();
  const profileDir = join(probeRoot, browser.id, profileName);
  await mkdir(profileDir, { recursive: true });

  const child = spawn(
    browser.executablePath,
    [
      '--remote-debugging-address=127.0.0.1',
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profileDir}`,
      '--profile-directory=OcentraManagedChild',
      '--no-first-run',
      '--no-default-browser-check',
      '--new-window',
      ...urls,
    ],
    {
      stdio: 'ignore',
      windowsHide: true,
    }
  );

  return {
    child,
    port,
    profileName,
    profileDir,
    launcherState: chromiumLauncherState(child, browser),
  };
}

async function launchFirefoxProfile(browser, profileName) {
  const port = await freePort();
  const profileDir = join(probeRoot, browser.id, profileName);
  await mkdir(profileDir, { recursive: true });

  const child = spawn(
    browser.executablePath,
    ['--no-remote', '--new-instance', '-profile', profileDir, '--remote-debugging-port', String(port), 'about:blank'],
    {
      stdio: 'ignore',
      windowsHide: true,
    }
  );

  return {
    child,
    port,
    profileName,
    profileDir,
  };
}

async function observeChromiumProfile(browser, profileRun) {
  const version = await waitForJson(profileRun.port, '/json/version');
  const initialTargets = await waitForTargets(profileRun.port);
  const initialPageTargets = pageTargetsFromChromiumList(initialTargets);
  const clients = [];
  const visitedUrlJournal = [];
  const activeTabChecks = [];
  const screenshots = [];
  let activeTargetId = null;

  try {
    for (const target of initialPageTargets) {
      if (target.webSocketDebuggerUrl === null) {
        continue;
      }
      const client = await DevToolsClient.connect(target.webSocketDebuggerUrl);
      clients.push(client);
      client.onEvent((event) => {
        const url = eventUrlFromChromiumEvent(event);
        if (url !== undefined) {
          visitedUrlJournal.push({
            source: 'cdp-event',
            method: event.method,
            targetId: target.targetId,
            url,
            observedAt: new Date().toISOString(),
          });
        }
      });
      await client.command('Page.enable', {});
      await client.command('Runtime.enable', {});
      await waitForChromiumReady(client);
      visitedUrlJournal.push({
        source: 'initial-snapshot',
        method: 'Runtime.evaluate',
        targetId: target.targetId,
        url: await evaluateString(client, 'location.href'),
        observedAt: new Date().toISOString(),
      });
    }

    const targetToActivate = firstMatchingClient(initialPageTargets, clients) ?? clients.at(-1) ?? null;
    if (targetToActivate !== undefined) {
      await targetToActivate.client.command('Page.bringToFront', {});
      await delay(750);
      activeTargetId = targetToActivate.target.targetId;
    }

    for (const pair of pairTargetsWithClients(initialPageTargets, clients)) {
      activeTabChecks.push(await chromiumActiveTabCheck(pair.target, pair.client));
    }

    for (const pair of pairTargetsWithClients(initialPageTargets, clients)) {
      const currentUrl = capturedUrl(pair.target.url)
        ? pair.target.url
        : await evaluateString(pair.client, 'location.href');
      const nextUrl = historyProbeUrl(currentUrl, visitedUrlJournal.length);
      await pair.client.command('Page.navigate', { url: nextUrl }).catch((error) => {
        visitedUrlJournal.push({
          source: 'cdp-navigation-error',
          method: 'Page.navigate',
          targetId: pair.target.targetId,
          url: nextUrl,
          error: String(error.message ?? error),
          observedAt: new Date().toISOString(),
        });
      });
      await waitForChromiumReady(pair.client, 'ocentra_history_probe=');
    }

    await delay(1500);

    for (const pair of pairTargetsWithClients(initialPageTargets, clients)) {
      visitedUrlJournal.push({
        source: 'post-navigation-snapshot',
        method: 'Runtime.evaluate',
        targetId: pair.target.targetId,
        url: await evaluateString(pair.client, 'location.href'),
        observedAt: new Date().toISOString(),
      });
      screenshots.push(await captureChromiumScreenshot(browser, profileRun.profileName, pair.target, pair.client));
    }
  } finally {
    for (const client of clients) {
      client.close();
    }
  }

  const finalTargets = await waitForTargets(profileRun.port);
  const pageTargets = pageTargetsFromChromiumList(finalTargets);
  for (const target of pageTargets) {
    if (capturedUrl(target.url)) {
      visitedUrlJournal.push({
        source: 'final-target-list-snapshot',
        method: '/json/list',
        targetId: target.targetId,
        url: target.url,
        observedAt: new Date().toISOString(),
      });
    }
  }
  const siteEvidence = urls.map((url) => siteEvidenceForUrl(url, pageTargets, visitedUrlJournal));
  const activeProof = activeTabProof(activeTabChecks, activeTargetId);
  const historyProof = visitedHistoryProof(visitedUrlJournal, urls);

  return {
    profileName: profileRun.profileName,
    launcherState: profileRun.launcherState,
    profilePathContainsManagedPrefix: profileRun.profileDir.includes('managed-browser-profile'),
    browserVersion: String(version.Browser ?? ''),
    protocolVersion: version['Protocol-Version'] === undefined ? null : String(version['Protocol-Version']),
    bridge: browser.bridge,
    devtoolsEndpoint: 'loopback-redacted',
    pageTargetCount: pageTargets.length,
    pageTargets: pageTargets.map(redactChromiumPageTargetForEvidence),
    activeTabChecks,
    screenshots,
    visitedUrlJournal,
    siteEvidence,
    assertions: {
      canConnectManagedProfile: Boolean(version.Browser),
      canSeeTabs: pageTargets.length >= urls.length,
      canSeeUrls: siteEvidence.every((site) => site.captured),
      canMatchRequestedSites: siteEvidence.every((site) => site.matchedRequestedHost),
      canProveActiveTab: activeProof.proven,
      activeTabProof: activeProof.reason,
      canJournalVisitedUrls: historyProof.proven,
      visitedHistoryProof: historyProof.reason,
    },
    browserFamily: browser.family,
    browserChannel: browser.channel,
  };
}

function chromiumLauncherState(child, browser) {
  return {
    managedState: 'running-managed',
    capabilityStatus: 'bridge-missing',
    bridgeEndpointRef: 'managed-loopback-devtools-redacted',
    bridgePortRef: 'managed-loopback-port-redacted',
    profilePathRef: 'managed-profile-redacted',
    custodyLabel: 'child-device-local',
    browserFamily: browser.family,
    browserChannel: browser.channel,
    processId: child.pid ?? null,
    loopbackOnly: true,
    rawDebuggerEndpointExposed: false,
  };
}

async function observeFirefoxProfile(browser, profileRun) {
  const bidi = await FirefoxBidiClient.connect(`ws://127.0.0.1:${profileRun.port}/session`, timeoutMs);
  const visitedUrlJournal = [];
  const activeTabChecks = [];
  const screenshots = [];
  const navigationErrors = [];
  const contexts = [];
  let activeContext = null;

  try {
    const session = await bidi.command('session.new', { capabilities: {} });
    await bidi.command('session.subscribe', {
      events: [
        'browsingContext.contextCreated',
        'browsingContext.navigationStarted',
        'browsingContext.navigationCommitted',
        'browsingContext.historyUpdated',
        'browsingContext.load',
      ],
    });
    bidi.onEvent((event) => {
      const url = event.params?.url;
      if (typeof url === 'string' && url.length > 0) {
        visitedUrlJournal.push({
          source: 'bidi-event',
          method: event.method,
          context: event.params?.context ?? null,
          url,
          observedAt: new Date().toISOString(),
        });
      }
    });

    for (const url of urls) {
      const created = await bidi.command('browsingContext.create', { type: 'tab' });
      contexts.push(created.context);
      await bidi
        .command('browsingContext.navigate', { context: created.context, url, wait: 'complete' })
        .catch((error) => {
          navigationErrors.push({
            context: created.context,
            url,
            error: String(error.message ?? error),
          });
        });
      await waitForFirefoxReady(bidi, created.context);
    }

    activeContext = contexts.at(-1) ?? null;
    if (activeContext !== undefined) {
      await bidi.command('browsingContext.activate', { context: activeContext }).catch((error) => {
        navigationErrors.push({
          context: activeContext,
          url: null,
          error: `activate failed: ${String(error.message ?? error)}`,
        });
      });
      await delay(750);
    }

    for (const context of contexts) {
      activeTabChecks.push(await firefoxActiveTabCheck(context, bidi));
    }

    for (const context of contexts) {
      const currentUrl = await evaluateFirefoxString(bidi, context, 'location.href');
      const nextUrl = historyProbeUrl(currentUrl, visitedUrlJournal.length);
      await bidi.command('browsingContext.navigate', { context, url: nextUrl, wait: 'complete' }).catch((error) => {
        navigationErrors.push({
          context,
          url: nextUrl,
          error: String(error.message ?? error),
        });
        visitedUrlJournal.push({
          source: 'bidi-navigation-error',
          method: 'browsingContext.navigate',
          context,
          url: nextUrl,
          error: String(error.message ?? error),
          observedAt: new Date().toISOString(),
        });
      });
      await waitForFirefoxReady(bidi, context, 'ocentra_history_probe=');
    }

    await delay(1500);

    const tree = await bidi.command('browsingContext.getTree', {});
    const pageTargets = flattenContexts(tree.contexts ?? [])
      .filter((context) => context.parent === undefined || context.parent === null)
      .map((context) => ({
        targetId: String(context.context ?? ''),
        type: 'page',
        url: String(context.url ?? ''),
        title: null,
        userContext: context.userContext === undefined ? null : String(context.userContext),
        urlCapture: capturedUrl(context.url),
        titleCapture: false,
      }));

    for (const context of contexts) {
      visitedUrlJournal.push({
        source: 'post-navigation-snapshot',
        method: 'script.evaluate',
        context,
        url: await evaluateFirefoxString(bidi, context, 'location.href'),
        observedAt: new Date().toISOString(),
      });
      screenshots.push(await captureFirefoxScreenshot(browser, profileRun.profileName, context, bidi));
    }

    const siteEvidence = urls.map((url) => siteEvidenceForUrl(url, pageTargets, visitedUrlJournal));
    const activeProof = activeTabProof(activeTabChecks, activeContext);
    const historyProof = visitedHistoryProof(visitedUrlJournal, urls);

    return {
      profileName: profileRun.profileName,
      profilePathContainsManagedPrefix: profileRun.profileDir.includes('managed-browser-profile'),
      browserVersion: browser.id,
      protocolVersion: session.capabilities?.browserVersion ?? null,
      bridge: browser.bridge,
      devtoolsEndpoint: 'loopback-redacted',
      pageTargetCount: pageTargets.length,
      pageTargets: pageTargets.map(redactChromiumPageTargetForEvidence),
      activeTabChecks,
      screenshots,
      visitedUrlJournal,
      navigationErrors,
      siteEvidence,
      assertions: {
        canConnectManagedProfile: true,
        canSeeTabs: pageTargets.length >= urls.length,
        canSeeUrls: siteEvidence.every((site) => site.captured),
        canMatchRequestedSites: siteEvidence.every((site) => site.matchedRequestedHost),
        canProveActiveTab: activeProof.proven,
        activeTabProof: activeProof.reason,
        canJournalVisitedUrls: historyProof.proven,
        visitedHistoryProof: historyProof.reason,
      },
      browserFamily: browser.family,
      browserChannel: browser.channel,
    };
  } finally {
    await bidi.command('browser.close', {}).catch(() => undefined);
    bidi.close();
  }
}

async function cleanupProfileRun(profileRun) {
  await stopWindowsProcessesByCommandLineFragment(profileRun.profileDir);
  if (profileRun.child.exitCode === null && profileRun.child.signalCode === null) {
    profileRun.child.kill();
  }
  await removeDirectoryWithRetry(profileRun.profileDir, { attempts: 20, delayMs: 250 });
}

function pageTargetsFromChromiumList(targets) {
  return targets
    .filter((target) => target.type === 'page')
    .map((target) => ({
      targetId: String(target.id ?? ''),
      type: String(target.type ?? ''),
      url: String(target.url ?? ''),
      title: target.title === undefined ? null : String(target.title),
      webSocketDebuggerUrl:
        typeof target.webSocketDebuggerUrl === 'string' && target.webSocketDebuggerUrl.length > 0
          ? target.webSocketDebuggerUrl
          : null,
      urlCapture: capturedUrl(target.url),
      titleCapture: capturedTitle(target.title),
    }));
}

function redactChromiumPageTargetForEvidence(target) {
  return {
    ...target,
    webSocketDebuggerUrl: target.webSocketDebuggerUrl === null ? null : 'loopback-redacted',
  };
}

function siteEvidenceForUrl(requestedUrl, pageTargets, visitedUrlJournal) {
  const requested = new URL(requestedUrl);
  const matchedTarget = pageTargets.find((target) => hostMatches(requested.hostname, target.url));
  const matchedJournalEntry = visitedUrlJournal.find((entry) => hostMatches(requested.hostname, entry.url));
  const fallbackTarget = pageTargets.find((target) => target.url === requestedUrl) ?? null;
  const target = matchedTarget ?? fallbackTarget;
  const journalEntry = matchedJournalEntry ?? null;
  return {
    requestedUrl,
    requestedHost: requested.hostname,
    captured:
      (target !== undefined && target !== undefined && capturedUrl(target.url)) ||
      (journalEntry !== undefined && capturedUrl(journalEntry.url)),
    matchedRequestedHost: matchedTarget !== undefined || matchedJournalEntry !== undefined,
    observedUrl: target?.url ?? journalEntry?.url ?? null,
    observedTitle: target?.title ?? null,
    journaled: journalEntry !== undefined,
  };
}

function activeTabProof(activeTabChecks, expectedActiveId) {
  const focused = activeTabChecks.filter((check) => check.hasFocus === true);
  const visible = activeTabChecks.filter((check) => check.visibilityState === 'visible');
  const expected = activeTabChecks.find(
    (check) => check.targetId === expectedActiveId || check.context === expectedActiveId
  );
  if (expected !== undefined && expected.hasFocus === true && expected.visibilityState === 'visible') {
    return {
      proven: true,
      reason: 'protocol-activated-tab-reported-visible-and-focused',
    };
  }
  if (focused.length === 1) {
    return {
      proven: true,
      reason: 'single-runtime-focused-tab-observed',
    };
  }
  if (visible.length === 1) {
    return {
      proven: true,
      reason: 'single-runtime-visible-tab-observed',
    };
  }
  return {
    proven: false,
    reason: `ambiguous-runtime-active-state focused=${focused.length} visible=${visible.length}`,
  };
}

function visitedHistoryProof(visitedUrlJournal, requestedUrls) {
  const journaledUrls = visitedUrlJournal.filter((entry) => capturedUrl(entry.url)).map((entry) => entry.url);
  const matchedRequestedCount = requestedUrls.filter((url) =>
    journaledUrls.some((journaledUrl) => hostMatches(new URL(url).hostname, journaledUrl))
  ).length;
  const probeCount = journaledUrls.filter((url) => url.includes('ocentra_history_probe=')).length;
  if (matchedRequestedCount >= requestedUrls.length && probeCount >= requestedUrls.length) {
    return {
      proven: true,
      reason: 'event-and-snapshot-journal-captured-initial-and-probe-navigation-urls',
    };
  }
  return {
    proven: false,
    reason: `journal-incomplete requestedMatches=${matchedRequestedCount}/${requestedUrls.length} probeCount=${probeCount}`,
  };
}

async function chromiumActiveTabCheck(target, client) {
  return {
    targetId: target.targetId,
    url: await evaluateString(client, 'location.href'),
    title: await evaluateString(client, 'document.title'),
    visibilityState: await evaluateString(client, 'document.visibilityState'),
    hasFocus: await evaluateBoolean(client, 'document.hasFocus()'),
  };
}

async function firefoxActiveTabCheck(context, bidi) {
  return {
    context,
    url: await evaluateFirefoxString(bidi, context, 'location.href'),
    title: await evaluateFirefoxString(bidi, context, 'document.title'),
    visibilityState: await evaluateFirefoxString(bidi, context, 'document.visibilityState'),
    hasFocus: await evaluateFirefoxBoolean(bidi, context, 'document.hasFocus()'),
  };
}

async function evaluateString(client, expression) {
  const result = await client.command('Runtime.evaluate', {
    expression,
    returnByValue: true,
  });
  const value = result.result?.value;
  return typeof value === 'string' ? value : String(value ?? '');
}

async function evaluateBoolean(client, expression) {
  const result = await client.command('Runtime.evaluate', {
    expression,
    returnByValue: true,
  });
  return result.result?.value === true;
}

async function evaluateFirefoxString(bidi, context, expression) {
  const result = await bidi.command('script.evaluate', {
    expression,
    target: { context },
    awaitPromise: false,
  });
  const value = result.result?.value;
  return typeof value === 'string' ? value : String(value ?? '');
}

async function evaluateFirefoxBoolean(bidi, context, expression) {
  const result = await bidi.command('script.evaluate', {
    expression,
    target: { context },
    awaitPromise: false,
  });
  return result.result?.value === true;
}

async function captureChromiumScreenshot(browser, profileName, target, client) {
  await client.command('Page.bringToFront', {}).catch(() => undefined);
  await delay(500);
  const result = await client
    .command('Page.captureScreenshot', {
      format: 'png',
      fromSurface: true,
    })
    .catch((error) => ({ error: String(error.message ?? error) }));
  if (typeof result.data !== 'string') {
    return {
      targetId: target.targetId,
      url: await evaluateString(client, 'location.href').catch(() => target.url),
      captured: false,
      error: result.error ?? 'missing screenshot data',
    };
  }
  const path = await saveScreenshot(browser, profileName, target.targetId, result.data);
  return {
    targetId: target.targetId,
    url: await evaluateString(client, 'location.href').catch(() => target.url),
    captured: true,
    path,
  };
}

async function captureFirefoxScreenshot(browser, profileName, context, bidi) {
  const result = await bidi.command('browsingContext.captureScreenshot', { context }).catch((error) => ({
    error: String(error.message ?? error),
  }));
  if (typeof result.data !== 'string') {
    return {
      context,
      url: await evaluateFirefoxString(bidi, context, 'location.href').catch(() => ''),
      captured: false,
      error: result.error ?? 'missing screenshot data',
    };
  }
  const path = await saveScreenshot(browser, profileName, context, result.data);
  return {
    context,
    url: await evaluateFirefoxString(bidi, context, 'location.href').catch(() => ''),
    captured: true,
    path,
  };
}

async function saveScreenshot(browser, profileName, targetId, base64Data) {
  const filename = `${sanitizeFilePart(browser.id)}-${sanitizeFilePart(profileName)}-${sanitizeFilePart(targetId)}.png`;
  const path = join(screenshotDirectory, filename);
  await writeFile(path, Buffer.from(base64Data, 'base64'));
  return path;
}

function sanitizeFilePart(value) {
  return String(value).replace(/[^a-zA-Z0-9._-]+/g, '_');
}

function eventUrlFromChromiumEvent(event) {
  if (event.method === 'Page.frameNavigated' && typeof event.params?.frame?.url === 'string') {
    return event.params.frame.url;
  }
  if (event.method === 'Page.navigatedWithinDocument' && typeof event.params?.url === 'string') {
    return event.params.url;
  }
  if (event.method === 'Page.frameRequestedNavigation' && typeof event.params?.url === 'string') {
    return event.params.url;
  }
  return null;
}

function firstMatchingClient(targets, clients) {
  for (const pair of pairTargetsWithClients(targets, clients)) {
    if (urls.some((url) => hostMatches(new URL(url).hostname, pair.target.url))) {
      return pair;
    }
  }
  return null;
}

function pairTargetsWithClients(targets, clients) {
  return targets
    .map((target) => ({
      target,
      client: clients.find((candidate) => candidate.url === target.webSocketDebuggerUrl) ?? null,
    }))
    .filter((pair) => pair.client !== undefined);
}

function flattenContexts(contexts) {
  const output = [];
  for (const context of contexts) {
    output.push(context);
    output.push(...flattenContexts(context.children ?? []));
  }
  return output;
}

async function waitForTargets(port) {
  const deadline = Date.now() + timeoutMs;
  let latestTargets = [];
  while (Date.now() < deadline) {
    latestTargets = await jsonRequest(port, '/json/list').catch(() => []);
    const pageTargets = latestTargets.filter((target) => target.type === 'page');
    const capturedCount = pageTargets.filter((target) => capturedUrl(target.url)).length;
    if (capturedCount >= urls.length) {
      return latestTargets;
    }
    await delay(500);
  }
  return latestTargets;
}

async function waitForJson(port, path) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      return await jsonRequest(port, path);
    } catch (error) {
      lastError = error;
      await delay(500);
    }
  }
  throw new Error(`Timed out waiting for http://127.0.0.1:${port}${path}: ${lastError?.message ?? 'no response'}`);
}

async function jsonRequest(port, path) {
  const response = await fetch(`http://127.0.0.1:${port}${path}`);
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`);
  }
  return response.json();
}

function capturedUrl(value) {
  return typeof value === 'string' && (value.startsWith('http://') || value.startsWith('https://'));
}

function capturedTitle(value) {
  return typeof value === 'string' && value.length > 0;
}

function hostMatches(expectedHost, observedUrl) {
  if (!capturedUrl(observedUrl)) {
    return false;
  }
  const observed = new URL(observedUrl);
  const expected = normalizeHost(expectedHost);
  return normalizeHost(observed.hostname) === expected || normalizeHost(observed.hostname).endsWith(`.${expected}`);
}

function normalizeHost(host) {
  return host.toLowerCase().replace(/^www\./, '');
}

function historyProbeUrl(url, index) {
  if (!capturedUrl(url)) {
    return url;
  }
  const parsed = new URL(url);
  parsed.hash = `ocentra_history_probe=${index + 1}`;
  return parsed.toString();
}

async function installedSupportedBrowsers() {
  const candidates = browserCandidates().filter(
    (candidate) => candidate.supported && browserCandidateAllowed(candidate)
  );
  const installed = [];
  const seen = new Set();
  for (const candidate of candidates) {
    const key = `${candidate.id}:${candidate.executablePath.toLowerCase()}`;
    if (!seen.has(key) && (await fileExists(candidate.executablePath))) {
      installed.push(candidate);
      seen.add(key);
    }
  }
  return installed;
}

async function installedUnsupportedBrowsers() {
  const candidates = browserCandidates().filter(
    (candidate) => !candidate.supported && browserCandidateAllowed(candidate)
  );
  const installed = [];
  for (const candidate of candidates) {
    if (candidate.appxPackageNamePattern !== undefined) {
      const packageInfo = await findWindowsAppPackage(candidate.appxPackageNamePattern);
      if (packageInfo != null) {
        installed.push({
          browser: {
            id: candidate.id,
            family: candidate.family,
            channel: candidate.channel,
            packageName: packageInfo.Name,
            packageFamilyName: packageInfo.PackageFamilyName,
            version: packageInfo.Version,
            installLocation: packageInfo.InstallLocation,
          },
          status: candidate.unsupportedStatus,
          reason: candidate.reason,
        });
      }
      continue;
    }
    if (await fileExists(candidate.executablePath)) {
      installed.push({
        browser: {
          id: candidate.id,
          family: candidate.family,
          channel: candidate.channel,
          executablePath: candidate.executablePath,
        },
        status: candidate.unsupportedStatus,
        reason: candidate.reason,
      });
    }
  }
  return installed;
}

function browserCandidates() {
  if (process.platform !== 'win32') {
    return [];
  }

  return [
    ...windowsRoots().flatMap((root) => [
      chromiumCandidate('edge-stable', 'edge', join(root, 'Microsoft', 'Edge', 'Application', 'msedge.exe')),
      chromiumCandidate('edge-beta', 'edge', join(root, 'Microsoft', 'Edge Beta', 'Application', 'msedge.exe')),
      chromiumCandidate('edge-dev', 'edge', join(root, 'Microsoft', 'Edge Dev', 'Application', 'msedge.exe')),
      chromiumCandidate('edge-canary', 'edge', join(root, 'Microsoft', 'Edge SxS', 'Application', 'msedge.exe')),
      chromiumCandidate('chrome-stable', 'chrome', join(root, 'Google', 'Chrome', 'Application', 'chrome.exe')),
      chromiumCandidate('chrome-beta', 'chrome', join(root, 'Google', 'Chrome Beta', 'Application', 'chrome.exe')),
      chromiumCandidate('chrome-dev', 'chrome', join(root, 'Google', 'Chrome Dev', 'Application', 'chrome.exe')),
      chromiumCandidate('chrome-canary', 'chrome', join(root, 'Google', 'Chrome SxS', 'Application', 'chrome.exe')),
      chromiumCandidate(
        'brave-stable',
        'brave',
        join(root, 'BraveSoftware', 'Brave-Browser', 'Application', 'brave.exe')
      ),
      chromiumCandidate('vivaldi-stable', 'vivaldi', join(root, 'Vivaldi', 'Application', 'vivaldi.exe')),
      chromiumCandidate('opera-stable', 'opera', join(root, 'Opera', 'opera.exe')),
      chromiumCandidate('chromium-stable', 'chromium', join(root, 'Chromium', 'Application', 'chrome.exe')),
      firefoxCandidate('firefox-stable', join(root, 'Mozilla Firefox', 'firefox.exe')),
      firefoxCandidate('firefox-developer', join(root, 'Firefox Developer Edition', 'firefox.exe')),
      firefoxCandidate('firefox-nightly', join(root, 'Firefox Nightly', 'firefox.exe')),
      unsupportedExecutableCandidate(
        'tor-browser',
        'tor-browser',
        join(root, 'Tor Browser', 'Browser', 'firefox.exe'),
        'unsupported-privacy-browser-without-managed-adapter'
      ),
      unsupportedExecutableCandidate(
        'safari-windows-legacy',
        'safari',
        join(root, 'Safari', 'Safari.exe'),
        'unsupported-legacy-windows-safari'
      ),
      unsupportedExecutableCandidate(
        'internet-explorer-legacy',
        'internet-explorer',
        join(root, 'Internet Explorer', 'iexplore.exe'),
        'unsupported-legacy-browser'
      ),
    ]),
    chromiumCandidate(
      'edge-local',
      'edge',
      join(process.env.LOCALAPPDATA ?? '', 'Microsoft', 'Edge', 'Application', 'msedge.exe')
    ),
    chromiumCandidate(
      'chrome-local',
      'chrome',
      join(process.env.LOCALAPPDATA ?? '', 'Google', 'Chrome', 'Application', 'chrome.exe')
    ),
    chromiumCandidate(
      'chrome-canary-local',
      'chrome',
      join(process.env.LOCALAPPDATA ?? '', 'Google', 'Chrome SxS', 'Application', 'chrome.exe')
    ),
    chromiumCandidate(
      'brave-local',
      'brave',
      join(process.env.LOCALAPPDATA ?? '', 'BraveSoftware', 'Brave-Browser', 'Application', 'brave.exe')
    ),
    chromiumCandidate(
      'vivaldi-local',
      'vivaldi',
      join(process.env.LOCALAPPDATA ?? '', 'Vivaldi', 'Application', 'vivaldi.exe')
    ),
    chromiumCandidate('opera-local', 'opera', join(process.env.LOCALAPPDATA ?? '', 'Programs', 'Opera', 'opera.exe')),
    chromiumCandidate(
      'opera-gx-local',
      'opera-gx',
      join(process.env.LOCALAPPDATA ?? '', 'Programs', 'Opera GX', 'opera.exe')
    ),
    firefoxCandidate('firefox-local', join(process.env.LOCALAPPDATA ?? '', 'Mozilla Firefox', 'firefox.exe')),
    firefoxCandidate(
      'firefox-developer-local',
      join(process.env.LOCALAPPDATA ?? '', 'Firefox Developer Edition', 'firefox.exe')
    ),
    firefoxCandidate('firefox-nightly-local', join(process.env.LOCALAPPDATA ?? '', 'Firefox Nightly', 'firefox.exe')),
    ...extraChromiumCandidates(),
    ...extraFirefoxCandidates(),
    unsupportedAppxCandidate(
      'duckduckgo-windows-appx',
      'duckduckgo',
      'DuckDuckGo*',
      'managed-shell-or-block-only-until-webview2-host-adapter-exists'
    ),
  ];
}

function browserCandidateAllowed(candidate) {
  if (browserFilters.length === 0) {
    return true;
  }
  return browserFilters.includes(candidate.id) || browserFilters.includes(candidate.family);
}

function chromiumCandidate(id, family, executablePath) {
  return {
    id,
    family,
    channel: channelFromPath(executablePath),
    executablePath,
    supported: true,
    bridge: 'chromium-devtools-protocol',
  };
}

function firefoxCandidate(id, executablePath) {
  return {
    id,
    family: 'firefox',
    channel: channelFromPath(executablePath),
    executablePath,
    supported: true,
    bridge: 'webdriver-bidi',
  };
}

function unsupportedExecutableCandidate(id, family, executablePath, reason) {
  return {
    id,
    family,
    channel: channelFromPath(executablePath),
    executablePath,
    supported: false,
    unsupportedStatus: 'installed-unsupported',
    reason,
  };
}

function unsupportedAppxCandidate(id, family, appxPackageNamePattern, reason) {
  return {
    id,
    family,
    channel: 'stable',
    appxPackageNamePattern,
    supported: false,
    unsupportedStatus: 'installed-unsupported',
    reason,
  };
}

function extraChromiumCandidates() {
  return envList('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_EXTRA_CHROMIUM_PATHS', []).map((executablePath, index) =>
    chromiumCandidate(`extra-chromium-${index + 1}`, 'extra-chromium', executablePath)
  );
}

function extraFirefoxCandidates() {
  return envList('OCENTRA_PARENT_MANAGED_BROWSER_MATRIX_EXTRA_FIREFOX_PATHS', []).map((executablePath, index) =>
    firefoxCandidate(`extra-firefox-${index + 1}`, executablePath)
  );
}

function windowsRoots() {
  return [process.env.ProgramFiles, process.env['ProgramFiles(x86)'], process.env.LOCALAPPDATA].filter(Boolean);
}

function channelFromPath(pathValue) {
  const normalized = pathValue.toLowerCase();
  if (normalized.includes('beta')) {
    return 'beta';
  }
  if (normalized.includes('dev')) {
    return 'dev';
  }
  if (normalized.includes('sxs') || normalized.includes('canary')) {
    return 'canary';
  }
  return 'stable';
}

async function fileExists(pathValue) {
  if (pathValue.length === 0) {
    return false;
  }
  try {
    return (await stat(pathValue)).isFile();
  } catch {
    return false;
  }
}

const appxPackageCache = new Map();

async function findWindowsAppPackage(namePattern) {
  if (process.platform !== 'win32') {
    return null;
  }
  if (appxPackageCache.has(namePattern)) {
    return appxPackageCache.get(namePattern);
  }
  const script = [
    "$pattern = [Environment]::GetEnvironmentVariable('OCENTRA_PARENT_APPX_PATTERN')",
    'Get-AppxPackage -Name $pattern | Select-Object -First 1 Name,PackageFamilyName,Version,InstallLocation | ConvertTo-Json -Compress',
  ].join('; ');
  const output = await collectProcessOutput(
    'powershell.exe',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script],
    {
      ...process.env,
      OCENTRA_PARENT_APPX_PATTERN: namePattern,
    }
  );
  const packageInfo = parsePackageInfo(output);
  appxPackageCache.set(namePattern, packageInfo);
  return packageInfo;
}

function parsePackageInfo(output) {
  const trimmed = output.trim();
  if (trimmed.length === 0) {
    return null;
  }
  try {
    const parsed = JSON.parse(trimmed);
    return typeof parsed === 'object' && parsed !== undefined ? parsed : null;
  } catch {
    return null;
  }
}

function collectProcessOutput(command, args, env) {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      env,
      windowsHide: true,
    });
    let stdout = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.once('exit', () => resolve(stdout));
    child.once('error', () => resolve(''));
  });
}

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  await new Promise((resolve) => server.close(resolve));
  return address.port;
}

async function stopWindowsProcessesByCommandLineFragment(fragment) {
  if (process.platform !== 'win32') {
    return;
  }
  const script = [
    "$fragment = [Environment]::GetEnvironmentVariable('OCENTRA_PARENT_STOP_FRAGMENT')",
    'Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($fragment) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }',
  ].join('; ');
  await new Promise((resolve) => {
    const child = spawn('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
      env: { ...process.env, OCENTRA_PARENT_STOP_FRAGMENT: fragment },
      stdio: 'ignore',
      windowsHide: true,
    });
    child.once('exit', resolve);
  });
}

function summarize(browserResults, unsupported) {
  const failures = [];
  const profileCount = browserResults.reduce((count, browser) => count + browser.profiles.length, 0);
  const capturedUrls = browserResults.reduce(
    (count, browser) =>
      count +
      browser.profiles.reduce(
        (profileCountForBrowser, profile) =>
          profileCountForBrowser + profile.siteEvidence.filter((site) => site.captured).length,
        0
      ),
    0
  );
  const activeProofCount = browserResults.reduce(
    (count, browser) => count + browser.profiles.filter((profile) => profile.assertions.canProveActiveTab).length,
    0
  );
  const historyProofCount = browserResults.reduce(
    (count, browser) => count + browser.profiles.filter((profile) => profile.assertions.canJournalVisitedUrls).length,
    0
  );
  for (const browser of browserResults) {
    for (const profile of browser.profiles) {
      if (!profile.assertions.canConnectManagedProfile) {
        failures.push(`${browser.browser.id}/${profile.profileName}: managed profile did not connect`);
      }
      if (!profile.assertions.canSeeTabs) {
        failures.push(`${browser.browser.id}/${profile.profileName}: expected ${urls.length} page targets`);
      }
      if (!profile.assertions.canSeeUrls) {
        failures.push(`${browser.browser.id}/${profile.profileName}: one or more tab URLs were not captured`);
      }
      if (!profile.assertions.canProveActiveTab) {
        failures.push(`${browser.browser.id}/${profile.profileName}: active tab proof failed`);
      }
      if (!profile.assertions.canJournalVisitedUrls) {
        failures.push(`${browser.browser.id}/${profile.profileName}: visited URL journal proof failed`);
      }
    }
  }

  return {
    supportedBrowserCount: browserResults.length,
    unsupportedInstalledBrowserCount: unsupported.length,
    managedProfileCount: profileCount,
    requestedUrlCount: urls.length,
    capturedUrlCount: capturedUrls,
    activeProofCount,
    historyProofCount,
    failures,
  };
}

function printSummary(evidence, evidencePath) {
  console.log(`managed-browser-profile-matrix-ok=${evidence.summary.failures.length === 0}`);
  console.log(`evidence=${evidencePath}`);
  console.log(
    `supportedBrowsers=${evidence.summary.supportedBrowserCount} managedProfiles=${evidence.summary.managedProfileCount} capturedUrls=${evidence.summary.capturedUrlCount} activeProofs=${evidence.summary.activeProofCount} historyProofs=${evidence.summary.historyProofCount}`
  );
  for (const browser of evidence.supportedBrowsers) {
    console.log(`${browser.browser.id} ${browser.browser.bridge} ${browser.browser.executablePath}`);
    for (const profile of browser.profiles) {
      const urlsForProfile = profile.siteEvidence
        .map((site) => `${site.requestedHost}=>${site.observedUrl}`)
        .join(' | ');
      console.log(
        `  ${profile.profileName}: tabs=${profile.pageTargetCount} active=${profile.assertions.activeTabProof} history=${profile.assertions.visitedHistoryProof} urls=${urlsForProfile}`
      );
    }
  }
  for (const item of evidence.unsupportedInstalledBrowsers) {
    const locator =
      item.browser.executablePath ??
      item.browser.packageFamilyName ??
      item.browser.packageName ??
      item.browser.installLocation ??
      'unknown';
    console.log(`unsupported=${item.browser.family} locator=${locator} status=${item.status} reason=${item.reason}`);
  }
  for (const failure of evidence.summary.failures) {
    console.error(`failure=${failure}`);
  }
}

function envList(name, fallback) {
  const value = process.env[name];
  if (value === undefined || value.trim().length === 0) {
    return fallback;
  }
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

class DevToolsClient {
  static async connect(url) {
    const webSocket = await connectWebSocket(url, timeoutMs);
    return new DevToolsClient(url, webSocket);
  }

  constructor(url, webSocket) {
    this.url = url;
    this.webSocket = webSocket;
    this.commandId = 0;
    this.pending = new Map();
    this.eventHandlers = [];
    this.webSocket.addEventListener('message', (message) => this.handleMessage(message));
  }

  command(method, params) {
    const id = ++this.commandId;
    const payload = { id, method, params };
    const promise = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      setTimeout(() => {
        if (this.pending.delete(id)) {
          reject(new Error(`Timed out waiting for ${method}`));
        }
      }, commandTimeoutMs).unref();
    });
    this.webSocket.send(JSON.stringify(payload));
    return promise;
  }

  onEvent(handler) {
    this.eventHandlers.push(handler);
  }

  close() {
    this.webSocket.close();
  }

  handleMessage(message) {
    const data = JSON.parse(message.data);
    if (data.id !== undefined) {
      const pending = this.pending.get(data.id);
      if (pending === undefined) {
        return;
      }
      this.pending.delete(data.id);
      if (data.error !== undefined) {
        pending.reject(new Error(JSON.stringify(data.error)));
      } else {
        pending.resolve(data.result ?? {});
      }
      return;
    }
    for (const handler of this.eventHandlers) {
      handler(data);
    }
  }
}

class FirefoxBidiClient {
  static async connect(url, timeout) {
    const webSocket = await connectWebSocket(url, timeout);
    return new FirefoxBidiClient(webSocket);
  }

  constructor(webSocket) {
    this.webSocket = webSocket;
    this.commandId = 0;
    this.pending = new Map();
    this.eventHandlers = [];
    this.webSocket.addEventListener('message', (message) => this.handleMessage(message));
  }

  command(method, params) {
    const id = ++this.commandId;
    const payload = { id, method, params };
    const promise = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      setTimeout(() => {
        if (this.pending.delete(id)) {
          reject(new Error(`Timed out waiting for ${method}`));
        }
      }, commandTimeoutMs).unref();
    });
    this.webSocket.send(JSON.stringify(payload));
    return promise;
  }

  onEvent(handler) {
    this.eventHandlers.push(handler);
  }

  close() {
    this.webSocket.close();
  }

  handleMessage(message) {
    const data = JSON.parse(message.data);
    if (data.type === 'event') {
      for (const handler of this.eventHandlers) {
        handler(data);
      }
      return;
    }
    const pending = this.pending.get(data.id);
    if (pending === undefined) {
      return;
    }
    this.pending.delete(data.id);
    if (data.type === 'success') {
      pending.resolve(data.result ?? {});
    } else {
      pending.reject(new Error(JSON.stringify(data)));
    }
  }
}

async function connectWebSocket(url, timeout) {
  const deadline = Date.now() + timeout;
  let lastError;
  while (Date.now() < deadline) {
    try {
      return await openWebSocket(url);
    } catch (error) {
      lastError = error;
      await delay(250);
    }
  }
  throw lastError;
}

function openWebSocket(url) {
  return new Promise((resolve, reject) => {
    const webSocket = new WebSocket(url);
    const timer = setTimeout(() => {
      webSocket.close();
      reject(new Error(`Timed out connecting to ${url}`));
    }, 3000);
    webSocket.addEventListener(
      'open',
      () => {
        clearTimeout(timer);
        resolve(webSocket);
      },
      { once: true }
    );
    webSocket.addEventListener(
      'error',
      () => {
        clearTimeout(timer);
        reject(new Error(`WebSocket error for ${url}`));
      },
      { once: true }
    );
  });
}

await main();
