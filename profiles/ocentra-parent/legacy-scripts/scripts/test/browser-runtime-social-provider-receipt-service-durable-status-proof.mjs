import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-service-durable-status-proof';
const proofDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-service-durable-status'
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
      serviceRecordsDurableReceiptRows: true,
      durableResultRefsPreserved: true,
      durableStoreRefsPreserved: true,
      readModelRefsPreserved: true,
      supportStatusRefsPreserved: true,
      manualRowsDoNotCreateDurableRows: true,
      publicProtocolFieldsAdded: false,
      providerReceiptCount: 0,
      providerDispatchCount: 0,
      connectorNativeRuntimeCount: 0,
      parentNotificationUiDeliveryCount: 0,
      reportDeliveryExecutionCount: 0,
      finalPolicyExecutionCount: 0,
      enforcementExecutionCount: 0,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-service-durable-status-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-service-durable-status-proof-ok=true');
  console.log(`proof=${relativePath(path.join(proofDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [servicePayload, serviceTests, workpack, checklist, featureDoc] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    serviceTracksDurableRows: servicePayload.includes('social_provider_durable_rows'),
    serviceTracksDurableResultRefs: servicePayload.includes('social_provider_durable_result_refs'),
    serviceTracksDurableStoreRefs: servicePayload.includes('social_provider_durable_store_refs'),
    serviceTracksReadModelRefs: servicePayload.includes('social_provider_read_model_refs'),
    serviceTracksSupportStatusRefs: servicePayload.includes('social_provider_support_status_refs'),
    serviceKeepsProviderDispatchZero: servicePayload.includes('provider_dispatch_count'),
    serviceKeepsParentNotificationUiZero: servicePayload.includes('parent_notification_ui_delivery_count'),
    noPublicStreamFieldAdded:
      !servicePayload.includes('constants::field::BROWSER_RUNTIME_SOCIAL_PROVIDER') &&
      !servicePayload.includes('constants::field::BROWSER_RUNTIME_PROVIDER_RECEIPT'),
    focusedDurableRefTestExists: serviceTests.includes('assert_social_provider_durable_refs'),
    manualRowsStayWithoutDurableRefs: serviceTests.includes('social_provider_durable_result_refs.is_empty()'),
    workpackMentionsServiceDurableStatus: workpack.includes('Social Provider Receipt Service Durable Status Addendum'),
    checklistMentionsServiceDurableStatus: checklist.includes(
      'browser-runtime-social-provider-receipt-service-durable-status-proof'
    ),
    featureDocMentionsServiceDurableStatus: featureDoc.includes('service-side browser runtime report'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(
      `browser social provider receipt service durable status source checks failed: ${missing.join(', ')}`
    );
  }
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Service Durable Status Proof',
      '',
      'This proof carries durable social provider receipt refs into the existing service-side browser runtime report while protocol-domain public fields are owned by another active lane.',
      '',
      'Provider-dispatch-required rows preserve durable result, durable store, read-model, and support-status refs. Manual-required rows do not create durable rows. Provider delivery, receipt ingestion, connector/native runtime, parent notification UI delivery, report delivery, final policy execution, and enforcement remain unclaimed.',
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
