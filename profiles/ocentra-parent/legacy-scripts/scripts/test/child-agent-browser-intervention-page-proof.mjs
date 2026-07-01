import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';

import {
  BrowserChildInterventionPageDefaults,
  renderBrowserChildInterventionPage,
} from '@ocentra-parent/portal-domain/browser-child-intervention-page';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const runId = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
const evidenceDirectory = join(repoRoot, 'test-results', 'child-agent-browser-intervention-page-proof');
const timeoutMs = envNumber('OCENTRA_PARENT_CHILD_AGENT_INTERVENTION_PAGE_TIMEOUT_MS', 30_000);
const requestedUrl =
  process.env.OCENTRA_PARENT_CHILD_AGENT_INTERVENTION_PAGE_URL ?? 'https://www.youtube.com/watch?v=XzUB8_gj6xM';

await main();

async function main() {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  const runRoot = await mkdtemp(join(tmpdir(), 'ocentra-parent-child-agent-intervention-page-'));
  const htmlPath = join(runRoot, 'browser-intervention-page.html');
  await writeFile(htmlPath, renderProofHtml(requestedUrl), 'utf8');

  const agentPort = await freePort();
  const service = spawnAgentService(runRoot, agentPort, htmlPath);
  const serviceOutput = collectOutput(service);
  try {
    await waitForHealth(agentPort, serviceOutput);
    const servedUrl = `http://127.0.0.1:${agentPort}/api/browser/intervention/page?target=${encodeURIComponent(
      requestedUrl
    )}`;
    const response = await fetch(servedUrl);
    const html = await response.text();
    const evidence = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      requestedUrl,
      servedUrl,
      htmlPath: relative(repoRoot, htmlPath),
      status: response.status,
      contentType: response.headers.get('content-type'),
      cacheControl: response.headers.get('cache-control'),
      bodyLength: html.length,
      assertions: {
        agentHealthy: true,
        blockMarkerPresent: html.includes(BrowserChildInterventionPageDefaults.BlockMarker),
        cacheDisabled: response.headers.get('cache-control') === 'no-store',
        htmlServedByChildAgent: response.ok,
        targetUrlPresent: html.includes(requestedUrl),
      },
    };
    const evidencePath = join(evidenceDirectory, `${runId}.json`);
    await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, 'utf8');
    printSummary(evidence, evidencePath);
    if (!Object.values(evidence.assertions).every(Boolean)) {
      process.exitCode = 1;
    }
  } finally {
    await stopProcessTreeAndWait(service);
  }
}

function renderProofHtml(url) {
  return renderBrowserChildInterventionPage({
    action: 'block',
    backdrop: {
      imageUrl: proofBackdropDataUrl(url),
      label: 'Proof target before block',
    },
    blockMarker: BrowserChildInterventionPageDefaults.BlockMarker,
    bridge: 'child-agent-served-browser-intervention-page',
    deliveryState: 'child-agent-page-rendered',
    outcome: 'blocked',
    parentRequestEnabled: true,
    reason: 'Your family rule blocks this exact video URL.',
    requestedUrl: url,
    ruleId: 'blocked-youtube-video-url',
    ruleLabel: 'Disallowed YouTube video URL',
    ruleMarker: BrowserChildInterventionPageDefaults.BlockMarker,
    targetType: 'video',
    theme: 'dark',
  });
}

function proofBackdropDataUrl(url) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720" viewBox="0 0 1280 720"><rect width="1280" height="720" fill="#111827"/><rect x="88" y="74" width="1104" height="572" rx="20" fill="#0f172a" stroke="#334155" stroke-width="3"/><rect x="126" y="128" width="720" height="390" rx="18" fill="#020617"/><circle cx="486" cy="323" r="72" fill="#ef4444"/><polygon points="468,278 468,368 548,323" fill="#fff"/><rect x="880" y="128" width="270" height="34" rx="8" fill="#1f2937"/><rect x="880" y="186" width="236" height="22" rx="6" fill="#1f2937"/><rect x="880" y="228" width="198" height="22" rx="6" fill="#1f2937"/><text x="126" y="570" fill="#e5e7eb" font-family="Arial, sans-serif" font-size="28">${escapeSvg(
    url
  )}</text></svg>`;
  return `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`;
}

function printSummary(evidence, evidencePath) {
  const ok = Object.values(evidence.assertions).every(Boolean);
  console.log(`child-agent-browser-intervention-page-proof-ok=${ok}`);
  console.log(`evidence=${evidencePath}`);
  console.log(`served=${evidence.servedUrl}`);
  for (const [name, passed] of Object.entries(evidence.assertions)) {
    console.log(`assertion.${name}=${passed}`);
  }
}

function spawnAgentService(runRoot, agentPort, htmlPath) {
  return spawn(resolveDebugAgentServicePath(), [], {
    cwd: repoRoot,
    env: {
      ...process.env,
      OCENTRA_PARENT_ACTIVITY_DB_PATH: join(runRoot, 'activity.sqlite'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: join(runRoot, 'activity.key'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: join(runRoot, 'activity.ndjson'),
      OCENTRA_PARENT_AGENT_ADDR: `127.0.0.1:${agentPort}`,
      OCENTRA_PARENT_AGENT_ENFORCEMENT_TIMER_STATE_PATH: join(runRoot, 'enforcement-timers.json'),
      OCENTRA_PARENT_DEV_LOG_DIR: join(runRoot, 'logs'),
      OCENTRA_PARENT_MANAGED_BROWSER_INTERVENTION_HTML_PATH: htmlPath,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
}

async function waitForHealth(agentPort, serviceOutput) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${agentPort}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for service health. ${serviceOutput()}`);
}

async function runCommand(command, args) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`))));
    child.once('error', reject);
  });
}

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function escapeSvg(value) {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}
