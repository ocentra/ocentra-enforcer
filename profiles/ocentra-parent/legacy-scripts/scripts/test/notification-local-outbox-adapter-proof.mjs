import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'notification-local-outbox-adapter-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const outboxDir = join(outputDir, 'local-outbox');
const outboxPath = join(outboxDir, 'outbox.jsonl');
const manifestPath = join(outboxDir, 'manifest.json');
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outboxDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/unit/notification-local-outbox-adapter-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  await assertPackageExport(proofModule);
  const readModel = proofModule.NotificationLocalOutboxAdapterProofReadModel;
  const stateCounts = proofModule.summarizeNotificationLocalOutboxStates(readModel.records);
  const channelCounts = proofModule.summarizeNotificationLocalOutboxChannels(readModel.records);
  const parsedRecords = await writeAndReadOutbox(proofModule, readModel.records);

  assert.equal(parsedRecords.length, readModel.records.length);
  assert.equal(stateCounts['queued-local'], 1);
  assert.equal(stateCounts['deferred-quiet-hours'], 1);
  assert.equal(stateCounts['retry-scheduled'], 1);
  assert.equal(stateCounts['dead-lettered'], 1);
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

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/notification-local-outbox-adapter-proof.ts',
      schemas: 'packages/schema-domain/src/notification-local-outbox-adapter-proof-schemas.ts',
      values: 'packages/schema-domain/src/notification-local-outbox-adapter-proof-values.ts',
      contractTest: 'packages/schema-domain/tests/unit/notification-local-outbox-adapter-proof.test.ts',
      builtModule: 'packages/schema-domain/dist/notification-local-outbox-adapter-proof.js',
      packageExport: '@ocentra-parent/schema-domain/notification-local-outbox-adapter-proof',
      featureDoc: 'docs/features/reports-notifications-sync.md',
      expectationDoc: 'docs/expectations/notifications.md',
      localOutboxJsonl: relativePath(outboxPath),
      localOutboxManifest: relativePath(manifestPath),
      output: relativePath(proofPath),
    },
    stateCounts,
    channelCounts,
    nonClaims: readModel.nonClaims,
    outboxArtifact: {
      rootRef: readModel.outboxRootRef,
      recordsWritten: parsedRecords.length,
      outboxFile: relativePath(outboxPath),
      manifest: relativePath(manifestPath),
    },
    claimBoundaries: {
      providerDeliveryRuntimeClaimed: readModel.providerDeliveryRuntimeClaimed,
      providerReceiptIngestionClaimed: readModel.providerReceiptIngestionClaimed,
      providerCredentialsClaimed: readModel.providerCredentialsClaimed,
      cloudRoutingClaimed: readModel.cloudRoutingClaimed,
      parentNotificationUiClaimed: readModel.parentNotificationUiClaimed,
    },
    forbiddenDetailFragmentsRejected: proofModule.NotificationLocalOutboxForbiddenDetailFragments,
    knownGaps: proofModule.NotificationLocalOutboxKnownGaps,
    claimsProved: [
      'Local notification outbox records are schema-validated and written to a deterministic parent-owned JSONL artifact',
      'Queued, quiet-hours deferred, retry scheduled, dead-lettered, receipt-required, and manual-required states are represented',
      'Push, email, SMS, WhatsApp, and in-app provider channels are abstracted without binding policy to one provider',
      'Minimal alert envelopes carry refs, reason, severity, channel, and parent action links only',
      'Provider delivery, provider receipt ingestion, credentials, cloud routing, UI, and sensitive metadata claims remain false',
    ],
    claimsNotProved: [
      'external push/email/SMS/WhatsApp/in-app provider delivery',
      'provider webhook receipt ingestion',
      'retry scheduler execution',
      'parent notification UI or preferences screen',
      'cloud notification routing',
      'raw child evidence, URLs, titles, message text, screenshots, reports, credentials, or provider metadata storage',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'notification-local-outbox-adapter-proof.js');
  return import(pathToFileURL(modulePath).href);
}

async function assertPackageExport(proofModule) {
  const schemaPackageJson = JSON.parse(
    await readFile(join(repoRoot, 'packages', 'schema-domain', 'package.json'), 'utf8')
  );
  assert.deepEqual(schemaPackageJson.exports['./notification-local-outbox-adapter-proof'], {
    import: './dist/notification-local-outbox-adapter-proof.js',
    types: './dist/notification-local-outbox-adapter-proof.d.ts',
  });

  const exportedModule = await import('@ocentra-parent/schema-domain/notification-local-outbox-adapter-proof');
  assert.equal(
    exportedModule.NotificationLocalOutboxAdapterProofReadModel.schemaVersion,
    proofModule.NotificationLocalOutboxAdapterProofReadModel.schemaVersion
  );
}

async function writeAndReadOutbox(proofModule, records) {
  const serialized = `${records.map((record) => JSON.stringify(record)).join('\n')}\n`;
  assertNoForbiddenDetails(serialized, proofModule.NotificationLocalOutboxForbiddenDetailFragments);
  await writeFile(outboxPath, serialized);
  await writeFile(
    manifestPath,
    `${JSON.stringify(
      {
        proofMode,
        outboxFile: relativePath(outboxPath),
        recordCount: records.length,
        outboxFileRef: records[0].outboxFileRef,
        localDataPathRef: records[0].localDataPathRef,
        generatedAt: new Date().toISOString(),
      },
      null,
      2
    )}\n`
  );

  const rawRecords = (await readFile(outboxPath, 'utf8'))
    .trim()
    .split('\n')
    .map((line) => JSON.parse(line));
  return rawRecords.map((record) => proofModule.NotificationLocalOutboxRecordSchema.parse(record));
}

function assertNoForbiddenDetails(serialized, forbiddenFragments) {
  const lowerSerialized = serialized.toLowerCase();
  for (const fragment of forbiddenFragments) {
    assert.equal(lowerSerialized.includes(fragment), false, `forbidden outbox detail leaked: ${fragment}`);
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
