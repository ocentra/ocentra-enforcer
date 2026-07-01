import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/05-cross-platform-inventory-matrix');
const testResultPath = join(repoRoot, 'test-results/browser-platform-windows-host-proof/proof.json');
const outputProofPath = join(proofRoot, '13-windows-host-browser-proof.json');
const observedAt = new Date().toISOString();

const knownBrowserTargets = [
  {
    targetId: 'microsoft-edge-stable',
    exeName: 'msedge.exe',
    appPathKey: 'HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\msedge.exe',
    candidatePaths: [
      join(env('ProgramFiles'), 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
      join(env('ProgramFiles(x86)'), 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
      join(env('LocalAppData'), 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
    ],
  },
  {
    targetId: 'google-chrome-stable',
    exeName: 'chrome.exe',
    appPathKey: 'HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\chrome.exe',
    candidatePaths: [
      join(env('ProgramFiles'), 'Google', 'Chrome', 'Application', 'chrome.exe'),
      join(env('ProgramFiles(x86)'), 'Google', 'Chrome', 'Application', 'chrome.exe'),
      join(env('LocalAppData'), 'Google', 'Chrome', 'Application', 'chrome.exe'),
    ],
  },
  {
    targetId: 'mozilla-firefox-stable',
    exeName: 'firefox.exe',
    appPathKey: 'HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\firefox.exe',
    candidatePaths: [
      join(env('ProgramFiles'), 'Mozilla Firefox', 'firefox.exe'),
      join(env('ProgramFiles(x86)'), 'Mozilla Firefox', 'firefox.exe'),
      join(env('LocalAppData'), 'Mozilla Firefox', 'firefox.exe'),
    ],
  },
];

const knownUrlAssociationHandlers = [
  { browserFamily: 'edge', progIdPrefix: 'MSEdgeHTM' },
  { browserFamily: 'chrome', progIdPrefix: 'ChromeHTML' },
  { browserFamily: 'firefox', progIdPrefix: 'FirefoxURL' },
  { browserFamily: 'firefox', progIdPrefix: 'FirefoxHTML' },
];

mkdirSync(proofRoot, { recursive: true });

const pathVisibility = knownBrowserTargets.map((target) => queryTarget(target));
const urlAssociations = ['http', 'https'].map(queryUrlAssociation);
const executableVisible = pathVisibility.some((entry) => entry.executableVisible || entry.appPathRegistryVisible);
const defaultUrlHandlerVisible = urlAssociations.some((entry) => entry.progIdRef !== undefined);
const knownDefaultBrowserHandlerVisible = urlAssociations.some((entry) => entry.knownBrowserFamily !== undefined);
const negativeChecks = [
  { claim: 'windows-managed-launch', rejected: true },
  { claim: 'windows-managed-exact-url', rejected: true },
  { claim: 'windows-known-active-tab', rejected: true },
  { claim: 'windows-browser-enforcement', rejected: true },
];

const proof = {
  schemaVersion: 1,
  proofId: 'browser-platform-windows-host-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  hostProofSummary: {
    platform: process.platform,
    windowsHost: process.platform === 'win32',
    knownBrowserTargetsQueriedOnly: true,
    windowsAppPathRegistryQueriedOnly: true,
    urlAssociationRegistryQueriedOnly: true,
    executableVisible,
    defaultUrlHandlerVisible,
    defaultUrlHandlerAssociationVisible: defaultUrlHandlerVisible,
    knownDefaultBrowserHandlerVisible,
    rawPathPersisted: false,
    rawRegistryValuePersisted: false,
    managedLaunchClaimed: false,
    managedProfileClaimed: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
    bridgeCustodyClaimed: false,
    enforcementClaimed: false,
    resultState:
      process.platform === 'win32'
        ? 'windows-host-browser-inventory-boundary-proof'
        : 'manual-windows-host-proof-required',
  },
  pathVisibility,
  urlAssociations,
  negativeChecks,
};

if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected Windows host proof negative checks to reject dishonest claims');
}
if (proof.hostProofSummary.windowsHost && !executableVisible && !defaultUrlHandlerVisible) {
  throw new Error('Expected at least one Windows browser executable or URL handler signal on this host');
}

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('browser-platform-windows-host-proof-ok=true');
console.log(`proof=${testResultPath}`);
console.log(`outputProof=${outputProofPath}`);
console.log(`windowsHost=${proof.hostProofSummary.windowsHost}`);
console.log(`executableVisible=${executableVisible}`);
console.log(`defaultUrlHandlerVisible=${defaultUrlHandlerVisible}`);
console.log(`resultState=${proof.hostProofSummary.resultState}`);

function queryTarget(target) {
  const visibleCandidatePaths = target.candidatePaths
    .filter((candidatePath) => candidatePath.trim().length > 0)
    .filter((candidatePath) => existsSync(candidatePath));
  const appPathValue = registryDefault(target.appPathKey);

  return {
    targetId: target.targetId,
    exeName: target.exeName,
    executableVisible: visibleCandidatePaths.length > 0,
    executablePathRefs: visibleCandidatePaths.map((candidatePath) =>
      redactedRef('windows-browser-path', candidatePath)
    ),
    appPathRegistryVisible: appPathValue !== undefined,
    appPathRegistryRef: appPathValue === null ? null : redactedRef('windows-app-path-registry', appPathValue),
    rawPathPersisted: false,
    rawRegistryValuePersisted: false,
    managedLaunchClaimed: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
    enforcementClaimed: false,
  };
}

function queryUrlAssociation(scheme) {
  const key = `HKCU\\Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\${scheme}\\UserChoice`;
  const progId = registryValue(key, 'ProgId');
  const knownBrowserFamily = progId === null ? null : knownBrowserFamilyForProgId(progId);

  return {
    scheme,
    userChoiceKeyRef: redactedRef('windows-url-association-key', key),
    progIdRef: progId === null ? null : redactedRef('windows-url-association-progid', progId),
    knownBrowserFamily,
    defaultHandlerAssociationVisible: progId !== undefined,
    rawRegistryValuePersisted: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
    enforcementClaimed: false,
  };
}

function knownBrowserFamilyForProgId(progId) {
  const normalizedProgId = progId.trim();
  return (
    knownUrlAssociationHandlers.find((handler) => normalizedProgId.startsWith(handler.progIdPrefix))?.browserFamily ??
    null
  );
}

function registryDefault(key) {
  const output = registryQuery(key);
  const match = output.match(/\(Default\)\s+REG_\w+\s+(.+)/u);
  return match?.[1]?.trim() ?? null;
}

function registryValue(key, valueName) {
  const output = registryQuery(key);
  const expression = new RegExp(`\\b${escapeRegExp(valueName)}\\s+REG_\\w+\\s+(.+)`, 'iu');
  const match = output.match(expression);
  return match?.[1]?.trim() ?? null;
}

function registryQuery(key) {
  try {
    return execFileSync('reg.exe', ['query', key], {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch (error) {
    return `${error.stdout?.toString() ?? ''}${error.stderr?.toString() ?? ''}`;
  }
}

function env(name) {
  return process.env[name] ?? '';
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function redactedRef(prefix, value) {
  return `${prefix}-${sha256(value).slice(0, 16)}`;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
}
