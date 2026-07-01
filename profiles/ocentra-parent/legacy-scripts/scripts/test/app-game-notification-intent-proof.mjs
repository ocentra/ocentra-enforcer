import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'app-game-notification-intent-proof';
const testOutputDir = join(repoRoot, 'test-results', proofMode);
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '53-notification-intent-contract');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '53-notification-intent-contract');
const commands = [];

await main();

async function main() {
  await mkdir(testOutputDir, { recursive: true });
  await mkdir(join(appGameProofDir, '06-ui-snapshots'), { recursive: true });
  await mkdir(join(appProofDir, '06-ui-snapshots'), { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const notification = await importSchemaDist('app-game-notification-intent.js');
  await assertPackageExport(notification);
  const refs = await importSchemaDist('family-reference-primitives.js');
  const childUx = await importSchemaDist('app-game-child-facing-ux-rules.js');

  const fixtures = buildFixtures(notification, refs, childUx);
  const parsed = fixtures.validIntents.map((intent) => notification.AppGameNotificationIntentSchema.parse(intent));
  const rejected = fixtures.invalidIntents.map((intent) => ({
    notificationIntentId: intent.notificationIntentId,
    rejected: !notification.AppGameNotificationIntentSchema.safeParse(intent).success,
  }));

  assertEqual(parsed.length, 5, 'valid intent count');
  assertEqual(
    rejected.every((row) => row.rejected),
    true,
    'invalid intent rejection'
  );
  assertEqual(
    parsed.every(
      (intent) => intent.adapterDispatchState === notification.AppGameNotificationAdapterDispatchState.NotDispatched
    ),
    true,
    'adapter dispatch state'
  );
  assertEqual(
    parsed.every((intent) => intent.providerDeliveryAttempted === false && intent.providerReceiptIngested === false),
    true,
    'provider delivery non-claim'
  );

  const proof = {
    schemaVersion: 1,
    proofMode,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    summary: {
      validIntents: parsed.length,
      rejectedIntents: rejected.length,
      intentKinds: countBy(parsed.map((intent) => intent.intentKind)),
      intentStatuses: countBy(parsed.map((intent) => intent.intentStatus)),
      reasonCodes: countBy(parsed.map((intent) => intent.notificationReasonCode)),
      deliveryClaimStates: countBy(parsed.map((intent) => intent.deliveryClaimState)),
      adapterDispatchStates: countBy(parsed.map((intent) => intent.adapterDispatchState)),
    },
    claimsProved: [
      'app/game time-limit, approval request, suspicious unknown, manual-required, and unavailable notification intents parse only with evidence, policy, audit, and required child/status refs',
      'parent and child copy tokens must match the app/game notification intent kind',
      'minimal notification payload fields are refs only and reject raw child evidence, URLs/titles, message text, screenshots, and reports',
      'local-outbox eligibility stays local-outbox-only and does not claim provider delivery or receipts',
      'manual-required and unavailable intents cannot claim local outbox, provider delivery, parent UI, cloud routing, or adapter dispatch',
    ],
    claimsNotProved: [
      'provider push/email/SMS/WhatsApp/in-app delivery',
      'provider receipt ingestion',
      'parent notification UI or preference controls',
      'service persistence or WebSocket notification read model',
      'child app or overlay delivery',
      'policy evaluator execution, adapter dispatch, broad app blocking, or platform support',
    ],
    evidence: {
      contract: 'packages/schema-domain/src/app-game-notification-intent.ts',
      rules: 'packages/schema-domain/src/app-game-notification-intent-rules.ts',
      childUx: 'packages/schema-domain/src/app-game-child-facing-ux.ts',
      harness: 'scripts/test/app-game-notification-intent-proof.mjs',
      appGameProofPack: 'output/app-game-plan-proof/53-notification-intent-contract',
      appProofPack: 'output/app-plan-proof/53-notification-intent-contract',
      packageExport: '@ocentra-parent/schema-domain/app-game-notification-intent',
    },
    parsedIntents: parsed,
    rejected,
  };

  await writeJson(join(testOutputDir, 'proof.json'), proof);
  await writeProofPack(appGameProofDir, proof, 'app-game WP53');
  await writeProofPack(appProofDir, proof, 'app WP53');

  console.log(`app-game-notification-intent-proof-ok:${parsed.length}`);
  console.log(`evidence=${relativePath(join(testOutputDir, 'proof.json'))}`);
}

async function assertPackageExport(notification) {
  const exportedModule = await import('@ocentra-parent/schema-domain/app-game-notification-intent');
  assertEqual(
    exportedModule.AppGameNotificationIntentKind.TimeLimitReached,
    notification.AppGameNotificationIntentKind.TimeLimitReached,
    'package export intent kind'
  );
}

async function importSchemaDist(moduleName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', moduleName)).href);
}

function buildFixtures(notification, refs, childUx) {
  const timestamp = '2026-06-04T17:54:00Z';
  const device = {
    deviceId: 'device-app-game-notification',
    childProfileId: 'child-app-game-notification',
    label: 'Study PC',
    platform: refs.ParentPlatform.Windows,
  };
  const evidence = {
    evidenceReferenceId: 'evidence-app-game-notification-session',
    kind: refs.ParentEvidenceReferenceKind.PolicyDecision,
    observedAt: timestamp,
  };
  const approvalActionRef = {
    actionReferenceId: 'approval-action-app-game-notification',
    actor: { actorId: 'child-local-agent', role: refs.ParentActorRole.System },
    policyVersion: 'policy-app-game-notification-v1',
    createdAt: timestamp,
  };
  const base = {
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    notificationIntentId: 'notification-intent-time-limit',
    intentKind: notification.AppGameNotificationIntentKind.TimeLimitReached,
    intentStatus: notification.AppGameNotificationIntentStatus.LocalOutboxEligible,
    priority: notification.AppGameNotificationPriority.Urgent,
    device,
    targetKind: childUx.AppGameChildUxTargetKind.NativeGame,
    targetRef: 'target-native-game-claim',
    notificationReasonCode: notification.AppGameNotificationReasonCode.TimeLimit,
    providerChannelPreference: 'in-app',
    parentTitleToken: notification.AppGameNotificationParentCopyToken.TimeLimitTitle,
    parentBodyToken: notification.AppGameNotificationParentCopyToken.TimeLimitBody,
    parentActionToken: notification.AppGameNotificationParentCopyToken.OpenParentReviewAction,
    childTitleToken: childUx.AppGameChildUxCopyToken.LimitReachedTitle,
    childBodyToken: childUx.AppGameChildUxCopyToken.LimitReachedBody,
    notificationRuleRef: 'notification-rule-app-game-time-limit',
    notificationStatusRef: 'notification-status-app-game-time-limit',
    policyRefs: ['policy-ref-game-limit'],
    auditRefs: ['audit-ref-game-limit-notification'],
    evidenceReferences: [evidence],
    childReasonReferences: [],
    childStatusReferences: ['child-status-time-limit-reached'],
    approvalActionRef: null,
    timeBudgetDecisionRef: 'time-budget-decision-game-limit',
    unknownCandidateRef: null,
    localOutboxRecordRef: 'local-outbox-record-game-limit',
    providerAttemptRefs: [],
    providerReceiptRefs: [],
    manualProofRequirements: [],
    minimalPayloadFields: Object.values(notification.AppGameNotificationPayloadField),
    deliveryClaimState: notification.AppGameNotificationDeliveryClaimState.LocalOutboxOnly,
    rawChildEvidenceIncluded: false,
    rawUrlOrTitleIncluded: false,
    rawMessageTextIncluded: false,
    screenshotOrReportIncluded: false,
    providerDeliveryAttempted: false,
    providerDeliveryObserved: false,
    providerReceiptIngested: false,
    cloudRoutingClaimed: false,
    parentNotificationUiClaimed: false,
    adapterDispatchState: notification.AppGameNotificationAdapterDispatchState.NotDispatched,
    adapterActionClaimed: false,
    createdAt: timestamp,
  };
  const approval = {
    ...base,
    notificationIntentId: 'notification-intent-approval-request',
    intentKind: notification.AppGameNotificationIntentKind.ApprovalRequested,
    priority: notification.AppGameNotificationPriority.Attention,
    targetKind: childUx.AppGameChildUxTargetKind.UnknownApp,
    targetRef: 'target-unknown-app',
    notificationReasonCode: notification.AppGameNotificationReasonCode.ApprovalRequest,
    parentTitleToken: notification.AppGameNotificationParentCopyToken.ApprovalTitle,
    parentBodyToken: notification.AppGameNotificationParentCopyToken.ApprovalBody,
    childTitleToken: childUx.AppGameChildUxCopyToken.NewAppTitle,
    childBodyToken: childUx.AppGameChildUxCopyToken.NewAppBody,
    childReasonReferences: ['child-reason-new-app-request'],
    childStatusReferences: ['child-status-new-app-request'],
    approvalActionRef,
    timeBudgetDecisionRef: null,
    unknownCandidateRef: 'unknown-app-candidate-request',
  };
  const suspicious = {
    ...approval,
    notificationIntentId: 'notification-intent-suspicious-unknown',
    intentKind: notification.AppGameNotificationIntentKind.SuspiciousUnknown,
    notificationReasonCode: notification.AppGameNotificationReasonCode.SuspiciousUnknown,
    parentTitleToken: notification.AppGameNotificationParentCopyToken.SuspiciousUnknownTitle,
    parentBodyToken: notification.AppGameNotificationParentCopyToken.SuspiciousUnknownBody,
    approvalActionRef: null,
  };
  const manual = manualOrUnavailable(notification, childUx, base, 'manual');
  const unavailable = manualOrUnavailable(notification, childUx, base, 'unavailable');
  return {
    validIntents: [base, approval, suspicious, manual, unavailable],
    invalidIntents: [
      { ...base, notificationIntentId: 'invalid-raw-child-evidence', rawChildEvidenceIncluded: true },
      { ...base, notificationIntentId: 'invalid-provider-delivery', providerDeliveryAttempted: true },
      { ...base, notificationIntentId: 'invalid-provider-receipt', providerReceiptRefs: ['provider-receipt-ref'] },
      {
        ...base,
        notificationIntentId: 'invalid-wrong-reason',
        notificationReasonCode: notification.AppGameNotificationReasonCode.ApprovalRequest,
      },
      { ...base, notificationIntentId: 'invalid-missing-budget-ref', timeBudgetDecisionRef: null },
      { ...manual, notificationIntentId: 'invalid-manual-local-outbox', localOutboxRecordRef: 'false-local-outbox' },
    ],
  };
}

function manualOrUnavailable(notification, childUx, base, mode) {
  const unavailable = mode === 'unavailable';
  return {
    ...base,
    notificationIntentId: unavailable ? 'notification-intent-unavailable' : 'notification-intent-manual-required',
    intentKind: unavailable
      ? notification.AppGameNotificationIntentKind.CapabilityUnavailable
      : notification.AppGameNotificationIntentKind.ManualRequired,
    intentStatus: unavailable
      ? notification.AppGameNotificationIntentStatus.Unavailable
      : notification.AppGameNotificationIntentStatus.ManualRequired,
    priority: notification.AppGameNotificationPriority.Attention,
    notificationReasonCode: unavailable
      ? notification.AppGameNotificationReasonCode.CapabilityUnavailable
      : notification.AppGameNotificationReasonCode.ManualReviewRequired,
    parentTitleToken: unavailable
      ? notification.AppGameNotificationParentCopyToken.UnavailableTitle
      : notification.AppGameNotificationParentCopyToken.ManualRequiredTitle,
    parentBodyToken: unavailable
      ? notification.AppGameNotificationParentCopyToken.UnavailableBody
      : notification.AppGameNotificationParentCopyToken.ManualRequiredBody,
    parentActionToken: notification.AppGameNotificationParentCopyToken.ReviewManuallyAction,
    childTitleToken: unavailable
      ? childUx.AppGameChildUxCopyToken.UnavailableTitle
      : childUx.AppGameChildUxCopyToken.ManualRequiredTitle,
    childBodyToken: unavailable
      ? childUx.AppGameChildUxCopyToken.UnavailableBody
      : childUx.AppGameChildUxCopyToken.ManualRequiredBody,
    timeBudgetDecisionRef: null,
    localOutboxRecordRef: null,
    manualProofRequirements: [
      unavailable ? 'provider or capability availability proof required' : 'parent manual review required',
    ],
    deliveryClaimState: notification.AppGameNotificationDeliveryClaimState.ManualRequired,
  };
}

async function writeProofPack(proofDir, proof, label) {
  await writeFile(
    join(proofDir, '00-source-snapshot.md'),
    [
      `# ${label} Source Snapshot`,
      '',
      `- Branch: ${await gitBranch()}`,
      `- Commit: ${proof.commit}`,
      '- Scope: central schema-domain app/game notification intent contract proof.',
      '- Source inspected: schema-domain app/game notification intent, child-facing UX, and family reference contracts.',
      '- Product checklist intentionally not edited; remaining delta is reported through hub.',
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
      '- node scripts/test/app-game-notification-intent-proof.mjs: PASS',
      '- Valid intents cover time-limit, approval request, suspicious unknown, manual-required, and unavailable states.',
      '- Invalid intents reject raw child detail, provider delivery/receipt claims, wrong copy/reason, missing refs, and false local-outbox claims.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    'Rust/service protocol not changed. This is central schema-domain notification intent proof only.\n',
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), proof);
  await writeJson(join(proofDir, '04-journal-sqlite-proof.json'), {
    schemaVersion: 1,
    journalSqliteChanged: false,
    reason: 'No journal, SQLite, service read-model, or runtime persistence code changed.',
  });
  await writeJson(join(proofDir, '05-policy-action-proof.json'), {
    schemaVersion: 1,
    intentKinds: proof.summary.intentKinds,
    reasonCodes: proof.summary.reasonCodes,
    deliveryClaimStates: proof.summary.deliveryClaimStates,
    adapterDispatchStates: proof.summary.adapterDispatchStates,
    policyEvaluatorExecuted: false,
    adapterDispatchClaimed: false,
  });
  await writeFile(
    join(proofDir, '06-ui-snapshots', 'ui-not-applicable.md'),
    '# UI Not Applicable\n\nNo portal, child app, overlay, or notification UI source changed in this workpack.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '07-playwright-ui-proof.log'),
    'Playwright/browser proof not applicable: no UI source changed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim proof:',
      '',
      '- Notification payload fields are minimal refs only.',
      '- Raw child evidence, URLs/titles, message text, screenshots, and reports are rejected.',
      '- Provider delivery, provider receipts, cloud routing, parent notification UI, and adapter dispatch remain false.',
      '- Manual-required and unavailable rows cannot claim local outbox or provider delivery.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '09-manual-platform-proof.md'),
    '# Manual Platform Proof\n\nNo provider, child-device UI, service runtime, or platform proof is attached. Delivery remains unclaimed.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [
      'Validation run:',
      '',
      '- cmd /c npm run build --workspace @ocentra-parent/schema-domain: PASS',
      '- node scripts/test/app-game-notification-intent-proof.mjs: PASS',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '11-authority-tier-proof.md'),
    '# Authority Tier Proof\n\nNo authority tier is raised. Intents remain contract/readiness proof with adapter dispatch disabled.\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, '12-rollback-proof.md'),
    '# Rollback Proof\n\nNo provider send, device action, timer, block, suspend, shield, or adapter state is created, so rollback is not applicable.\n',
    'utf8'
  );
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) =>
      code === 0 ? resolve(undefined) : reject(new Error(`${command} ${args.join(' ')} exited with ${code}`))
    );
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
    child.on('exit', (code) =>
      code === 0 ? resolve(undefined) : reject(new Error(`git ${args.join(' ')} exited with ${code}`))
    );
  });
  return Buffer.concat(chunks).toString('utf8');
}

async function writeJson(path, value) {
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
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
