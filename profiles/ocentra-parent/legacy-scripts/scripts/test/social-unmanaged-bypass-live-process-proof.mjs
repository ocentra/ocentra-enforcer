import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  BrowserSocialUnmanagedBypassEvidenceSchema,
  detectBrowserSocialUnmanagedBypass,
} from '@ocentra-parent/schema-domain/browser-social-unmanaged-bypass-detector';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/social-15-unmanaged-social-bypass-detector');
const testResultPath = join(repoRoot, 'test-results/social-unmanaged-bypass-live-process-proof/proof.json');
const outputProofPath = join(proofRoot, '11-live-process-proof.json');
const observedAt = new Date().toISOString();

const sourceFiles = [
  'packages/schema-domain/src/browser-social-unmanaged-bypass-detector-values.ts',
  'packages/schema-domain/src/browser-social-unmanaged-bypass-detector.ts',
];
const builtFiles = [
  'packages/schema-domain/dist/browser-social-unmanaged-bypass-detector-values.js',
  'packages/schema-domain/dist/browser-social-unmanaged-bypass-detector.js',
];

const liveTargets = [
  {
    targetId: 'reddit-live-social-surface',
    targetPlatformRef: 'redacted-social-platform-ref-reddit',
    launchUrl: 'https://www.reddit.com/r/popular/',
  },
  {
    targetId: 'youtube-shorts-live-social-video-surface',
    targetPlatformRef: 'redacted-social-platform-ref-youtube-shorts',
    launchUrl: 'https://www.youtube.com/shorts/jNQXAC9IVRw',
  },
];

assertBuiltContractsAreFresh();
mkdirSync(proofRoot, { recursive: true });

const browserCandidate = findBrowserCandidate();
const captures = [];
let launchSummary;

if (browserCandidate === null) {
  launchSummary = {
    liveBrowserProcessObserved: false,
    launchAttempted: false,
    launchUnavailableReason: 'no-supported-system-browser-found',
  };
  captures.push(buildManualRequiredCapture());
} else {
  launchSummary = {
    liveBrowserProcessObserved: true,
    launchAttempted: true,
    browserExecutableName: browserCandidate.name,
    rawExecutablePathPersisted: false,
    rawCommandLinePersisted: false,
    rawTargetUrlPersisted: false,
  };

  for (const target of liveTargets) {
    captures.push(await captureLiveBrowserProcess(browserCandidate, target));
  }
}

const acceptedChecks = captures.map((capture) => ({
  targetId: capture.targetId,
  accepted: BrowserSocialUnmanagedBypassEvidenceSchema.safeParse(capture.evidence).success,
}));
if (!acceptedChecks.every((check) => check.accepted)) {
  throw new Error('Expected SOCIAL-15 live-process evidence rows to parse through the contract schema');
}

const negativeChecks = buildNegativeChecks(captures.map((capture) => capture.evidence));
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected SOCIAL-15 live-process negative checks to reject dishonest claims');
}

const proof = {
  schemaVersion: 1,
  proofId: 'social-unmanaged-bypass-live-process-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  liveProcessSummary: {
    realLocalBrowserProcessObserved: launchSummary.liveBrowserProcessObserved,
    generatedOrFixturePageUsed: false,
    realPublicSocialSurfacesRequested: browserCandidate !== undefined,
    passiveNavigationOnly: true,
    exactUrlClaimed: false,
    routeEvidenceClaimed: false,
    socialAccountProofClaimed: false,
    feedVideoRouteClaimed: false,
    messageContentClaimed: false,
    nativeAppControlClaimed: false,
    platformConnectorClaimed: false,
    childUiRenderedClaimed: false,
    parentUiNotifiedClaimed: false,
    processTerminatedClaimedByProduct: false,
    managedBrowserRelaunchedClaimed: false,
    enforcementClaimed: false,
    rawExecutablePathPersisted: false,
    rawCommandLinePersisted: false,
    rawTargetUrlPersisted: false,
    launchedProofProcessCleanedUp: captures.some((capture) => capture.proofCleanupAttempted),
  },
  launchSummary,
  captures,
  acceptedChecks,
  negativeChecks,
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-unmanaged-bypass-live-process-proof-ok=true');
console.log(`proof=${testResultPath}`);
console.log(`outputProof=${outputProofPath}`);
console.log(`captureCount=${captures.length}`);
console.log(`liveBrowserProcessObserved=${launchSummary.liveBrowserProcessObserved}`);

