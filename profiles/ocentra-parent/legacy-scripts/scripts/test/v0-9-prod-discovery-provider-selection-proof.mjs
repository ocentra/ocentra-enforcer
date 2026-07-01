import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-9-prod-discovery-provider-selection-proof');
const proofPath = join(outputDir, 'proof.json');
const householdProofPath = join(repoRoot, 'test-results', 'v0-9-production-discovery-household-proof', 'proof.json');
const matrixPath = join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json');
const command = 'node scripts/test/v0-9-prod-discovery-provider-selection-proof.mjs';
const claimId = 'v0-9-prod-discovery-provider-selection-proof';
const schemaVersion = 'v0.9';
const commands = [];
const proofLabels = [];
const sensitiveEvidenceMarkers = [
  'activity.sqlite',
  'activity.ndjson',
  'decryptedEvidence',
  'journalPath',
  'rawEvidence',
  'rawProofSecret',
  'rawToken',
  'registryPath',
  'sqlitePath',
];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'lan_pairing_provider_selection']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'lan_pairing_provider_selection_read_model',
  ]);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-production-discovery-household-proof.mjs']);

  const providerSelectionReadModel = providerSelectionFixture();
  await validateBuiltContract(providerSelectionReadModel);

  const householdProof = await readJson(householdProofPath);
  const matrix = await readJson(matrixPath);
  const householdSummary = assertHouseholdProof(householdProof);
  const providerSelectionSummary = assertProviderSelectionReadModel(providerSelectionReadModel);
  const matrixRegistration = assertProofMatrix(matrix);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: claimId,
    commands,
    evidence: {
      householdProductionDiscovery: relativePath(householdProofPath),
      providerSelectionContract: 'packages/schema-domain/src/lan-pairing-provider-selection-proof.ts',
      providerSelectionRustProtocol: 'crates/agent-protocol/src/lan_pairing_provider_selection.rs',
      providerSelectionRustServiceReadModel: 'crates/agent-service/src/lan_pairing_provider_selection_read_model.rs',
      proofMatrix: relativePath(matrixPath),
    },
    proofLabels,
    householdSummary,
    providerSelectionReadModel,
    providerSelectionSummary,
    matrixRegistration,
    claimsProved: [
      'production discovery candidate lifecycle is typed for selected, rejected, degraded, unavailable, manual-required, and not-implemented provider states',
      'authorized provider selection is machine-checked by TypeScript contracts, Rust protocol serialization, and Rust service read-model tests',
      'stale, offline, revoked, wrong-origin, wrong-device, replayed, unpaired, unsupported, busy, degraded, and unavailable routes stay explicit policy states',
      'physical household provider proof and optional cloud relay remain manual-required or not-implemented states instead of product-complete claims',
    ],
    claimsNotProved: [
      'two physical household LAN devices were exercised',
      'router discovery, firewall prompt handling, NAT behavior, or OS local-network permission behavior',
      'real Android or iOS parent mobile controller UX or background LAN behavior',
      'cloud relay routing, storage, authentication, or runtime implementation',
    ],
  };

  assertNoSensitiveEvidenceMarkers(proof);
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-9-prod-discovery-provider-selection-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function providerSelectionFixture() {
  return {
    schemaVersion,
    checkedAt: new Date().toISOString(),
    selectedProviderRouteId: 'lan-route-local-network',
    authorizedProviderSelectionState: 'ci-mechanical-proof',
    physicalHouseholdProviderProofState: 'manual-required',
    cloudRelayImplementationState: 'not-implemented',
    cloudRelayDecisionState: 'manual-decision-required',
    candidates: [
      candidate(
        'parent-desktop-controller-ai-provider',
        'lan-route-local-network',
        'candidate-selected',
        'paired',
        'paired',
        'online',
        'authorized-result',
        null,
        'select-authorized-provider',
        'parent-desktop-controller-ai-provider:controller-job-completed-observer-job-rejected'
      ),
      candidate(
        'parent-desktop-controller-ai-provider',
        'lan-route-local-network',
        'candidate-rejected',
        'paired',
        'paired',
        'online',
        'unsupported-capability',
        'lan-ai-job-unauthorized',
        'refuse-unsupported-capability',
        'parent-desktop-controller-ai-provider:unsupported-capability-rejected'
      ),
      candidate(
        'parent-desktop-busy-ai-provider',
        'lan-route-local-network',
        'candidate-degraded',
        'paired',
        'paired',
        'online',
        'busy',
        null,
        'degrade-busy-provider',
        'parent-desktop-busy-ai-provider:busy-job-degraded'
      ),
      candidate(
        'first-child-agent',
        'route-v0-9-household-lan-product-proof',
        'candidate-unavailable',
        'stale',
        'paired',
        'stale',
        'unavailable',
        'stale',
        'refuse-route-blocked-provider',
        'rust-service:selected-device-stale-control-rejected'
      ),
      candidate(
        'first-child-agent',
        'route-v0-9-household-lan-product-proof',
        'candidate-unavailable',
        'offline',
        'paired',
        'offline',
        'unavailable',
        'offline',
        'refuse-route-blocked-provider',
        'rust-service:selected-device-offline-control-rejected'
      ),
      candidate(
        'unknown-host',
        'unsupported-route',
        'candidate-unavailable',
        'unavailable',
        'unpaired',
        'offline',
        'unavailable',
        'anonymous',
        'refuse-unpaired-provider',
        'first-child-agent:anonymous-rejected'
      ),
      candidate(
        'manual-household-provider-host',
        'unsupported-route',
        'manual-required',
        'unavailable',
        'unpaired',
        'offline',
        'unavailable',
        'local-network-disabled',
        'require-physical-household-proof',
        'manual physical household provider proof gate',
        'manual-required'
      ),
      candidate(
        'cloud-relay-provider',
        'unsupported-route',
        'not-implemented',
        'unavailable',
        'unpaired',
        'offline',
        'unavailable',
        'local-network-disabled',
        'require-cloud-relay-decision',
        'cloud relay provider selection requires separate authenticated relay proof',
        'not-implemented'
      ),
    ],
    manualRequirements: [
      manualRequirement(
        'physical-household-provider-host',
        'two physical household devices plus provider host proof before LAN provider readiness can be claimed'
      ),
      manualRequirement(
        'provider-route-origin-allowlist',
        'physical controller origin allowlist evidence from the household host'
      ),
      manualRequirement(
        'provider-route-stale-offline-artifact',
        'physical stale and offline provider route artifact from the selected household provider'
      ),
      manualRequirement('provider-revocation-artifact', 'physical provider revocation and rejected follow-up command'),
      manualRequirement(
        'cloud-relay-provider-decision',
        'separate product decision and authenticated relay proof before cloud relay provider selection is claimed',
        'not-implemented'
      ),
    ],
  };
}

