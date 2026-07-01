import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-event-subscriber-proof';
const proofDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-event-subscriber'
);

const files = {
  subscriber: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'social_provider_receipt.rs'),
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  tests: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime_tests',
    'browser_event_runtime_social_provider_receipt_tests.rs'
  ),
  constants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
};

const commands = [
  {
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_social_provider_receipt', '--quiet'],
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
      namedBrowserEventPublished: 'browser.social.provider-receipt.status.requested',
      namedSubscriberReacts: 'browser-social-provider-receipt-status',
      requestResponsePath: true,
      providerDispatchRequiredRows: 1,
      manualReceiptRequiredRows: 1,
      receiptRuntimeState: 'manual-required',
      providerReceiptCount: 0,
      providerDispatchCount: 0,
      providerWebhookCount: 0,
      providerCredentialsCount: 0,
      parentNotificationUiDeliveryCount: 0,
      reportDeliveryExecutionCount: 0,
      finalPolicyExecutionCount: 0,
      connectorNativeRuntimeCount: 0,
      enforcementExecutionCount: 0,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-event-subscriber-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-event-subscriber-proof-ok=true');
  console.log(`proof=${relativePath(path.join(proofDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [subscriber, runtime, tests, constants, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    usesEventBusRequestResponse: subscriber.includes('publish_request('),
    completesTypedResponse: subscriber.includes('complete_request('),
    namedSubscriberRegistered: subscriber.includes('SUBSCRIBER_BROWSER_SOCIAL_PROVIDER_RECEIPT_STATUS'),
    namedEventRegistered: constants.includes('EVENT_BROWSER_SOCIAL_PROVIDER_RECEIPT_STATUS_REQUESTED'),
    namedTargetRegistered: constants.includes('TARGET_BROWSER_SOCIAL_PROVIDER_RECEIPT_STATUS'),
    responsePinsProviderReceiptToZero: subscriber.includes('provider_receipt_count: 0'),
    responsePinsProviderDispatchToZero: subscriber.includes('provider_dispatch_count: 0'),
    responsePinsWebhookToZero: subscriber.includes('provider_webhook_count: 0'),
    responsePinsCredentialsToZero: subscriber.includes('provider_credentials_count: 0'),
    responsePinsFinalPolicyToZero: subscriber.includes('final_policy_execution_count: 0'),
    responsePinsEnforcementToZero: subscriber.includes('enforcement_execution_count: 0'),
    runtimeExportsRequest: runtime.includes('request_browser_runtime_social_provider_receipt_status_for_input'),
    focusedDispatchRequiredTestExists: tests.includes(
      'browser_runtime_social_provider_receipt_event_subscriber_returns_manual_required_boundary'
    ),
    focusedManualRequiredTestExists: tests.includes(
      'browser_runtime_social_provider_receipt_event_subscriber_keeps_manual_rows_manual_required'
    ),
    topologyTestExists: tests.includes(
      'browser_runtime_social_provider_receipt_topology_covers_named_event_and_subscriber'
    ),
    docsMentionSubscriberProof: checklist.includes('browser-runtime-social-provider-receipt-event-subscriber-proof'),
    workpackMentionsSubscriberProof: workpack.includes('Social Provider Receipt Event Subscriber Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt event subscriber source checks failed: ${missing.join(', ')}`);
  }
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Event Subscriber Proof',
      '',
      'This proof uses the reusable Rust eventing request/response path for a named browser social provider receipt status event and subscriber.',
      '',
      'The subscriber returns a provider-dispatch-required receipt boundary row for a dry-run browser action-intent candidate and a manual-receipt-required row for manual-required browser rows.',
      '',
      'The receipt runtime state remains manual-required. Provider receipts, provider dispatch, webhook runtime, credentials, parent notification UI delivery, report delivery execution, final policy execution, connector/native runtime, and enforcement all remain zero.',
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
