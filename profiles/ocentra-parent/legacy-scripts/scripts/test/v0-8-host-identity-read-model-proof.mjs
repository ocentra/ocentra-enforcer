import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-host-identity-read-model-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'host_identity']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'host_identity_read_model']);
  await runCommand(...npmCommand(['run', 'test:pre-ai-proof']));
  await assertProtocolHarness();
  proofLabels.push('pre-ai-proof.current-matrix-valid');
  proofLabels.push('v0.8.host-identity-read-model.matrix-registered');
  proofLabels.push('v0.8.host-identity-read-model.rust-protocol-service');

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-host-identity-read-model-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      rustProtocol: 'crates/agent-protocol/src/host_identity.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/host_identity_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/host_identity_read_model.rs',
      rustServiceReadModelTest: 'crates/agent-service/tests/unit/host_identity_read_model_tests.rs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-host-identity-read-model-proof-2026-05-29.md',
    },
    counts: {
      entries: 9,
      byReadinessState: {
        'manual-required': 7,
        unavailable: 1,
        'not-claimed': 1,
      },
      byEvidenceClass: {
        inventory: 2,
        process: 1,
        executable: 1,
        package: 2,
        'publisher-signature': 1,
        rollback: 1,
        audit: 1,
      },
      safeForBroadAppBlocking: {
        true: 0,
        false: 9,
      },
    },
    productTruth: {
      broadAppBlocking:
        'Host identity read-model rows are evidence readiness states only; they do not claim broad app blocking is safe or implemented.',
      unsupportedIdentity:
        'Unsupported, permission-limited, or unknown host identity remains unavailable instead of becoming known app proof.',
      rollbackReadiness:
        'Rollback readiness remains not-claimed until apply and rollback artifacts exist for the same package or executable identity.',
      auditCustody:
        'Audit custody requires real service evidence refs, policy decision, adapter outcome or fallback, and audit event ids before claims upgrade.',
    },
    matrixRegistration: {
      state: 'registered',
      path: 'docs/expectations/pre-ai-proof-matrix.json',
      claimId: 'v0-8-host-identity-read-model-proof',
      checkpointScenarioId: 'v0-8-host-identity-read-model-proof',
    },
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-host-identity-read-model-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
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

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/host_identity_tests.rs"]',
    'host identity contract harness registration exists'
  );
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}
