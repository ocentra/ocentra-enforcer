import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-durable-proof';
const proofDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-social-provider-receipt-durable');

const files = {
  durable: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime',
    'social_provider_receipt_durable.rs'
  ),
  durableTypes: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime',
    'social_provider_receipt_durable_types.rs'
  ),
  tests: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime_tests',
    'browser_event_runtime_social_provider_receipt_tests.rs'
  ),
  constants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
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
    args: ['test', '-p', 'ocentra-parent-agent-core', 'social_provider_receipt_durable', '--quiet'],
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
      durableRowsBuiltFromNamedReceiptSubscriber: true,
      duplicateRequestEventsRejected: true,
      providerDispatchRequiredRows: 1,
      manualReceiptRequiredRows: 0,
      providerAttemptRefsPreserved: true,
      providerReceiptProofRefsPreserved: true,
      durableStoreRefsPreserved: true,
      readModelRefsPreserved: true,
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
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-durable-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-durable-proof-ok=true');
  console.log(`proof=${relativePath(path.join(proofDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [durable, durableTypes, tests, constants, workpack, checklist, featureDoc] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    durableCallsNamedReceiptSubscriber: durable.includes(
      'request_browser_runtime_social_provider_receipt_status_for_input'
    ),
    durableRejectsDuplicateRequestEvents: durable.includes('DuplicateRequestEvent'),
    durableRejectsUnsupportedClaims: durable.includes('has_unsupported_claims'),
    durablePreservesProviderAttemptRef: durable.includes('provider_attempt_ref'),
    durablePreservesProviderReceiptProofRef: durable.includes('provider_receipt_proof_ref'),
    durableTypesDefineReadModelState: durableTypes.includes('ProviderDispatchRequiredManualReceipt'),
    durableResultRefConstantExists: constants.includes(
      'TEST_BROWSER_RUNTIME_SOCIAL_PROVIDER_RECEIPT_DURABLE_RESULT_REF'
    ),
    durableStoreRefConstantExists: constants.includes('TEST_BROWSER_RUNTIME_SOCIAL_PROVIDER_RECEIPT_DURABLE_STORE_REF'),
    readModelRefConstantExists: constants.includes('TEST_BROWSER_RUNTIME_SOCIAL_PROVIDER_RECEIPT_READ_MODEL_REF'),
    supportStatusRefConstantExists: constants.includes(
      'TEST_BROWSER_RUNTIME_SOCIAL_PROVIDER_RECEIPT_SUPPORT_STATUS_REF'
    ),
    focusedTestExists: tests.includes(
      'browser_runtime_social_provider_receipt_durable_preserves_refs_without_execution'
    ),
    workpackMentionsDurableProof: workpack.includes('Social Provider Receipt Durable Addendum'),
    checklistMentionsDurableProof: checklist.includes('browser-runtime-social-provider-receipt-durable-proof'),
    featureDocMentionsDurableProof: featureDoc.includes('durable social provider receipt'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt durable source checks failed: ${missing.join(', ')}`);
  }
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Durable Proof',
      '',
      'This proof projects the named browser social provider receipt request into a durable read-model row that preserves the request event, action intent, provider attempt, receipt proof, durable store, read model, support status, source, and evidence references.',
      '',
      'It intentionally keeps provider receipt ingestion, provider dispatch, connector/native runtime, parent notification UI delivery, report delivery, final policy execution, and enforcement unclaimed.',
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
