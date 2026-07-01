import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', 'social-report-writer-delivery-proof');
const resultDirectory = join(root, 'test-results', 'social-report-writer-delivery-proof');

const requiredFiles = [
  'packages/schema-domain/src/social-report-writer-delivery-proof.ts',
  'scripts/test/social-report-writer-delivery-proof.mjs',
];

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const packageJson = await readText('packages/schema-domain/package.json');
  const featureDoc = await readText('docs/features/social-video-control.md');
  const workpackReadme = await readText('docs/plans/browser-plan/social-platform-account-feed/readme.md');
  const contract = await readText('packages/schema-domain/src/social-report-writer-delivery-proof.ts');
  const proofModule = await import('../../packages/schema-domain/dist/social-report-writer-delivery-proof.js');
  const receiptBoundaryModule =
    await import('../../packages/schema-domain/dist/social-alert-report-provider-receipt-boundary-proof.js');
  const receiptIngestionModule =
    await import('../../packages/schema-domain/dist/social-alert-report-provider-receipt-ingestion-readiness.js');

  const readModel = buildReceiptIngestionBackedReadModel(proofModule, receiptBoundaryModule, receiptIngestionModule);
  const summary = proofModule.summarizeSocialReportWriterDeliveryProof(readModel);
  const checks = [
    checkFilesExist(),
    checkIncludes(packageJson, './social-report-writer-delivery-proof', 'schema-domain package export'),
    checkIncludes(featureDoc, 'social-report-writer-delivery-proof', 'social/video feature proof note'),
    checkIncludes(workpackReadme, 'social-report-writer-delivery-proof', 'social workpack README proof note'),
    checkIncludes(
      contract,
      'externalRuntimeReportDeliveryClaimed: Schema.Literal(false)',
      'external report delivery guard'
    ),
    checkIncludes(contract, 'providerDeliveryAttempted: Schema.Literal(false)', 'provider delivery guard'),
    checkIncludes(contract, 'finalPolicyDecisionClaimed: Schema.Literal(false)', 'final policy guard'),
    checkIncludes(contract, 'enforcementClaimed: Schema.Literal(false)', 'enforcement guard'),
    checkIncludes(
      contract,
      'buildSocialReportWriterDeliveryProofFromReceiptIngestionReadiness',
      'receipt ingestion readiness builder'
    ),
    {
      label: 'receipt ingestion backed rows stay manual or unavailable',
      pass: summary.reportDeliveryReadyRows === 0 && summary.manualRequiredRows === 2 && summary.unavailableRows === 1,
    },
  ].flat();

  const failures = checks.filter((check) => !check.pass).map((check) => check.label);
  const proof = {
    schemaVersion: 1,
    proofMode: 'social-report-writer-delivery-proof',
    generatedAt: new Date().toISOString(),
    files: requiredFiles,
    outputDirectory: relativePath(outputDirectory),
    checks,
    summary,
    rows: readModel.reportWriterDeliveryRows.map((row) => ({
      reportWriterDeliveryRowId: row.reportWriterDeliveryRowId,
      sourceIntentRef: row.sourceIntentRef,
      reportWriterDeliveryState: row.reportWriterDeliveryState,
      reportWriterReceiptState: row.reportWriterReceiptState,
      manualProofRequirements: row.manualProofRequirements,
      parentOwnedReportArtifactWritten: row.parentOwnedReportArtifactWritten,
      parentOwnedReportReceiptRecorded: row.parentOwnedReportReceiptRecorded,
      externalRuntimeReportDeliveryClaimed: row.externalRuntimeReportDeliveryClaimed,
      providerDeliveryAttempted: row.providerDeliveryAttempted,
      providerReceiptIngested: row.providerReceiptIngested,
      finalPolicyDecisionClaimed: row.finalPolicyDecisionClaimed,
      enforcementClaimed: row.enforcementClaimed,
    })),
    nonClaims: readModel.nonClaims,
    failures,
  };

  if (failures.length > 0) {
    throw new Error(`Social report writer delivery proof failed:\n${failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-social-report-writer-delivery-proof.md');
  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(proof)}\n`);

  console.log('social-report-writer-delivery-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
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
    '# Social Report Writer Delivery Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    `Rows: ${proof.summary.totalRows}`,
    `Report delivery ready rows: ${proof.summary.reportDeliveryReadyRows}`,
    `Manual-required rows: ${proof.summary.manualRequiredRows}`,
    `Unavailable rows: ${proof.summary.unavailableRows}`,
    `External runtime report delivery claimed: ${proof.summary.externalRuntimeReportDeliveryClaimed}`,
    `Provider delivery attempted: ${proof.summary.providerDeliveryAttempted}`,
    `Enforcement claimed: ${proof.summary.enforcementClaimed}`,
    '',
    'This proof adds a parent-owned social report writer delivery-readiness',
    'boundary from receipt ingestion readiness. It proves provider-dispatch,',
    'manual-receipt, and provider-unavailable rows stay manual or unavailable',
    'until webhook, credential, durable receipt, and observed receipt proofs',
    'exist. It preserves explicit non-claims for external runtime report',
    'delivery, provider dispatch, provider receipt ingestion, raw social',
    'content, final policy execution, and enforcement.',
  ].join('\n');
}

