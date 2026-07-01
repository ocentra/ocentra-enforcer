import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'notification-local-outbox-scheduler-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const schedulerDir = join(outputDir, 'local-outbox-scheduler');
const schedulerPath = join(schedulerDir, 'scheduler.jsonl');
const manifestPath = join(schedulerDir, 'manifest.json');
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(schedulerDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/unit/notification-local-outbox-scheduler-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  await assertPackageExport(proofModule);
  const readModel = proofModule.NotificationLocalOutboxSchedulerProofReadModel;
  const stateCounts = proofModule.summarizeNotificationLocalOutboxSchedulerStates(readModel.records);
  const channelCounts = proofModule.summarizeNotificationLocalOutboxSchedulerChannels(readModel.records);
  const parsedRecords = await writeAndReadScheduler(proofModule, readModel.records);
  const dueRecords = proofModule.dueNotificationLocalOutboxSchedulerRecords(readModel.records);
  const retryRecord = readModel.records.find((record) => record.schedulerState === 'retry-window-scheduled');
  const quietHoursRecord = readModel.records.find((record) => record.schedulerState === 'held-quiet-hours');

  assert.equal(parsedRecords.length, readModel.records.length);
  assert.equal(dueRecords.length, 1);
  assert.equal(dueRecords[0].nextAttemptAt, readModel.schedulerNowAt);
  assert.equal(quietHoursRecord.nextAttemptAt, quietHoursRecord.quietHoursWindow.endsAt);
  assert.equal(retryRecord.nextAttemptAt, retryRecord.retryWindow.opensAt);
  assert.equal(retryRecord.retryWindow.attemptNumber, 2);
  assert.equal(stateCounts['due-local'], 1);
  assert.equal(stateCounts['held-quiet-hours'], 1);
  assert.equal(stateCounts['retry-window-scheduled'], 1);
  assert.equal(stateCounts['dead-letter-review'], 1);
  assert.equal(stateCounts['receipt-required'], 1);
  assert.equal(stateCounts['manual-required'], 1);
  assert.equal(channelCounts.push, 1);
  assert.equal(channelCounts.email, 1);
  assert.equal(channelCounts.sms, 1);
  assert.equal(channelCounts.whatsapp, 1);
  assert.equal(channelCounts['in-app'], 2);
  assert.equal(readModel.providerDeliveryRuntimeClaimed, false);
  assert.equal(readModel.providerReceiptIngestionClaimed, false);
  assert.equal(readModel.providerCredentialsClaimed, false);
  assert.equal(readModel.cloudRoutingClaimed, false);
  assert.equal(readModel.parentNotificationUiClaimed, false);
  assert.equal(readModel.retryExecutionRuntimeClaimed, false);
  assert.equal(readModel.quietHoursTimerRuntimeClaimed, false);
  assert.equal(readModel.productionDurableOutboxStorageClaimed, false);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/notification-local-outbox-scheduler-proof.ts',
      schemas: 'packages/schema-domain/src/notification-local-outbox-scheduler-proof-schemas.ts',
      guards: 'packages/schema-domain/src/notification-local-outbox-scheduler-proof-guards.ts',
      values: 'packages/schema-domain/src/notification-local-outbox-scheduler-proof-values.ts',
      contractTest: 'packages/schema-domain/tests/unit/notification-local-outbox-scheduler-proof.test.ts',
      builtModule: 'packages/schema-domain/dist/notification-local-outbox-scheduler-proof.js',
      packageExport: '@ocentra-parent/schema-domain/notification-local-outbox-scheduler-proof',
      featureDoc: 'docs/features/reports-notifications-sync.md',
      expectationDoc: 'docs/expectations/notifications.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      schedulerJsonl: relativePath(schedulerPath),
      schedulerManifest: relativePath(manifestPath),
      output: relativePath(proofPath),
    },
    stateCounts,
    channelCounts,
    dueSourceEntries: dueRecords.map((record) => record.sourceEntryId),
    deterministicWindows: {
      schedulerNowAt: readModel.schedulerNowAt,
      quietHoursNextAttemptAt: quietHoursRecord.nextAttemptAt,
      retryNextAttemptAt: retryRecord.nextAttemptAt,
      retryWindow: retryRecord.retryWindow,
    },
    nonClaims: readModel.nonClaims,
    schedulerArtifact: {
      rootRef: readModel.schedulerArtifactRootRef,
      recordsWritten: parsedRecords.length,
      schedulerFile: relativePath(schedulerPath),
      manifest: relativePath(manifestPath),
    },
    claimBoundaries: {
      providerDeliveryRuntimeClaimed: readModel.providerDeliveryRuntimeClaimed,
      providerReceiptIngestionClaimed: readModel.providerReceiptIngestionClaimed,
      providerCredentialsClaimed: readModel.providerCredentialsClaimed,
      cloudRoutingClaimed: readModel.cloudRoutingClaimed,
      parentNotificationUiClaimed: readModel.parentNotificationUiClaimed,
      retryExecutionRuntimeClaimed: readModel.retryExecutionRuntimeClaimed,
      quietHoursTimerRuntimeClaimed: readModel.quietHoursTimerRuntimeClaimed,
      productionDurableOutboxStorageClaimed: readModel.productionDurableOutboxStorageClaimed,
    },
    knownGaps: proofModule.NotificationLocalOutboxSchedulerKnownGaps,
    claimsProved: [
      'Local notification outbox scheduler records are schema-validated and written to a deterministic parent-owned JSONL artifact',
      'Due, quiet-hours held, retry-window scheduled, dead-letter review, receipt-required, and manual-required scheduler states are represented',
      'Deterministic nextAttemptAt and retry window behavior is asserted from the read model and reread artifact',
      'Push, email, SMS, WhatsApp, and in-app channels remain provider abstractions without delivery execution',
      'Provider delivery, receipt ingestion, credentials, cloud routing, UI, retry worker execution, quiet-hours timer execution, and durable production outbox claims remain false',
    ],
    claimsNotProved: [
      'external push/email/SMS/WhatsApp/in-app provider delivery',
      'provider webhook receipt ingestion',
      'production retry worker execution',
      'production quiet-hours timer loop',
      'parent notification UI, history, or preferences screen',
      'cloud notification routing',
      'durable production outbox storage',
      'raw child evidence, URLs, titles, message text, screenshots, reports, credentials, or provider metadata storage',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'notification-local-outbox-scheduler-proof.js'
  );
  return import(pathToFileURL(modulePath).href);
}

