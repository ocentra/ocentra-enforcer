import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofRoot = join(repoRoot, 'output', 'tracking-plan-proof');
const workpackRoot = join(proofRoot, '32-journal-sqlite-and-read-model-proof');
const retentionDeleteProofPath = join(workpackRoot, '14-retention-delete-proof.json');
const validationCommandsPath = join(workpackRoot, '16-validation-commands.log');
const serviceProofPath = join(workpackRoot, '18-service-read-model-proof.json');
const productSurfaceProofPath = join(workpackRoot, '21-product-surface-summary-proof.json');
const proofSummaryPath = join(workpackRoot, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/agent-protocol-domain',
    '--',
    'vitest',
    'run',
    'tests/unit/generated-agent-protocol-contracts.test.ts',
  ]);
  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/portal-domain',
    '--',
    'vitest',
    'run',
    'tests/unit/contracts.test.ts',
    'tests/unit/tracking-status-panel.test.ts',
  ]);
  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/portal',
    '--',
    'vitest',
    'run',
    'tests/unit/tracking-status-panel.test.ts',
  ]);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'tracking_read_model']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'tracking_read_model']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'tracking_read_model']);
  await assertTrackingProtocolHarness();

  const checkedAt = new Date().toISOString();
  const trackingReadModelContract = await loadTrackingReadModelContract();
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    workpackId: '32-journal-sqlite-and-read-model-proof',
    proofMode: 'tracking-service-read-model',
    requiredProofTier: 'P2_HOSTED_CI',
    currentProofTier: 'P2_HOSTED_CI',
    currentStatus: 'proved',
    productClaimReady: false,
    commands,
    serviceBoundary: {
      command: trackingReadModelContract.command,
      event: trackingReadModelContract.event,
      payloadField: trackingReadModelContract.payloadField,
      sourceStore: 'ActivityStore SQLite activity_events',
      portalProjectionModel: 'packages/portal-domain/src/tracking-status-panel.ts',
      portalConsumer: 'apps/portal/src/TrackingStatusRoutePanel.tsx',
      coveredEventKinds: [
        'activity.location.observed',
        'activity.tracking.geofence-transition.evaluated',
        'activity.tracking.expected-place.evaluated',
        'activity.tracking.child-check-in.responded',
        'activity.tracking.retention.deleted',
      ],
      citationField: 'evidenceReferenceIds',
      portalCitationSurface: {
        consumer: 'trackingStatusLiveSummary',
        citationRows: 'TrackingStatusLiveCitation[]',
        citedFields: [
          'eventId',
          'observedAt',
          'deviceId',
          'platform',
          'observer',
          'kind',
          'subjectKind',
          'subjectId',
          'queryVisibility',
          'capabilityStatus',
          'evidenceReferenceIds',
          'deletedEvidenceReferenceIds',
        ],
        productClaimReady: false,
      },
      tombstoneReplay: {
        rowVisibilityField: 'queryVisibility',
        activeRowValue: 'active',
        tombstoneRowValue: 'tombstone',
        deletedAtField: 'deletedAt',
        deletedEvidenceReferenceIdsField: 'deletedEvidenceReferenceIds',
        summaryFields: [
          'activeRows',
          'tombstoneRows',
          'latestTombstoneEventId',
          'latestTombstoneObservedAt',
          'deletedEvidenceReferenceIds',
        ],
        retentionEventKind: 'activity.tracking.retention.deleted',
        sourceOfTruth: 'ActivityStore SQLite rows replayed from journaled ActivityEvent records',
      },
      activeProductSurfaceSummary: {
        latestActiveEventIdField: 'latestActiveEventId',
        latestActiveObservedAtField: 'latestActiveObservedAt',
        activeKindCountsField: 'activeKindCounts',
        activeDeviceCountsField: 'activeDeviceCounts',
        activeCapabilityStatusCountsField: 'activeCapabilityStatusCounts',
        sourceRows: 'same trackingReadModel.rows payload derived from ActivityStore SQLite rows',
        deletedHistoryBoundary:
          'counts include active rows only; retention tombstones remain in tombstone-specific fields',
        intendedConsumers: [
          'parent report summaries',
          'policy evidence drill-in',
          'future full parent/child tracking UI',
        ],
      },
    },
    proofArtifacts: {
      typescriptCommandEventContract: 'packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts',
      typescriptProtocolDefaults: 'packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts',
      rustProtocolReadModel: 'crates/agent-protocol/src/tracking_read_model.rs',
      rustCoreReadModel: 'crates/agent-core/src/activity_store_tracking.rs',
      rustCoreRows: 'crates/agent-core/src/activity_store_tracking_rows.rs',
      rustServiceDispatcher: 'crates/agent-service/src/websocket.rs',
      rustProtocolPayload: 'crates/agent-protocol/src/tracking_read_model_payload.rs',
      rustServiceTest: 'crates/agent-service/tests/tracking_read_model.rs',
      rustProtocolReadModelTest: 'crates/agent-protocol/tests/contract/tracking_read_model.rs',
      portalLiveState: 'packages/portal-domain/src/live-activity-state.ts',
      portalTrackingProjection: 'packages/portal-domain/src/tracking-status-panel.ts',
      portalTrackingSurface: 'apps/portal/src/TrackingStatusRoutePanel.tsx',
      portalTrackingSurfaceTest: 'apps/portal/tests/unit/tracking-status-panel.test.ts',
    },
    nonClaims: [
      'This proof does not claim Android or iOS physical background tracking behavior.',
      'This proof does not claim enrolled-device authority, production pilot readiness, or provider delivery.',
      'This proof claims live service-backed portal citation rows for the tracking read model, not complete parent/child tracking UI.',
    ],
    remainingGapsBeforeProductOrPrReady: [
      'Hosted portal screenshot, accessibility, and browser-to-service proof remain pending.',
      'Full UI/report/policy consumers for the active product-surface summary remain pending.',
      'Child-device UI and device permission screenshots remain pending.',
      'Android/iOS physical background geofence proof remains manual-required.',
      'Authority-enrolled and production-pilot proof remain absent.',
    ],
  };

  const productSurfaceProof = {
    schemaVersion: 1,
    checkedAt,
    commit: proof.commit,
    workpackId: '32-journal-sqlite-and-read-model-proof',
    proofMode: 'tracking-active-product-surface-summary',
    requiredProofTier: 'P2_HOSTED_CI',
    currentProofTier: 'P2_HOSTED_CI',
    currentStatus: 'proved',
    productClaimReady: false,
    commands,
    serviceCommand: proof.serviceBoundary.command,
    event: proof.serviceBoundary.event,
    payloadField: proof.serviceBoundary.payloadField,
    provedFields: proof.serviceBoundary.activeProductSurfaceSummary,
    assertions: [
      'Rust ActivityStore computes latestActiveEventId and latestActiveObservedAt from active rows only.',
      'Rust ActivityStore computes activeKindCounts, activeDeviceCounts, and activeCapabilityStatusCounts from active rows only.',
      'Retention-delete tombstone rows are excluded from active product-surface counts and remain visible only through tombstone metadata.',
      'The Rust protocol serializes the active product-surface summary fields in the same trackingReadModel payload.',
      'The TypeScript parser accepts the new fields while defaulting older events to empty/null summary values.',
      'The proof does not claim Android/iOS physical background behavior, provider delivery, full UI, or authority enrollment.',
    ],
    artifacts: {
      rustProtocolReadModelTest: 'crates/agent-protocol/tests/contract/tracking_read_model.rs',
      rustProtocol: 'crates/agent-protocol/src/tracking_read_model.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/tracking_read_model.rs',
      rustCoreReadModel: 'crates/agent-core/src/activity_store_tracking.rs',
      rustCoreReadModelTest: 'crates/agent-core/tests/unit/tracking_read_model.rs',
      rustProtocolPayload: 'crates/agent-protocol/src/tracking_read_model_payload.rs',
      rustProtocolPayloadTest: 'crates/agent-protocol/tests/contract/tracking_read_model_payload.rs',
      rustServiceCommandTest: 'crates/agent-service/tests/unit/tracking_read_model_service_tests.rs',
    },
    manualRequiredGaps: [
      'Parent reports, policy drill-in, and full tracking UI must consume this active summary before product UI is complete.',
      'Android/iOS physical background geofence and authority-enrolled behavior remain manual-required.',
      'Provider delivery, notification delivery, and production pilot proof remain absent.',
    ],
  };

  const retentionDeleteProof = {
    schemaVersion: 1,
    checkedAt,
    commit: proof.commit,
    workpackId: '32-journal-sqlite-and-read-model-proof',
    proofMode: 'tracking-service-retention-tombstone-replay',
    requiredProofTier: 'P2_HOSTED_CI',
    currentProofTier: 'P2_HOSTED_CI',
    currentStatus: 'proved',
    productClaimReady: false,
    commands,
    serviceCommand: proof.serviceBoundary.command,
    event: proof.serviceBoundary.event,
    payloadField: proof.serviceBoundary.payloadField,
    provedFields: proof.serviceBoundary.tombstoneReplay,
    assertions: [
      'Rust ActivityStore includes activity.tracking.retention.deleted rows in the tracking read model query.',
      'Retention-delete rows are exposed as tombstone queryVisibility rows instead of active tracking history rows.',
      'Deleted evidence reference ids are preserved on tombstone rows and summarized on the read model.',
      'Latest tombstone event id and observed timestamp are serialized through the Rust protocol and TypeScript parser.',
      'The portal citation surface renders deleted evidence refs only as tombstone metadata and keeps productClaimReady=false.',
      'The proof does not claim Android/iOS physical background behavior, provider delivery, complete UI, or authority enrollment.',
    ],
    artifacts: {
      rustProtocolReadModelTest: 'crates/agent-protocol/tests/contract/tracking_read_model.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/tracking_read_model.rs',
      rustCoreReadModelTest: 'crates/agent-core/tests/unit/tracking_read_model.rs',
      rustServiceCommandTest: 'crates/agent-service/tests/unit/tracking_read_model_service_tests.rs',
      serviceProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/18-service-read-model-proof.json',
    },
    manualRequiredGaps: [
      'Live parent/child UI delete controls and hosted accessibility proof remain pending.',
      'Platform replay from Android Studio, WSL/local, iOS simulator, and physical devices remains separate proof.',
      'Android/iOS physical background geofence and authority-enrolled behavior remain manual-required.',
    ],
  };

  const proofSummary = {
    schemaVersion: 1,
    checkedAt,
    commit: proof.commit,
    workpackId: '32-journal-sqlite-and-read-model-proof',
    proofState: 'p2-service-read-model-tombstone-replay-proof',
    summary:
      'Tracking service read-model proof now includes ActivityStore SQLite retention-delete tombstone replay, active/tombstone row counts, deleted evidence citation summaries, and live portal citation rows. Hosted UI/accessibility, broader product read models, platform replay, and physical-device proof remain pending.',
    commands,
    proofArtifacts: {
      retentionDeleteProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/14-retention-delete-proof.json',
      validationCommands:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/16-validation-commands.log',
      serviceReadModelProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/18-service-read-model-proof.json',
      productSurfaceSummaryProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/21-product-surface-summary-proof.json',
    },
    productClaims: {
      contractProof: true,
      serviceTombstoneReplayProof: true,
      activeProductSurfaceSummaryProof: true,
      livePortalCitationRows: true,
      androidIosBackgroundLocationClaimed: false,
      preciseLocationFromLanIpWifiClaimed: false,
      uiCompleteClaimed: false,
      providerDeliveryClaimed: false,
      remoteSyncEnabledByDefault: false,
    },
  };

  await mkdir(workpackRoot, { recursive: true });
  await writeFile(retentionDeleteProofPath, `${JSON.stringify(retentionDeleteProof, null, 2)}\n`);
  await writeFile(productSurfaceProofPath, `${JSON.stringify(productSurfaceProof, null, 2)}\n`);
  await writeFile(
    validationCommandsPath,
    `${commands.map(({ command, exitCode }) => `${command} # exit ${exitCode}`).join('\n')}\n`
  );
  await writeFile(serviceProofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(proofSummaryPath, `${JSON.stringify(proofSummary, null, 2)}\n`);
  console.log('tracking-plan-service-read-model-proof-ok');
  console.log(`evidence=${relative(repoRoot, serviceProofPath).replace(/\\/gu, '/')}`);
  console.log(`retention=${relative(repoRoot, retentionDeleteProofPath).replace(/\\/gu, '/')}`);
}