function candidate(
  providerPeerId,
  routeId,
  lifecycleState,
  discoveryState,
  trustState,
  reachability,
  routingState,
  rejectionReason,
  policyDecision,
  evidenceLabel,
  proofState = 'ci-mechanical-proof'
) {
  return {
    schemaVersion,
    providerPeerId,
    routeId,
    lifecycleState,
    discoveryState,
    trustState,
    reachability,
    routingState,
    rejectionReason,
    policyDecision,
    proofState,
    evidenceLabel,
  };
}

function manualRequirement(requirement, requiredArtifactSummary, state = 'manual-required') {
  return {
    schemaVersion,
    requirement,
    state,
    requiredArtifactSummary,
  };
}

async function validateBuiltContract(readModel) {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'lan-pairing-provider-selection-proof.js');
  if (!existsSync(modulePath)) {
    throw new Error(`Built schema-domain LAN provider-selection module is missing: ${modulePath}`);
  }
  const module = await import(pathToFileURL(modulePath));
  module.LanProviderSelectionReadModelSchema.parse(readModel);
  proofLabels.push('v0.9.provider-selection.contract-parse');
}

function assertHouseholdProof(proof) {
  for (const expected of [
    'v0.9.production-discovery.states-explicit',
    'v0.9.selected-provider-policy.read-model-evidence',
    'v0.9.household-manual-and-cloud-nonclaims-preserved',
  ]) {
    assertArrayIncludes(proof.proofLabels, expected, 'household production discovery proof labels');
  }
  assertEqual(proof.manualStates.physicalHouseholdLan, 'manual-required', 'physical household proof');
  assertEqual(proof.manualStates.cloudRelayImplementation, 'not-implemented', 'cloud relay implementation');
  assertEqual(proof.manualStates.cloudRelayDecision, 'manual-decision-required', 'cloud relay decision');
  proofLabels.push('v0.9.provider-selection.composes-household-proof');
  return {
    productionDiscoveryState: proof.productionDiscoveryState.proofState,
    providerPolicyEvidenceCount: proof.providerPolicySummary.evidenceCount,
    physicalHouseholdLan: proof.manualStates.physicalHouseholdLan,
    cloudRelayImplementation: proof.manualStates.cloudRelayImplementation,
  };
}

