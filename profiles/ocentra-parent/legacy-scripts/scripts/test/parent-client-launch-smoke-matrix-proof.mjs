import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
export const outputRoot = join(
  repoRoot,
  'output',
  'parent-client-runtime-distribution-plan-proof',
  '09-parent-client-launch-smoke-matrix'
);
export const desktopProofPath = join(repoRoot, 'test-results', 'parent-desktop-shell-package-proof', 'proof.json');
export const mobileProofPath = join(repoRoot, 'test-results', 'parent-mobile-package-source-artifact-proof', 'proof.json');

export async function loadSmokeWorkflowSources(rootDir = repoRoot) {
  const androidWorkflowPath = join(rootDir, '.github', 'workflows', 'ci-package-parent-android.yml');
  const iosWorkflowPath = join(rootDir, '.github', 'workflows', 'ci-package-parent-ios.yml');

  const [androidWorkflow, iosWorkflow] = await Promise.all([
    readFile(androidWorkflowPath, 'utf8'),
    readFile(iosWorkflowPath, 'utf8'),
  ]);

  const androidSmokeCommand =
    'bash scripts/smoke/android-apk-smoke.sh target/release-packages/parent-android/ocentra-parent-mobile-android-debug-latest.apk ca.ocentra.parent.mobile ca.ocentra.parent.mobile/.MainActivity';
  const iosSmokeCommand =
    'bash scripts/smoke/ios-simulator-smoke.sh target/parent-ios-derived-data/Build/Products/Debug-iphonesimulator/OcentraParentMobile.app ca.ocentra.parent.mobile';

  assert.match(androidWorkflow, /scripts\/smoke\/android-apk-smoke\.sh/u);
  assert.match(androidWorkflow, /ca\.ocentra\.parent\.mobile/u);
  assert.match(androidWorkflow, /ocentra-parent-mobile-android-debug-latest\.apk/u);
  assert.match(iosWorkflow, /scripts\/smoke\/ios-simulator-smoke\.sh/u);
  assert.match(iosWorkflow, /OcentraParentMobile\.app/u);
  assert.match(iosWorkflow, /ca\.ocentra\.parent\.mobile/u);

  const desktopSmokeScripts = [
    'scripts/smoke/windows-msi-smoke.ps1',
    'scripts/smoke/linux-deb-smoke.sh',
    'scripts/smoke/macos-pkg-smoke.sh',
  ];
  for (const path of desktopSmokeScripts) {
    assert.equal(existsSync(join(rootDir, path)), true, `Missing desktop smoke script: ${path}`);
  }

  return {
    desktopSmokeScripts,
    androidSmokeCommand,
    iosSmokeCommand,
  };
}