async function loadTrackingReadModelContract() {
  const [{ ParentAgentCommand, ParentAgentEvent, ParentAgentProtocolField }] = await Promise.all([
    importTsModule('packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts'),
  ]);

  return {
    command: ParentAgentCommand.ActivityTrackingReadModelGet,
    event: ParentAgentEvent.ActivityTrackingReadModelReported,
    payloadField: ParentAgentProtocolField.ActivityReadModel,
  };
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

async function assertTrackingProtocolHarness() {
  const harness = await readText('crates/agent-protocol/tests/contract.rs');
  assertIncludes(
    harness,
    '#[path = "contract/tracking_read_model.rs"]',
    'tracking read model contract harness registration exists'
  );
  assertIncludes(
    harness,
    '#[path = "contract/tracking_read_model_payload.rs"]',
    'tracking read model payload contract harness registration exists'
  );
}

async function readText(relativePath) {
  return readFile(join(repoRoot, relativePath), 'utf8');
}

async function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  await runCommand(command, commandArgs, ...rest);
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => {
      commands.push({ command: commandLine, exitCode: code });
      code === 0 ? resolve() : reject(new Error(`${commandLine} exited with ${code}`));
    });
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

async function importTsModule(relativePath) {
  return tsImport(pathToFileURL(join(repoRoot, relativePath)).href, import.meta.url);
}
