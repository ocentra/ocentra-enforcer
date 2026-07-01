import { spawnSync } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repoRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const testOutputDir = join(repoRoot, 'test-results', 'app-game-notification-parent-surface-intent-proof');
const appGameProofDir = join(repoRoot, 'output', 'app-game-plan-proof', '66-notification-parent-surface-intent');
const appProofDir = join(repoRoot, 'output', 'app-plan-proof', '66-notification-parent-surface-intent');
const timestamp = '2026-06-05T09:12:00Z';
const commands = [];
const initialGitStatusShort = gitOutput(['status', '--short']);

for (const path of [testOutputDir, appGameProofDir, appProofDir]) {
  await rm(path, { recursive: true, force: true });
  await mkdir(path, { recursive: true });
}

runNpm(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const parentSurface = await importSchemaDist('app-game-notification-parent-surface-intent.js');
const providerStatus = await importSchemaDist('app-game-notification-provider-status-handoff.js');
const preferenceStatus = await importSchemaDist('app-game-notification-preference-status-handoff.js');
const providerPreflight = await importSchemaDist('app-game-notification-provider-preflight.js');
const preferencePreflight = await importSchemaDist('app-game-notification-preference-preflight.js');
const refs = await importSchemaDist('family-reference-primitives.js');

const providerReadModel = providerStatus.AppGameNotificationProviderStatusHandoffReadModelSchema.parse(
  providerStatusReadModel(providerPreflight, refs)
);
const preferenceReadModel = preferenceStatus.AppGameNotificationPreferenceStatusHandoffReadModelSchema.parse(
  preferenceStatusReadModel(preferencePreflight, refs)
);
const readModel = buildParentSurfaceReadModel(parentSurface, providerReadModel, preferenceReadModel, refs);
const proof = {
  proofMode: 'app-game-notification-parent-surface-intent',
  generatedAt: timestamp,
  branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
  commit: gitOutput(['rev-parse', 'HEAD']),
  gitStatusShort: initialGitStatusShort,
  commands,
  summary: summarize(readModel),
  nonClaims: {
    parentNotificationUiRendered: readModel.parentNotificationUiRendered,
    parentPreferenceUiRendered: readModel.parentPreferenceUiRendered,
    parentFrequencyControlUiRendered: readModel.parentFrequencyControlUiRendered,
    providerDeliveryRuntimeClaimed: readModel.providerDeliveryRuntimeClaimed,
    providerReceiptIngestionClaimed: readModel.providerReceiptIngestionClaimed,
    providerCredentialsClaimed: readModel.providerCredentialsClaimed,
    cloudRoutingClaimed: readModel.cloudRoutingClaimed,
    childDeliveryClaimed: readModel.childDeliveryClaimed,
    productionRuntimeClaimed: readModel.productionRuntimeClaimed,
    productionDurableOutboxStorageClaimed: readModel.productionDurableOutboxStorageClaimed,
    adapterDispatchClaimed: readModel.adapterDispatchClaimed,
  },
  proofPaths: {
    source: 'packages/schema-domain/src/app-game-notification-parent-surface-intent.ts',
    providerStatusSource: 'packages/schema-domain/src/app-game-notification-provider-status-handoff.ts',
    preferenceStatusSource: 'packages/schema-domain/src/app-game-notification-preference-status-handoff.ts',
    harness: 'scripts/test/app-game-notification-parent-surface-intent-proof.mjs',
    evidence: 'test-results/app-game-notification-parent-surface-intent-proof/proof.json',
    appGameProofPack: 'output/app-game-plan-proof/66-notification-parent-surface-intent',
    appProofPack: 'output/app-plan-proof/66-notification-parent-surface-intent',
  },
  readModel,
};

assertProof(proof);
await writeJson(join(testOutputDir, 'parent-surface-intent-read-model.json'), readModel);
await writeJson(join(testOutputDir, 'proof.json'), proof);
await writeProofPack(appGameProofDir, proof, 'app-game WP66');
await writeProofPack(appProofDir, proof, 'app WP66');

console.log('app-game-notification-parent-surface-intent-proof-ok');
console.log(`evidence=${join('test-results', 'app-game-notification-parent-surface-intent-proof', 'proof.json')}`);

async function importSchemaDist(moduleName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', moduleName)).href);
}

function buildParentSurfaceReadModel(parentSurface, providerReadModel, preferenceReadModel, refs) {
  const rows = providerReadModel.rows.map((providerRow, index) => {
    const preferenceRow = preferenceReadModel.rows[index];
    const providerStatus = providerRow.providerStatusBoundaryEntry.providerStatus;
    const preferenceState = preferenceRow.notificationPreferenceStatusEntry.parentPreferenceState;
    const preferenceVisibility =
      preferenceState === 'channel-disabled' ? 'preference-disabled-visible' : 'preference-setup-required';

    return {
      surfaceRowId: `app-game-parent-surface-row-${providerRow.handoffRowId}`,
      sourceProviderHandoffRowId: providerRow.handoffRowId,
      sourcePreferenceHandoffRowId: preferenceRow.handoffRowId,
      sourceSchedulerEntryRef: providerRow.sourceSchedulerEntryRef,
      sourceOutboxRecordRef: providerRow.sourceOutboxRecordRef,
      providerStatus,
      deliveryResultState: preferenceRow.notificationPreferenceStatusEntry.deliveryResultState,
      parentPreferenceState: preferenceState,
      quietHoursDecision: preferenceRow.notificationPreferenceStatusEntry.quietHoursDecision,
      providerChannel: preferenceRow.notificationPreferenceStatusEntry.providerChannel,
      parentSurfaceStatus: providerStatus === 'unavailable' ? 'unavailable-visible' : 'manual-action-required',
      historyVisibility: parentSurface.appGameNotificationParentSurfaceHistoryVisibilityFor(providerStatus),
      preferenceVisibility,
      drillInRefs: [
        providerRow.providerStatusBoundaryEntry.statusEntryId,
        preferenceRow.notificationPreferenceStatusEntry.contractEntryId,
      ],
      auditRefs: [
        ...providerRow.providerStatusBoundaryEntry.auditRefs,
        ...preferenceRow.notificationPreferenceStatusEntry.auditRefs,
      ],
      manualProofRequirements: [...providerRow.manualProofRequirements, ...preferenceRow.manualProofRequirements],
      minimalSurfacePayloadBoundary:
        'Parent surface rows remain manual or unavailable visibility only without rendered UI.',
      sensitiveDetailIncluded: false,
      providerDeliveryClaimed: false,
      providerReceiptClaimed: false,
      parentPreferenceMutationClaimed: false,
      childDeliveryClaimed: false,
    };
  });

  return parentSurface.AppGameNotificationParentSurfaceIntentReadModelSchema.parse({
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    intentId: 'app-game-notification-parent-surface-intent-proof',
    generatedAt: timestamp,
    family: providerReadModel.family,
    sourceProviderStatusHandoffId: providerReadModel.handoffId,
    sourcePreferenceStatusHandoffId: preferenceReadModel.handoffId,
    sourceContractRefs: [
      'app-game-notification-provider-status-handoff',
      'app-game-notification-preference-status-handoff',
      'notifications-expectation-parent-surface-boundary',
    ],
    rows,
    manualActionRequiredCount: rows.filter((row) => row.parentSurfaceStatus === 'manual-action-required').length,
    unavailableVisibleCount: rows.filter((row) => row.parentSurfaceStatus === 'unavailable-visible').length,
    historyVisibleCount: rows.length,
    preferenceSetupRequiredCount: rows.filter((row) => row.preferenceVisibility === 'preference-setup-required').length,
    parentSurfaceNonClaims: parentSurface.RequiredAppGameNotificationParentSurfaceIntentNonClaims,
    parentNotificationUiRendered: false,
    parentPreferenceUiRendered: false,
    parentFrequencyControlUiRendered: false,
    providerDeliveryRuntimeClaimed: false,
    providerReceiptIngestionClaimed: false,
    providerCredentialsClaimed: false,
    cloudRoutingClaimed: false,
    childDeliveryClaimed: false,
    productionRuntimeClaimed: false,
    productionDurableOutboxStorageClaimed: false,
    adapterDispatchClaimed: false,
  });
}

function providerStatusReadModel(providerPreflight, refs) {
  return {
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    handoffId: 'app-game-provider-status-handoff-parent-surface',
    generatedAt: timestamp,
    family: { familyId: 'family-app-game-parent-surface' },
    sourceProviderPreflightId: 'app-game-provider-preflight-parent-surface',
    sourceContractRefs: ['app-game-notification-provider-preflight'],
    providerStatusBoundaryReadModelRef: 'v0-8-notification-provider-status-boundary',
    providerStatusBoundaryCoverageRefs: [
      'notification-provider-queued-contract',
      'notification-provider-delivered-receipt-required',
      'notification-provider-failed-contract',
      'notification-provider-unavailable-contract',
      'notification-provider-manual-required-contract',
    ],
    rows: [
      providerStatusRow(
        providerPreflight,
        refs,
        'time-limit',
        providerPreflight.AppGameNotificationProviderPreflightStatus.ProviderAdapterRequired
      ),
      providerStatusRow(
        providerPreflight,
        refs,
        'manual-required',
        providerPreflight.AppGameNotificationProviderPreflightStatus.ManualRequired
      ),
      providerStatusRow(
        providerPreflight,
        refs,
        'unavailable',
        providerPreflight.AppGameNotificationProviderPreflightStatus.Unavailable
      ),
    ],
    providerStatusManualRequiredCount: 2,
    providerStatusUnavailableCount: 1,
    handoffNonClaims: [
      'no-provider-delivery-execution',
      'no-provider-receipt-ingestion',
      'no-provider-credentials',
      'no-cloud-routing',
      'no-parent-notification-ui',
      'no-child-delivery',
      'no-retry-worker-runtime',
      'no-quiet-hours-timer-runtime',
      'no-production-durable-outbox-storage',
      'no-adapter-dispatch',
    ],
    providerDeliveryRuntimeClaimed: false,
    providerReceiptIngestionClaimed: false,
    providerCredentialsClaimed: false,
    cloudRoutingClaimed: false,
    parentNotificationUiClaimed: false,
    childDeliveryClaimed: false,
    retryExecutionRuntimeClaimed: false,
    quietHoursTimerRuntimeClaimed: false,
    productionDurableOutboxStorageClaimed: false,
    adapterDispatchClaimed: false,
  };
}

function providerStatusRow(providerPreflight, refs, label, status) {
  const unavailable = status === providerPreflight.AppGameNotificationProviderPreflightStatus.Unavailable;
  const manualRef = `manual-proof-provider-${label}`;

  return {
    handoffRowId: `provider-status-handoff-${label}`,
    sourcePreflightRowId: `provider-preflight-${label}`,
    sourcePreflightStatus: status,
    sourceSchedulerEntryRef: unavailable ? null : `scheduler-entry-app-game-${label}`,
    sourceOutboxRecordRef: unavailable ? null : `outbox-record-app-game-${label}`,
    sourceProviderChannelRef: unavailable ? null : 'in-app',
    providerStatusBoundaryEntry: {
      schemaVersion: refs.ParentContractSchemaVersion.V0_6,
      statusEntryId: `app-game-provider-status-${label}`,
      providerStatus: unavailable ? 'unavailable' : 'manual-required',
      statusProofState: unavailable ? 'provider-unavailable-contract' : 'manual-action-required',
      quietHoursReadiness: unavailable ? 'unavailable' : 'manual-required',
      escalationReadiness: unavailable ? 'unavailable' : 'manual-required',
      deliveryClaimState: unavailable ? 'not-implemented' : 'not-observed',
      notificationIntentRef: `app-game-provider-status-intent-${label}`,
      notificationStatusRef: `app-game-provider-status-ref-${label}`,
      providerAttemptRef: `app-game-provider-status-attempt-${label}`,
      auditRefs: [`app-game-provider-status-audit-${label}`],
      preferenceRefs: [`app-game-provider-status-preference-${label}`],
      readinessRefs: [`app-game-provider-status-readiness-${label}`],
      providerReceiptRefs: [],
      manualProofRequirements: [manualRef],
      minimalPayloadBoundary: 'Provider status remains a manual or unavailable setup row without delivery.',
      providerDeliveryImplemented: false,
      providerDeliveryObserved: false,
      deliveredNotificationClaimed: false,
      sensitiveProviderPayloadClaimed: false,
      providerStoresChildEvidenceClaimed: false,
      lastCheckedAt: timestamp,
    },
    manualProofRequirements: [manualRef],
  };
}

function preferenceStatusReadModel(preferencePreflight, refs) {
  return {
    schemaVersion: refs.ParentContractSchemaVersion.V0_6,
    handoffId: 'app-game-preference-status-handoff-parent-surface',
    generatedAt: timestamp,
    family: { familyId: 'family-app-game-parent-surface' },
    sourcePreferencePreflightId: 'app-game-preference-preflight-parent-surface',
    sourceContractRefs: ['app-game-notification-preference-preflight'],
    notificationRuleProviderRetryReadModelRef: 'v3-notification-rule-provider-retry-contract',
    notificationRuleProviderRetryCoverageRefs: [
      'notification-rule-provider-retry-policy-violation',
      'notification-rule-provider-retry-parent-request',
      'notification-rule-provider-retry-suspicious-unknown',
      'notification-rule-provider-retry-device-offline',
      'notification-rule-provider-retry-sync-failure',
      'notification-rule-provider-retry-provider-failure',
    ],
    rows: [
      preferenceStatusRow(
        preferencePreflight,
        refs,
        'time-limit',
        preferencePreflight.AppGameNotificationPreferencePreflightStatus.ParentPreferenceRequired
      ),
      preferenceStatusRow(
        preferencePreflight,
        refs,
        'manual-required',
        preferencePreflight.AppGameNotificationPreferencePreflightStatus.ManualRequired
      ),
      preferenceStatusRow(
        preferencePreflight,
        refs,
        'unavailable',
        preferencePreflight.AppGameNotificationPreferencePreflightStatus.Unavailable
      ),
    ],
    parentPreferenceManualSetupRequiredCount: 2,
    quietHoursManualRequiredCount: 2,
    preferenceStatusUnavailableCount: 1,
    handoffNonClaims: [
      'no-parent-preference-ui',
      'no-parent-frequency-control-ui',
      'no-parent-notification-ui',
      'no-quiet-hours-timer-runtime',
      'no-provider-delivery-execution',
      'no-provider-receipt-ingestion',
      'no-provider-credentials',
      'no-cloud-routing',
      'no-child-delivery',
      'no-retry-worker-runtime',
      'no-production-durable-outbox-storage',
      'no-adapter-dispatch',
    ],
    parentPreferenceUiClaimed: false,
    parentFrequencyControlUiClaimed: false,
    parentNotificationUiClaimed: false,
    quietHoursTimerRuntimeClaimed: false,
    providerDeliveryRuntimeClaimed: false,
    providerReceiptIngestionClaimed: false,
    providerCredentialsClaimed: false,
    cloudRoutingClaimed: false,
    childDeliveryClaimed: false,
    retryExecutionRuntimeClaimed: false,
    productionDurableOutboxStorageClaimed: false,
    adapterDispatchClaimed: false,
  };
}

function preferenceStatusRow(preferencePreflight, refs, label, status) {
  const unavailable = status === preferencePreflight.AppGameNotificationPreferencePreflightStatus.Unavailable;
  const manualRef = `manual-proof-preference-${label}`;

  return {
    handoffRowId: `preference-status-handoff-${label}`,
    sourcePreferenceRowId: `preference-preflight-${label}`,
    sourcePreferenceStatus: status,
    sourceSchedulerEntryRef: unavailable ? null : `scheduler-entry-app-game-${label}`,
    sourceOutboxRecordRef: unavailable ? null : `outbox-record-app-game-${label}`,
    sourceProviderChannelRef: unavailable ? null : 'in-app',
    sourceReasonCodeRef: unavailable ? null : 'policy-violation',
    sourceParentPreferenceState: unavailable ? null : 'manual-setup-required',
    sourceQuietHoursDecision: unavailable ? null : 'manual-required',
    sourceParentPreferenceRequirementRefs: [manualRef],
    sourceQuietHoursRequirementRefs: [manualRef],
    notificationPreferenceStatusEntry: {
      schemaVersion: refs.ParentContractSchemaVersion.V0_6,
      contractEntryId: `app-game-preference-status-${label}`,
      reasonCode: unavailable ? 'provider-failure' : 'policy-violation',
      providerChannel: 'in-app',
      deliveryAttemptState: unavailable ? 'provider-disabled' : 'eligible',
      deliveryResultState: unavailable ? 'not-sent' : 'manual-required',
      retryPolicyState: unavailable ? 'provider-disabled' : 'manual-review',
      quietHoursDecision: unavailable ? 'allow' : 'manual-required',
      escalationDecision: unavailable ? 'none' : 'manual-review',
      parentPreferenceState: unavailable ? 'channel-disabled' : 'manual-setup-required',
      notificationRuleRef: `app-game-preference-status-rule-${label}`,
      notificationIntentRef: `app-game-preference-status-intent-${label}`,
      deliveryAttemptRef: `app-game-preference-status-attempt-${label}`,
      deliveryResultRef: `app-game-preference-status-result-${label}`,
      retryPolicyRef: `app-game-preference-status-retry-${label}`,
      quietHoursPolicyRef: `app-game-preference-status-quiet-hours-${label}`,
      escalationPolicyRef: `app-game-preference-status-escalation-${label}`,
      parentPreferenceRef: `app-game-preference-status-parent-preference-${label}`,
      auditRefs: [`app-game-preference-status-audit-${label}`],
      evidenceRefs: [manualRef],
      providerReceiptRefs: [],
      manualProofRequirements: [manualRef],
      minimalProviderPayloadBoundary: 'Preference status remains setup-only without provider delivery.',
      providerAdapterImplemented: false,
      deliveryAttemptExecuted: false,
      providerReceiptObserved: false,
      rawEvidenceInProviderPayload: false,
      providerStoresChildEvidenceClaimed: false,
      lastCheckedAt: timestamp,
    },
    manualProofRequirements: [manualRef],
  };
}

function summarize(readModel) {
  return {
    rows: readModel.rows.length,
    manualActionRequiredCount: readModel.manualActionRequiredCount,
    unavailableVisibleCount: readModel.unavailableVisibleCount,
    historyVisibleCount: readModel.historyVisibleCount,
    preferenceSetupRequiredCount: readModel.preferenceSetupRequiredCount,
    parentSurfaceStatuses: countBy(readModel.rows.map((row) => row.parentSurfaceStatus)),
    preferenceVisibility: countBy(readModel.rows.map((row) => row.preferenceVisibility)),
  };
}

function assertProof(proof) {
  if (
    proof.summary.rows !== 3 ||
    proof.summary.manualActionRequiredCount !== 2 ||
    proof.summary.unavailableVisibleCount !== 1 ||
    proof.summary.historyVisibleCount !== 3 ||
    proof.summary.preferenceSetupRequiredCount !== 2
  ) {
    throw new Error(`Unexpected parent surface intent summary: ${JSON.stringify(proof.summary)}`);
  }
  if (Object.values(proof.nonClaims).some((value) => value !== false)) {
    throw new Error(`Parent surface intent overclaimed runtime behavior: ${JSON.stringify(proof.nonClaims)}`);
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
      '- Scope: central app/game provider-status plus preference-status rows to parent-surface history and preference intent rows.',
      '- Source inspected: schema-domain provider status handoff, preference status handoff, and parent surface intent contracts.',
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
      '- node scripts/test/app-game-notification-parent-surface-intent-proof.mjs: PASS',
      '',
      JSON.stringify(proof.summary, null, 2),
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '02-rust-protocol-proof.log'),
    'N/A: WP66 is central schema-domain parent-surface intent proof only; no Rust protocol or service route is added.\n',
    'utf8'
  );
  await writeJson(join(proofDir, '03-runtime-evidence.json'), {
    runtimeClaimed: false,
    uiRendered: false,
    providerDeliveryClaimed: false,
    providerReceiptClaimed: false,
    childDeliveryClaimed: false,
    reason:
      'WP66 produces parent-surface intent rows only; a future portal/runtime slice must render and execute them.',
  });
  await writeJson(join(proofDir, '05-policy-action-proof.json'), {
    policyExecutionClaimed: false,
    adapterDispatchClaimed: false,
    rows: proof.readModel.rows.map((row) => ({
      surfaceRowId: row.surfaceRowId,
      providerStatus: row.providerStatus,
      parentPreferenceState: row.parentPreferenceState,
      manualProofRequirements: row.manualProofRequirements,
    })),
  });
  await writeFile(
    join(proofDir, '08-security-negative-proof.log'),
    [
      'Security/no-claim proof:',
      '',
      '- Parent notification UI rendered: false',
      '- Parent preference UI rendered: false',
      '- Sensitive detail included: false for every row',
      '- Provider delivery runtime claimed: false',
      '- Provider receipt ingestion claimed: false',
      '- Provider credentials claimed: false',
      '- Child delivery claimed: false',
      '- Adapter dispatch claimed: false',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeFile(
    join(proofDir, '10-validation-commands.log'),
    [...commands, 'node scripts/test/app-game-notification-parent-surface-intent-proof.mjs'].join('\n') + '\n',
    'utf8'
  );
  await writeFile(
    join(proofDir, 'README.md'),
    [
      `# ${label}`,
      '',
      'This proof maps central app/game notification provider-status and preference-status handoff rows into parent-surface intent rows.',
      '',
      'It does not claim rendered parent notification UI, preference UI, provider delivery, provider receipts, credentials, production runtime, child delivery, adapter dispatch, broad blocking, or platform support.',
      '',
    ].join('\n'),
    'utf8'
  );
  await writeJson(join(proofDir, 'proof.json'), proof);
}

async function writeJson(path, data) {
  await writeFile(path, `${JSON.stringify(data, null, 2)}\n`, 'utf8');
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function run(command, args) {
  commands.push([command, ...args].join(' '));
  const result = spawnSync(command, args, { cwd: repoRoot, shell: false, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed\n${result.stdout}${result.stderr}`);
  }
}

function gitOutput(args) {
  const result = spawnSync('git', args, { cwd: repoRoot, shell: false, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed\n${result.stdout}${result.stderr}`);
  }
  return result.stdout.trim();
}

function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return run(command, commandArgs, ...rest);
}
