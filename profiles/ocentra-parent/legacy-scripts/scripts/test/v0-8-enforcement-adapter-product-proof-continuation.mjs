import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-enforcement-adapter-product-proof-continuation');
const proofPath = join(outputDir, 'proof.json');
const proofCommand = 'node scripts/test/v0-8-enforcement-adapter-product-proof-continuation.mjs';
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'enforcement_app_time_limit']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-core',
    'manual_required_network_and_browser_targets_return_unavailable_audit_without_adapter_execution',
  ]);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_execute_reports_manual_required_service_states_for_unwired_adapters',
  ]);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-8-os-adapter-proof-hardening.mjs']);

  const osAdapterHardening = await readJson(
    join(repoRoot, 'test-results', 'v0-8-os-adapter-proof-hardening', 'proof.json')
  );
  const appTimeLimit = await latestJson(join(repoRoot, 'test-results', 'v0-8-windows-app-time-limit-adapter-mvp'));
  const productionHardening = await latestJson(join(repoRoot, 'test-results', 'v0-8-production-enforcement-hardening'));
  const browserBoundary = await latestJson(
    join(repoRoot, 'test-results', 'windows-managed-unmanaged-browser-enforcement-proof')
  );
  const matrix = await readJson(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json'));

  assertOsAdapterHardening(osAdapterHardening);
  assertAppTimeLimit(appTimeLimit.data);
  assertProductionHardening(productionHardening.data);
  assertBrowserBoundary(browserBoundary.data);
  assertProofMatrix(matrix);

  const upgradeRefusal = productClaimUpgradeRefusal();
  assertProductClaimUpgradeRefusal(upgradeRefusal);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-enforcement-adapter-product-proof-continuation',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      continuationProof: relative(repoRoot, proofPath),
      osAdapterHardening: relative(
        repoRoot,
        join(repoRoot, 'test-results', 'v0-8-os-adapter-proof-hardening', 'proof.json')
      ),
      appTimeLimit: relative(repoRoot, appTimeLimit.path),
      productionEnforcement: relative(repoRoot, productionHardening.path),
      browserBoundary: relative(repoRoot, browserBoundary.path),
      proofMatrix: relative(repoRoot, join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')),
    },
    realServiceCoverage: {
      ownedProcessTerminate: assertionById(productionHardening.data, 'process-terminate-owned-process'),
      appTimeLimitLifecycle: appTimeLimit.data.assertions,
      broadAdapterStates: productionHardening.data.assertions.filter((assertion) =>
        ['app-block-process-control', 'domain-block-network-control', 'site-block-managed-browser-control'].includes(
          assertion.id
        )
      ),
      browserBoundary: browserBoundary.data.states,
      auditRecoveryTruth: {
        appTimeLimitJournal: appTimeLimit.data.artifacts.activityJournal,
        productionJournal: productionHardening.data.artifacts.activityJournal,
        appTimeLimitStore: appTimeLimit.data.artifacts.activityStore,
        productionStore: productionHardening.data.artifacts.activityStore,
        encryptedJournalPlaintextDecisionIdsAbsent: true,
        restartRecoveryState: appTimeLimit.data.assertions.recover.timerEventKind,
        parentCancelAuditState: appTimeLimit.data.assertions.cancel.auditEventKind,
        expiryState: appTimeLimit.data.assertions.expire.timerEventKind,
      },
    },
    productClaimUpgradeRefusal: upgradeRefusal,
    claimsProved: [
      'owned-process terminate uses real service and OS adapter status where the host supports it',
      'app time-limit execute, restart recovery, parent cancel, unavailable recovery, expiry, audit, encrypted journal, and SQLite storage are service-proved',
      'broad app blocking, network/domain blocking, and managed-browser service commands return unavailable or manual-required states when no adapter proof exists',
      'unmanaged-browser process evidence is kept as process-only evidence and is not upgraded to exact URL, active tab, title, download source, page text, or user intent',
      'product-ready enforcement claim upgrade is rejected without real OS-approved broad app/domain/browser artifacts',
    ],
    claimsNotProved: [
      'global OS app blocking',
      'network or domain blocking on the host',
      'managed-browser exact URL enforcement from service command target strings',
      'unmanaged-browser exact URL, active tab, title, download source, page content, HTTPS content, or user intent',
      'anti-tamper, admin hardening, Android device-owner behavior, iOS Family Controls, signing, stores, or production mobile enforcement',
    ],
    manualProofRequirements: manualProofRequirements(),
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-enforcement-adapter-product-proof-continuation-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function assertOsAdapterHardening(evidence) {
  assertArrayIncludes(
    evidence.proofLabels,
    'v0.8.production-hardening.manual-required-service-boundaries',
    'OS adapter hardening manual-required label'
  );
  assertArrayIncludes(
    evidence.proofLabels,
    'v0.8.browser-boundary.pid-name-unmanaged-managed-nonclaim-proof',
    'OS adapter hardening browser label'
  );
  assertEqual(
    evidence.osAdapterTruth.browserBoundary.exactManagedBrowserServiceCommandUrlClaim,
    'not-claimed-service-command-manual-required',
    'managed-browser service command URL claim'
  );
  assertEqual(
    evidence.osAdapterTruth.browserBoundary.exactUnmanagedUrlClaim,
    'not-claimed',
    'unmanaged browser URL claim'
  );
  proofLabels.push('v0.8.continuation.os-adapter-hardening-artifact-accepted');
}

function assertAppTimeLimit(evidence) {
  assertEqual(evidence.serviceScope?.timeLimitCreateRecoverCancelExpireProven, true, 'time-limit lifecycle');
  assertEqual(evidence.serviceScope?.expiryAdapterReachedThroughService, true, 'expiry service path');
  assertEqual(evidence.assertions?.execute?.statePersisted, true, 'execute persists timer state');
  assertEqual(evidence.assertions?.recover?.timerEventKind, 'restart-recovered', 'restart recovery event kind');
  assertEqual(evidence.assertions?.recover?.recoveredAfterRestart, true, 'restart recovery flag');
  assertEqual(evidence.assertions?.cancel?.auditEventKind, 'cancelled', 'parent cancel audit');
  assertEqual(evidence.assertions?.cancel?.stateCleared, true, 'parent cancel clears state');
  assertEqual(
    evidence.assertions?.unavailable?.reason,
    'enforcement-active-timer-state-required',
    'unavailable recovery reason'
  );
  assertEqual(evidence.assertions?.expire?.stateCleared, true, 'expiry clears timer state');
  assertOneOf(evidence.assertions?.expire?.status, ['expired', 'unavailable'], 'expiry status');
  assertArtifactPaths(evidence.artifacts, ['activityJournal', 'activityStore', 'timerStatePath']);
  proofLabels.push('v0.8.continuation.app-time-limit-audit-recovery-truth');
}

function assertProductionHardening(evidence) {
  assertEqual(evidence.serviceScope?.manualRequiredStatesProvenThroughService, true, 'manual-required service states');
  assertEqual(evidence.serviceScope?.unsupportedBlockingClaimsRejected, true, 'unsupported blocking rejected');
  assertEqual(evidence.serviceScope?.auditStoragePathProven, true, 'audit storage path');
  assertOneOf(
    evidence.serviceScope?.processTerminateServiceProof,
    ['actually-enforced', 'unsupported-platform'],
    'process terminate proof state'
  );
  assertArtifactPaths(evidence.artifacts, ['activityJournal', 'activityStore', 'devLogDirectory']);

  const processTerminate = assertionById(evidence, 'process-terminate-owned-process');
  assertOneOf(processTerminate.status, ['actually-enforced', 'unavailable'], 'owned process status');
  assertOneOf(
    processTerminate.adapterResultCode,
    ['process-terminated', 'process-already-exited', 'unsupported-platform'],
    'owned process adapter result'
  );

  for (const id of [
    'app-block-process-control',
    'domain-block-network-control',
    'site-block-managed-browser-control',
  ]) {
    const assertion = assertionById(evidence, id);
    assertEqual(assertion.status, 'unavailable', `${id} status`);
    assertOneOf(assertion.capabilityState, ['manual-required', 'unavailable'], `${id} capability`);
    assertOneOf(assertion.unavailableReason, ['manual-required', 'unsupported-platform'], `${id} unavailable reason`);
    assertEqual(assertion.auditEventKind, 'unavailable', `${id} audit`);
  }
  proofLabels.push('v0.8.continuation.broad-adapter-manual-required-truth');
}

function assertBrowserBoundary(evidence) {
  const states = evidence.states ?? {};
  assertEqual(states.processIdRequiredRejection, 'rejected', 'process id required rejection');
  assertEqual(states.windowsProcessAdapterGuard, 'rejected-without-termination', 'process name guard');
  assertOneOf(states.windowsProcessAdapterRuntime, ['terminated', 'already-exited'], 'owned process runtime');
  assertEqual(states.broadAppBlockingCapability, 'manual-required', 'broad app blocking boundary');
  assertEqual(states.managedBrowserServiceCommand, 'manual-required', 'managed browser service command boundary');
  assertEqual(states.exactUnmanagedUrlClaim, 'not-claimed', 'unmanaged URL non-claim');
  assertEqual(
    states.exactManagedBrowserServiceCommandUrlClaim,
    'not-claimed-service-command-manual-required',
    'managed browser service command URL non-claim'
  );
  assertArtifactPaths(evidence.artifacts, ['activityJournal', 'activityStore', 'devLogDirectory']);
  const unmanaged = assertionById(evidence, 'unmanaged-browser-terminate');
  assertOneOf(unmanaged.state, ['terminated', 'manual-required'], 'unmanaged browser process boundary');
  assertEqual(unmanaged.exactUrlClaimState, 'not-claimed', 'unmanaged exact URL claim');
  assertEqual(assertionById(evidence, 'unmanaged-browser-warn').state, 'warned', 'unmanaged browser warn');
  proofLabels.push('v0.8.continuation.browser-process-only-nonclaim-truth');
}

function assertProofMatrix(matrix) {
  assertArrayIncludes(
    matrix.requiredCompletedClaimIds,
    'v0-8-enforcement-adapter-product-proof-continuation',
    'required completed claim id'
  );
  const claim = matrix.claims.find(
    (candidate) => candidate.id === 'v0-8-enforcement-adapter-product-proof-continuation'
  );
  if (!claim) {
    throw new Error('Proof matrix is missing v0-8-enforcement-adapter-product-proof-continuation claim.');
  }
  assertEqual(claim.platformCoverage.windows, 'real-local-windows-proof', 'continuation Windows coverage');
  assertArrayIncludes(claim.ciProof.commands, proofCommand, 'continuation claim command');
  assertEqual(
    claim.runtimeSurfaceCoverage.claimUpgradeRefusal.state,
    'manual-required',
    'continuation claim upgrade refusal state'
  );
  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-enforcement-adapter-product-proof-continuation'
  );
  if (!scenario) {
    throw new Error('Proof matrix is missing v0-8-enforcement-adapter-product-proof-continuation scenario.');
  }
  assertArrayIncludes(scenario.ciCommands, proofCommand, 'continuation scenario command');
  proofLabels.push('proof-matrix.v0-8-enforcement-adapter-product-proof-continuation');
}