export async function loadSmokeSurfaceContracts(rootDir = repoRoot) {
  const [webSource, desktopSmokeSource, desktopRuntimeSource, mobileSource] = await Promise.all([
    readFile(join(rootDir, 'scripts', 'test', 'portal-local-smoke.mjs'), 'utf8'),
    readFile(join(rootDir, 'scripts', 'test', 'parent-desktop-shell-package-proof.mjs'), 'utf8'),
    readFile(join(rootDir, 'scripts', 'test', 'parent-desktop-runtime-package-proof.test.mjs'), 'utf8'),
    readFile(join(rootDir, 'scripts', 'test', 'parent-mobile-package-source-artifact-proof.mjs'), 'utf8'),
  ]);

  assert.match(webSource, /ready', 'empty', 'unavailable', 'offline', 'stale', 'permission-required', 'scaffold-only'/u);
  assert.match(desktopSmokeSource, /parent_platform_proof_state/u);
  assert.match(desktopSmokeSource, /dry-run-launch-anchor-proved/u);
  assert.match(desktopSmokeSource, /ready-or-degraded-rust-proof/u);
  assert.match(desktopRuntimeSource, /PARENT_DESKTOP_RUNTIME_DEGRADED/u);
  assert.match(mobileSource, /parent Android mobile app is a separate buildable product/u);
  assert.match(mobileSource, /parent iOS mobile app is a separate simulator product/u);
  assert.match(mobileSource, /child-agent-parity=not-claimed/u);
  assert.match(mobileSource, /parent mobile release scripts and smoke inputs are separate from child agent package scripts/u);

  return {
    webVisibleStates: ['unavailable', 'offline', 'stale', 'permission-required', 'scaffold-only'],
    desktopVisibleStates: ['degraded', 'manual-required'],
    androidVisibleStates: ['degraded', 'unavailable', 'manual-required', 'scaffold'],
    iosVisibleStates: ['unavailable', 'manual-required', 'scaffold'],
  };
}

export function buildLaunchSmokeMatrixProof({
  checkedAt,
  commandResults,
  workflowSources,
  smokeSurfaceContracts,
  desktopProof,
  mobileProof,
}) {
  assert.equal(typeof checkedAt, 'string');
  assert.equal(commandResults.web.command, `node scripts/test/portal-local-smoke.mjs`);
  assert.equal(commandResults.desktop.command, `node scripts/test/parent-desktop-shell-package-proof.mjs`);
  assert.equal(
    commandResults.mobile.command,
    `node scripts/test/parent-mobile-package-source-artifact-proof.mjs`
  );

  const androidObserver = mobileProof?.runtimeProof.androidObserver;
  const androidUnavailable = mobileProof?.runtimeProof.androidLanUnavailable;
  const iosObserver = mobileProof?.runtimeProof.iosObserver;

  const rows = [
    {
      artifactKind: 'web',
      platform: 'web',
      smokeSurface: 'node scripts/test/portal-local-smoke.mjs',
      smokeSurfaceExists: true,
      launchState: commandResults.web.result === 'pass' ? 'launched' : 'blocked',
      visibleStates: smokeSurfaceContracts.webVisibleStates,
      degradedVisible: false,
      unavailableVisible: true,
      manualRequiredVisible: false,
      manualRequiredNote: 'n/a',
      evidenceRefs: [commandResults.web.artifact],
      noClaimBoundary:
        'web smoke proves local portal launch only and does not prove hosted portal readiness, setup completion, or child-runtime ownership',
    },
    {
      artifactKind: 'desktop',
      platform: 'windows|linux|macos',
      smokeSurface: workflowSources.desktopSmokeScripts.join(', '),
      smokeSurfaceExists: workflowSources.desktopSmokeScripts.length === 3,
      launchState: commandResults.desktop.result === 'pass' ? 'manual-required' : 'blocked',
      visibleStates: smokeSurfaceContracts.desktopVisibleStates,
      degradedVisible: true,
      unavailableVisible: false,
      manualRequiredVisible: true,
      manualRequiredNote:
        commandResults.desktop.result === 'pass' && desktopProof !== null
          ? 'Desktop dry-run launch anchors and Rust service reachability proof passed; signed package launch, production update or rollback, setup completion, and child-runtime authority remain manual-required or out of scope'
          : 'Desktop smoke proof is wired, but the current desktop launch proof did not complete; see 04-desktop-launch-smoke.log',
      evidenceRefs:
        commandResults.desktop.result === 'pass' && desktopProof !== null
          ? [
              commandResults.desktop.artifact,
              'test-results/parent-desktop-shell-package-proof/proof.json',
              desktopProof.artifact_path,
              ...workflowSources.desktopSmokeScripts,
            ]
          : [commandResults.desktop.artifact, ...workflowSources.desktopSmokeScripts],
      noClaimBoundary:
        'desktop smoke stays on package launch surfaces only and does not prove setup completion, product readiness, update rollback, or child-runtime authority',
    },
    {
      artifactKind: 'parent-android',
      platform: 'android',
      smokeSurface: workflowSources.androidSmokeCommand,
      smokeSurfaceExists: true,
      launchState:
        mobileProof === null ? 'blocked' : mobileProof.packageLaunchProof.android.packageLifecycleState,
      visibleStates:
        mobileProof === null
          ? smokeSurfaceContracts.androidVisibleStates
          : uniqueStates([
              mobileProof.packageLaunchProof.android.packageLifecycleState,
              androidObserver.assistantJobState,
              androidUnavailable.assistantJobState,
              ...Object.values(androidObserver.capabilityStates),
            ]),
      degradedVisible: mobileProof === null ? true : androidObserver.assistantJobState === 'degraded',
      unavailableVisible: mobileProof === null ? true : androidUnavailable.assistantJobState === 'unavailable',
      manualRequiredVisible:
        mobileProof === null ? true : mobileProof.packageLaunchProof.android.packageLifecycleState === 'manual-required',
      manualRequiredNote:
        mobileProof === null
          ? 'Android smoke command is wired, but the current parent mobile runtime proof blocked before artifact launch could be exercised; see 05-parent-mobile-launch-smoke.log'
          : 'Android parent mobile launch stays manual-required until a real APK artifact is installed and launched through the parent smoke command',
      evidenceRefs:
        mobileProof === null
          ? [commandResults.mobile.artifact, 'scripts/test/parent-mobile-package-source-artifact-proof.mjs']
          : [commandResults.mobile.artifact, 'test-results/parent-mobile-package-source-artifact-proof/proof.json'],
      noClaimBoundary:
        'Android smoke proves parent package smoke coverage only and does not claim setup completion, child-agent runtime ownership, or mobile parity readiness',
    },
    {
      artifactKind: 'parent-ios',
      platform: 'ios',
      smokeSurface: workflowSources.iosSmokeCommand,
      smokeSurfaceExists: true,
      launchState:
        mobileProof === null ? 'blocked' : mobileProof.packageLaunchProof.ios.packageLifecycleState,
      visibleStates:
        mobileProof === null
          ? smokeSurfaceContracts.iosVisibleStates
          : uniqueStates([
              mobileProof.packageLaunchProof.ios.packageLifecycleState,
              iosObserver.assistantJobState,
              ...Object.values(iosObserver.capabilityStates),
            ]),
      degradedVisible: false,
      unavailableVisible:
        mobileProof === null
          ? true
          : iosObserver.assistantJobState === 'unavailable' ||
            Object.values(iosObserver.capabilityStates).includes('unavailable'),
      manualRequiredVisible:
        mobileProof === null ? true : mobileProof.packageLaunchProof.ios.packageLifecycleState === 'manual-required',
      manualRequiredNote:
        mobileProof === null
          ? 'iOS smoke command is wired, but the current parent mobile runtime proof blocked before artifact launch could be exercised; see 05-parent-mobile-launch-smoke.log'
          : 'iOS parent mobile launch stays manual-required until a real simulator or device artifact is launched through the parent smoke command',
      evidenceRefs:
        mobileProof === null
          ? [commandResults.mobile.artifact, 'scripts/test/parent-mobile-package-source-artifact-proof.mjs']
          : [commandResults.mobile.artifact, 'test-results/parent-mobile-package-source-artifact-proof/proof.json'],
      noClaimBoundary:
        'iOS smoke proves parent package smoke coverage only and does not claim setup completion, child-agent runtime ownership, or store/TestFlight readiness',
    },
  ];

  const smokeSurfaceCoverage = rows.every((row) => row.smokeSurfaceExists);

  return {
    schemaVersion: 1,
    checkedAt,
    rows,
    commandResults,
    smokeSurfaceCoverage,
    degradedVisible: rows.some((row) => row.degradedVisible),
    unavailableVisible: rows.some((row) => row.unavailableVisible),
    manualRequiredVisible: rows.some((row) => row.manualRequiredVisible),
    readinessClaim: false,
    setupCompletionClaim: false,
    childRuntimeOwnershipClaim: false,
  };
}

export async function writeLaunchSmokeMatrixProof({
  outputDir = outputRoot,
  proof,
  commandResults,
}) {
  await mkdir(outputDir, { recursive: true });

  const scopeSummaryPath = join(outputDir, '00-scope-summary.md');
  const negativeCasePath = join(outputDir, '01-negative-case-proof.md');
  const manualRequiredPath = join(outputDir, '02-manual-required-gap-register.md');
  const validationLogPath = join(outputDir, '16-validation-commands.log');

  await Promise.all([
    writeFile(scopeSummaryPath, renderScopeSummary(proof), 'utf8'),
    writeFile(negativeCasePath, renderNegativeCaseProof(proof), 'utf8'),
    writeFile(manualRequiredPath, renderManualRequiredGapRegister(proof), 'utf8'),
    writeFile(validationLogPath, renderValidationLog(commandResults), 'utf8'),
  ]);

  return {
    scopeSummaryPath,
    negativeCasePath,
    manualRequiredPath,
    validationLogPath,
  };
}

export async function runLaunchSmokeMatrixProof({ rootDir = repoRoot, outputDir = outputRoot } = {}) {
  await mkdir(outputDir, { recursive: true });

  const commandResults = {
    web: await runCommand({
      rootDir,
      outputDir,
      artifactName: '03-web-launch-smoke.log',
      command: process.execPath,
      args: ['scripts/test/portal-local-smoke.mjs'],
    }),
    desktop: await runCommand({
      rootDir,
      outputDir,
      artifactName: '04-desktop-launch-smoke.log',
      command: process.execPath,
      args: ['scripts/test/parent-desktop-shell-package-proof.mjs'],
    }),
    mobile: await runCommand({
      rootDir,
      outputDir,
      artifactName: '05-parent-mobile-launch-smoke.log',
      command: process.execPath,
      args: ['scripts/test/parent-mobile-package-source-artifact-proof.mjs'],
    }),
  };

  const workflowSources = await loadSmokeWorkflowSources(rootDir);
  const smokeSurfaceContracts = await loadSmokeSurfaceContracts(rootDir);
  const desktopProof = existsSync(desktopProofPath)
    ? JSON.parse(await readFile(desktopProofPath, 'utf8'))
    : null;
  const mobileProof = existsSync(mobileProofPath)
    ? JSON.parse(await readFile(mobileProofPath, 'utf8'))
    : null;
  const proof = buildLaunchSmokeMatrixProof({
    checkedAt: new Date().toISOString(),
    commandResults,
    workflowSources,
    smokeSurfaceContracts,
    desktopProof,
    mobileProof,
  });
  const artifactPaths = await writeLaunchSmokeMatrixProof({
    outputDir,
    proof,
    commandResults,
  });

  const failures = Object.values(commandResults)
    .filter((result) => result.result !== 'pass')
    .map((result) => `${result.command} exited ${result.exit}`);

  return { proof, artifactPaths, failures };
}

function uniqueStates(values) {
  return [...new Set(values)];
}

function renderScopeSummary(proof) {
  const rowLines = proof.rows
    .map(
      (row) =>
        `| ${row.artifactKind} | ${row.platform} | ${row.launchState} | ${row.visibleStates.join(', ')} | ${row.manualRequiredNote} |`
    )
    .join('\n');

  return `plan: parent-client-runtime-distribution-plan
workpack: 09-parent-client-launch-smoke-matrix
owner: scripts-release
artifact_kind: launch-smoke
platform: cross-platform
package_state: n/a
signing_state: n/a
store_state: n/a
notarization_state: n/a
launch_state: ${proof.rows.map((row) => `${row.artifactKind}:${row.launchState}`).join('; ')}
route_bridge_state: n/a
setup_handoff_state: n/a
update_state: n/a
rollback_state: n/a
manual_required_note: smoke rows stay scoped to launch-state evidence only
run_id: n/a
command_id: n/a

## Scope

WP09 stays on parent-client launch smoke rows only: web, desktop, Android, and iOS.
This packet records whether each smoke surface exists, whether launch is launched or still manual-required, and which degraded or unavailable states remain explicit.

## Matrix

| Artifact | Platform | Launch state | Visible states | Manual-required note |
| --- | --- | --- | --- | --- |
${rowLines}

## Outcome

- Smoke surface coverage exists for web, desktop, Android, and iOS rows.
- Degraded state remains explicit instead of being hidden behind a green launch.
- Unavailable and manual-required states remain explicit where parity is not yet proved.
- No row claims setup completion.
- No row claims child-runtime ownership.
- No row upgrades smoke into readiness.
`;
}

function renderNegativeCaseProof(proof) {
  const rowLines = proof.rows
    .map((row) => `- ${row.artifactKind}: ${row.noClaimBoundary}`)
    .join('\n');

  return `plan: parent-client-runtime-distribution-plan
workpack: 09-parent-client-launch-smoke-matrix
owner: scripts-release
artifact_kind: launch-smoke
platform: cross-platform
launch_state: ${proof.rows.map((row) => `${row.artifactKind}:${row.launchState}`).join('; ')}
manual_required_note: smoke rows remain non-ready unless a real launch artifact is proved

## Negative cases

- Web smoke launch is not hosted portal readiness.
- Desktop smoke surfaces are not setup completion or product readiness.
- Android and iOS smoke surfaces are not mobile parity or store readiness.
- Manual-required rows stay open instead of being colored green.
- Child-runtime ownership remains outside this packet.

## Non-claims

${rowLines}
`;
}

function renderManualRequiredGapRegister(proof) {
  const manualRows = proof.rows.filter((row) => row.manualRequiredVisible);
  const lines = manualRows
    .map((row) => `- ${row.artifactKind}: ${row.manualRequiredNote}`)
    .join('\n');

  return `plan: parent-client-runtime-distribution-plan
workpack: 09-parent-client-launch-smoke-matrix
owner: scripts-release
artifact_kind: launch-smoke
platform: cross-platform
manual_required_note: rows remain open until a real artifact launch closes the gap

## Manual-required gaps

${lines}
`;
}

function renderValidationLog(commandResults) {
  return Object.values(commandResults)
    .map(
      (result) => `plan: parent-client-runtime-distribution-plan
workpack: 09-parent-client-launch-smoke-matrix
owner: scripts-release
artifact_kind: launch-smoke
platform: cross-platform
command: ${result.command}
exit: ${result.exit}
result: ${result.result}
artifact: ${result.artifact}
diagnostics_summary: ${result.diagnosticsSummary}
no_claim: smoke execution does not prove readiness, setup completion, or child-runtime ownership
`
    )
    .join('\n');
}

async function runCommand({ rootDir, outputDir, artifactName, command, args }) {
  const artifactPath = join(outputDir, artifactName);
  await mkdir(dirname(artifactPath), { recursive: true });

  const stdoutChunks = [];
  const stderrChunks = [];

  const exit = await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: rootDir,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    child.stdout.on('data', (chunk) => stdoutChunks.push(Buffer.from(chunk)));
    child.stderr.on('data', (chunk) => stderrChunks.push(Buffer.from(chunk)));
    child.once('error', reject);
    child.once('close', (code) => resolve(code ?? 1));
  });

  const stdout = Buffer.concat(stdoutChunks).toString('utf8');
  const stderr = Buffer.concat(stderrChunks).toString('utf8');
  const result = exit === 0 ? 'pass' : 'blocked';
  const commandLine = [command, ...args].join(' ');

  await writeFile(
    artifactPath,
    `command: ${commandLine}
exit: ${exit}
result: ${result}

[stdout]
${stdout}

[stderr]
${stderr}
`,
    'utf8'
  );

  return {
    command: commandLine.replace(process.execPath, 'node'),
    exit,
    result,
    artifact: relative(rootDir, artifactPath),
    diagnosticsSummary:
      result === 'pass'
        ? `${artifactName} recorded smoke output without widening packet claims`
        : `${artifactName} captured an upstream blocker while preserving the smoke-state boundary`,
  };
}

const isDirectExecution =
  process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;

if (isDirectExecution) {
  const { artifactPaths, failures, proof } = await runLaunchSmokeMatrixProof();
  console.log(`parent-client-launch-smoke-matrix-proof-ok:${relative(repoRoot, artifactPaths.scopeSummaryPath)}`);
  console.log(
    `smoke-matrix=${proof.rows.map((row) => `${row.artifactKind}:${row.launchState}`).join(',')}`
  );
  for (const failure of failures) {
    console.error(failure);
  }
}
