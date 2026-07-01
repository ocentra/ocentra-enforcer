import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'platform-lan-enforcement-production-proof');
const proofPath = join(outputDir, 'proof.json');

const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-8-enforcement-adapter-product-proof-continuation.mjs']);
  const v08Continuation = await readJson(
    join(repoRoot, 'test-results', 'v0-8-enforcement-adapter-product-proof-continuation', 'proof.json')
  );
  assertV08Continuation(v08Continuation);

  const v08OsAdapterHardening = await readJson(
    join(repoRoot, 'test-results', 'v0-8-os-adapter-proof-hardening', 'proof.json')
  );
  assertV08OsAdapterHardening(v08OsAdapterHardening);

  const v08AppTimeLimit = await latestJson(join(repoRoot, 'test-results', 'v0-8-windows-app-time-limit-adapter-mvp'));
  assertV08AppTimeLimit(v08AppTimeLimit.data);

  const v08Production = await latestJson(join(repoRoot, 'test-results', 'v0-8-production-enforcement-hardening'));
  assertV08Production(v08Production.data);

  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-household-lan-production-discovery-proof.mjs']);
  const v09Production = await readJson(
    join(repoRoot, 'test-results', 'v0-9-production-lan-multidevice-hardening', 'proof.json')
  );
  const v09HouseholdReadiness = await readJson(
    join(repoRoot, 'test-results', 'v0-9-household-lan-proof-readiness', 'proof.json')
  );
  const v09HouseholdProductionDiscovery = await readJson(
    join(repoRoot, 'test-results', 'v0-9-household-lan-production-discovery-proof', 'proof.json')
  );
  assertV09Production(v09Production);
  assertV09HouseholdReadiness(v09HouseholdReadiness);
  assertV09HouseholdProductionDiscovery(v09HouseholdProductionDiscovery);

  const matrix = await readJson(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json'));
  assertProofMatrix(matrix);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      v08EnforcementAdapterProductProofContinuation: relative(
        repoRoot,
        join(repoRoot, 'test-results', 'v0-8-enforcement-adapter-product-proof-continuation', 'proof.json')
      ),
      v08OsAdapterHardening: relative(
        repoRoot,
        join(repoRoot, 'test-results', 'v0-8-os-adapter-proof-hardening', 'proof.json')
      ),
      v08AppTimeLimit: relative(repoRoot, v08AppTimeLimit.path),
      v08ProductionEnforcement: relative(repoRoot, v08Production.path),
      v08BrowserBoundary: v08OsAdapterHardening.evidence.browserBoundary,
      v09ProductionLanMultidevice: relative(
        repoRoot,
        join(repoRoot, 'test-results', 'v0-9-production-lan-multidevice-hardening', 'proof.json')
      ),
      v09HouseholdLanReadiness: relative(
        repoRoot,
        join(repoRoot, 'test-results', 'v0-9-household-lan-proof-readiness', 'proof.json')
      ),
      v09HouseholdLanProductionDiscovery: relative(
        repoRoot,
        join(repoRoot, 'test-results', 'v0-9-household-lan-production-discovery-proof', 'proof.json')
      ),
    },
    productionTruth: {
      v08RealAdapterState:
        'owned-process terminate and app time-limit proof are real when the host supports them; broad app/domain/browser blocking stays manual-required or unavailable; browser boundary proof does not claim exact unmanaged URLs or managed-browser service-command URL enforcement; product claim upgrades are rejected without missing host/platform artifacts',
      v09RealLanState:
        'local multi-service proof covers route, lease, registry, rejection, provider routing mechanics, explicit production discovery state labels, and a verifier that refuses household LAN readiness upgrades without physical device artifacts',
      parentMobileState:
        'parent mobile backend/controller-observer behavior is proof-first scaffold or degraded/unavailable until a real app/device proof exists',
      cloudRelayDecision:
        'no cloud relay behavior is implemented or claimed in this branch; cloud relay remains an explicit future product decision',
    },
    manualProofRequirements: {
      windowsEnforcement: [
        'run on a real Windows child host from this commit',
        'archive V0.8 continuation proof JSON with the claim-upgrade refusal and missing artifact list',
        'archive V0.8 OS adapter proof JSON with the child process pid, time-limit, process-terminate, and browser-boundary adapter results',
        'record whether app block, domain block, managed-browser service command, and unmanaged browser process control remain manual-required, unavailable, or process-only on that host',
        'capture Rust service log snippets for execute, restart recover, parent cancel, expiry, and unavailable states',
      ],
      householdLan: v09HouseholdReadiness.readinessGate.physicalHouseholdLan,
      parentMobileAndChildPlatforms: [
        'record parent mobile observer/controller-takeover backend state from a real mobile package before claiming UX parity',
        'record Android UsageStats, accessibility, VPN/DNS, device-owner, managed-profile, foreground service, and package lifecycle artifacts before upgrading Android child coverage',
        'record iOS Family Controls, DeviceActivity, Screen Time, Network Extension, notification, background execution, signing, and TestFlight artifacts before upgrading iOS child coverage',
      ],
    },
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`platform-lan-enforcement-production-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function assertV08Continuation(evidence) {
  assertEqual(
    evidence.proofMode,
    'v0-8-enforcement-adapter-product-proof-continuation',
    'V0.8 continuation proof mode'
  );
  assertArrayIncludes(
    evidence.proofLabels,
    'v0.8.continuation.claim-upgrade-refusal-proof',
    'V0.8 claim upgrade refusal proof'
  );
  assertEqual(evidence.productClaimUpgradeRefusal?.decision, 'rejected', 'V0.8 product claim upgrade refusal decision');
  assertEqual(
    evidence.productClaimUpgradeRefusal?.currentState,
    'manual-required',
    'V0.8 product claim upgrade refusal state'
  );
  assertArrayIncludes(evidence.claimsNotProved, 'global OS app blocking', 'V0.8 broad app non-claim');
  assertArrayIncludes(
    evidence.claimsNotProved,
    'network or domain blocking on the host',
    'V0.8 network/domain non-claim'
  );
  if (
    !Array.isArray(evidence.realServiceCoverage?.broadAdapterStates) ||
    evidence.realServiceCoverage.broadAdapterStates.length !== 3
  ) {
    throw new Error('V0.8 continuation must prove three broad adapter unavailable/manual-required states.');
  }
  proofLabels.push('v0.8.enforcement-adapter-product-proof-continuation');
}

function assertV08AppTimeLimit(evidence) {
  assertEqual(evidence.serviceScope?.timeLimitCreateRecoverCancelExpireProven, true, 'V0.8 timer lifecycle');
  assertEqual(evidence.serviceScope?.expiryAdapterReachedThroughService, true, 'V0.8 expiry service path');
  assertEqual(evidence.assertions?.recover?.timerEventKind, 'restart-recovered', 'V0.8 restart recovery');
  assertEqual(evidence.assertions?.recover?.recoveredAfterRestart, true, 'V0.8 restart flag');
  assertEqual(evidence.assertions?.cancel?.auditEventKind, 'cancelled', 'V0.8 parent cancel audit');
  assertEqual(evidence.assertions?.cancel?.stateCleared, true, 'V0.8 cancel clears persisted state');
  assertEqual(
    evidence.assertions?.unavailable?.reason,
    'enforcement-active-timer-state-required',
    'V0.8 unavailable recovery reason'
  );
  assertEqual(evidence.assertions?.expire?.stateCleared, true, 'V0.8 expiry clears persisted state');
  assertOneOf(evidence.assertions?.expire?.status, ['expired', 'unavailable'], 'V0.8 expiry honest state');
  proofLabels.push('v0.8.owned-process-time-limit-service-proof');
  proofLabels.push('v0.8.restart-recovery-parent-cancel-expiry-proof');
}

function assertV08Production(evidence) {
  assertEqual(
    evidence.serviceScope?.manualRequiredStatesProvenThroughService,
    true,
    'V0.8 manual-required states through service'
  );
  assertEqual(evidence.serviceScope?.unsupportedBlockingClaimsRejected, true, 'V0.8 unsupported blocking rejected');
  assertEqual(evidence.serviceScope?.auditStoragePathProven, true, 'V0.8 audit storage path');
  assertOneOf(
    evidence.serviceScope?.processTerminateServiceProof,
    ['actually-enforced', 'unsupported-platform'],
    'V0.8 process terminate service proof'
  );
  const assertionIds = new Set(evidence.assertions.map((assertion) => assertion.id));
  assertSetHas(assertionIds, 'process-terminate-owned-process', 'V0.8 process terminate proof');
  for (const id of [
    'app-block-process-control',
    'domain-block-network-control',
    'site-block-managed-browser-control',
  ]) {
    assertSetHas(assertionIds, id, 'V0.8 unavailable adapter proof');
  }
  for (const assertion of evidence.assertions) {
    if (assertion.id === 'process-terminate-owned-process') {
      assertOneOf(assertion.status, ['actually-enforced', 'unavailable'], 'V0.8 process terminate status');
      assertOneOf(
        assertion.adapterResultCode,
        ['process-terminated', 'process-already-exited', 'unsupported-platform'],
        'V0.8 process terminate adapter result'
      );
      continue;
    }
    assertEqual(assertion.status, 'unavailable', 'V0.8 broad blocking unavailable status');
    assertOneOf(assertion.capabilityState, ['manual-required', 'unavailable'], 'V0.8 capability state');
  }
  proofLabels.push('v0.8.owned-process-terminate-service-proof');
  proofLabels.push('v0.8.manual-required-broad-adapter-state-proof');
}

function assertV08OsAdapterHardening(evidence) {
  assertArrayIncludes(
    evidence.proofLabels,
    'v0.8.windows-capability-specific-os-adapter-states',
    'V0.8 capability states'
  );
  assertArrayIncludes(
    evidence.proofLabels,
    'v0.8.browser-boundary.pid-name-unmanaged-managed-nonclaim-proof',
    'V0.8 browser boundary proof'
  );
  assertEqual(
    evidence.osAdapterTruth.browserBoundary.exactManagedBrowserServiceCommandUrlClaim,
    'not-claimed-service-command-manual-required',
    'managed-browser service command exact URL non-claim'
  );
  assertEqual(
    evidence.osAdapterTruth.browserBoundary.exactUnmanagedUrlClaim,
    'not-claimed',
    'unmanaged browser exact URL non-claim'
  );
  proofLabels.push('v0.8.os-adapter-proof-hardening-aggregate');
}

function assertV09Production(evidence) {
  assertEqual(evidence.proofMode, 'local-multi-service-production-lan-hardening', 'V0.9 proof mode');
  const stepLabels = new Set(evidence.checkedSteps.map((step) => step.label));
  for (const label of ['discovery-challenge', 'pairing-control', 'lan-ai-provider-pool']) {
    assertSetHas(stepLabels, label, 'V0.9 proof step');
  }
  assertArrayIncludes(
    evidence.claimsProvedLocally,
    'trusted registry persists selected route and recovers it after restart',
    'V0.9 route recovery claim'
  );
  assertEqual(
    evidence.controllerAuthorityProof.revocationBeforeControl.controlRejectedAssertion,
    'first-child-agent:revoked-control-rejected',
    'V0.9 revocation-before-control claim'
  );
  assertEqual(
    evidence.parentMobileControllerObserverProof.mobileWriteAuthorityState,
    'manual-required-real-mobile-package-proof',
    'V0.9 parent mobile write authority boundary'
  );
  assertEqual(evidence.cloudRelayDecision.state, 'not-implemented', 'V0.9 cloud relay non-claim');
  assertArrayIncludes(
    evidence.claimsNotProvedLocally,
    'real household router discovery across two physical devices',
    'V0.9 household discovery boundary'
  );
  if (!Array.isArray(evidence.manualTwoDeviceChecklist) || evidence.manualTwoDeviceChecklist.length === 0) {
    throw new Error('V0.9 production proof must carry a manual two-device checklist.');
  }
  proofLabels.push('v0.9.production-lan-local-multiservice-proof');
  proofLabels.push('v0.9.physical-household-lan-manual-required');
}

function assertV09HouseholdReadiness(evidence) {
  assertEqual(evidence.proofMode, 'household-lan-readiness-gate', 'V0.9 household readiness proof mode');
  assertEqual(
    evidence.productReadinessDecision,
    'not-ready-for-product-ready-household-lan-claim',
    'V0.9 household readiness decision'
  );
  assertEqual(evidence.readinessGate.physicalHouseholdLan.state, 'manual-required', 'V0.9 physical household LAN gate');
  assertEqual(evidence.readinessGate.cloudRelay.state, 'not-implemented', 'V0.9 cloud relay gate');
  assertArrayIncludes(
    evidence.claimsProvedByThisGate,
    'route/controller/selected-device/provider states are gathered from existing real-service proof artifacts',
    'V0.9 gathered state claim'
  );
  assertArrayIncludes(
    evidence.claimsNotProvedByThisGate,
    'product-ready LAN behavior on a household router',
    'V0.9 product-ready LAN non-claim'
  );
  proofLabels.push('v0.9.household-readiness-gate-proof');
}

function assertV09HouseholdProductionDiscovery(evidence) {
  assertEqual(
    evidence.proofMode,
    'household-lan-production-discovery-boundary',
    'V0.9 household production discovery proof mode'
  );
  assertEqual(evidence.physicalClaimUpgradeVerifier.decision, 'rejected', 'V0.9 physical claim upgrade rejection');
  assertEqual(
    evidence.physicalClaimUpgradeVerifier.currentState,
    'manual-required',
    'V0.9 physical claim upgrade state'
  );
  assertArrayIncludes(
    evidence.proofLabels,
    'v0.9.production-discovery.explicit-state-labels',
    'V0.9 explicit production discovery states'
  );
  assertArrayIncludes(
    evidence.claimsProved,
    'physical household LAN readiness is refused without required two-device, router, firewall, origin, stale/offline, failed-unpaired, and provider artifacts',
    'V0.9 physical artifact verifier claim'
  );
  proofLabels.push('v0.9.household-production-discovery-boundary-proof');
}

function assertProofMatrix(matrix) {
  const claim = matrix.claims.find((candidate) => candidate.id === 'platform-lan-enforcement-production-proof');
  if (!claim) {
    throw new Error('Proof matrix is missing platform-lan-enforcement-production-proof claim.');
  }
  assertEqual(claim.platformCoverage.windows, 'real-local-windows-proof', 'Windows proof state');
  assertEqual(claim.platformCoverage.android, 'manual-required', 'Android proof state');
  assertEqual(claim.platformCoverage.ios, 'manual-required', 'iOS proof state');
  assertEqual(
    claim.runtimeSurfaceCoverage.cloudRelay.state,
    'not-implemented',
    'Cloud relay remains an explicit non-claim'
  );
  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'platform-lan-enforcement-production-proof'
  );
  if (!scenario) {
    throw new Error('Proof matrix is missing platform-lan-enforcement-production-proof checkpoint scenario.');
  }
  assertSetHas(
    new Set(scenario.ciCommands),
    'node scripts/test/platform-lan-enforcement-production-proof.mjs',
    'Production proof command is matrix-listed'
  );
  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-enforcement-adapter-product-proof-continuation',
    'V0.8 continuation claim is required'
  );
  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-9-household-lan-production-discovery-proof',
    'Household LAN production discovery claim is required'
  );
  proofLabels.push('proof-matrix.platform-lan-enforcement-production-states');
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
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

function assertSetHas(set, value, label) {
  if (!set.has(value)) {
    throw new Error(`${label}: missing ${value}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
