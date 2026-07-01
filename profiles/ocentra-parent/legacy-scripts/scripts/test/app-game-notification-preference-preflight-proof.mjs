import { spawnSync } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repoRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const testOutputDir = join(repoRoot, 'test-results', 'app-game-notification-preference-preflight-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '62-notification-preference-preflight');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '62-notification-preference-preflight');
const timestamp = '2026-06-05T03:03:00Z';
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
const preferencePreflight = await importSchemaDist('app-game-notification-preference-preflight.js');
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
const preferenceReadModel = preferencePreflight.buildAppGameNotificationPreferencePreflightReadModel(
  preferencePreflightOptions(),
  schedulerReadModel
);
const summary = summarize(preferenceReadModel);
const proof = {
  proofMode: 'app-game-notification-preference-preflight',
  generatedAt: timestamp,
  branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
  commit: gitOutput(['rev-parse', 'HEAD']),
  gitStatusShort: initialGitStatusShort,
  commands,
  summary,
  nonClaims: {
    parentPreferenceUiClaimed: preferenceReadModel.parentPreferenceUiClaimed,
    parentFrequencyControlUiClaimed: preferenceReadModel.parentFrequencyControlUiClaimed,
    quietHoursTimerRuntimeClaimed: preferenceReadModel.quietHoursTimerRuntimeClaimed,
    providerDeliveryRuntimeClaimed: preferenceReadModel.providerDeliveryRuntimeClaimed,
    providerReceiptIngestionClaimed: preferenceReadModel.providerReceiptIngestionClaimed,
    providerCredentialsClaimed: preferenceReadModel.providerCredentialsClaimed,
    cloudRoutingClaimed: preferenceReadModel.cloudRoutingClaimed,
    childDeliveryClaimed: preferenceReadModel.childDeliveryClaimed,
    retryExecutionRuntimeClaimed: preferenceReadModel.retryExecutionRuntimeClaimed,
    productionDurableOutboxStorageClaimed: preferenceReadModel.productionDurableOutboxStorageClaimed,
    adapterDispatchClaimed: preferenceReadModel.adapterDispatchClaimed,
  },
  proofPaths: {
    source: 'packages/schema-domain/src/app-game-notification-preference-preflight.ts',
    schedulerBridge: 'packages/schema-domain/src/app-game-notification-scheduler-bridge.ts',
    localOutboxBridge: 'packages/schema-domain/src/app-game-notification-local-outbox-bridge.ts',
    intentContract: 'packages/schema-domain/src/app-game-notification-intent.ts',
    harness: 'scripts/test/app-game-notification-preference-preflight-proof.mjs',
    evidence: 'test-results/app-game-notification-preference-preflight-proof/proof.json',
    appGameProofPack: 'output/app-game-plan-proof/62-notification-preference-preflight',
    appProofPack: 'output/app-plan-proof/62-notification-preference-preflight',
  },
  readModel: preferenceReadModel,
};

assertProof(proof);
await writeJson(join(testOutputDir, 'preference-preflight-read-model.json'), preferenceReadModel);
await writeJson(join(testOutputDir, 'proof.json'), proof);
await writeProofPack(appGameProofDir, proof, 'app-game WP62');
await writeProofPack(appProofDir, proof, 'app WP62');

console.log('app-game-notification-preference-preflight-proof-ok');
console.log(`evidence=${join('test-results', 'app-game-notification-preference-preflight-proof', 'proof.json')}`);

async function importSchemaDist(moduleName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', moduleName)).href);
}

function preferencePreflightOptions() {
  return {
    generatedAt: timestamp,
    preferencePreflightId: 'app-game-notification-preference-preflight-proof',
    sourceContractRefs: [
      'app-game-notification-scheduler-bridge',
      'notification-parent-preference-boundary',
      'notification-quiet-hours-policy-boundary',
      'notification-feature-expectations-parent-controls',
    ],
  };
}

function schedulerOptions() {
  return {
    generatedAt: timestamp,
    schedulerBridgeId: 'app-game-notification-scheduler-bridge-for-preference-preflight',
    schedulerArtifactRootRef: 'parent-owned-app-game-notification-scheduler-root-for-preference-preflight',
    schedulerArtifactRef: 'parent-owned-app-game-notification-scheduler-jsonl-for-preference-preflight',
    schedulerNowAt: timestamp,
  };
}

function bridgeOptions(refs) {
  return {
    family: { familyId: 'family-app-game-preference-preflight' },
    parentAction: {
      actionReferenceId: 'parent-action-app-game-preference-preflight',
      actor: { actorId: 'parent-app-game-preference-preflight', role: refs.ParentActorRole.Parent },
      policyVersion: 'policy-app-game-preference-preflight-v1',
      createdAt: timestamp,
    },
    generatedAt: timestamp,
    bridgeId: 'app-game-notification-local-outbox-bridge-for-preference-preflight',
    outboxRootRef: 'parent-owned-app-game-local-outbox-root-for-preference-preflight',
    outboxFileRef: 'parent-owned-app-game-local-outbox-jsonl-for-preference-preflight',
    localDataPathRef: 'parent-owned-app-game-local-outbox-data-path-for-preference-preflight',
  };
}

function proofIntents(intent, childUx, refs) {
  const base = {
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    notificationIntentId: 'notification-intent-time-limit-preference-preflight',
    intentKind: intent.AppGameNotificationIntentKind.TimeLimitReached,
    intentStatus: intent.AppGameNotificationIntentStatus.LocalOutboxEligible,
    priority: intent.AppGameNotificationPriority.Urgent,
    device: {
      deviceId: 'device-app-game-preference-preflight',
      childProfileId: 'child-app-game-preference-preflight',
      label: 'Study PC',
      platform: refs.ParentPlatform.Windows,
    },
    targetKind: childUx.AppGameChildUxTargetKind.NativeGame,
    targetRef: 'target-native-game-preference-preflight',
    notificationReasonCode: intent.AppGameNotificationReasonCode.TimeLimit,
    providerChannelPreference: 'in-app',
    parentTitleToken: intent.AppGameNotificationParentCopyToken.TimeLimitTitle,
    parentBodyToken: intent.AppGameNotificationParentCopyToken.TimeLimitBody,
    parentActionToken: intent.AppGameNotificationParentCopyToken.OpenParentReviewAction,
    childTitleToken: childUx.AppGameChildUxCopyToken.LimitReachedTitle,
    childBodyToken: childUx.AppGameChildUxCopyToken.LimitReachedBody,
    notificationRuleRef: 'notification-rule-app-game-time-limit-preference-preflight',
    notificationStatusRef: 'notification-status-app-game-time-limit-preference-preflight',
    policyRefs: ['policy-ref-app-game-time-limit-preference-preflight'],
    auditRefs: ['audit-ref-app-game-time-limit-preference-preflight'],
    evidenceReferences: [
      {
        evidenceReferenceId: 'evidence-ref-app-game-time-limit-preference-preflight',
        kind: refs.ParentEvidenceReferenceKind.PolicyDecision,
        observedAt: timestamp,
      },
    ],
    childReasonReferences: [],
    childStatusReferences: ['child-status-app-game-time-limit-preference-preflight'],
    approvalActionRef: null,
    timeBudgetDecisionRef: 'time-budget-decision-app-game-preference-preflight',
    unknownCandidateRef: null,
    localOutboxRecordRef: 'local-outbox-record-app-game-time-limit-preference-preflight',
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
      notificationIntentId: 'notification-intent-suspicious-unknown-preference-preflight',
      intentKind: intent.AppGameNotificationIntentKind.SuspiciousUnknown,
      priority: intent.AppGameNotificationPriority.Attention,
      targetKind: childUx.AppGameChildUxTargetKind.UnknownApp,
      notificationReasonCode: intent.AppGameNotificationReasonCode.SuspiciousUnknown,
      providerChannelPreference: 'email',
      parentTitleToken: intent.AppGameNotificationParentCopyToken.SuspiciousUnknownTitle,
      parentBodyToken: intent.AppGameNotificationParentCopyToken.SuspiciousUnknownBody,
      childTitleToken: childUx.AppGameChildUxCopyToken.NewAppTitle,
      childBodyToken: childUx.AppGameChildUxCopyToken.NewAppBody,
      localOutboxRecordRef: 'local-outbox-record-app-game-suspicious-unknown-preference-preflight',
      timeBudgetDecisionRef: null,
      unknownCandidateRef: 'unknown-app-candidate-preference-preflight',
    },
    manualIntent(base, intent, childUx),
    unavailableIntent(base, intent, childUx),
  ];
}

