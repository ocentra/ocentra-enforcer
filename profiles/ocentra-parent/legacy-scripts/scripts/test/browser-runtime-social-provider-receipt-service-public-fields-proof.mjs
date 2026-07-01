import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-service-public-fields-proof';
const proofDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-service-public-fields'
);

const files = {
  fieldConstants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'field.rs'),
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
    commit: gitOutput(['rev-parse', '--short', 'HEAD']),
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    sourceChecks,
    verified: {
      rustProtocolFieldConstantsAdded: true,
      servicePayloadPublishesSocialProviderReceiptRows: true,
      servicePayloadPublishesProviderAttemptAndReceiptProofRefs: true,
      servicePayloadPublishesDurableResultStoreReadModelAndSupportRefs: true,
      manualRowsPublishZeroDurableRows: true,
      providerDispatchCount: 0,
      connectorNativeRuntimeCount: 0,
      parentNotificationUiDeliveryCount: 0,
      reportDeliveryExecutionCount: 0,
      finalPolicyExecutionCount: 0,
      enforcementExecutionCount: 0,
      typescriptDefaultsParserPortalUpdated: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-service-public-fields-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-service-public-fields-proof-ok=true');
  console.log(`proof=${relativePath(path.join(proofDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [fieldConstants, servicePayload, serviceTests, workpack, checklist, featureDoc] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    fieldConstantsIncludeReceiptBoundaryRows: fieldConstants.includes(
      'BROWSER_RUNTIME_SOCIAL_PROVIDER_RECEIPT_BOUNDARY_ROWS'
    ),
    fieldConstantsIncludeDurableRefs: fieldConstants.includes('BROWSER_RUNTIME_SOCIAL_PROVIDER_DURABLE_RESULT_REFS'),
    servicePayloadIncludesReceiptBoundaryRows: servicePayload.includes(
      'BROWSER_RUNTIME_SOCIAL_PROVIDER_RECEIPT_BOUNDARY_ROWS'
    ),
    servicePayloadIncludesAttemptRefs: servicePayload.includes('BROWSER_RUNTIME_SOCIAL_PROVIDER_ATTEMPT_REFS'),
    servicePayloadIncludesDurableRefs: servicePayload.includes('BROWSER_RUNTIME_SOCIAL_PROVIDER_DURABLE_RESULT_REFS'),
    serviceTestAssertsPublicPayloadFields: serviceTests.includes('assert_social_provider_public_payload_fields'),
    serviceTestAssertsManualPayloadFields: serviceTests.includes('assert_social_provider_manual_public_payload_fields'),
    docsMentionPublicRustFields: workpack.includes('Social Provider Receipt Service Public Fields Addendum'),
    checklistMentionsProof: checklist.includes('browser-runtime-social-provider-receipt-service-public-fields-proof'),
    featureDocMentionsRustServicePayloadFields: featureDoc.includes('Rust service payload fields'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt public field source checks failed: ${missing.join(', ')}`);
  }
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Service Public Fields Proof',
      '',
      'This proof adds Rust protocol field constants and service payload fields for the existing social provider receipt status path.',
      '',
      'The service payload now exposes social provider receipt boundary rows, provider-dispatch-required rows, manual-receipt-required rows, provider attempt refs, receipt proof refs, durable result refs, durable store refs, read-model refs, and support-status refs. Manual-required rows publish zero durable rows and empty durable refs.',
      '',
      'This does not update TypeScript defaults, the shared parser, or portal state while another lane owns the shared protocol defaults file. Provider delivery, receipt ingestion runtime, connector/native runtime, parent notification UI delivery, report delivery, final policy execution, and enforcement remain unclaimed.',
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