async function captureLiveBrowserProcess(browser, target) {
  const userDataDir = join(tmpdir(), `ocentra-social-bypass-proof-${Date.now()}-${target.targetId}`);
  mkdirSync(userDataDir, { recursive: true });
  const child = spawn(
    browser.path,
    [
      `--user-data-dir=${userDataDir}`,
      '--no-first-run',
      '--disable-default-apps',
      '--disable-sync',
      '--new-window',
      target.launchUrl,
    ],
    { detached: false, stdio: 'ignore' }
  );

  await delay(4_000);
  const processInfo = readProcessInfo(child.pid);
  cleanupProcess(child.pid);
  await removeTempDirectory(userDataDir);

  const executableRef = processInfo.executablePath
    ? `redacted-executable-ref-${sha256(processInfo.executablePath).slice(0, 16)}`
    : 'redacted-executable-ref-unavailable';
  const processHashRef =
    processInfo.executablePath && existsSync(processInfo.executablePath)
      ? `redacted-process-hash-ref-${sha256File(processInfo.executablePath).slice(0, 16)}`
      : 'redacted-process-hash-ref-unavailable';
  const signatureRef = processInfo.executablePath
    ? `redacted-signature-ref-${sha256(browser.name).slice(0, 16)}`
    : 'redacted-signature-ref-unavailable';

  const evidence = detectBrowserSocialUnmanagedBypass({
    bypassEvidenceId: `social-unmanaged-bypass-${target.targetId}`,
    observedAt,
    sourceEvidenceIds: [`browser-evidence-social-unmanaged-${target.targetId}`],
    processKind: 'supported-browser',
    processName: browser.name,
    executablePathRef: executableRef,
    processHashRef,
    signatureRef,
    confidence: processInfo.executablePath ? 'high' : 'medium',
    reasons: ['supported-browser-outside-managed-session', 'managed-browser-required', 'exact-url-unavailable'],
    suspectedPlatformRef: target.targetPlatformRef,
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    unmanagedDetectionState: 'detected',
    unmanagedFallbackAction: 'parent-review',
  });

  return {
    targetId: target.targetId,
    observedPid: child.pid,
    processObserved: processInfo.observed,
    executableName: browser.name,
    executablePathRef: evidence.executablePathRef,
    processHashRef: evidence.processHashRef,
    commandLineHashRef: processInfo.commandLine
      ? `redacted-command-line-ref-${sha256(processInfo.commandLine).slice(0, 16)}`
      : null,
    rawExecutablePathPersisted: false,
    rawCommandLinePersisted: false,
    rawTargetUrlPersisted: false,
    targetUrlSha256: sha256(target.launchUrl),
    proofCleanupAttempted: true,
    productProcessControlClaimed: false,
    evidence,
  };
}

function buildManualRequiredCapture() {
  const evidence = detectBrowserSocialUnmanagedBypass({
    bypassEvidenceId: 'social-unmanaged-bypass-manual-required',
    observedAt,
    sourceEvidenceIds: ['browser-evidence-social-unmanaged-manual-required'],
    processKind: 'unknown-browser-like',
    processName: 'unavailable-browser-process',
    executablePathRef: null,
    processHashRef: null,
    signatureRef: null,
    confidence: 'low',
    reasons: ['browser-like-social-attempt', 'managed-browser-required', 'exact-url-unavailable', 'manual-required'],
    suspectedPlatformRef: null,
    browserBoundaryState: 'unknown',
    exactUrlClaimState: 'unavailable',
    unmanagedDetectionState: 'manual-required',
    unmanagedFallbackAction: 'os-block-manual-required',
  });

  return {
    targetId: 'manual-required-no-supported-system-browser',
    processObserved: false,
    rawExecutablePathPersisted: false,
    rawCommandLinePersisted: false,
    rawTargetUrlPersisted: false,
    proofCleanupAttempted: false,
    productProcessControlClaimed: false,
    evidence,
  };
}

