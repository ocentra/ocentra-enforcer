import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-windows-adapter-capability-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'windows_adapter_capability']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'windows_adapter_capability_read_model']);
  await runCommand('node', ['scripts/test/v0-8-broad-os-adapter-proof-readiness.mjs']);
  await runCommand('node', ['scripts/test/v0-8-host-identity-read-model-proof.mjs']);
  await runCommand(...npmCommand(['run', 'test:pre-ai-proof']));
  await assertProtocolHarness();

  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-windows-adapter-capability-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      rustProtocol: 'crates/agent-protocol/src/windows_adapter_capability.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/windows_adapter_capability_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/windows_adapter_capability_read_model.rs',
      rustServiceReadModelTest: 'crates/agent-service/tests/unit/windows_adapter_capability_read_model_tests.rs',
      broadReadinessHarness: 'scripts/test/v0-8-broad-os-adapter-proof-readiness.mjs',
      hostIdentityHarness: 'scripts/test/v0-8-host-identity-read-model-proof.mjs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-windows-adapter-capability-proof-2026-05-29.md',
    },
    counts: {
      entries: 6,
      linkedReadinessRows: 8,
      linkedHostIdentityRows: 9,
      exactUrlClaimed: 0,
      broadBlockingClaimed: 0,
    },
    productTruth: {
      appTargets:
        'Windows app targets stay manual-required until host identity, apply, rollback, and audit artifacts exist.',
      domainNetworkTargets:
        'Domain and network targets stay manual-required or unavailable until a host network adapter proves apply and rollback.',
      managedBrowser: 'Managed-browser service commands stay manual-required and are not exact URL enforcement proof.',
      unmanagedBrowser:
        'Unmanaged-browser support is process-only; exact URL, active tab, title, download, page text, HTTPS content, and intent are not claimed.',
      unsupportedOs:
        'Unsupported OS targets are represented as unavailable instead of borrowing Windows adapter claims.',
      rollbackAudit:
        'Rollback and audit readiness stay manual-required until same-identity apply, rollback, and custody evidence exist.',
    },
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-windows-adapter-capability-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function assertProofMatrix(matrix) {
  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-windows-adapter-capability-proof',
    'windows adapter capability claim is required'
  );
  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-windows-adapter-capability-proof'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 Windows adapter capability checkpoint scenario.');
  }
  assertSetHas(
    new Set(scenario.ciCommands),
    'node scripts/test/v0-8-windows-adapter-capability-proof.mjs',
    'windows adapter capability command is matrix-listed'
  );
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-windows-adapter-capability-proof');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 Windows adapter capability claim.');
  }
  assertEqual(claim.runtimeSurfaceCoverage.appTargets.state, 'manual-required', 'app target state');
  assertEqual(claim.runtimeSurfaceCoverage.domainNetworkTargets.state, 'manual-required', 'domain target state');
  assertEqual(claim.runtimeSurfaceCoverage.managedBrowserTargets.state, 'manual-required', 'managed browser state');
  assertEqual(claim.runtimeSurfaceCoverage.unmanagedBrowserExactEvidence.state, 'not-claimed', 'exact evidence state');
  assertEqual(claim.runtimeSurfaceCoverage.unsupportedOsTargets.state, 'unavailable', 'unsupported OS state');
  proofLabels.push('proof-matrix.v0-8-windows-adapter-capability-proof');
  proofLabels.push('v0.8.windows-adapter-capability.claim-boundaries');
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/windows_adapter_capability_tests.rs"]',
    'windows adapter capability contract harness registration exists'
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
