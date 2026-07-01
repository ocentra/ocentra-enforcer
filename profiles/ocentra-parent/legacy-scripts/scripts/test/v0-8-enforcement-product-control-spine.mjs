import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-enforcement-product-control-spine');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'enforcement_product_control_spine']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'product_control']);
  await assertProtocolHarness();

  const { V08EnforcementProductControlSpineReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-enforcement-product-control-spine');
  const summary = summarizeReadModel(V08EnforcementProductControlSpineReadModel);

  assertReadModel(V08EnforcementProductControlSpineReadModel, summary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-enforcement-product-control-spine',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      generatedContract: 'packages/schema-domain/src/v0-8-enforcement-product-control-spine.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_product_control_spine.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/unit/enforcement_product_control_spine_tests.rs',
      rustServiceCommand: 'agent.enforcement.product-control-spine.get',
      rustServiceEvent: 'agent.enforcement.product-control-spine.reported',
      rustServiceReadModel:
        'crates/agent-service/src/enforcement_os_adapter_product_proof_read_model/product_control_spine.rs',
      rustServiceReadModelTest:
        'crates/agent-service/tests/unit/enforcement_os_adapter_product_proof_read_model_tests/product_control_spine_tests.rs',
      rustServiceApiTest:
        'crates/agent-service/tests/unit/enforcement_os_adapter_product_proof_read_model_tests/product_control_api_tests.rs',
      proofHarness: 'scripts/test/v0-8-enforcement-product-control-spine.mjs',
    },
    counts: summary,
    claimsProved: [
      'Parent-visible V0.8 product-control actions are typed per surface',
      'Rust service product-control read model links the spine to cross-platform, browser/domain, and OS-adapter proof sources',
      'The generated schema-domain output and Rust service command expose the service-backed product-control read model to runtime consumers',
      'Owned-process, app-time-limit, managed-browser-session, approval, restart, rollback, and policy preview states stay separated',
      'Broad app blocking, network/domain blocking, managed exact URL control, unmanaged exact URL evidence, permission-loss alerts, and tamper/uninstall alerts remain manual-required or not-claimed',
    ],
    claimsNotProved: [
      'broad installed-app blocking',
      'host network or domain blocking',
      'managed active-tab exact URL enforcement',
      'unmanaged browser exact URL evidence',
      'notification delivery for permission loss',
      'tamper resistance or uninstall hardening',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-enforcement-product-control-spine-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byClaimState: countBy(readModel.entries.map((entry) => entry.productClaimState)),
    byDevicePolicyState: countBy(readModel.entries.map((entry) => entry.devicePolicyState)),
    bySurfaceKind: countBy(readModel.entries.map((entry) => entry.surfaceKind)),
    broadAppBlockingClaimed: readModel.entries.filter((entry) => entry.broadAppBlockingClaimed).length,
    networkDomainBlockingClaimed: readModel.entries.filter((entry) => entry.networkDomainBlockingClaimed).length,
    managedExactUrlBlockingClaimed: readModel.entries.filter((entry) => entry.managedExactUrlBlockingClaimed).length,
    unmanagedExactUrlClaimed: readModel.entries.filter((entry) => entry.unmanagedExactUrlClaimed).length,
    tamperResistanceClaimed: readModel.entries.filter((entry) => entry.tamperResistanceClaimed).length,
    notificationDeliveryClaimed: readModel.entries.filter((entry) => entry.notificationDeliveryClaimed).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-enforcement-product-control-spine', 'read model id');
  assertEqual(summary.entries, 15, 'entry count');
  assertEqual(summary.byClaimState['implemented-boundary'], 6, 'implemented-boundary count');
  assertEqual(summary.byClaimState['degraded-boundary'], 1, 'degraded-boundary count');
  assertEqual(summary.byClaimState['dry-run-only'], 1, 'dry-run-only count');
  assertEqual(summary.byClaimState['manual-required'], 6, 'manual-required count');
  assertEqual(summary.byClaimState['not-claimed'], 1, 'not-claimed count');
  assertEqual(summary.byDevicePolicyState['control-capable'], 5, 'control-capable count');
  assertEqual(summary.byDevicePolicyState['preview-only'], 1, 'preview-only count');
  assertEqual(summary.byDevicePolicyState['report-only'], 2, 'report-only count');
  assertEqual(summary.byDevicePolicyState['manual-required'], 6, 'manual-required count');
  assertEqual(summary.byDevicePolicyState['not-claimed'], 1, 'not-claimed count');
  assertEqual(summary.broadAppBlockingClaimed, 0, 'broad app blocking claim count');
  assertEqual(summary.networkDomainBlockingClaimed, 0, 'network/domain blocking claim count');
  assertEqual(summary.managedExactUrlBlockingClaimed, 0, 'managed exact URL claim count');
  assertEqual(summary.unmanagedExactUrlClaimed, 0, 'unmanaged exact URL claim count');
  assertEqual(summary.tamperResistanceClaimed, 0, 'tamper resistance claim count');
  assertEqual(summary.notificationDeliveryClaimed, 0, 'notification delivery claim count');

  for (const expectedState of expectedSurfaceStates()) {
    assertSurfaceState(readModel, expectedState);
  }

  proofLabels.push('v0.8.product-control-spine.read-model');
  proofLabels.push('v0.8.product-control-spine.websocket-runtime-path');
  proofLabels.push('v0.8.product-control-spine.generated-contract');
  proofLabels.push('v0.8.product-control-spine.action-state-guard');
  proofLabels.push('v0.8.product-control-spine.no-claim-upgrade');
}

function expectedSurfaceStates() {
  return [
    ['windows-owned-process-time-limit', 'control-capable', ['observe', 'time-limit', 'block-scoped-process']],
    ['windows-app-time-limit-lifecycle', 'control-capable', ['observe', 'time-limit', 'ask-parent']],
    ['windows-managed-browser-session-intervention', 'control-capable', ['observe', 'warn', 'time-limit']],
    ['windows-unmanaged-browser-process-fallback', 'report-only', ['observe', 'warn', 'report-only']],
    ['windows-policy-dry-run-preview', 'preview-only', ['dry-run-preview', 'ask-parent']],
    ['windows-approval-override-audit', 'control-capable', ['ask-parent', 'report-only']],
    ['windows-restart-recovery-timer', 'control-capable', ['time-limit', 'report-only']],
    ['windows-rollback-audit-boundary', 'report-only', ['report-only']],
    ['windows-child-facing-explanation', 'manual-required', ['report-only']],
    ['windows-broad-app-blocking', 'manual-required', ['report-only']],
    ['windows-network-domain-blocking', 'manual-required', ['report-only']],
    ['windows-managed-exact-url-control', 'manual-required', ['report-only']],
    ['windows-unmanaged-exact-url-not-claimed', 'not-claimed', ['report-only']],
    ['windows-permission-loss-alerts', 'manual-required', ['report-only']],
    ['windows-tamper-uninstall-alerts', 'manual-required', ['report-only']],
  ];
}

function assertSurfaceState(readModel, [surface, devicePolicyState, parentVisibleActions]) {
  const entry = readModel.entries.find((candidate) => candidate.surface === surface);
  if (entry === undefined) {
    throw new Error(`product-control surface guard: missing ${surface}`);
  }
  assertEqual(entry.devicePolicyState, devicePolicyState, `${surface} devicePolicyState`);
  assertArrayEqual(entry.parentVisibleActions, parentVisibleActions, `${surface} parentVisibleActions`);
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

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/unit.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "unit/enforcement_product_control_spine_tests.rs"]',
    'enforcement product control spine unit harness registration exists'
  );
}

function assertArrayEqual(actual, expected, label) {
  assertEqual(actual.length, expected.length, `${label} length`);
  for (const [index, expectedValue] of expected.entries()) {
    assertEqual(actual[index], expectedValue, `${label}[${index}]`);
  }
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  commands.push(commandLine);
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${commandLine} exited with ${code}`));
      }
    });
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], {
      cwd: repoRoot,
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    child.stdout.on('data', (chunk) => chunks.push(chunk));
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`git rev-parse HEAD exited with ${code}`));
      }
    });
  });
  return Buffer.concat(chunks).toString('utf8').trim();
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
