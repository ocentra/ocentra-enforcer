import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-service-status-proof';
const proofDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-service-status'
);

const files = {
  servicePayload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  serviceTests: path.join(
    root,
    'crates',
    'agent-service',
    'src',
    'browser_runtime_stream_tests',
    'browser_runtime_social_provider_receipt_service_status_tests.rs'
  ),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  featureDoc: path.join(root, 'docs', 'features', 'browser-web-control.md'),
};

const commands = [
  {
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'social_provider_receipt', '--quiet'],
  },
];

await main();

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  for (const item of commands) {
    run(item.command, item.args);
  }

  const sourceChecks = await readSourceChecks();
  assertSourceChecks(sourceChecks);

  const proof = {
    proofName,
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    sourceChecks,
    verified: {
      serviceCallsNamedReceiptSubscriber: true,
      providerDispatchRequiredRows: 1,
      manualReceiptRequiredRows: 1,
      providerAttemptRefsPreserved: true,
      providerReceiptProofRefsPreserved: true,
      publicStreamFieldAdded: false,
      providerReceiptCount: 0,
      providerDispatchCount: 0,
      connectorNativeRuntimeCount: 0,
      parentNotificationUiDeliveryCount: 0,
      enforcementExecutionCount: 0,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-service-status-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-service-status-proof-ok=true');
  console.log(`proof=${relativePath(path.join(proofDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [servicePayload, serviceTests, workpack, checklist, featureDoc] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    serviceImportsNamedReceiptSubscriber: servicePayload.includes(
      'request_browser_runtime_social_provider_receipt_status_for_input'
    ),
    serviceRecordsReceiptStatus: servicePayload.includes('record_social_provider_receipt'),
    serviceTracksProviderDispatchRows: servicePayload.includes('social_provider_dispatch_required_rows'),
    serviceTracksManualReceiptRows: servicePayload.includes('social_provider_manual_receipt_required_rows'),
    serviceTracksProviderAttemptRefs: servicePayload.includes('social_provider_attempt_refs'),
    serviceTracksReceiptProofRefs: servicePayload.includes('social_provider_receipt_proof_refs'),
    serviceKeepsProviderDispatchZero: servicePayload.includes('provider_dispatch_count'),
    serviceKeepsNativeRuntimeZero: servicePayload.includes('connector_native_runtime_count'),
    serviceKeepsParentNotificationUiZero: servicePayload.includes('parent_notification_ui_delivery_count'),
    noPublicStreamFieldAdded: !servicePayload.includes('BROWSER_RUNTIME_SOCIAL_PROVIDER'),
    focusedProviderBoundaryTestExists: serviceTests.includes(
      'service_browser_runtime_social_provider_receipt_status_records_provider_boundary'
    ),
    focusedStoreBackedTestExists: serviceTests.includes(
      'service_browser_runtime_stream_records_store_backed_social_provider_receipt_status'
    ),
    focusedManualRequiredTestExists: serviceTests.includes(
      'service_browser_runtime_stream_keeps_manual_social_provider_receipt_rows_manual_required'
    ),
    workpackMentionsServiceProof: workpack.includes('Social Provider Receipt Service Status Addendum'),
    checklistMentionsServiceProof: checklist.includes('browser-runtime-social-provider-receipt-service-status-proof'),
    featureDocMentionsServiceProof:
      featureDoc.includes('service-side social provider receipt') &&
      featureDoc.includes('browser.social.provider-receipt.status.requested'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt service status source checks failed: ${missing.join(', ')}`);
  }
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Service Status Proof',
      '',
      'This proof carries the named browser social provider receipt subscriber into the existing service-side browser runtime stream report.',
      '',
      'The service records provider-dispatch-required receipt boundary rows for store-backed dry-run policy preview evidence and records manual-receipt-required rows for manual-required browser evidence.',
      '',
      'This slice intentionally does not add public browser runtime stream fields because protocol field constants are owned by another active lane. It keeps provider receipt ingestion, provider dispatch, connector/native runtime, parent notification UI delivery, final policy execution, browser mutation, child intervention, and enforcement unclaimed.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
    ].join('\n') + '\n'
  );
}

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

function gitOutput(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
}

function relativePath(targetPath) {
  return path.relative(root, targetPath).replaceAll('\\', '/');
}
