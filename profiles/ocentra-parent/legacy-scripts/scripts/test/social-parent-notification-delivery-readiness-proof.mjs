import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(
  root,
  'output',
  'browser-plan-proof',
  'social-parent-notification-delivery-readiness-proof'
);
const resultDirectory = join(root, 'test-results', 'social-parent-notification-delivery-readiness-proof');

const requiredFiles = [
  'packages/schema-domain/src/social-parent-notification-delivery-readiness.ts',
  'scripts/test/social-parent-notification-delivery-readiness-proof.mjs',
];

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const packageJson = await readText('packages/schema-domain/package.json');
  const contract = await readText('packages/schema-domain/src/social-parent-notification-delivery-readiness.ts');
  const featureDoc = await readText('docs/features/browser-web-control.md');
  const socialFeatureDoc = await readText('docs/features/social-video-control.md');
  const socialExpectationDoc = await readText('docs/expectations/social-video-control.md');
  const socialWorkpackReadme = await readText('docs/plans/browser-plan/social-platform-account-feed/readme.md');
  const checklist = await readText('docs/plans/browser-plan/implementation-checklist.md');
  const proofModule =
    await import('../../packages/schema-domain/dist/social-parent-notification-delivery-readiness.js');
  const reportWriterModule = await import('../../packages/schema-domain/dist/social-report-writer-delivery-proof.js');
  const receiptBoundaryModule =
    await import('../../packages/schema-domain/dist/social-alert-report-provider-receipt-boundary-proof.js');
  const receiptIngestionModule =
    await import('../../packages/schema-domain/dist/social-alert-report-provider-receipt-ingestion-readiness.js');

  const staticReadModel = proofModule.buildSocialParentNotificationDeliveryReadinessReadModel(
    {
      generatedAt: '2026-06-08T09:55:00Z',
      readinessId: 'social-parent-notification-delivery-readiness-proof',
      sourceReportWriterProofRef: 'social-report-writer-delivery-proof',
    },
    reportWriterModule.SocialReportWriterDeliveryProofReadModel
  );
  const receiptBackedReadModel = proofModule.buildSocialParentNotificationDeliveryReadinessReadModel(
    {
      generatedAt: '2026-06-08T09:55:00Z',
      readinessId: 'social-parent-notification-delivery-from-receipt-ingestion-proof',
      sourceReportWriterProofRef: 'social-report-writer-delivery-from-receipt-ingestion-proof',
    },
    buildReceiptIngestionBackedReportWriterReadModel(reportWriterModule, receiptBoundaryModule, receiptIngestionModule)
  );
  const staticSummary = proofModule.summarizeSocialParentNotificationDeliveryReadiness(staticReadModel);
  const receiptBackedSummary = proofModule.summarizeSocialParentNotificationDeliveryReadiness(receiptBackedReadModel);
  const allRows = [...staticReadModel.rows, ...receiptBackedReadModel.rows];

  const checks = [
    checkFilesExist(),
    checkIncludes(
      contract,
      'parentNotificationUiDelivered: Schema.Literal(false)',
      'parent notification UI delivery guard'
    ),
    checkIncludes(
      contract,
      'externalRuntimeReportDeliveryClaimed: Schema.Literal(false)',
      'external report delivery guard'
    ),
    checkIncludes(contract, 'providerDeliveryAttempted: Schema.Literal(false)', 'provider delivery guard'),
    checkIncludes(contract, 'providerReceiptIngested: Schema.Literal(false)', 'provider receipt guard'),
    checkIncludes(contract, 'finalPolicyDecisionClaimed: Schema.Literal(false)', 'final policy guard'),
    checkIncludes(contract, 'enforcementClaimed: Schema.Literal(false)', 'enforcement guard'),
    checkIncludes(packageJson, './social-parent-notification-delivery-readiness', 'schema-domain package export'),
    checkIncludes(
      featureDoc,
      'social-parent-notification-delivery-readiness-proof',
      'feature doc notification delivery readiness note'
    ),
    checkIncludes(
      socialFeatureDoc,
      'parent-owned local delivery result ref',
      'social feature doc local delivery result note'
    ),
    checkIncludes(
      socialExpectationDoc,
      'local delivery result refs only for report-ready rows',
      'social expectation local delivery result boundary'
    ),
    checkIncludes(
      socialWorkpackReadme,
      'parent-owned local delivery result ref',
      'social workpack README local delivery result note'
    ),
    checkIncludes(
      checklist,
      'social-parent-notification-delivery-readiness-proof',
      'checklist notification delivery readiness proof note'
    ),
    {
      label: 'static report writer row yields one parent report status ready row',
      pass:
        staticSummary.totalRows === 2 &&
        staticSummary.parentReportStatusReadyCount === 1 &&
        staticSummary.parentLocalDeliveryResultCount === 1 &&
        staticSummary.manualRequiredCount === 1 &&
        staticSummary.parentNotificationUiDeliveryClaimed === false,
    },
    {
      label: 'receipt ingestion backed rows stay manual or unavailable',
      pass:
        receiptBackedSummary.totalRows === 3 &&
        receiptBackedSummary.parentReportStatusReadyCount === 0 &&
        receiptBackedSummary.parentLocalDeliveryResultCount === 0 &&
        receiptBackedSummary.manualRequiredCount === 2 &&
        receiptBackedSummary.unavailableCount === 1,
    },
    {
      label: 'parent local delivery result is recorded only for parent report status ready rows',
      pass: allRows.every(
        (row) =>
          (row.notificationDeliveryReadinessState === 'parent-report-status-ready') ===
            row.parentLocalDeliveryResultRecorded &&
          (row.notificationDeliveryReadinessState === 'parent-report-status-ready') ===
            (row.parentLocalDeliveryResultRef !== undefined)
      ),
    },
    {
      label: 'no row claims UI delivery external report delivery provider delivery final policy or enforcement',
      pass: allRows.every(
        (row) =>
          row.parentNotificationUiDelivered === false &&
          row.externalRuntimeReportDeliveryClaimed === false &&
          row.providerDeliveryAttempted === false &&
          row.providerReceiptIngested === false &&
          row.finalPolicyDecisionClaimed === false &&
          row.enforcementClaimed === false
      ),
    },
  ].flat();

  const failures = checks.filter((check) => !check.pass).map((check) => check.label);
  const proof = {
    schemaVersion: 1,
    proofMode: 'social-parent-notification-delivery-readiness-proof',
    generatedAt: new Date().toISOString(),
    files: requiredFiles,
    outputDirectory: relativePath(outputDirectory),
    staticSummary,
    receiptBackedSummary,
    rows: allRows.map((row) => ({
      notificationDeliveryReadinessRowId: row.notificationDeliveryReadinessRowId,
      sourceReportWriterDeliveryRowRef: row.sourceReportWriterDeliveryRowRef,
      sourceIntentRef: row.sourceIntentRef,
      notificationDeliveryReadinessState: row.notificationDeliveryReadinessState,
      reportDeliveryExecutionState: row.reportDeliveryExecutionState,
      parentVisibleReportStatusRef: row.parentVisibleReportStatusRef,
      parentNotificationUiRef: row.parentNotificationUiRef,
      parentLocalDeliveryResultRef: row.parentLocalDeliveryResultRef,
      manualProofRequirements: row.manualProofRequirements,
      parentLocalDeliveryResultRecorded: row.parentLocalDeliveryResultRecorded,
      parentNotificationUiDelivered: row.parentNotificationUiDelivered,
      externalRuntimeReportDeliveryClaimed: row.externalRuntimeReportDeliveryClaimed,
      providerDeliveryAttempted: row.providerDeliveryAttempted,
      providerReceiptIngested: row.providerReceiptIngested,
      finalPolicyDecisionClaimed: row.finalPolicyDecisionClaimed,
      enforcementClaimed: row.enforcementClaimed,
    })),
    nonClaims: staticReadModel.nonClaims,
    checks,
    failures,
  };

  if (failures.length > 0) {
    throw new Error(`Social parent notification delivery readiness proof failed:\n${failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-social-parent-notification-delivery-readiness-proof.md');
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(proof)}\n`);

  console.log('social-parent-notification-delivery-readiness-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
}

function buildReceiptIngestionBackedReportWriterReadModel(
  reportWriterModule,
  receiptBoundaryModule,
  receiptIngestionModule
) {
  const sourceReadModel =
    receiptIngestionModule.SocialAlertReportProviderReceiptIngestionReadinessReadModelSchema.parse({
      schemaVersion: reportWriterModule.SocialReportWriterDeliveryProofReadModel.schemaVersion,
      readinessId: 'social-parent-notification-provider-receipt-ingestion-readiness',
      generatedAt: '2026-06-08T09:55:00Z',
      sourceReceiptBoundaryId: 'social-parent-notification-provider-receipt-boundary',
      sourceContractRefs: ['social-alert-report-provider-receipt-boundary-proof'],
      sourceReceiptBoundaryNonClaims: receiptBoundaryModule.RequiredSocialAlertReportProviderReceiptBoundaryNonClaims,
      rows: [
        receiptIngestionRow('social-parent-notification-high-risk', 'provider-dispatch-required'),
        receiptIngestionRow('social-parent-notification-manual-required', 'manual-receipt-required'),
        receiptIngestionRow('social-parent-notification-unavailable', 'provider-unavailable'),
      ],
      ingestionContractRequiredCount: 1,
      manualReceiptRequiredCount: 1,
      providerUnavailableCount: 1,
      providerReceiptObservedCount: 0,
      receiptIngestionReadinessNonClaims:
        receiptIngestionModule.RequiredSocialAlertReportProviderReceiptIngestionReadinessNonClaims,
      providerDeliveryRuntimeClaimed: false,
      providerReceiptIngestionRuntimeClaimed: false,
      providerWebhookRuntimeClaimed: false,
      providerCredentialsClaimed: false,
      providerReceiptObservedClaimed: false,
      cloudRoutingClaimed: false,
      parentNotificationUiDeliveryClaimed: false,
      reportDeliveryExecutionClaimed: false,
      finalPolicyExecutionClaimed: false,
      connectorNativeRuntimeClaimed: false,
      enforcementClaimed: false,
    });

  return reportWriterModule.buildSocialReportWriterDeliveryProofFromReceiptIngestionReadiness(
    {
      generatedAt: '2026-06-08T09:55:00Z',
      proofId: 'social-report-writer-delivery-from-receipt-ingestion-proof',
      sourceAlertReportIntentProofRef: 'social-provider-receipt-ingestion-readiness-proof',
    },
    sourceReadModel
  );
}

function receiptIngestionRow(sourceIntentRef, sourceReceiptBoundaryState) {
  const ingestionReadinessState =
    sourceReceiptBoundaryState === 'provider-dispatch-required'
      ? 'ingestion-contract-required'
      : sourceReceiptBoundaryState;

  return {
    ingestionRowId: `social-provider-receipt-ingestion-${sourceIntentRef}`,
    sourceReceiptRowRef: `social-provider-receipt-row-${sourceIntentRef}`,
    sourceIntentRef,
    sourceProviderAttemptRef: `social-provider-attempt-${sourceIntentRef}`,
    sourceReceiptBoundaryState,
    ingestionReadinessState,
    webhookEndpointRef: null,
    providerCredentialRef: null,
    durableReceiptResultRef: null,
    providerReceiptObservedRefs: [],
    receiptProofRequirements: [`social-provider-receipt-proof-required-${sourceIntentRef}`],
    ingestionProofRequirements: [`social-provider-receipt-ingestion-proof-required-${sourceIntentRef}`],
    providerDeliveryExecutionClaimed: false,
    providerReceiptIngestionRuntimeClaimed: false,
    providerWebhookRuntimeClaimed: false,
    providerCredentialsClaimed: false,
    providerReceiptObservedClaimed: false,
    cloudRoutingClaimed: false,
    parentNotificationUiDeliveryClaimed: false,
    reportDeliveryExecutionClaimed: false,
    finalPolicyExecutionClaimed: false,
    connectorNativeRuntimeClaimed: false,
    enforcementClaimed: false,
  };
}

function checkFilesExist() {
  return requiredFiles.map((path) => ({
    label: `${path} exists`,
    pass: existsSync(join(root, path)),
  }));
}

function checkIncludes(text, expected, label) {
  return {
    label,
    pass: text.includes(expected),
  };
}

function markdownFor(proof) {
  return [
    '# Social Parent Notification Delivery Readiness Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    `Static rows: ${proof.staticSummary.totalRows}`,
    `Static parent report status ready rows: ${proof.staticSummary.parentReportStatusReadyCount}`,
    `Static local delivery result rows: ${proof.staticSummary.parentLocalDeliveryResultCount}`,
    `Receipt-backed rows: ${proof.receiptBackedSummary.totalRows}`,
    `Receipt-backed manual-required rows: ${proof.receiptBackedSummary.manualRequiredCount}`,
    `Receipt-backed unavailable rows: ${proof.receiptBackedSummary.unavailableCount}`,
    `Receipt-backed local delivery result rows: ${proof.receiptBackedSummary.parentLocalDeliveryResultCount}`,
    '',
    'This proof carries schema-domain social report writer readiness into a',
    'parent notification/report delivery readiness boundary. A parent-owned',
    'report artifact, receipt, and local delivery result can become a',
    'parent-visible report status row, but the proof still records no parent',
    'notification UI delivery, no external',
    'runtime report delivery, no provider dispatch or receipt ingestion, no',
    'final policy execution, and no enforcement. Receipt-ingestion-backed',
    'rows remain manual-required or unavailable until webhook, credential,',
    'durable receipt, observed receipt, and UI delivery proofs exist.',
  ].join('\n');
}

async function readText(path) {
  return readFile(join(root, path), 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
