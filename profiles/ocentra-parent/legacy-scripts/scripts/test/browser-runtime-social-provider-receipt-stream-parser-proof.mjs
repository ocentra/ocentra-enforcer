import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-stream-parser-proof';
const resultDir = path.join(root, 'test-results', proofName);
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-stream-parser'
);

const files = {
  protocol: path.join(root, 'crates', 'agent-protocol', 'src', 'browser', 'social_provider_receipt.rs'),
  durable: path.join(root, 'crates', 'agent-protocol', 'src', 'browser', 'social_provider_receipt_durable.rs'),
  core: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'social_provider_receipt.rs'),
  coreTest: path.join(
    root,
    'crates',
    'agent-core',
    'tests',
    'unit',
    'browser_event_runtime_tests',
    'browser_event_runtime_social_provider_receipt_tests.rs'
  ),
};

const commands = [
  {
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'social_provider_receipt', '--quiet'],
    label: 'cargo test -p ocentra-parent-agent-protocol social_provider_receipt --quiet',
  },
  {
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'browser_event_runtime_social_provider_receipt', '--quiet'],
    label: 'cargo test -p ocentra-parent-agent-core browser_event_runtime_social_provider_receipt --quiet',
  },
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commandResults = commands.map(runCommand);
  const sourceChecks = await readSourceChecks();
  assertSourceChecks(sourceChecks);

  const proof = {
    proofName,
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    commit: gitOutput(['rev-parse', '--short', 'HEAD']),
    commandResults,
    sourceChecks,
    verified: {
      rustSocialProviderReceiptParserOwned: true,
      rustSocialProviderReceiptDurableOwned: true,
      browserReceiptBoundaryStatusIsRustOwned: true,
      providerDeliveryClaimed: false,
      receiptIngestionClaimed: false,
      parentNotificationDeliveryClaimed: false,
      reportDeliveryClaimed: false,
      finalPolicyExecutionClaimed: false,
      connectorNativeRuntimeClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(resultDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-social-provider-receipt-stream-parser-proof.md'),
    markdownFor(proof)
  );

  console.log('browser-runtime-social-provider-receipt-stream-parser-proof-ok=true');
  console.log(`proof=${relativePath(path.join(resultDir, 'proof.json'))}`);
}

async function readSourceChecks() {
  const [protocol, durable, core, coreTest] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    protocolExposesReceiptBoundaryCounts:
      protocol.includes('receipt_boundary_row_count: usize') &&
      protocol.includes('provider_dispatch_required_count: usize') &&
      protocol.includes('manual_receipt_required_count: usize'),
    protocolExposesDurableRefs:
      durable.includes('durable_result_ref: SourceComponent') &&
      durable.includes('durable_store_ref: SourceComponent') &&
      durable.includes('support_status_ref: SourceComponent'),
    coreDerivesReceiptStatus: core.includes('social_provider_receipt_status_response_from_payload'),
    coreDerivesManualRequiredBoundary: core.includes('manual_receipt_required_count'),
    coreTestCoversReceiptStatus:
      coreTest.includes('browser_runtime_social_provider_receipt_event_subscriber_returns_manual_required_boundary') &&
      coreTest.includes('browser_runtime_social_provider_receipt_event_subscriber_keeps_manual_rows_manual_required'),
    coreTestCoversDurableRefs:
      coreTest.includes('browser_runtime_social_provider_receipt_topology_covers_named_event_and_subscriber') &&
      coreTest.includes('browser_runtime_social_provider_receipt_durable_preserves_refs_without_execution'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser social provider receipt stream parser proof failed: ${missing.join(', ')}`);
  }
}

function runCommand(item) {
  execFileSync(item.command, item.args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  return {
    command: item.label,
    status: 'passed',
  };
}

function markdownFor(proof) {
  return (
    [
      '# Browser Runtime Social Provider Receipt Stream Parser Proof',
      '',
      'This proof closes the Rust social provider receipt stream boundary.',
      '',
      'The shared agent-protocol and agent-core owners now define the receipt boundary counts, durable refs, derived receipt status, and durable preservation checks without relying on the deleted TypeScript parser surface.',
      '',
      'No-claim boundary: provider delivery, receipt ingestion runtime, parent notification delivery, report delivery, final policy execution, connector/native runtime, browser mutation, child intervention execution, unmanaged exact URL support, and enforcement remain unclaimed.',
      '',
      'Validation:',
      ...proof.commandResults.map((result) => `- \`${result.command}\` (${result.status})`),
    ].join('\n') + '\n'
  );
}

function gitOutput(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
}

function relativePath(targetPath) {
  return path.relative(root, targetPath).replaceAll('\\', '/');
}