function buildReceiptIngestionBackedReadModel(proofModule, receiptBoundaryModule, receiptIngestionModule) {
  const sourceReadModel =
    receiptIngestionModule.SocialAlertReportProviderReceiptIngestionReadinessReadModelSchema.parse({
      schemaVersion: proofModule.SocialReportWriterDeliveryProofReadModel.schemaVersion,
      readinessId: 'social-report-writer-provider-receipt-ingestion-readiness',
      generatedAt: '2026-06-08T06:20:00Z',
      sourceReceiptBoundaryId: 'social-report-writer-provider-receipt-boundary',
      sourceContractRefs: [
        'social-alert-report-provider-receipt-boundary-proof',
        'provider-receipt-webhook-contract',
        'provider-receipt-durable-store-contract',
      ],
      sourceReceiptBoundaryNonClaims: receiptBoundaryModule.RequiredSocialAlertReportProviderReceiptBoundaryNonClaims,
      rows: [
        receiptIngestionRow(
          'social-report-writer-high-risk',
          'provider-dispatch-required',
          'ingestion-contract-required'
        ),
        receiptIngestionRow(
          'social-report-writer-manual-required',
          'manual-receipt-required',
          'manual-receipt-required'
        ),
        receiptIngestionRow('social-report-writer-unavailable', 'provider-unavailable', 'provider-unavailable'),
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

  return proofModule.buildSocialReportWriterDeliveryProofFromReceiptIngestionReadiness(
    {
      generatedAt: '2026-06-08T06:20:00Z',
      proofId: 'social-report-writer-delivery-from-receipt-ingestion-proof',
      sourceAlertReportIntentProofRef: 'social-alert-report-provider-receipt-ingestion-readiness-proof',
    },
    sourceReadModel
  );
}

function receiptIngestionRow(sourceIntentRef, sourceReceiptBoundaryState, ingestionReadinessState) {
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
    ingestionProofRequirements: ingestionProofRequirements(sourceIntentRef, ingestionReadinessState),
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

function ingestionProofRequirements(sourceIntentRef, ingestionReadinessState) {
  if (ingestionReadinessState === 'provider-unavailable') {
    return [`social-provider-receipt-ingestion-provider-unavailable-${sourceIntentRef}`];
  }
  if (ingestionReadinessState === 'manual-receipt-required') {
    return [`social-provider-receipt-ingestion-manual-provider-setup-${sourceIntentRef}`];
  }
  return [
    `social-provider-receipt-webhook-contract-required-${sourceIntentRef}`,
    `social-provider-receipt-credential-proof-required-${sourceIntentRef}`,
    `social-provider-receipt-durable-store-required-${sourceIntentRef}`,
  ];
}

async function readText(path) {
  return readFile(join(root, path), 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