function manualIntent(base, intent, childUx) {
  return {
    ...base,
    notificationIntentId: 'notification-intent-manual-required-preference-preflight',
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
    notificationIntentId: 'notification-intent-unavailable-preference-preflight',
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
    parentPreferenceRequiredCount: readModel.parentPreferenceRequiredCount,
    manualRequiredCount: readModel.manualRequiredCount,
    unavailableCount: readModel.unavailableCount,
    statuses: countBy(readModel.rows.map((row) => row.status)),
    providerChannels: countBy(
      readModel.rows.flatMap((row) => (row.providerChannelRef === null ? [] : [row.providerChannelRef]))
    ),
    parentPreferenceStates: countBy(
      readModel.rows.flatMap((row) => (row.parentPreferenceState === null ? [] : [row.parentPreferenceState]))
    ),
    quietHoursDecisions: countBy(
      readModel.rows.flatMap((row) => (row.quietHoursDecision === null ? [] : [row.quietHoursDecision]))
    ),
  };
}

function assertProof(proof) {
  if (
    proof.summary.parentPreferenceRequiredCount !== 2 ||
    proof.summary.manualRequiredCount !== 1 ||
    proof.summary.unavailableCount !== 1
  ) {
    throw new Error(`Unexpected preference preflight summary: ${JSON.stringify(proof.summary)}`);
  }
  if (Object.values(proof.nonClaims).some((value) => value !== false)) {
    throw new Error(`Preference preflight overclaimed runtime behavior: ${JSON.stringify(proof.nonClaims)}`);
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
      '- Scope: central app/game notification scheduler rows to parent-preference preflight rows.',
      '- Source inspected: schema-domain notification scheduler bridge, local outbox bridge, preference preflight, and central notification expectation refs.',
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
      '- node scripts/test/app-game-notification-preference-preflight-proof.mjs: PASS',
      '- Scheduled rows become parent-preference-required preflight rows.',
      '- Manual-required and unavailable rows remain blocked before preference preflight.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    'Rust protocol proof not applicable: this workpack validates the central schema-domain preference preflight boundary only.\n',
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), proof.summary);
  await writeJson(join(proofDir, '04-journal-sqlite-proof.json'), {
    schemaVersion: 1,
    journalSqliteChanged: false,
    preferencePreflightArtifact:
      'test-results/app-game-notification-preference-preflight-proof/preference-preflight-read-model.json',
  });
  await writeJson(join(proofDir, '05-policy-action-proof.json'), {
    schemaVersion: 1,
    parentPreferenceRequiredRows: proof.summary.parentPreferenceRequiredCount,
    parentPreferenceUiClaimed: false,
    providerDeliveryRuntimeClaimed: false,
    adapterDispatchClaimed: false,
  });
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo parent notification preference UI, frequency control UI, history UI, or child-facing UI source changed in this workpack.\n',
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
      '- Parent preference preflight rows require preference, frequency, and quiet-hours policy proof before delivery can be claimed.',
      '- Manual-required and unavailable rows stay blocked before preference preflight.',
      '- Parent preference UI, parent frequency controls, quiet-hours timers, provider delivery, receipt ingestion, credentials, cloud routing, retry workers, durable outbox storage, child delivery, and adapter dispatch remain false.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo live platform authority tier is raised. Parent preference UI, provider delivery, child delivery, and platform execution remain unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [...commands, 'node scripts/test/app-game-notification-preference-preflight-proof.mjs'].join('\n') + '\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nThe preflight is a contract boundary only. It does not dispatch adapters, send provider payloads, mutate parent preferences, or raise child-device/platform authority.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo provider send, retry worker, child-device notification, block, suspend, shield, adapter state, or parent preference mutation is created. Rollback is limited to deleting generated proof artifacts.\n',
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
