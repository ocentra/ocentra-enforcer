import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'tamper-integrity-audit-contract-proof');
const proofPath = join(outputDir, 'proof.json');
const rustProofPath = join(outputDir, 'rust-proof.json');
const commands = [];
const timeline = [];

try {
  await main();
} catch (error) {
  record('proof-failed', {
    error: error instanceof Error ? error.message : String(error),
  });
  throw error;
}

async function main() {
  await mkdir(outputDir, { recursive: true });

  record('proof-start', {
    proofMode: 'tamper-integrity-audit-contract-proof',
    rustTestBinary: 'crates/agent-service/tests/enforcement_runtime.rs',
    rustProofArtifact: relative(repoRoot, rustProofPath),
  });

  await runCommand(
    'cargo',
    cargoCommand([
      'test',
      '-p',
      'ocentra-parent-agent-service',
      '--test',
      'enforcement_runtime',
      'enforcement_integrity_runtime_audit_read_model_writes_proof_artifact',
      '--quiet',
    ])
  );

  const rustProof = await readJson(rustProofPath);
  validateRustProof(rustProof);
  const commit = await gitHead();

  const proof = {
    schemaVersion: 1,
    proofMode: 'tamper-integrity-audit-contract-proof',
    checkedAt: new Date().toISOString(),
    commit,
    commands,
    timeline,
    evidence: {
      rustTestBinary: 'crates/agent-service/tests/enforcement_runtime.rs',
      rustProofArtifact: relative(repoRoot, rustProofPath),
      rustSourceFiles: [
        'crates/agent-service/src/enforcement_api/enforcement_integrity_runtime_audit_read_model.rs',
        'crates/agent-service/tests/unit/enforcement_integrity_runtime_audit_proof.rs',
        'crates/agent-protocol/src/constants/v08_enforcement_integrity_runtime_audit.rs',
      ],
    },
    claimsProved: [
      'Rust agent-service proof writes the tamper-integrity audit artifact from the enforcement runtime read model.',
      'Tamper manual-required rows still require service stop proof, uninstall detection artifact, and security review.',
      'Permission-loss, stale-heartbeat, adapter-unavailable, and mobile-unsupported rows remain non-success states.',
      'Bridge stopped-or-removed and notification delivered rows remain parent-visible non-delivery states with zero claim flags.',
    ],
    claimsNotProved: [
      'TS logging-domain ownership of tamper integrity proof truth',
      'uninstall detection as an implemented anti-tamper behavior',
      'provider delivery, tamper resistance, or delivered notification claims',
    ],
    rustProof,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  record('proof-complete', { proofOutput: relative(repoRoot, proofPath) });
}

function validateRustProof(rustProof) {
  assert.equal(rustProof.schemaVersion, 1);
  assert.equal(rustProof.proofMode, 'tamper-integrity-audit-contract-proof');
  assert.equal(rustProof.readModelId, 'v0-8-enforcement-integrity-runtime-audit');
  assert.equal(rustProof.entryCount, 14);
  assert.equal(rustProof.bridgeEntryCount, 4);
  assert.equal(rustProof.boundaryEntryCount, 5);

  assertProofCount(rustProof.resultCounts, 'succeeded', 1);
  assertProofCount(rustProof.resultCounts, 'failed', 2);
  assertProofCount(rustProof.resultCounts, 'unavailable', 3);
  assertProofCount(rustProof.resultCounts, 'expired', 1);
  assertProofCount(rustProof.resultCounts, 'rolled-back', 1);
  assertProofCount(rustProof.resultCounts, 'superseded', 1);
  assertProofCount(rustProof.resultCounts, 'no-op', 1);
  assertProofCount(rustProof.resultCounts, 'manual-required', 2);
  assertProofCount(rustProof.resultCounts, 'unsupported', 1);
  assertProofCount(rustProof.resultCounts, 'observe-only', 1);

  assertProofCount(rustProof.integrityCounts, 'running', 8);
  assertProofCount(rustProof.integrityCounts, 'permission-missing', 1);
  assertProofCount(rustProof.integrityCounts, 'adapter-unavailable', 1);
  assertProofCount(rustProof.integrityCounts, 'stale-heartbeat', 1);
  assertProofCount(rustProof.integrityCounts, 'tamper-signal-manual-required', 1);

  const negativeClaims = rustProof.negativeClaims ?? {};
  for (const [key, expected] of Object.entries({
    notificationDeliveryClaimed: 0,
    tamperHardeningClaimed: 0,
    mobilePrivilegeClaimed: 0,
    stealthPersistenceClaimed: 0,
    privilegeEscalationClaimed: 0,
    providerDeliveryClaimed: 0,
    tamperResistanceClaimed: 0,
    providerDeliveryObserved: 0,
    deliveredNotificationClaimed: 0,
  })) {
    assert.equal(negativeClaims[key], expected, `unexpected negative claim count for ${key}`);
  }

  assertRow(
    rustProof.rows,
    'tamper-uninstall-detection-manual-required',
    'manual-required',
    'tamper-signal-manual-required',
    [
      'service-manager stop proof',
      'uninstall detection artifact',
      'security review before hardening',
    ]
  );
  assertRow(
    rustProof.rows,
    'permission-loss-unavailable',
    'unavailable',
    'permission-missing',
    ['permission restoration artifact', 'operator-visible permission state']
  );
  assertRow(
    rustProof.rows,
    'stale-integrity-heartbeat',
    'unavailable',
    'stale-heartbeat',
    ['fresh heartbeat proof', 'parent-visible stale agent alert']
  );
  assertRow(
    rustProof.rows,
    'adapter-unavailable-recovery-needed',
    'unavailable',
    'adapter-unavailable',
    ['adapter recovery artifact', 'service restart recovery proof']
  );
  assertRow(
    rustProof.rows,
    'mobile-child-control-unsupported',
    'unsupported',
    'not-applicable',
    ['Family Controls entitlement artifact', 'DeviceActivity proof artifact']
  );

  assert.ok(
    rustProof.rows.every((row) => Object.values(row.claimFlags ?? {}).every((value) => value === false)),
    'audit row claim flags must remain false'
  );

  assert.ok(
    rustProof.bridgeRows.some((row) => row.bridgeEntryId === 'stopped-or-removed-alert-status'),
    'missing stopped-or-removed bridge row'
  );
  assert.ok(
    rustProof.bridgeRows.every(
      (row) => row.providerDeliveryClaimed === false && row.tamperResistanceClaimed === false
    ),
    'bridge claim flags must remain false'
  );
  assert.ok(
    rustProof.boundaryRows.some((row) => row.statusEntryId === 'notification-provider-delivered-receipt-required'),
    'missing delivered boundary row'
  );
  assert.ok(
    rustProof.boundaryRows.every(
      (row) => row.providerDeliveryObserved === false && row.deliveredNotificationClaimed === false
    ),
    'boundary claim flags must remain false'
  );
}

function record(step, data = {}) {
  const entry = {
    at: new Date().toISOString(),
    step,
    ...data,
  };
  timeline.push(entry);
  return entry;
}

async function readJson(filePath) {
  const content = await readFile(filePath, 'utf8');
  return JSON.parse(content);
}

function assertProofCount(counts, key, expected) {
  assert.equal(counts?.[key], expected, `unexpected count for ${key}`);
}

function assertRow(rows, auditEntryId, result, integrityState, manualProofRequirements) {
  const row = rows.find((candidate) => candidate.auditEntryId === auditEntryId);
  assert.ok(row, `missing row ${auditEntryId}`);
  assert.equal(row.result, result);
  assert.equal(row.integrityState, integrityState);
  assert.deepEqual(row.manualProofRequirements, manualProofRequirements);
}

async function runCommand(commandName, args) {
  const commandLine = [commandName, ...args].join(' ');
  commands.push(commandLine);
  record('proof-command-start', { command: commandLine });
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, {
      cwd: repoRoot,
      stdio: 'inherit',
      windowsHide: true,
    });
    child.once('exit', (code) =>
      code === 0
        ? resolve()
        : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
  record('proof-command-complete', { command: commandLine });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))
    );
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function cargoCommand(args) {
  return args;
}
