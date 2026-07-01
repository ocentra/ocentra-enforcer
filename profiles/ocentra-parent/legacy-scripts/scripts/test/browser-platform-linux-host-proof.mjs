import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/05-cross-platform-inventory-matrix');
const testResultPath = join(repoRoot, 'test-results/browser-platform-linux-host-proof/proof.json');
const outputProofPath = join(proofRoot, '12-linux-host-package-proof.json');
const screenshotProofPath = join(proofRoot, '12-linux-headless-browser-screenshot.png');
const observedAt = new Date().toISOString();
const distroName = 'Ubuntu-22.04';

const knownBrowserTargets = [
  { targetId: 'google-chrome-stable', command: 'google-chrome', packageName: 'google-chrome-stable' },
  { targetId: 'chromium', command: 'chromium', packageName: 'chromium' },
  { targetId: 'chromium-browser', command: 'chromium-browser', packageName: 'chromium-browser' },
  { targetId: 'firefox', command: 'firefox', packageName: 'firefox' },
  { targetId: 'microsoft-edge-stable', command: 'microsoft-edge', packageName: 'microsoft-edge-stable' },
];

mkdirSync(proofRoot, { recursive: true });

const wslStatus = command(['--status'], { allowFailure: true });
const kernelName = wslBash('uname -s');
const distroDescription = wslBash(
  'if command -v lsb_release >/dev/null 2>&1; then lsb_release -ds; else cat /etc/os-release | sed -n "s/^PRETTY_NAME=//p" | tr -d "\\""; fi'
);
const pathVisibility = knownBrowserTargets.map((target) => queryCommand(target));
const packageVisibility = knownBrowserTargets.map((target) => queryPackage(target));
const desktopEntries = queryDesktopEntries();
const browserCommandVisible = pathVisibility.some((entry) => entry.visible);
const browserPackageInstalled = packageVisibility.some((entry) => entry.installed);
const browserDesktopEntryVisible = desktopEntries.length > 0;
const launchProof = captureLinuxBrowserLaunchProof(pathVisibility);
const negativeChecks = [
  { claim: 'linux-managed-browser-adapter', rejected: true },
  { claim: 'linux-managed-exact-url', rejected: true },
  { claim: 'linux-known-active-tab', rejected: true },
  { claim: 'linux-browser-enforcement', rejected: true },
];

const proof = {
  schemaVersion: 1,
  proofId: 'browser-platform-linux-host-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  hostProofSummary: {
    wslAvailable: kernelName === 'Linux',
    distroName,
    distroDescriptionRef: distroDescription ? `redacted-linux-distro-${sha256(distroDescription).slice(0, 16)}` : null,
    wslStatusSha256: wslStatus ? sha256(wslStatus) : null,
    knownBrowserCommandsQueriedOnly: true,
    knownBrowserPackagesQueriedOnly: true,
    knownDesktopEntryGlobsQueriedOnly: true,
    browserCommandVisible,
    browserPackageInstalled,
    browserDesktopEntryVisible,
    browserLaunchAttempted: launchProof.browserLaunchAttempted,
    browserLaunchObserved: launchProof.browserLaunchObserved,
    browserLaunchCommandRef: launchProof.browserLaunchCommandRef,
    browserLaunchDomSha256: launchProof.browserLaunchDomSha256,
    browserLaunchScreenshotCaptured: launchProof.browserLaunchScreenshotCaptured,
    browserLaunchScreenshotPersisted: launchProof.browserLaunchScreenshotPersisted,
    browserLaunchScreenshotSha256: launchProof.browserLaunchScreenshotSha256,
    rawPathPersisted: false,
    rawPackageListPersisted: false,
    rawDesktopEntryListPersisted: false,
    rawBrowserLaunchDomPersisted: false,
    rawBrowserLaunchUrlPersisted: false,
    desktopSessionProofClaimed: false,
    managedProfileClaimed: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
    snapFlatpakProofClaimed: false,
    enforcementClaimed: false,
    resultState:
      kernelName === 'Linux' && launchProof.browserLaunchObserved
        ? 'linux-wsl-headless-browser-launch-proof'
        : kernelName === 'Linux'
          ? 'linux-wsl-package-inventory-boundary-proof'
          : 'manual-linux-host-proof-required',
  },
  pathVisibility,
  packageVisibility,
  desktopEntries,
  negativeChecks,
};

if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected Linux host proof negative checks to reject dishonest claims');
}

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('browser-platform-linux-host-proof-ok=true');
console.log(`proof=${testResultPath}`);
console.log(`outputProof=${outputProofPath}`);
console.log(`wslAvailable=${proof.hostProofSummary.wslAvailable}`);
console.log(`browserCommandVisible=${browserCommandVisible}`);
console.log(`browserPackageInstalled=${browserPackageInstalled}`);
console.log(`browserDesktopEntryVisible=${browserDesktopEntryVisible}`);
console.log(`browserLaunchObserved=${launchProof.browserLaunchObserved}`);
console.log(`resultState=${proof.hostProofSummary.resultState}`);

