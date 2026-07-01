import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-enforcement-integrity-runtime-audit');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'enforcement_integrity_runtime_audit']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'integrity_alert_status_bridge']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'enforcement_integrity_runtime_audit']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'integrity_alert_status_bridge']);
  await assertProtocolHarness();

  const { V08EnforcementIntegrityRuntimeAuditReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-enforcement-integrity-runtime-audit');
  const summary = summarizeReadModel(V08EnforcementIntegrityRuntimeAuditReadModel);

  assertReadModel(V08EnforcementIntegrityRuntimeAuditReadModel, summary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-enforcement-integrity-runtime-audit',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      generatedContract: 'packages/schema-domain/src/v0-8-enforcement-integrity-runtime-audit.ts',
      generatedSupportedAdapterExport: 'packages/schema-domain/src/v0-8-supported-adapter-runtime-proof.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_integrity_runtime_audit.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/enforcement_integrity_runtime_audit_tests.rs',
      rustServiceReadModel:
        'crates/agent-service/src/enforcement_api/enforcement_integrity_runtime_audit_read_model.rs',
      rustServiceTest:
        'crates/agent-service/tests/unit/enforcement_integrity_runtime_audit_read_model_tests.rs',
      rustServiceEventPayload:
        'agent.enforcement.supported-adapter-runtime-proof.reported:enforcementIntegrityRuntimeAuditReadModel',
      proofHarness: 'scripts/test/v0-8-enforcement-integrity-runtime-audit.mjs',
    },
    counts: summary,
    claimsProved: [
      'Enforcement action/result/audit outcomes are represented in generated schema-domain output',
      'Rust protocol mirrors the audit read model state values and non-claim flags',
      'The service includes the integrity audit read model in the supported-adapter runtime proof WebSocket event',
      'Dry-run, observe-only, stale, wrong-device, manual-required, unavailable, recovery-needed, and unsupported paths do not execute adapters',
      'Timer, rollback, child-status, parent-override, permission-loss, heartbeat, and tamper states stay explicit',
      'Broad app blocking, host network/domain blocking, exact active-tab enforcement, notification delivery, tamper hardening, mobile privilege, stealth persistence, and privilege escalation remain unclaimed',
    ],
    claimsNotProved: [
      'global installed-app blocking',
      'host network or domain blocking',
      'managed active-tab exact URL enforcement',
      'notification delivery',
      'tamper hardening or uninstall resistance',
      'mobile child-device enforcement',
      'service restart timer persistence beyond recovery-needed state',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-enforcement-integrity-runtime-audit-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    integrityAlertStatusBridgeEntries: readModel.integrityAlertStatusBridge.entries.length,
    integrityAlertStatusBridgeStates: countBy(
      readModel.integrityAlertStatusBridge.entries.map((entry) => entry.integrityAlertState)
    ),
    byPlatform: countBy(readModel.entries.map((entry) => entry.platform)),
    byResult: countBy(readModel.entries.map((entry) => entry.result)),
    byExecution: countBy(readModel.entries.map((entry) => entry.execution)),
    byIntegrityState: countBy(readModel.entries.map((entry) => entry.integrityState)),
    broadInstalledAppBlockingClaimed: readModel.entries.filter((entry) => entry.broadInstalledAppBlockingClaimed)
      .length,
    hostNetworkDomainBlockingClaimed: readModel.entries.filter((entry) => entry.hostNetworkDomainBlockingClaimed)
      .length,
    exactActiveTabEnforcementClaimed: readModel.entries.filter((entry) => entry.exactActiveTabEnforcementClaimed)
      .length,
    notificationDeliveryClaimed: readModel.entries.filter((entry) => entry.notificationDeliveryClaimed).length,
    tamperHardeningClaimed: readModel.entries.filter((entry) => entry.tamperHardeningClaimed).length,
    mobilePrivilegeClaimed: readModel.entries.filter((entry) => entry.mobilePrivilegeClaimed).length,
    stealthPersistenceClaimed: readModel.entries.filter((entry) => entry.stealthPersistenceClaimed).length,
    privilegeEscalationClaimed: readModel.entries.filter((entry) => entry.privilegeEscalationClaimed).length,
    integrityAlertProviderDeliveryClaimed: readModel.integrityAlertStatusBridge.entries.filter(
      (entry) => entry.providerDeliveryClaimed
    ).length,
    integrityAlertTamperResistanceClaimed: readModel.integrityAlertStatusBridge.entries.filter(
      (entry) => entry.tamperResistanceClaimed
    ).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-enforcement-integrity-runtime-audit', 'read model id');
  assertEqual(summary.entries, 14, 'entry count');
  assertEqual(summary.integrityAlertStatusBridgeEntries, 4, 'integrity alert bridge entry count');
  assertEqual(summary.integrityAlertStatusBridgeStates['permission-loss'], 1, 'permission alert count');
  assertEqual(summary.integrityAlertStatusBridgeStates['stale-heartbeat'], 1, 'stale alert count');
  assertEqual(summary.integrityAlertStatusBridgeStates['stopped-or-removed'], 1, 'stopped alert count');
  assertEqual(summary.integrityAlertStatusBridgeStates['tamper-manual-required'], 1, 'tamper alert count');
  assertEqual(summary.byPlatform.windows, 13, 'Windows entry count');
  assertEqual(summary.byPlatform.ios, 1, 'iOS entry count');
  assertEqual(summary.byResult.succeeded, 1, 'succeeded count');
  assertEqual(summary.byResult.failed, 2, 'failed count');
  assertEqual(summary.byResult.unavailable, 3, 'unavailable count');
  assertEqual(summary.byResult.expired, 1, 'expired count');
  assertEqual(summary.byResult['rolled-back'], 1, 'rolled-back count');
  assertEqual(summary.byResult.superseded, 1, 'superseded count');
  assertEqual(summary.byResult['no-op'], 1, 'no-op count');
  assertEqual(summary.byResult['manual-required'], 2, 'manual-required count');
  assertEqual(summary.byResult.unsupported, 1, 'unsupported count');
  assertEqual(summary.byResult['observe-only'], 1, 'observe-only count');
  assertEqual(summary.byExecution['executed-supported-boundary'], 4, 'executed-supported-boundary count');
  assertEqual(summary.byExecution['dry-run-no-adapter-execution'], 1, 'dry-run no-execution count');
  assertEqual(summary.byExecution['rejected-before-adapter'], 2, 'rejected-before-adapter count');
  assertEqual(summary.byExecution['observe-only-no-execution'], 1, 'observe-only no-execution count');
  assertEqual(summary.byExecution['manual-required-no-execution'], 2, 'manual-required no-execution count');
  assertEqual(summary.byExecution['unavailable-no-execution'], 2, 'unavailable no-execution count');
  assertEqual(summary.byExecution['recovery-needed-no-execution'], 1, 'recovery-needed no-execution count');
  assertEqual(summary.byExecution['unsupported-no-execution'], 1, 'unsupported no-execution count');
  assertEqual(summary.byIntegrityState.running, 8, 'running integrity count');
  assertEqual(summary.byIntegrityState['permission-missing'], 1, 'permission-missing count');
  assertEqual(summary.byIntegrityState['adapter-unavailable'], 1, 'adapter-unavailable count');
  assertEqual(summary.byIntegrityState['stale-heartbeat'], 1, 'stale-heartbeat count');
  assertEqual(summary.byIntegrityState['tamper-signal-manual-required'], 1, 'tamper manual count');
  assertEqual(summary.broadInstalledAppBlockingClaimed, 0, 'broad app claim count');
  assertEqual(summary.hostNetworkDomainBlockingClaimed, 0, 'host network/domain claim count');
  assertEqual(summary.exactActiveTabEnforcementClaimed, 0, 'exact active-tab claim count');
  assertEqual(summary.notificationDeliveryClaimed, 0, 'notification claim count');
  assertEqual(summary.tamperHardeningClaimed, 0, 'tamper hardening claim count');
  assertEqual(summary.mobilePrivilegeClaimed, 0, 'mobile privilege claim count');
  assertEqual(summary.stealthPersistenceClaimed, 0, 'stealth persistence claim count');
  assertEqual(summary.privilegeEscalationClaimed, 0, 'privilege escalation claim count');
  assertEqual(summary.integrityAlertProviderDeliveryClaimed, 0, 'integrity alert provider delivery claim count');
  assertEqual(summary.integrityAlertTamperResistanceClaimed, 0, 'integrity alert tamper claim count');

  for (const sourceReadModelId of [
    'v0-8-supported-adapter-runtime-proof',
    'v0-8-enforcement-policy-dispatch-proof',
    'v0-8-enforcement-product-control-spine',
    'enforcement-audit-journal',
    'enforcement-timer-recovery-state',
  ]) {
    assertSetHas(new Set(readModel.sourceReadModelIds), sourceReadModelId, 'source read model ids');
  }

  assertEntry(readModel, 'app-time-limit-action-succeeded', {
    result: 'succeeded',
    execution: 'executed-supported-boundary',
  });
  assertEntry(readModel, 'app-time-limit-action-expired', {
    result: 'expired',
    execution: 'executed-supported-boundary',
  });
  assertEntry(readModel, 'app-time-limit-action-rolled-back', {
    result: 'rolled-back',
    execution: 'executed-supported-boundary',
  });
  assertEntry(readModel, 'parent-override-superseded-action', {
    result: 'superseded',
    execution: 'executed-supported-boundary',
  });
  assertEntry(readModel, 'network-domain-observe-only', {
    result: 'observe-only',
    execution: 'observe-only-no-execution',
  });
  assertEntry(readModel, 'adapter-unavailable-recovery-needed', {
    result: 'unavailable',
    execution: 'recovery-needed-no-execution',
  });
  assertEntry(readModel, 'tamper-uninstall-detection-manual-required', {
    result: 'manual-required',
    execution: 'manual-required-no-execution',
  });

  proofLabels.push('v0.8.enforcement-integrity-runtime-audit.contract-boundary');
  proofLabels.push('v0.8.enforcement-integrity-runtime-audit.protocol-parity');
  proofLabels.push('v0.8.enforcement-integrity-runtime-audit.service-event-payload');
  proofLabels.push('v0.8.enforcement-integrity-runtime-audit.alert-status-bridge');
  proofLabels.push('v0.8.enforcement-integrity-runtime-audit.no-execution-boundaries');
  proofLabels.push('v0.8.enforcement-integrity-runtime-audit.no-claim-upgrade');
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/enforcement_integrity_runtime_audit_tests.rs"]',
    'enforcement integrity runtime audit contract harness registration exists'
  );
  assertIncludes(
    harness,
    '#[path = "contract/integrity_alert_status_bridge_tests.rs"]',
    'integrity alert status bridge contract harness registration exists'
  );
}

function assertEntry(readModel, auditEntryId, expected) {
  const entry = readModel.entries.find((candidate) => candidate.auditEntryId === auditEntryId);
  if (entry === undefined) {
    throw new Error(`missing enforcement integrity audit entry ${auditEntryId}`);
  }
  assertEqual(entry.result, expected.result, `${auditEntryId} result`);
  assertEqual(entry.execution, expected.execution, `${auditEntryId} execution`);
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  commands.push(commandLine);
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${commandLine} exited with ${code}`))));
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
