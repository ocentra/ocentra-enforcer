import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-integrity-alert-status-bridge');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'integrity_alert_status_bridge']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'integrity_alert_status_bridge']);
  await assertProtocolHarness();

  const { V08IntegrityAlertStatusBridgeReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-integrity-alert-status-bridge');
  const summary = summarizeReadModel(V08IntegrityAlertStatusBridgeReadModel);

  assertReadModel(V08IntegrityAlertStatusBridgeReadModel, summary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-integrity-alert-status-bridge',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      generatedContract: 'packages/schema-domain/src/v0-8-integrity-alert-status-bridge.ts',
      rustProtocol: 'crates/agent-protocol/src/integrity_alert_status_bridge.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/integrity_alert_status_bridge_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/enforcement_api/integrity_alert_status_bridge_read_model.rs',
      rustServiceTest: 'crates/agent-service/tests/unit/integrity_alert_status_bridge_read_model_tests.rs',
      rustServiceEventPayload:
        'agent.enforcement.supported-adapter-runtime-proof.reported:enforcementIntegrityRuntimeAuditReadModel.integrityAlertStatusBridge',
      proofHarness: 'scripts/test/v0-8-integrity-alert-status-bridge.mjs',
    },
    counts: summary,
    claimsProved: [
      'Permission loss, stale heartbeat, stopped-or-removed, and tamper manual-required states are parent-visible status rows',
      'Each status row carries notification intent, notification status, audit, integrity, and drill-in references',
      'The supported-adapter runtime proof event exposes the bridge through the existing integrity audit read model payload',
      'Provider notification delivery, broad blocking, tamper resistance, mobile enforcement, stealth persistence, and privilege escalation remain unclaimed',
    ],
    claimsNotProved: [
      'notification provider delivery',
      'portal UI rendering',
      'anti-tamper or uninstall resistance',
      'stealth or persistence behavior',
      'privilege escalation',
      'broad app/browser/network blocking',
      'mobile child-device enforcement',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-integrity-alert-status-bridge-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byIntegrityAlertState: countBy(readModel.entries.map((entry) => entry.integrityAlertState)),
    byParentVisibleStatus: countBy(readModel.entries.map((entry) => entry.parentVisibleStatus)),
    byDeliveryState: countBy(readModel.entries.map((entry) => entry.deliveryState)),
    providerDeliveryClaimed: readModel.entries.filter((entry) => entry.providerDeliveryClaimed).length,
    broadBlockingClaimed: readModel.entries.filter((entry) => entry.broadBlockingClaimed).length,
    tamperResistanceClaimed: readModel.entries.filter((entry) => entry.tamperResistanceClaimed).length,
    mobileEnforcementClaimed: readModel.entries.filter((entry) => entry.mobileEnforcementClaimed).length,
    stealthPersistenceClaimed: readModel.entries.filter((entry) => entry.stealthPersistenceClaimed).length,
    privilegeEscalationClaimed: readModel.entries.filter((entry) => entry.privilegeEscalationClaimed).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-integrity-alert-status-bridge', 'read model id');
  assertEqual(summary.entries, 4, 'entry count');
  assertEqual(summary.byIntegrityAlertState['permission-loss'], 1, 'permission-loss count');
  assertEqual(summary.byIntegrityAlertState['stale-heartbeat'], 1, 'stale-heartbeat count');
  assertEqual(summary.byIntegrityAlertState['stopped-or-removed'], 1, 'stopped-or-removed count');
  assertEqual(summary.byIntegrityAlertState['tamper-manual-required'], 1, 'tamper manual count');
  assertEqual(summary.byDeliveryState['not-delivered-provider-not-configured'], 4, 'provider not configured count');
  assertEqual(summary.providerDeliveryClaimed, 0, 'provider delivery claim count');
  assertEqual(summary.broadBlockingClaimed, 0, 'broad blocking claim count');
  assertEqual(summary.tamperResistanceClaimed, 0, 'tamper resistance claim count');
  assertEqual(summary.mobileEnforcementClaimed, 0, 'mobile enforcement claim count');
  assertEqual(summary.stealthPersistenceClaimed, 0, 'stealth persistence claim count');
  assertEqual(summary.privilegeEscalationClaimed, 0, 'privilege escalation claim count');

  for (const bridgeEntryId of [
    'permission-loss-alert-status',
    'stale-heartbeat-alert-status',
    'stopped-or-removed-alert-status',
    'tamper-manual-alert-status',
  ]) {
    const entry = readModel.entries.find((candidate) => candidate.bridgeEntryId === bridgeEntryId);
    if (entry === undefined) {
      throw new Error(`missing integrity alert bridge entry ${bridgeEntryId}`);
    }
    if (
      entry.notificationIntentRefs.length === 0 ||
      entry.notificationStatusRefs.length === 0 ||
      entry.auditRefs.length === 0 ||
      entry.integrityRefs.length === 0 ||
      entry.drillInRefs.length === 0
    ) {
      throw new Error(`${bridgeEntryId}: missing required notification, audit, integrity, or drill-in refs`);
    }
  }

  proofLabels.push('v0.8.integrity-alert-status-bridge.contract-boundary');
  proofLabels.push('v0.8.integrity-alert-status-bridge.protocol-adapter');
  proofLabels.push('v0.8.integrity-alert-status-bridge.service-event-payload');
  proofLabels.push('v0.8.integrity-alert-status-bridge.no-provider-delivery-claim');
  proofLabels.push('v0.8.integrity-alert-status-bridge.no-tamper-or-privilege-claim');
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

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/integrity_alert_status_bridge_tests.rs"]',
    'integrity alert status bridge contract harness registration exists'
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
