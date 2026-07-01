import { spawnSync } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repoRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const testOutputDir = join(repoRoot, 'test-results', 'app-game-notification-provider-preflight-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '61-notification-provider-preflight');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '61-notification-provider-preflight');
const timestamp = '2026-06-05T02:43:00Z';
const commands = [];
const initialGitStatusShort = gitOutput(['status', '--short']);

for (const path of [testOutputDir, appGameProofDir, appProofDir]) {
  await rm(path, { recursive: true, force: true });
  await mkdir(path, { recursive: true });
}
for (const path of [join(appGameProofDir, '06-ui-snapshots'), join(appProofDir, '06-ui-snapshots')]) {
  await mkdir(path, { recursive: true });
}

runNpm(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const localOutbox = await importSchemaDist('app-game-notification-local-outbox-bridge.js');
const scheduler = await importSchemaDist('app-game-notification-scheduler-bridge.js');
const providerPreflight = await importSchemaDist('app-game-notification-provider-preflight.js');
const intent = await importSchemaDist('app-game-notification-intent.js');
const childUx = await importSchemaDist('app-game-child-facing-ux.js');
const refs = await importSchemaDist('family-reference-primitives.js');

const localOutboxReadModel = localOutbox.buildAppGameNotificationLocalOutboxBridgeReadModel(
  bridgeOptions(refs),
  proofIntents(intent, childUx, refs)
);
const schedulerReadModel = scheduler.buildAppGameNotificationSchedulerBridgeReadModel(
  schedulerOptions(),
  localOutboxReadModel
);
const preflightReadModel = providerPreflight.buildAppGameNotificationProviderPreflightReadModel(
  providerPreflightOptions(),
  schedulerReadModel
);
const summary = summarize(preflightReadModel);
const proof = {
  proofMode: 'app-game-notification-provider-preflight',
  generatedAt: timestamp,
  branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
  commit: gitOutput(['rev-parse', 'HEAD']),
  gitStatusShort: initialGitStatusShort,
  commands,
  summary,
  nonClaims: {
    providerDeliveryRuntimeClaimed: preflightReadModel.providerDeliveryRuntimeClaimed,
    providerReceiptIngestionClaimed: preflightReadModel.providerReceiptIngestionClaimed,
    providerCredentialsClaimed: preflightReadModel.providerCredentialsClaimed,
    cloudRoutingClaimed: preflightReadModel.cloudRoutingClaimed,
    parentNotificationUiClaimed: preflightReadModel.parentNotificationUiClaimed,
    childDeliveryClaimed: preflightReadModel.childDeliveryClaimed,
    retryExecutionRuntimeClaimed: preflightReadModel.retryExecutionRuntimeClaimed,
    quietHoursTimerRuntimeClaimed: preflightReadModel.quietHoursTimerRuntimeClaimed,
    productionDurableOutboxStorageClaimed: preflightReadModel.productionDurableOutboxStorageClaimed,
    adapterDispatchClaimed: preflightReadModel.adapterDispatchClaimed,
  },
  proofPaths: {
    source: 'packages/schema-domain/src/app-game-notification-provider-preflight.ts',
    schedulerBridge: 'packages/schema-domain/src/app-game-notification-scheduler-bridge.ts',
    localOutboxBridge: 'packages/schema-domain/src/app-game-notification-local-outbox-bridge.ts',
    intentContract: 'packages/schema-domain/src/app-game-notification-intent.ts',
    harness: 'scripts/test/app-game-notification-provider-preflight-proof.mjs',
    evidence: 'test-results/app-game-notification-provider-preflight-proof/proof.json',
    appGameProofPack: 'output/app-game-plan-proof/61-notification-provider-preflight',
    appProofPack: 'output/app-plan-proof/61-notification-provider-preflight',
  },
  readModel: preflightReadModel,
};

assertProof(proof);
await writeJson(join(testOutputDir, 'provider-preflight-read-model.json'), preflightReadModel);
await writeJson(join(testOutputDir, 'proof.json'), proof);
await writeProofPack(appGameProofDir, proof, 'app-game WP61');
await writeProofPack(appProofDir, proof, 'app WP61');

console.log('app-game-notification-provider-preflight-proof-ok');
console.log(`evidence=${join('test-results', 'app-game-notification-provider-preflight-proof', 'proof.json')}`);

async function importSchemaDist(moduleName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', moduleName)).href);
}

function providerPreflightOptions() {
  return {
    generatedAt: timestamp,
    providerPreflightId: 'app-game-notification-provider-preflight-proof',
    sourceContractRefs: [
      'app-game-notification-scheduler-bridge',
      'notification-local-outbox-scheduler-proof',
      'notification-provider-adapter-boundary-required',
      'notification-feature-expectations-provider-boundary',
    ],
  };
}

function schedulerOptions() {
  return {
    generatedAt: timestamp,
    schedulerBridgeId: 'app-game-notification-scheduler-bridge-for-provider-preflight',
    schedulerArtifactRootRef: 'parent-owned-app-game-notification-scheduler-root-for-provider-preflight',
    schedulerArtifactRef: 'parent-owned-app-game-notification-scheduler-jsonl-for-provider-preflight',
    schedulerNowAt: timestamp,
  };
}

function bridgeOptions(refs) {
  return {
    family: { familyId: 'family-app-game-provider-preflight' },
    parentAction: {
      actionReferenceId: 'parent-action-app-game-provider-preflight',
      actor: { actorId: 'parent-app-game-provider-preflight', role: refs.ParentActorRole.Parent },
      policyVersion: 'policy-app-game-provider-preflight-v1',
      createdAt: timestamp,
    },
    generatedAt: timestamp,
    bridgeId: 'app-game-notification-local-outbox-bridge-for-provider-preflight',
    outboxRootRef: 'parent-owned-app-game-local-outbox-root-for-provider-preflight',
    outboxFileRef: 'parent-owned-app-game-local-outbox-jsonl-for-provider-preflight',
    localDataPathRef: 'parent-owned-app-game-local-outbox-data-path-for-provider-preflight',
  };
}

function proofIntents(intent, childUx, refs) {
  const base = {
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    notificationIntentId: 'notification-intent-time-limit-provider-preflight',
    intentKind: intent.AppGameNotificationIntentKind.TimeLimitReached,
    intentStatus: intent.AppGameNotificationIntentStatus.LocalOutboxEligible,
    priority: intent.AppGameNotificationPriority.Urgent,
    device: {
      deviceId: 'device-app-game-provider-preflight',
      childProfileId: 'child-app-game-provider-preflight',
      label: 'Study PC',
      platform: refs.ParentPlatform.Windows,
    },
    targetKind: childUx.AppGameChildUxTargetKind.NativeGame,
    targetRef: 'target-native-game-provider-preflight',
    notificationReasonCode: intent.AppGameNotificationReasonCode.TimeLimit,
    providerChannelPreference: 'in-app',
    parentTitleToken: intent.AppGameNotificationParentCopyToken.TimeLimitTitle,
    parentBodyToken: intent.AppGameNotificationParentCopyToken.TimeLimitBody,
    parentActionToken: intent.AppGameNotificationParentCopyToken.OpenParentReviewAction,
    childTitleToken: childUx.AppGameChildUxCopyToken.LimitReachedTitle,
    childBodyToken: childUx.AppGameChildUxCopyToken.LimitReachedBody,
    notificationRuleRef: 'notification-rule-app-game-time-limit-provider-preflight',
    notificationStatusRef: 'notification-status-app-game-time-limit-provider-preflight',
    policyRefs: ['policy-ref-app-game-time-limit-provider-preflight'],
    auditRefs: ['audit-ref-app-game-time-limit-provider-preflight'],
    evidenceReferences: [
      {
        evidenceReferenceId: 'evidence-ref-app-game-time-limit-provider-preflight',
        kind: refs.ParentEvidenceReferenceKind.PolicyDecision,
        observedAt: timestamp,
      },
    ],
    childReasonReferences: [],
    childStatusReferences: ['child-status-app-game-time-limit-provider-preflight'],
    approvalActionRef: null,
    timeBudgetDecisionRef: 'time-budget-decision-app-game-provider-preflight',
    unknownCandidateRef: null,
    localOutboxRecordRef: 'local-outbox-record-app-game-time-limit-provider-preflight',
    providerAttemptRefs: [],
    providerReceiptRefs: [],
    manualProofRequirements: [],
    minimalPayloadFields: Object.values(intent.AppGameNotificationPayloadField),
    deliveryClaimState: intent.AppGameNotificationDeliveryClaimState.LocalOutboxOnly,
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
    createdAt: timestamp,
  };
  return [
    base,
    {
      ...base,
      notificationIntentId: 'notification-intent-suspicious-unknown-provider-preflight',
      intentKind: intent.AppGameNotificationIntentKind.SuspiciousUnknown,
      priority: intent.AppGameNotificationPriority.Attention,
      targetKind: childUx.AppGameChildUxTargetKind.UnknownApp,
      notificationReasonCode: intent.AppGameNotificationReasonCode.SuspiciousUnknown,
      providerChannelPreference: 'email',
      parentTitleToken: intent.AppGameNotificationParentCopyToken.SuspiciousUnknownTitle,
      parentBodyToken: intent.AppGameNotificationParentCopyToken.SuspiciousUnknownBody,
      childTitleToken: childUx.AppGameChildUxCopyToken.NewAppTitle,
      childBodyToken: childUx.AppGameChildUxCopyToken.NewAppBody,
      localOutboxRecordRef: 'local-outbox-record-app-game-suspicious-unknown-provider-preflight',
      timeBudgetDecisionRef: null,
      unknownCandidateRef: 'unknown-app-candidate-provider-preflight',
    },
    manualIntent(base, intent, childUx),
    unavailableIntent(base, intent, childUx),
  ];
}

function manualIntent(base, intent, childUx) {
  return {
    ...base,
    notificationIntentId: 'notification-intent-manual-required-provider-preflight',
    intentKind: intent.AppGameNotificationIntentKind.ManualRequired,
    intentStatus: intent.AppGameNotificationIntentStatus.ManualRequired,
    notificationReasonCode: intent.AppGameNotificationReasonCode.ManualReviewRequired,
    parentTitleToken: intent.AppGameNotificationParentCopyToken.ManualRequiredTitle,
    parentBodyToken: intent.AppGameNotificationParentCopyToken.ManualRequiredBody,
    parentActionToken: intent.AppGameNotificationParentCopyToken.ReviewManuallyAction,
    childTitleToken: childUx.AppGameChildUxCopyToken.ManualRequiredTitle,
    childBodyToken: childUx.AppGameChildUxCopyToken.ManualRequiredBody,
    localOutboxRecordRef: null,
    timeBudgetDecisionRef: null,
    manualProofRequirements: ['provider preference setup before app game notification can be scheduled'],
    deliveryClaimState: intent.AppGameNotificationDeliveryClaimState.ManualRequired,
  };
}

function unavailableIntent(base, intent, childUx) {
  return {
    ...base,
    notificationIntentId: 'notification-intent-unavailable-provider-preflight',
    intentKind: intent.AppGameNotificationIntentKind.CapabilityUnavailable,
    intentStatus: intent.AppGameNotificationIntentStatus.Unavailable,
    priority: intent.AppGameNotificationPriority.Info,
    notificationReasonCode: intent.AppGameNotificationReasonCode.CapabilityUnavailable,
    parentTitleToken: intent.AppGameNotificationParentCopyToken.UnavailableTitle,
    parentBodyToken: intent.AppGameNotificationParentCopyToken.UnavailableBody,
    parentActionToken: intent.AppGameNotificationParentCopyToken.ReviewManuallyAction,
    childTitleToken: childUx.AppGameChildUxCopyToken.UnavailableTitle,
    childBodyToken: childUx.AppGameChildUxCopyToken.UnavailableBody,
    localOutboxRecordRef: null,
    timeBudgetDecisionRef: null,
    manualProofRequirements: ['local evidence and policy readiness before unavailable notification can be scheduled'],
    deliveryClaimState: intent.AppGameNotificationDeliveryClaimState.ManualRequired,
  };
}

function summarize(readModel) {
  return {
    rows: readModel.rows.length,
    providerAdapterRequiredCount: readModel.providerAdapterRequiredCount,
    manualRequiredCount: readModel.manualRequiredCount,
    unavailableCount: readModel.unavailableCount,
    statuses: countBy(readModel.rows.map((row) => row.status)),
    providerChannels: countBy(
      readModel.rows.flatMap((row) => (row.providerChannelRef === null ? [] : [row.providerChannelRef]))
    ),
  };
}

function assertProof(proof) {
  if (
    proof.summary.providerAdapterRequiredCount !== 2 ||
    proof.summary.manualRequiredCount !== 1 ||
    proof.summary.unavailableCount !== 1
  ) {
    throw new Error(`Unexpected provider preflight summary: ${JSON.stringify(proof.summary)}`);
  }
  if (Object.values(proof.nonClaims).some((value) => value !== false)) {
    throw new Error(`Provider preflight overclaimed runtime behavior: ${JSON.stringify(proof.nonClaims)}`);
  }
}

async function writeProofPack(proofDir, proof, label) {
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `- Branch: ${proof.branch}`,
      `- Commit: ${proof.commit}`,
      '- Git status at proof generation:',
      '',
      '```text',
      proof.gitStatusShort.length === 0 ? 'clean' : proof.gitStatusShort,
      '```',
      '',
      '- Scope: central app/game notification scheduler rows to provider-adapter preflight rows.',
      '- Source inspected: schema-domain notification scheduler bridge, local outbox bridge, provider preflight, and central notification expectation refs.',
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
      '- node scripts/test/app-game-notification-provider-preflight-proof.mjs: PASS',
      '- Scheduled rows become provider-adapter-required preflight rows.',
      '- Manual-required and unavailable rows remain blocked before provider preflight.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    'Rust protocol proof not applicable: this workpack validates the central schema-domain provider preflight boundary only.\n',
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), proof.summary);
  await writeJson(join(proofDir, '04-journal-sqlite-proof.json'), {
    schemaVersion: 1,
    journalSqliteChanged: false,
    providerPreflightArtifact:
      'test-results/app-game-notification-provider-preflight-proof/provider-preflight-read-model.json',
  });
  await writeJson(join(proofDir, '05-policy-action-proof.json'), {
    schemaVersion: 1,
    providerAdapterRequiredRows: proof.summary.providerAdapterRequiredCount,
    providerDeliveryRuntimeClaimed: false,
    adapterDispatchClaimed: false,
  });
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo parent notification UI, preferences UI, history UI, or child-facing UI source changed in this workpack.\n',
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
      '- Provider preflight rows require adapter implementation, credentials, and provider smoke proof before delivery can be claimed.',
      '- Manual-required and unavailable rows stay blocked before provider preflight.',
      '- Provider delivery, receipt ingestion, credentials, cloud routing, retry workers, quiet-hours timers, parent UI, child delivery, durable outbox storage, and adapter dispatch remain false.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo live platform authority tier is raised. Provider adapter implementation, credentials, child delivery, and platform execution remain unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [...commands, 'node scripts/test/app-game-notification-provider-preflight-proof.mjs'].join('\n') + '\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nThe preflight is a contract boundary only. It does not dispatch adapters, send provider payloads, or raise child-device/platform authority.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo provider send, retry worker, child-device notification, block, suspend, shield, or adapter state is created. Rollback is limited to deleting generated proof artifacts.\n',
    'utf8'
  );
}

function run(command, args) {
  commands.push([command, ...args].join(' '));
  const result = spawnSync(command, args, { cwd: repoRoot, stdio: 'inherit', shell: false });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(' ')}`);
  }
}

function gitOutput(args) {
  return spawnSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).stdout.trim();
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

function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return run(command, commandArgs, ...rest);
}