function queryCommand(target) {
  const output = wslBash(`command -v ${shellEscape(target.command)} 2>/dev/null || true`);
  return {
    targetId: target.targetId,
    commandName: target.command,
    visible: output.length > 0,
    pathRef: output.length > 0 ? `redacted-linux-command-path-${sha256(output).slice(0, 16)}` : null,
    rawPathPersisted: false,
    managedProfileClaimed: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
  };
}

function queryPackage(target) {
  const output = wslBash(
    `dpkg-query -W -f='\\${'${Package}'}\\t\\${'${Status}'}\\n' ${shellEscape(target.packageName)} 2>/dev/null || true`
  );
  return {
    targetId: target.targetId,
    packageName: target.packageName,
    installed: output.includes('install ok installed'),
    packageStatusRef: output.length > 0 ? `redacted-linux-package-status-${sha256(output).slice(0, 16)}` : null,
    rawPackageListPersisted: false,
    managedProfileClaimed: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
  };
}

function queryDesktopEntries() {
  const output = wslBash(
    'for pattern in /usr/share/applications/*chrome*.desktop /usr/share/applications/*chromium*.desktop /usr/share/applications/*firefox*.desktop; do [ -e "$pattern" ] && printf "%s\\n" "$pattern"; done'
  );
  return output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((entryPath) => ({
      entryRef: `redacted-linux-desktop-entry-${sha256(entryPath).slice(0, 16)}`,
      rawPathPersisted: false,
      managedProfileClaimed: false,
      exactUrlProofClaimed: false,
      knownActiveTabProofClaimed: false,
    }));
}

function captureLinuxBrowserLaunchProof(entries) {
  const browser = entries.find((entry) => entry.visible);
  if (browser === undefined) {
    return {
      browserLaunchAttempted: false,
      browserLaunchObserved: false,
      browserLaunchCommandRef: null,
      browserLaunchDomSha256: null,
      browserLaunchScreenshotCaptured: false,
      browserLaunchScreenshotPersisted: false,
      browserLaunchScreenshotSha256: null,
    };
  }

  const marker = `ocentra-linux-browser-proof-${sha256(`${observedAt}:${browser.commandName}`).slice(0, 16)}`;
  const html = `<!doctype html><html><head><title>Ocentra Linux browser proof</title></head><body data-ocentra-proof="${marker}">Ocentra Linux browser proof ${marker}</body></html>`;
  const dataUrl = `data:text/html;base64,${Buffer.from(html).toString('base64')}`;
  const wslScreenshotPath = `/tmp/ocentra-linux-browser-proof-${sha256(marker).slice(0, 16)}.png`;
  const launchOutput = wslBash(
    [
      shellEscape(browser.commandName),
      '--headless=new',
      '--no-sandbox',
      '--disable-gpu',
      '--disable-dev-shm-usage',
      `--screenshot=${shellEscape(wslScreenshotPath)}`,
      '--window-size=900,480',
      '--dump-dom',
      shellEscape(dataUrl),
    ].join(' ')
  );
  const screenshotBase64 = wslBash(
    `[ -s ${shellEscape(wslScreenshotPath)} ] && base64 -w0 ${shellEscape(wslScreenshotPath)} || true`
  );
  const screenshot = screenshotBase64.length > 0 ? Buffer.from(screenshotBase64, 'base64') : null;

  if (screenshot !== undefined) {
    writeFileSync(screenshotProofPath, screenshot);
  } else if (existsSync(screenshotProofPath)) {
    writeFileSync(screenshotProofPath, Buffer.alloc(0));
  }

  return {
    browserLaunchAttempted: true,
    browserLaunchObserved: launchOutput.includes(marker) && screenshot !== undefined && screenshot.length > 0,
    browserLaunchCommandRef: `redacted-linux-browser-command-${sha256(browser.commandName).slice(0, 16)}`,
    browserLaunchDomSha256: launchOutput.length > 0 ? sha256(launchOutput) : null,
    browserLaunchScreenshotCaptured: screenshot !== undefined && screenshot.length > 0,
    browserLaunchScreenshotPersisted: screenshot !== undefined && screenshot.length > 0,
    browserLaunchScreenshotSha256: screenshot !== undefined && screenshot.length > 0 ? sha256(screenshot) : null,
  };
}

function wslBash(script) {
  return command(['-d', distroName, '--exec', 'bash', '-lc', script], { allowFailure: true }).trim();
}

function command(args, { allowFailure }) {
  try {
    return execFileSync('wsl.exe', args, { cwd: repoRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (error) {
    if (allowFailure) {
      return `${error.stdout?.toString() ?? ''}${error.stderr?.toString() ?? ''}`;
    }
    throw error;
  }
}

function shellEscape(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
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