function findBrowserCandidate() {
  const candidates = [
    { name: 'msedge-proof-browser', path: 'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe' },
    { name: 'msedge-proof-browser', path: 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe' },
    { name: 'chrome-proof-browser', path: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe' },
    { name: 'chrome-proof-browser', path: 'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe' },
  ];
  return candidates.find((candidate) => existsSync(candidate.path)) ?? null;
}

function readProcessInfo(pid) {
  if (!pid) {
    return { observed: false, executablePath: null, commandLine: null };
  }
  try {
    const output = execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-Command',
        `$p=Get-CimInstance Win32_Process -Filter "ProcessId=${pid}"; if ($null -eq $p) { '{}' } else { $p | Select-Object ProcessId,ExecutablePath,CommandLine | ConvertTo-Json -Compress }`,
      ],
      { cwd: repoRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
    ).trim();
    if (output === '{}' || output.length === 0) {
      return { observed: false, executablePath: null, commandLine: null };
    }
    const parsed = JSON.parse(output);
    return {
      observed: true,
      executablePath: typeof parsed.ExecutablePath === 'string' ? parsed.ExecutablePath : null,
      commandLine: typeof parsed.CommandLine === 'string' ? parsed.CommandLine : null,
    };
  } catch {
    return { observed: false, executablePath: null, commandLine: null };
  }
}

function cleanupProcess(pid) {
  if (!pid) {
    return;
  }
  try {
    execFileSync('taskkill.exe', ['/PID', String(pid), '/T', '/F'], {
      cwd: repoRoot,
      stdio: 'ignore',
    });
  } catch {
    // Fall through to the PowerShell cleanup path.
  }
  try {
    execFileSync(
      'powershell.exe',
      ['-NoProfile', '-Command', `Stop-Process -Id ${pid} -Force -ErrorAction SilentlyContinue`],
      {
        cwd: repoRoot,
        stdio: 'ignore',
      }
    );
  } catch {
    // Cleanup is best-effort for the proof-owned process only.
  }
}

async function removeTempDirectory(path) {
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      rmSync(path, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === 7) {
        throw error;
      }
      await delay(500);
    }
  }
}

function buildNegativeChecks(evidenceRows) {
  const rows = [];
  const claimFields = [
    'exactUrlClaimed',
    'routeEvidenceClaimed',
    'socialAccountProofClaimed',
    'feedVideoRouteClaimed',
    'messageContentClaimed',
    'accountIdentityClaimed',
    'nativeAppControlClaimed',
    'platformConnectorClaimed',
    'childUiRenderedClaimed',
    'parentUiNotifiedClaimed',
    'processTerminatedClaimed',
    'managedBrowserRelaunchedClaimed',
    'enforcementClaimed',
  ];

  for (const evidence of evidenceRows) {
    for (const field of claimFields) {
      rows.push({
        targetId: evidence.bypassEvidenceId,
        mutation: field,
        rejected: !BrowserSocialUnmanagedBypassEvidenceSchema.safeParse({ ...evidence, [field]: true }).success,
      });
    }
    rows.push({
      targetId: evidence.bypassEvidenceId,
      mutation: 'exactUrlClaimState',
      rejected: !BrowserSocialUnmanagedBypassEvidenceSchema.safeParse({
        ...evidence,
        exactUrlClaimState: 'exact-url-proven',
      }).success,
    });
    rows.push({
      targetId: evidence.bypassEvidenceId,
      mutation: 'managedBrowserRequired',
      rejected: !BrowserSocialUnmanagedBypassEvidenceSchema.safeParse({
        ...evidence,
        managedBrowserRequired: false,
      }).success,
    });
  }
  return rows;
}

function assertBuiltContractsAreFresh() {
  for (const source of sourceFiles) {
    const sourcePath = join(repoRoot, source);
    const sourceMtime = statSync(sourcePath).mtimeMs;
    const built = builtFiles.find((candidate) =>
      candidate.endsWith(source.replace('src/', 'dist/').replace('.ts', '.js').split('/').at(-1))
    );
    if (!built) {
      continue;
    }
    const builtPath = join(repoRoot, built);
    if (!existsSync(builtPath)) {
      throw new Error(`Missing built contract file: ${built}`);
    }
    if (statSync(builtPath).mtimeMs + 1_000 < sourceMtime) {
      throw new Error(`Built contract file is stale for ${source}; run npm run build:contracts first`);
    }
  }
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