function productClaimUpgradeRefusal() {
  return {
    decision: 'rejected',
    currentState: 'manual-required',
    requestedUpgrade: 'product-ready-broad-app-domain-browser-enforcement',
    acceptedStates: {
      ownedProcessTerminate: 'implemented-where-os-supported',
      appTimeLimit: 'implemented-where-os-supported',
      broadAppBlocking: 'manual-required',
      networkDomainBlocking: 'manual-required',
      managedBrowserServiceCommand: 'manual-required',
      unmanagedBrowserEvidence: 'process-only-non-url-proof',
    },
    missingArtifacts: [
      'OS-approved broad app blocking adapter proof on the host',
      'network or DNS/VPN/domain blocking adapter proof on the host',
      'managed-browser exact URL enforcement proof from a managed browser boundary',
      'unmanaged-browser exact URL, active tab, title, download source, page text, or user-intent evidence',
      'anti-tamper, admin hardening, rollback, bypass-resistance, and platform permission proof',
      'Android device-owner or iOS Family Controls entitlement proof',
    ],
    refusalReason:
      'The real service proves owned-process and app time-limit mechanics plus unavailable/manual-required broad states; it does not prove product-ready broad blocking.',
  };
}

function assertProductClaimUpgradeRefusal(refusal) {
  assertEqual(refusal.decision, 'rejected', 'claim upgrade decision');
  assertEqual(refusal.currentState, 'manual-required', 'claim upgrade state');
  assertEqual(refusal.acceptedStates.broadAppBlocking, 'manual-required', 'broad app accepted state');
  assertEqual(refusal.acceptedStates.networkDomainBlocking, 'manual-required', 'network accepted state');
  assertEqual(
    refusal.acceptedStates.unmanagedBrowserEvidence,
    'process-only-non-url-proof',
    'unmanaged browser accepted state'
  );
  for (const artifact of [
    'OS-approved broad app blocking adapter proof on the host',
    'network or DNS/VPN/domain blocking adapter proof on the host',
    'managed-browser exact URL enforcement proof from a managed browser boundary',
    'Android device-owner or iOS Family Controls entitlement proof',
  ]) {
    assertArrayIncludes(refusal.missingArtifacts, artifact, 'claim upgrade missing artifact');
  }
  proofLabels.push('v0.8.continuation.claim-upgrade-refusal-proof');
}

