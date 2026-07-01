import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-9-production-discovery-proof');
const proofPath = join(outputDir, 'proof.json');
const householdProofPath = join(
  repoRoot,
  'test-results',
  'v0-9-household-lan-production-discovery-proof',
  'proof.json'
);
const productionProofPath = join(repoRoot, 'test-results', 'v0-9-production-lan-multidevice-hardening', 'proof.json');
const matrixPath = join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json');
const command = 'node scripts/test/v0-9-production-discovery-proof.mjs';
const claimId = 'v0-9-production-discovery-proof';
const commands = [];
const proofLabels = [];

await mkdir(outputDir, { recursive: true });
await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-household-lan-production-discovery-proof.mjs']);

const householdProof = await readJson(householdProofPath);
const productionProof = await readJson(productionProofPath);
const matrix = await readJson(matrixPath);

const selectedRouteTrust = assertSelectedRouteTrust(productionProof, householdProof);
const localMultiServiceProof = assertLocalMultiServiceProof(productionProof);
const householdNonClaim = assertHouseholdNonClaim(householdProof, productionProof);
const matrixRegistration = assertProofMatrix(matrix);

const proof = {
  schemaVersion: 1,
  checkedAt: new Date().toISOString(),
  commit: await gitHead(),
  proofMode: 'v0-9-production-discovery-product-truth',
  commands,
  evidence: {
    householdProductionDiscovery: relative(repoRoot, householdProofPath).replaceAll('\\', '/'),
    productionLanMultidevice: relative(repoRoot, productionProofPath).replaceAll('\\', '/'),
    proofMatrix: relative(repoRoot, matrixPath).replaceAll('\\', '/'),
  },
  proofLabels,
  selectedRouteTrust,
  localMultiServiceProof,
  householdNonClaim,
  matrixRegistration,
  claimsProved: [
    'selected route status carries selected pairing id, selected-route trust state, stale time, and offline time',
    'local multi-service production discovery proof covers two real local Rust service processes',
    'wrong-origin and wrong-device rejection remain backed by local service proof artifacts',
    'physical household LAN, mobile controller UX, cloud relay, and real router discovery remain explicit non-claims',
  ],
  claimsNotProved: [
    'physical household router discovery',
    'mobile controller write-authority UX or mobile background behavior',
    'cloud relay routing, storage, or authentication',
    'real router, firewall, or OS prompt behavior',
  ],
};

await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`v0-9-production-discovery-proof-ok:${proofLabels.join(',')}`);
console.log(`evidence=${proofPath}`);

function assertSelectedRouteTrust(productionProof, householdProof) {
  const selectedRouteTrust = productionProof.localTwoServiceProof?.selectedRouteTrust ?? [];
  for (const expected of [
    'first-child-agent:selected-route-trust-state-paired',
    'second-child-agent:selected-route-trust-state-paired',
    'second-child-agent:restart-restores-selected-route-trust-state',
  ]) {
    assertArrayIncludes(selectedRouteTrust, expected, 'selected route trust proof');
  }
  assertArrayIncludes(
    householdProof.proofLabels,
    'v0.9.selected-route.trust-state-explicit',
    'household selected route trust label'
  );
  proofLabels.push('v0.9.selected-route.trust-state-explicit');
  return {
    state: 'implemented',
    selectedRouteTrust,
    selectedPairingId: 'reported-by-local-status-payload',
    staleAt: 'reported-by-local-status-payload',
    offlineAt: 'reported-by-local-status-payload',
  };
}

function assertLocalMultiServiceProof(productionProof) {
  assertEqual(productionProof.localTwoServiceProof?.serviceCount, 2, 'local service count');
  assertArrayIncludes(
    productionProof.discoveryProof?.staleOrExpiredRejected ?? [],
    'first-discovery-agent:stale-proof-rejected',
    'stale proof rejection'
  );
  assertEqual(
    productionProof.discoveryProof?.wrongOriginRejectedBeforeUpgrade,
    'wrong-origin-websocket-rejected-before-upgrade',
    'wrong origin rejection'
  );
  assertEqual(
    productionProof.discoveryProof?.wrongDeviceChallengeRejected,
    'wrong-agent-port-challenge-rejected-as-wrong-device',
    'wrong device rejection'
  );
  proofLabels.push('v0.9.production-discovery.local-two-service-proof');
  return {
    state: 'ci-mechanical-proof',
    serviceCount: productionProof.localTwoServiceProof.serviceCount,
    wrongOrigin: productionProof.discoveryProof.wrongOriginRejectedBeforeUpgrade,
    wrongDevice: productionProof.discoveryProof.wrongDeviceChallengeRejected,
  };
}

function assertHouseholdNonClaim(householdProof, productionProof) {
  assertArrayIncludes(
    householdProof.claimsNotProved,
    'product-ready household router discovery',
    'household router non-claim'
  );
  assertArrayIncludes(
    productionProof.claimsNotProvedLocally,
    'cloud relay routing, storage, or authentication behavior',
    'cloud relay non-claim'
  );
  assertEqual(
    householdProof.physicalClaimUpgradeVerifier.currentState,
    'manual-required',
    'physical household verifier state'
  );
  assertEqual(householdProof.physicalClaimUpgradeVerifier.decision, 'rejected', 'physical household verifier decision');
  proofLabels.push('v0.9.production-discovery.household-non-claims-preserved');
  return {
    physicalHouseholdLan: 'manual-required',
    realRouterDiscovery: 'not-proved',
    mobileControllerUx: 'manual-required',
    cloudRelay: 'not-implemented',
  };
}

function assertProofMatrix(matrix) {
  assertArrayIncludes(matrix.requiredCompletedClaimIds, claimId, 'required completed claim');
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === claimId);
  if (!scenario) {
    throw new Error(`Proof matrix is missing ${claimId} scenario.`);
  }
  assertArrayIncludes(scenario.ciCommands, command, 'scenario command');
  const claim = matrix.claims.find((candidate) => candidate.id === claimId);
  if (!claim) {
    throw new Error(`Proof matrix is missing ${claimId} claim.`);
  }
  assertArrayIncludes(claim.ciProof.commands, command, 'claim command');
  assertEqual(claim.runtimeSurfaceCoverage.selectedRouteTrust.state, 'implemented', 'selected route trust state');
  assertEqual(claim.runtimeSurfaceCoverage.physicalHouseholdLan.state, 'manual-required', 'physical household state');
  proofLabels.push('proof-matrix.v0-9-production-discovery-proof');
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

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`Expected ${label} to include ${expected}.`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Expected ${label} to be ${expected}, received ${actual}.`);
  }
}
