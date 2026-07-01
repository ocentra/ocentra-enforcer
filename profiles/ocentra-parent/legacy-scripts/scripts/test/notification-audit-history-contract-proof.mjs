import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'notification-audit-history-contract-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/logging-domain']));
  await runCommand(
    ...npmCommand(['run', 'test', '--workspace', '@ocentra-parent/logging-domain', '--', 'notification-audit-history'])
  );

  const { NotificationAuditHistoryReadModel } =
    await import('@ocentra-parent/schema-domain/notification-audit-history');
  const summary = summarizeReadModel(NotificationAuditHistoryReadModel);
  assertReadModel(NotificationAuditHistoryReadModel, summary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'notification-audit-history-contract-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsContract: 'packages/schema-domain/src/notification-audit-history.ts',
      tsContractTest: 'packages/logging-domain/tests/unit/notification-audit-history.test.ts',
      packageExport: 'packages/logging-domain/package.json#exports.notification-audit-history',
      featureDoc: 'docs/features/reports-notifications-sync.md',
      expectationDoc: 'docs/expectations/notifications.md',
      proofHarness: 'scripts/test/notification-audit-history-contract-proof.mjs',
      proofArtifact: 'test-results/notification-audit-history-contract-proof/proof.json',
    },
    counts: summary,
    claimsProved: [
      'Provider status audit/history rows cover queued, delivered, failed, unavailable, and manual-required states',
      'Retry lifecycle audit/history rows cover not-scheduled, receipt-required, retry-scheduled, manual-review, provider-unavailable, and quiet-hours-deferred states',
      'Receipt-required, manual-required, quiet-hours, and escalation references are present on matching rows',
      'Payload fields are limited to redaction-safe operational refs and status fields',
      'Child activity data remains outside Ocentra-hosted custody in this logging contract',
    ],
    claimsNotProved: [
      'provider adapter implementation',
      'provider send execution',
      'provider retry execution',
      'provider webhook receipt ingestion',
      'notification history UI',
      'provider credential readiness',
      'Ocentra-hosted storage of child activity, raw evidence, reports, URLs, titles, messages, screenshots, or provider child evidence',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`notification-audit-history-contract-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byProviderStatus: countBy(readModel.entries.map((entry) => entry.providerStatus)),
    byRetryLifecycleState: countBy(readModel.entries.map((entry) => entry.retryLifecycleState)),
    byQuietHoursState: countBy(readModel.entries.map((entry) => entry.quietHoursState)),
    byEscalationState: countBy(readModel.entries.map((entry) => entry.escalationState)),
    redactionSafePayloadFieldSets: new Set(readModel.entries.map((entry) => entry.redactionSafePayloadFields.join('|')))
      .size,
    providerAdapterImplemented: countTrue(readModel.entries, 'providerAdapterImplemented'),
    sendAttemptExecuted: countTrue(readModel.entries, 'sendAttemptExecuted'),
    retryExecutionObserved: countTrue(readModel.entries, 'retryExecutionObserved'),
    webhookReceiptIngested: countTrue(readModel.entries, 'webhookReceiptIngested'),
    providerCredentialPresent: countTrue(readModel.entries, 'providerCredentialPresent'),
    notificationHistoryUiClaimed: countTrue(readModel.entries, 'notificationHistoryUiClaimed'),
    rawChildDataIncluded: countTrue(readModel.entries, 'rawChildDataIncluded'),
    rawEvidencePayloadIncluded: countTrue(readModel.entries, 'rawEvidencePayloadIncluded'),
    sensitiveProviderPayloadIncluded: countTrue(readModel.entries, 'sensitiveProviderPayloadIncluded'),
    ocentraHostedChildDataStored: countTrue(readModel.entries, 'ocentraHostedChildDataStored'),
    providerStoresChildEvidenceClaimed: countTrue(readModel.entries, 'providerStoresChildEvidenceClaimed'),
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'notification-audit-history-contract-proof', 'read model id');
  assertEqual(summary.entries, 6, 'entry count');
  assertEqual(summary.byProviderStatus.queued, 1, 'queued provider status count');
  assertEqual(summary.byProviderStatus.delivered, 1, 'delivered provider status count');
  assertEqual(summary.byProviderStatus.failed, 2, 'failed provider status count');
  assertEqual(summary.byProviderStatus.unavailable, 1, 'unavailable provider status count');
  assertEqual(summary.byProviderStatus['manual-required'], 1, 'manual-required provider status count');
  assertEqual(summary.byRetryLifecycleState['not-scheduled'], 1, 'not-scheduled retry lifecycle count');
  assertEqual(summary.byRetryLifecycleState['receipt-required-contract'], 1, 'receipt-required retry lifecycle count');
  assertEqual(summary.byRetryLifecycleState['retry-scheduled-contract'], 1, 'retry-scheduled lifecycle count');
  assertEqual(summary.byRetryLifecycleState['manual-review-required'], 1, 'manual-review lifecycle count');
  assertEqual(summary.byRetryLifecycleState['provider-unavailable'], 1, 'provider-unavailable lifecycle count');
  assertEqual(
    summary.byRetryLifecycleState['quiet-hours-deferred-contract'],
    1,
    'quiet-hours-deferred lifecycle count'
  );
  assertEqual(summary.byQuietHoursState.allow, 2, 'quiet-hours allow count');
  assertEqual(summary.byQuietHoursState['defer-noncritical'], 2, 'quiet-hours defer count');
  assertEqual(summary.byQuietHoursState['manual-required'], 1, 'quiet-hours manual count');
  assertEqual(summary.byQuietHoursState.unavailable, 1, 'quiet-hours unavailable count');
  assertEqual(summary.byEscalationState.none, 1, 'escalation none count');
  assertEqual(summary.byEscalationState['waiting-window'], 2, 'escalation waiting count');
  assertEqual(summary.byEscalationState['manual-required'], 2, 'escalation manual count');
  assertEqual(summary.byEscalationState.unavailable, 1, 'escalation unavailable count');
  assertEqual(summary.redactionSafePayloadFieldSets, 1, 'redaction-safe field set count');
  assertEqual(summary.providerAdapterImplemented, 0, 'provider adapter implementation claim count');
  assertEqual(summary.sendAttemptExecuted, 0, 'send attempt execution claim count');
  assertEqual(summary.retryExecutionObserved, 0, 'retry execution observation claim count');
  assertEqual(summary.webhookReceiptIngested, 0, 'webhook receipt ingestion claim count');
  assertEqual(summary.providerCredentialPresent, 0, 'provider credential readiness claim count');
  assertEqual(summary.notificationHistoryUiClaimed, 0, 'notification history UI claim count');
  assertEqual(summary.rawChildDataIncluded, 0, 'raw child data claim count');
  assertEqual(summary.rawEvidencePayloadIncluded, 0, 'raw evidence payload claim count');
  assertEqual(summary.sensitiveProviderPayloadIncluded, 0, 'sensitive provider payload claim count');
  assertEqual(summary.ocentraHostedChildDataStored, 0, 'Ocentra-hosted child data storage claim count');
  assertEqual(summary.providerStoresChildEvidenceClaimed, 0, 'provider child evidence storage claim count');

  const delivered = entryFor(readModel, 'notification-audit-delivered-receipt-required');
  assertArrayEqual(delivered.receiptRefs, ['provider-receipt-required-ref'], 'delivered receipt refs');
  const manual = entryFor(readModel, 'notification-audit-manual-quiet-hours-deferred');
  assertArrayEqual(manual.manualRequiredRefs, ['quiet-hours-parent-preference-required-ref'], 'manual refs');
  assertArrayEqual(manual.quietHoursRefs, ['quiet-hours-defer-noncritical-ref'], 'quiet-hours refs');
  assertArrayEqual(manual.escalationRefs, ['escalation-waiting-window-ref'], 'escalation refs');

  proofLabels.push('notification-audit-history.provider-status-coverage');
  proofLabels.push('notification-audit-history.retry-lifecycle-coverage');
  proofLabels.push('notification-audit-history.receipt-manual-quiet-hours-escalation-refs');
  proofLabels.push('notification-audit-history.redaction-safe-payload-fields');
  proofLabels.push('notification-audit-history.no-child-data-custody-claim');
  proofLabels.push('notification-audit-history.no-provider-runtime-claim');
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

function countTrue(entries, key) {
  return entries.filter((entry) => entry[key] === true).length;
}

function entryFor(readModel, auditEntryId) {
  const entry = readModel.entries.find((candidate) => candidate.auditEntryId === auditEntryId);
  if (entry === undefined) {
    throw new Error(`missing notification audit history entry: ${auditEntryId}`);
  }
  return entry;
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertArrayEqual(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
