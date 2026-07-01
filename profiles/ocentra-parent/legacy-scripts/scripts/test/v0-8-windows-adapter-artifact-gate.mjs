import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-windows-adapter-artifact-gate');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'windows_adapter_artifact_gate']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'windows_adapter_artifact_gate_read_model']);
  await runCommand('node', ['scripts/test/v0-8-windows-adapter-capability-proof.mjs']);
  await runCommand(...npmCommand(['run', 'test:pre-ai-proof']));
  await assertProtocolHarness();

  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-windows-adapter-artifact-gate',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      rustProtocol: 'crates/agent-protocol/src/windows_adapter_artifact_gate.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/windows_adapter_artifact_gate_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/windows_adapter_artifact_gate_read_model.rs',
      rustServiceReadModelTest: 'crates/agent-service/tests/unit/windows_adapter_artifact_gate_read_model_tests.rs',
      capabilityHarness: 'scripts/test/v0-8-windows-adapter-capability-proof.mjs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-windows-adapter-artifact-gate-2026-05-29.md',
    },
    counts: {
      gateEntries: 6,
      defaultClaimUpgradeAllowed: 0,
      manualReviewOnlyWhenArtifactsPresent: true,
      unsupportedSurfaceRefusals: 2,
    },
    productTruth: {
      appTargets:
        'App claims stay refused until same-identity app evidence, apply result, rollback result, and audit custody event refs are present.',
      domainNetworkTargets:
        'Domain/network claims stay refused until network filter apply, rollback, and audit custody refs are present.',
      managedBrowser:
        'Managed-browser exact URL claims stay refused until exact URL evidence and audit custody refs are present.',
      unmanagedBrowser:
        'Unmanaged-browser capability remains process-only and cannot upgrade exact URL claims from this gate.',
      unsupportedOs: 'Unsupported OS targets refuse claim upgrades instead of borrowing Windows adapter artifacts.',
      rollbackAudit:
        'Rollback/audit claims stay refused until same-identity apply, rollback, and custody refs are present.',
    },
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-windows-adapter-artifact-gate-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function assertProofMatrix(matrix) {
  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-windows-adapter-artifact-gate',
    'windows adapter artifact gate claim is required'
  );
  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-windows-adapter-artifact-gate'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 Windows adapter artifact gate checkpoint scenario.');
  }
  assertSetHas(
    new Set(scenario.ciCommands),
    'node scripts/test/v0-8-windows-adapter-artifact-gate.mjs',
    'windows adapter artifact gate command is matrix-listed'
  );
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-windows-adapter-artifact-gate');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 Windows adapter artifact gate claim.');
  }
  assertEqual(claim.runtimeSurfaceCoverage.appTargets.state, 'refused-missing-artifacts', 'app target gate state');
  assertEqual(
    claim.runtimeSurfaceCoverage.domainNetworkTargets.state,
    'refused-missing-artifacts',
    'domain target gate state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.managedBrowserExactUrl.state,
    'refused-missing-artifacts',
    'managed browser gate state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.unmanagedBrowserExactUrl.state,
    'refused-unsupported-surface',
    'unmanaged browser gate state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.unsupportedOsTargets.state,
    'refused-unsupported-surface',
    'unsupported OS gate state'
  );
  proofLabels.push('proof-matrix.v0-8-windows-adapter-artifact-gate');
  proofLabels.push('v0.8.windows-adapter-artifact-gate.claim-upgrade-refusal');
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/windows_adapter_artifact_gate_tests.rs"]',
    'windows adapter artifact gate contract harness registration exists'
  );
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`))));
    child.once('error', reject);
  });
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

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
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
