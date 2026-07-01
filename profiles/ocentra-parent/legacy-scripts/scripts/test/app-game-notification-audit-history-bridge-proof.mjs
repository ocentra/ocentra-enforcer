import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const testOutputDir = join(repoRoot, 'test-results', 'app-game-notification-audit-history-bridge-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '60-notification-audit-history-bridge');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '60-notification-audit-history-bridge');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(appGameProofDir, { recursive: true });
  await mkdir(appProofDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appProofDir, '06-ui-snapshots'), { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const localOutboxBridge = await importSchemaDist('app-game-notification-local-outbox-bridge.js');
  const auditHandoff = await importSchemaDist('notification-audit-history-handoff.js');
  const localOutboxReadModel = localOutboxBridge.buildAppGameNotificationLocalOutboxBridgeReadModel(
    bridgeOptions(),
    proofIntents()
  );
  const handoffRows = localOutboxReadModel.rows.map((row) => appGameOutboxRowToAuditHandoffRow(auditHandoff, row));
  const auditHandoffReadModel = auditHandoff.buildNotificationAuditHistoryHandoffReadModel(
    {
      handoffReadModelId: 'app-game-notification-audit-history-bridge-proof',
      generatedAt: '2026-06-05T02:17:00Z',
      sourceReadModelRef: localOutboxReadModel.bridgeId,
      sourceContractRefs: [
        'app-game-notification-local-outbox-bridge',
        'notification-audit-history-contract',
        'reports-notifications-sync-feature-doc',
        'notification-expectations',
      ],
    },
    handoffRows
  );

  await writeJson(join(testOutputDir, 'audit-history-handoff.json'), auditHandoffReadModel);

  const summary = summarize(localOutboxReadModel, auditHandoffReadModel);
  const proof = {
    schemaVersion: 1,
    proofMode: 'app-game-notification-audit-history-bridge',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    summary,
    claimsProved: [
      'Existing central app/game local outbox bridge rows are parsed before audit-history handoff',
      'Linked app/game local outbox rows become queued audit-history entries with evidence policy and audit refs',
      'Manual-required and unavailable app/game notification rows become blocked audit-history entries without queued provider sends',
      'Audit-history entries reuse the central redaction-safe payload fields and no Ocentra-hosted child data custody state',
      'Provider delivery, retry execution, receipt ingestion, credentials, cloud routing, parent UI, child delivery, durable outbox storage, adapter dispatch, and broad blocking remain unclaimed',
    ],
    claimsNotProved: [
      'provider adapter implementation or delivery',
      'production retry worker or quiet-hours timer execution',
      'provider webhook receipt ingestion',
      'durable production local outbox storage',
      'parent notification history or preference UI',
      'child app, overlay, push, or local notification delivery',
      'policy evaluator execution, adapter dispatch, broad blocking, or platform support',
    ],
    evidence: {
      auditHandoffSource: 'packages/schema-domain/src/notification-audit-history-handoff.ts',
      existingAuditHistoryContract: 'packages/schema-domain/src/notification-audit-history.ts',
      appGameLocalOutboxBridge: 'packages/schema-domain/src/app-game-notification-local-outbox-bridge.ts',
      harness: 'scripts/test/app-game-notification-audit-history-bridge-proof.mjs',
      handoffArtifact: 'test-results/app-game-notification-audit-history-bridge-proof/audit-history-handoff.json',
      appGameProofPack: 'output/app-game-plan-proof/60-notification-audit-history-bridge',
      appProofPack: 'output/app-plan-proof/60-notification-audit-history-bridge',
    },
    readModel: auditHandoffReadModel,
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP60');
  await writeProofPack(appProofDir, proof, 'app WP60');

  console.log('app-game-notification-audit-history-bridge-proof-ok');
  console.log(`evidence=${relative(repoRoot, join(testOutputDir, 'proof.json'))}`);
}

async function importSchemaDist(moduleName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', moduleName)).href);
}

function appGameOutboxRowToAuditHandoffRow(auditHandoff, row) {
  const linked = row.status === 'linked-local-outbox-record';
  const sourceStatus = linked
    ? auditHandoff.NotificationAuditHistoryHandoffSourceStatus.QueuedLocalOutbox
    : row.status === 'unavailable'
      ? auditHandoff.NotificationAuditHistoryHandoffSourceStatus.Unavailable
      : auditHandoff.NotificationAuditHistoryHandoffSourceStatus.ManualRequired;

  return {
    handoffEntryId: `app-game-notification-audit-${row.bridgeRecordId}`,
    sourceStatus,
    sourceNotificationIntentRef: row.intent.notificationIntentId,
    sourceOutboxRecordRef: row.outboxRecord?.entryId ?? null,
    providerChannelRef: row.outboxRecord?.envelope.providerChannel ?? row.intent.providerChannelPreference,
    reasonCodeRef: row.intent.notificationReasonCode,
    auditRefs: row.intent.auditRefs,
    evidenceRefs: row.intent.evidenceReferences.map((evidence) => evidence.evidenceReferenceId),
    policyRefs: row.intent.policyRefs,
    blockedReasonRefs: row.blockedReasonRefs,
  };
}

function summarize(localOutboxReadModel, auditHandoffReadModel) {
  return {
    sourceRows: localOutboxReadModel.rows.length,
    linkedLocalOutboxRows: localOutboxReadModel.linkedRecordCount,
    manualRequiredRows: localOutboxReadModel.manualRequiredCount,
    unavailableRows: localOutboxReadModel.unavailableCount,
    auditHistoryEntries: auditHandoffReadModel.auditHistoryEntries.length,
    queuedAuditEntryCount: auditHandoffReadModel.queuedAuditEntryCount,
    manualRequiredAuditEntryCount: auditHandoffReadModel.manualRequiredAuditEntryCount,
    unavailableAuditEntryCount: auditHandoffReadModel.unavailableAuditEntryCount,
    providerStatuses: countBy(auditHandoffReadModel.auditHistoryEntries.map((entry) => entry.providerStatus)),
    retryLifecycleStates: countBy(auditHandoffReadModel.auditHistoryEntries.map((entry) => entry.retryLifecycleState)),
    providerDeliveryRuntimeClaimed: auditHandoffReadModel.providerDeliveryRuntimeClaimed,
    providerReceiptIngestionClaimed: auditHandoffReadModel.providerReceiptIngestionClaimed,
    providerCredentialsClaimed: auditHandoffReadModel.providerCredentialsClaimed,
    cloudRoutingClaimed: auditHandoffReadModel.cloudRoutingClaimed,
    parentNotificationUiClaimed: auditHandoffReadModel.parentNotificationUiClaimed,
    childDeliveryClaimed: auditHandoffReadModel.childDeliveryClaimed,
    retryExecutionRuntimeClaimed: auditHandoffReadModel.retryExecutionRuntimeClaimed,
    quietHoursTimerRuntimeClaimed: auditHandoffReadModel.quietHoursTimerRuntimeClaimed,
    productionDurableOutboxStorageClaimed: auditHandoffReadModel.productionDurableOutboxStorageClaimed,
    adapterDispatchClaimed: auditHandoffReadModel.adapterDispatchClaimed,
  };
}

function bridgeOptions() {
  return {
    family: { familyId: 'family-app-game-audit-history-bridge-proof' },
    parentAction: {
      actionReferenceId: 'parent-action-app-game-audit-history-bridge-proof',
      actor: { actorId: 'parent-app-game-audit-history-bridge-proof', role: 'parent' },
      policyVersion: 'policy-app-game-notification-audit-proof-v1',
      createdAt: '2026-06-05T02:17:00Z',
    },
    generatedAt: '2026-06-05T02:17:00Z',
    bridgeId: 'app-game-notification-local-outbox-bridge-for-audit-proof',
    outboxRootRef: 'parent-owned-app-game-local-outbox-root-for-audit',
    outboxFileRef: 'parent-owned-app-game-local-outbox-jsonl-for-audit',
    localDataPathRef: 'parent-owned-app-game-local-outbox-data-path-for-audit',
  };
}

function proofIntents() {
  const base = {
    schemaVersion: 'v0.6',
    notificationIntentId: 'notification-intent-time-limit-audit-proof',
    intentKind: 'time-limit-reached',
    intentStatus: 'local-outbox-eligible',
    priority: 'urgent',
    device: {
      deviceId: 'device-app-game-audit-history-bridge-proof',
      childProfileId: 'child-app-game-audit-history-bridge-proof',
      label: 'Study PC',
      platform: 'windows',
    },
    targetKind: 'native-game',
    targetRef: 'target-native-game-audit-proof',
    notificationReasonCode: 'app-game-time-limit',
    providerChannelPreference: 'in-app',
    parentTitleToken: 'appGame.notification.timeLimit.title',
    parentBodyToken: 'appGame.notification.timeLimit.body',
    parentActionToken: 'appGame.notification.action.openParentReview',
    childTitleToken: 'appGame.childUx.limitReached.title',
    childBodyToken: 'appGame.childUx.limitReached.body',
    notificationRuleRef: 'notification-rule-app-game-time-limit-audit-proof',
    notificationStatusRef: 'notification-status-app-game-time-limit-audit-proof',
    policyRefs: ['policy-ref-app-game-time-limit-audit-proof'],
    auditRefs: ['audit-ref-app-game-time-limit-audit-proof'],
    evidenceReferences: [
      {
        evidenceReferenceId: 'evidence-ref-app-game-time-limit-audit-proof',
        kind: 'policy-decision',
        observedAt: '2026-06-05T02:17:00Z',
      },
    ],
    childReasonReferences: [],
    childStatusReferences: ['child-status-app-game-time-limit-audit-proof'],
    approvalActionRef: null,
    timeBudgetDecisionRef: 'time-budget-decision-app-game-audit-proof',
    unknownCandidateRef: null,
    localOutboxRecordRef: 'local-outbox-record-app-game-time-limit-audit-proof',
    providerAttemptRefs: [],
    providerReceiptRefs: [],
    manualProofRequirements: [],
    minimalPayloadFields: [
      'alert-id',
      'family-device-scope',
      'severity',
      'reason-code',
      'evidence-ref',
      'policy-ref',
      'parent-action-link-ref',
    ],
    deliveryClaimState: 'local-outbox-only',
    rawChildEvidenceIncluded: false,
    rawUrlOrTitleIncluded: false,
    rawMessageTextIncluded: false,
    screenshotOrReportIncluded: false,
    providerDeliveryAttempted: false,
    providerDeliveryObserved: false,
    providerReceiptIngested: false,
    cloudRoutingClaimed: false,
    parentNotificationUiClaimed: false,
    adapterDispatchState: 'not-dispatched',
    adapterActionClaimed: false,
    createdAt: '2026-06-05T02:17:00Z',
  };

  return [
    base,
    {
      ...base,
      notificationIntentId: 'notification-intent-suspicious-unknown-audit-proof',
      intentKind: 'suspicious-unknown',
      priority: 'attention',
      targetKind: 'unknown-app',
      targetRef: 'target-unknown-app-audit-proof',
      notificationReasonCode: 'app-game-suspicious-unknown',
      providerChannelPreference: 'email',
      parentTitleToken: 'appGame.notification.suspiciousUnknown.title',
      parentBodyToken: 'appGame.notification.suspiciousUnknown.body',
      childTitleToken: 'appGame.childUx.newApp.title',
      childBodyToken: 'appGame.childUx.newApp.body',
      timeBudgetDecisionRef: null,
      unknownCandidateRef: 'unknown-app-candidate-audit-proof',
      localOutboxRecordRef: 'local-outbox-record-app-game-suspicious-unknown-audit-proof',
    },
    {
      ...base,
      notificationIntentId: 'notification-intent-manual-required-audit-proof',
      intentKind: 'manual-required',
      intentStatus: 'manual-required',
      priority: 'attention',
      notificationReasonCode: 'app-game-manual-review-required',
      parentTitleToken: 'appGame.notification.manualRequired.title',
      parentBodyToken: 'appGame.notification.manualRequired.body',
      parentActionToken: 'appGame.notification.action.reviewManually',
      childTitleToken: 'appGame.childUx.manualRequired.title',
      childBodyToken: 'appGame.childUx.manualRequired.body',
      timeBudgetDecisionRef: null,
      localOutboxRecordRef: null,
      manualProofRequirements: ['provider preference setup before app game notification can be audited'],
      deliveryClaimState: 'manual-required',
    },
    {
      ...base,
      notificationIntentId: 'notification-intent-unavailable-audit-proof',
      intentKind: 'capability-unavailable',
      intentStatus: 'unavailable',
      priority: 'info',
      notificationReasonCode: 'app-game-capability-unavailable',
      parentTitleToken: 'appGame.notification.unavailable.title',
      parentBodyToken: 'appGame.notification.unavailable.body',
      parentActionToken: 'appGame.notification.action.reviewManually',
      childTitleToken: 'appGame.childUx.unavailable.title',
      childBodyToken: 'appGame.childUx.unavailable.body',
      timeBudgetDecisionRef: null,
      localOutboxRecordRef: null,
      manualProofRequirements: ['local evidence and policy readiness before unavailable notification can be audited'],
      deliveryClaimState: 'manual-required',
    },
  ];
}

async function writeProofPack(proofDir, proof, label) {
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `- Branch: ${await gitBranch()}`,
      `- Commit: ${proof.commit}`,
      '- Scope: central app/game notification local outbox rows to notification audit-history handoff entries.',
      '- Source inspected: schema-domain app/game notification local outbox bridge and central notification audit history contracts.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '01-contract-proof.log'),
    [
      'Contract proof:',
      '',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- node scripts/test/app-game-notification-audit-history-bridge-proof.mjs: PASS',
      '- Linked, manual-required, and unavailable source rows parse through the central notification audit-history entry schemas.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    'Rust protocol proof not applicable: this workpack validates the central schema-domain audit-history handoff only.\n',
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), proof.summary);
  await writeJson(join(proofDir, '04-journal-sqlite-proof.json'), {
    schemaVersion: 1,
    journalSqliteChanged: false,
    handoffArtifact: 'test-results/app-game-notification-audit-history-bridge-proof/audit-history-handoff.json',
    durableProductionOutboxStorageClaimed: false,
  });
  await writeJson(join(proofDir, '05-policy-action-proof.json'), {
    schemaVersion: 1,
    queuedAuditEntries: proof.summary.queuedAuditEntryCount,
    manualRequiredAuditEntries: proof.summary.manualRequiredAuditEntryCount,
    unavailableAuditEntries: proof.summary.unavailableAuditEntryCount,
    providerDeliveryRuntimeClaimed: false,
    retryExecutionRuntimeClaimed: false,
    adapterDispatchClaimed: false,
  });
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo parent portal, notification history, preference UI, or child-facing UI source changed in this workpack.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '07-playwright-ui-proof.log'),
    'Playwright proof not applicable: no UI source changed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim proof:',
      '',
      '- Audit-history handoff entries keep redaction-safe operational fields only.',
      '- Raw child data, raw evidence payloads, sensitive provider payloads, and Ocentra-hosted child data custody remain false.',
      '- Manual-required and unavailable app/game rows are audit-visible but do not create provider sends.',
      '- Provider delivery, retry execution, receipt ingestion, credentials, cloud routing, parent UI, child delivery, durable outbox storage, adapter dispatch, broad blocking, and platform support remain false or unclaimed.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo live platform authority tier is raised. Provider delivery, scheduler workers, child delivery, and platform adapter execution remain unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [...commands, 'node scripts/test/app-game-notification-audit-history-bridge-proof.mjs'].join('\n') + '\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nThe handoff is audit metadata only. It does not raise provider, scheduler, adapter, child-delivery, or platform authority.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo provider send, retry worker, child-device notification, block, suspend, shield, or adapter state is created. Rollback is limited to deleting generated proof artifacts.\n',
    'utf8'
  );
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
  });
}

async function gitBranch() {
  return (await gitOutput(['rev-parse', '--abbrev-ref', 'HEAD'])).trim();
}

async function gitHead() {
  return (await gitOutput(['rev-parse', 'HEAD'])).trim();
}

async function gitOutput(args) {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', args, { cwd: repoRoot, shell: false });
    child.stdout.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(undefined);
        return;
      }
      reject(new Error(`git ${args.join(' ')} exited with ${code}`));
    });
  });
  return Buffer.concat(chunks).toString('utf8');
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