function assertProviderSelectionReadModel(readModel) {
  const lifecycleStates = readModel.candidates.map((candidateEntry) => candidateEntry.lifecycleState);
  for (const expected of [
    'candidate-selected',
    'candidate-rejected',
    'candidate-degraded',
    'candidate-unavailable',
    'manual-required',
    'not-implemented',
  ]) {
    assertArrayIncludes(lifecycleStates, expected, 'provider selection lifecycle states');
  }
  const policyDecisions = readModel.candidates.map((candidateEntry) => candidateEntry.policyDecision);
  for (const expected of [
    'select-authorized-provider',
    'refuse-unpaired-provider',
    'refuse-route-blocked-provider',
    'refuse-unsupported-capability',
    'degrade-busy-provider',
    'require-physical-household-proof',
    'require-cloud-relay-decision',
  ]) {
    assertArrayIncludes(policyDecisions, expected, 'provider selection policy decisions');
  }
  assertEqual(readModel.authorizedProviderSelectionState, 'ci-mechanical-proof', 'authorized provider selection');
  assertEqual(readModel.physicalHouseholdProviderProofState, 'manual-required', 'physical provider proof');
  assertEqual(readModel.cloudRelayImplementationState, 'not-implemented', 'cloud relay implementation');
  assertEqual(readModel.cloudRelayDecisionState, 'manual-decision-required', 'cloud relay decision');
  assertArrayIncludes(
    readModel.manualRequirements.map((requirement) => requirement.requirement),
    'physical-household-provider-host',
    'manual provider requirement'
  );
  assertArrayIncludes(
    readModel.manualRequirements.map((requirement) => requirement.requirement),
    'cloud-relay-provider-decision',
    'cloud provider requirement'
  );
  proofLabels.push('v0.9.provider-selection.lifecycle-and-policy-evidence');
  return {
    candidateCount: readModel.candidates.length,
    lifecycleStates: Array.from(new Set(lifecycleStates)),
    policyDecisions: Array.from(new Set(policyDecisions)),
    manualRequirementCount: readModel.manualRequirements.length,
  };
}

function assertProofMatrix(matrix) {
  assertArrayIncludes(matrix.requiredCompletedClaimIds, claimId, 'required completed claim');
  const scenario = matrix.checkpointScenarios.find((candidateEntry) => candidateEntry.id === claimId);
  if (!scenario) {
    throw new Error(`Proof matrix is missing ${claimId} scenario.`);
  }
  assertArrayIncludes(scenario.ciCommands, command, 'scenario command');
  const claim = matrix.claims.find((candidateEntry) => candidateEntry.id === claimId);
  if (!claim) {
    throw new Error(`Proof matrix is missing ${claimId} claim.`);
  }
  assertArrayIncludes(claim.ciProof.commands, command, 'claim command');
  assertEqual(claim.runtimeSurfaceCoverage.providerSelectionLifecycle.state, 'implemented', 'lifecycle coverage');
  assertEqual(claim.runtimeSurfaceCoverage.authorizedProviderPolicy.state, 'implemented', 'policy coverage');
  assertEqual(claim.runtimeSurfaceCoverage.physicalHouseholdProvider.state, 'manual-required', 'physical provider');
  assertEqual(
    claim.runtimeSurfaceCoverage.cloudRelayProvider.implementationState,
    'not-implemented',
    'cloud provider implementation'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.cloudRelayProvider.decisionState,
    'manual-decision-required',
    'cloud provider decision'
  );
  proofLabels.push('proof-matrix.v0-9-prod-discovery-provider-selection-proof');
  return {
    scenario: scenario.id,
    claim: claim.id,
  };
}

async function runCommand(commandName, args) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`));
    });
    child.once('error', reject);
  });
}

async function readJson(path) {
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

function assertNoSensitiveEvidenceMarkers(value) {
  const serialized = JSON.stringify(value);
  for (const marker of sensitiveEvidenceMarkers) {
    if (serialized.includes(marker)) {
      throw new Error(`Provider selection proof includes sensitive marker ${marker}.`);
    }
  }
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