function manualProofRequirements() {
  return [
    'Run this continuation proof on a real Windows child host and archive all generated proof JSON with the commit SHA.',
    'Capture Rust service logs for app time-limit execute, restart recovery, parent cancel, unavailable recovery, expiry, audit, encrypted journal, and SQLite storage.',
    'Capture owned-process terminate, process-id-required, and process-name-mismatch service logs without presenting them as global app blocking.',
    'Capture broad app, network/domain, and managed-browser service commands returning manual-required or unavailable until real host adapters exist.',
    'Capture managed-browser exact URL proof from the managed browser boundary before upgrading any exact URL enforcement claim.',
    'Capture Android device-owner and iOS Family Controls entitlement/device artifacts before upgrading mobile child enforcement claims.',
  ];
}

function assertionById(evidence, id) {
  const assertion = evidence.assertions?.find((candidate) => candidate.id === id);
  if (!assertion) {
    throw new Error(`Missing assertion: ${id}`);
  }
  return assertion;
}

function assertArtifactPaths(artifacts, requiredKeys) {
  for (const key of requiredKeys) {
    const value = artifacts?.[key];
    if (typeof value !== 'string' || value.length === 0) {
      throw new Error(`Missing artifact path: ${key}`);
    }
  }
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function latestJson(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const jsonFiles = [];
  for (const entry of entries) {
    if (entry.isFile() && entry.name.endsWith('.json')) {
      const path = join(directory, entry.name);
      jsonFiles.push({ path, data: JSON.parse(await readFile(path, 'utf8')) });
    }
  }
  if (jsonFiles.length === 0) {
    throw new Error(`No JSON evidence files found in ${directory}`);
  }
  jsonFiles.sort((left, right) => left.path.localeCompare(right.path));
  return jsonFiles.at(-1);
}

async function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Missing proof artifact: ${path}`);
  }
  return JSON.parse(await readFile(path, 'utf8'));
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertOneOf(actual, expectedValues, label) {
  if (!expectedValues.includes(actual)) {
    throw new Error(`${label}: expected one of ${expectedValues.join(', ')}, received ${actual}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