async function assertPackageExport(proofModule) {
  const schemaPackageJson = JSON.parse(
    await readFile(join(repoRoot, 'packages', 'schema-domain', 'package.json'), 'utf8')
  );
  assert.deepEqual(schemaPackageJson.exports['./notification-local-outbox-scheduler-proof'], {
    import: './dist/notification-local-outbox-scheduler-proof.js',
    types: './dist/notification-local-outbox-scheduler-proof.d.ts',
  });

  const exportedModule = await import('@ocentra-parent/schema-domain/notification-local-outbox-scheduler-proof');
  assert.equal(
    exportedModule.NotificationLocalOutboxSchedulerProofReadModel.schemaVersion,
    proofModule.NotificationLocalOutboxSchedulerProofReadModel.schemaVersion
  );
}

async function writeAndReadScheduler(proofModule, records) {
  const serialized = `${records.map((record) => JSON.stringify(record)).join('\n')}\n`;
  assertNoForbiddenDetails(serialized);
  await writeFile(schedulerPath, serialized);
  await writeFile(
    manifestPath,
    `${JSON.stringify(
      {
        proofMode,
        schedulerFile: relativePath(schedulerPath),
        recordCount: records.length,
        schedulerArtifactRef: records[0].schedulerArtifactRef,
        localDataPathRef: records[0].localDataPathRef,
        generatedAt: new Date().toISOString(),
      },
      null,
      2
    )}\n`
  );

  const rawRecords = (await readFile(schedulerPath, 'utf8'))
    .trim()
    .split('\n')
    .map((line) => JSON.parse(line));
  return rawRecords.map((record) => proofModule.NotificationLocalOutboxSchedulerRecordSchema.parse(record));
}

function assertNoForbiddenDetails(serialized) {
  const lowerSerialized = serialized.toLowerCase();
  for (const fragment of [
    'http://',
    'https://',
    'screenshot-bytes',
    'raw-title-value',
    'raw-message-body',
    'sqlite-private-path',
    'oauth-secret',
    'provider-token',
    'report-body',
  ]) {
    assert.equal(lowerSerialized.includes(fragment), false, `forbidden scheduler detail leaked: ${fragment}`);
  }
}

async function gitHead() {
  return (await commandOutput('git', ['rev-parse', 'HEAD'])).trim();
}

async function commandOutput(command, args) {
  const chunks = [];
  const child = spawn(command, args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
  child.stdout.on('data', (chunk) => chunks.push(chunk));
  child.stderr.on('data', (chunk) => chunks.push(chunk));
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  const output = Buffer.concat(chunks).toString('utf8');
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}\n${output}`);
  }
  return output;
}

async function runCommand(command, args) {
  const startedAt = new Date().toISOString();
  const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  commands.push({ command: `${command} ${args.join(' ')}`, startedAt, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}`);
  }
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
