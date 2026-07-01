import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-os-adapter-product-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'enforcement_os_adapter_product_proof']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_os_adapter_product_proof_read_model',
  ]);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'enforcement_timer']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_execute_reports_manual_required_service_states_for_unwired_adapters',
  ]);
  await assertProtocolHarness();

  const { V08OsAdapterProductProofReadModel } =
    await import('@ocentra-parent/schema-domain/enforcement-os-adapter-product-proof');
  const proofSummary = summarizeReadModel(V08OsAdapterProductProofReadModel);
  assertReadModel(V08OsAdapterProductProofReadModel, proofSummary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-os-adapter-product-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsContract: 'packages/schema-domain/src/enforcement-os-adapter-product-proof.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_os_adapter_product_proof.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/enforcement_os_adapter_product_proof_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/enforcement_os_adapter_product_proof_read_model.rs',
      rustServiceReadModelTest: 'crates/agent-service/tests/unit/enforcement_os_adapter_product_proof_read_model_tests.rs',
      proofHarness: 'scripts/test/v0-8-os-adapter-product-proof.mjs',
    },
    counts: proofSummary,
    claimsProved: [
      'typed V0.8 OS-adapter product proof read model compiles in TypeScript and Rust protocol',
      'service read model links readiness, capability, artifact-gate, timer recovery, parent cancel, audit, and rollback rows',
      'owned-process and app time-limit proof states stay separate from broad app/domain/browser blocking claims',
      'unmanaged browser support remains process-only and exact URL evidence remains not-claimed',
      'claim upgrade flags remain false for every row until real OS/browser artifacts exist',
    ],
    claimsNotProved: [
      'global OS app blocking',
      'network or domain blocking on the host',
      'managed-browser exact URL enforcement',
      'unmanaged-browser exact URL, active tab, title, download source, page content, HTTPS content, or intent',
      'admin hardening, anti-tamper, bypass resistance, production signing, store release, or mobile OS enforcement',
    ],
    runtimeWorkersStillNeedToWire: [
      'real OS-approved app/package blocking adapter and rollback artifacts',
      'real host network/domain filter adapter and rollback artifacts',
      'managed browser exact URL apply/audit bridge artifacts',
      'admin hardening and anti-tamper evidence before broad rollback claims upgrade',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-os-adapter-product-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byReadinessState: countBy(readModel.entries.map((entry) => entry.readinessState)),
    byCapabilityState: countBy(readModel.entries.map((entry) => entry.capabilityState)),
    byAuditState: countBy(readModel.entries.map((entry) => entry.auditState)),
    byTimerRecoveryState: countBy(readModel.entries.map((entry) => entry.timerRecoveryState)),
    byParentOverrideState: countBy(readModel.entries.map((entry) => entry.parentOverrideState)),
    claimUpgradeAllowed: readModel.entries.filter((entry) => entry.claimUpgradeAllowed).length,
    broadBlockingClaimed: readModel.entries.filter((entry) => entry.broadBlockingClaimed).length,
    exactUrlClaimed: readModel.entries.filter((entry) => entry.exactUrlClaimed).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(summary.entries, 12, 'product proof entry count');
  assertEqual(summary.byReadinessState.implemented, 6, 'implemented proof count');
  assertEqual(summary.byReadinessState['manual-required'], 5, 'manual-required proof count');
  assertEqual(summary.byReadinessState['not-claimed'], 1, 'not-claimed proof count');
  assertEqual(summary.claimUpgradeAllowed, 0, 'claim upgrade count');
  assertEqual(summary.broadBlockingClaimed, 0, 'broad blocking claim count');
  assertEqual(summary.exactUrlClaimed, 0, 'exact URL claim count');

  const surfaces = new Set(readModel.entries.map((entry) => entry.surface));
  for (const expectedSurface of [
    'owned-process-terminate',
    'app-time-limit-lifecycle',
    'broad-app-blocking',
    'network-domain-blocking',
    'managed-browser-service-command',
    'managed-browser-exact-url',
    'unmanaged-browser-process-only',
    'unmanaged-browser-exact-evidence',
    'restart-recovery',
    'parent-cancel-override',
    'audit-custody',
    'rollback-artifact-gate',
  ]) {
    assertSetHas(surfaces, expectedSurface, 'product proof surface');
  }
  proofLabels.push('v0.8.os-adapter-product-proof.contract-counts');
  proofLabels.push('v0.8.os-adapter-product-proof.claim-upgrade-refusal');
  proofLabels.push('v0.8.os-adapter-product-proof.lifecycle-audit-parent-cancel-read-model');
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
    '#[path = "contract/enforcement_os_adapter_product_proof_tests.rs"]',
    'os adapter product contract harness registration exists'
  );
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
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
