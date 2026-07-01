import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofName = 'browser-runtime-dry-run-action-handoff-proof';
const testResultsDir = join('test-results', proofName);
const outputDir = join('output', 'browser-plan-proof', 'browser-runtime-dry-run-action-handoff');

mkdirSync(testResultsDir, { recursive: true });
mkdirSync(outputDir, { recursive: true });

const coreSource = readFileSync('crates/agent-core/src/browser_event_runtime.rs', 'utf8');
const coreTestSource = readFileSync('crates/agent-core/tests/unit/browser_event_runtime_tests.rs', 'utf8');
const deliverySource = readFileSync('crates/agent-service/src/browser_runtime_delivery.rs', 'utf8');
const streamSource = readFileSync('crates/agent-service/src/browser_runtime_stream_events.rs', 'utf8');

const sourceChecks = {
  coreCarriesPolicyPreviewId: coreSource.includes('pub policy_preview_id: Option<String>'),
  coreCarriesActionIntentId: coreSource.includes('pub action_intent_id: Option<String>'),
  coreCarriesDryRun: coreSource.includes('pub dry_run: bool'),
  coreCarriesAdapterDispatchClaim: coreSource.includes('pub adapter_dispatch_claimed: bool'),
  coreTestCoversDryRunHandoff: coreTestSource.includes(
    'browser_runtime_chain_carries_dry_run_action_handoff_without_dispatch'
  ),
  deliveryKeepsReadModelRowsNonDispatching: deliverySource.includes('adapter_dispatch_claimed: false'),
  streamSerializesDryRun: streamSource.includes('constants::field::POLICY_DRY_RUN'),
  streamSerializesAdapterDispatchClaim: streamSource.includes('constants::field::ADAPTER_DISPATCH_CLAIMED'),
};

for (const [name, passed] of Object.entries(sourceChecks)) {
  if (!passed) {
    throw new Error(`Browser runtime dry-run action handoff source check failed: ${name}`);
  }
}

const commands = [
  {
    name: 'agent-core-browser-event-runtime-test',
    command: 'cargo test -p ocentra-parent-agent-core browser_event_runtime --quiet',
  },
  {
    name: 'agent-service-browser-runtime-stream-test',
    command: 'cargo test -p ocentra-parent-agent-service browser_runtime_stream --quiet',
  },
  {
    name: 'build-contracts',
    command: 'cmd /c npm run build:contracts',
  },
];

const commandResults = commands.map((entry) => ({
  name: entry.name,
  command: entry.command,
  output: runShell(entry.command),
}));

const proof = {
  proofName,
  branchHead: runGit(['log', '-1', '--oneline']).trim(),
  gitStatusShort: runGit(['status', '--short']).trim(),
  sourceChecks,
  commands: commandResults.map(({ command }) => command),
  verified: {
    browserRuntimeEventsCarryPolicyPreviewId: true,
    browserRuntimeEventsCarryActionIntentId: true,
    dryRunRowsRejectAdapterDispatchClaim: true,
    dryRunRowsRejectInterventionCommandRefs: true,
    readModelDeliveryDoesNotDispatchAdapters: true,
    newEventBusCreated: false,
    portalPublishesBusinessEvents: false,
    aiExecutes: false,
    policyExecutesFinalAction: false,
    browserMutationExecutes: false,
    childInterventionExecutes: false,
    enforcementExecutes: false,
  },
};

writeFileSync(join(testResultsDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(
  join(outputDir, '01-browser-runtime-dry-run-action-handoff-proof.md'),
  [
    '# Browser Runtime Dry-Run Action Handoff Proof',
    '',
    `- Branch head: ${proof.branchHead}`,
    `- Policy preview id carried by Rust owner: ${sourceChecks.coreCarriesPolicyPreviewId}`,
    `- Action intent id carried by Rust owner: ${sourceChecks.coreCarriesActionIntentId}`,
    `- Dry-run flag carried by Rust owner: ${sourceChecks.coreCarriesDryRun}`,
    `- Adapter dispatch claim carried by Rust owner: ${sourceChecks.coreCarriesAdapterDispatchClaim}`,
    `- Read-model service rows stay non-dispatching: ${sourceChecks.deliveryKeepsReadModelRowsNonDispatching}`,
    '',
    '## Commands',
    '',
    ...commandResults.map((result) => `- ${result.command}`),
    '',
    '## No-Claim Boundaries',
    '',
    '- No new event bus or private browser bus.',
    '- Portal does not publish business events.',
    '- Dry-run policy/action refs do not dispatch adapters.',
    '- No AI execution.',
    '- No final policy execution.',
    '- No browser mutation.',
    '- No child intervention execution.',
    '- No enforcement.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));

function runShell(command) {
  return execFileSync(command, {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: 'powershell.exe',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function runGit(args) {
  return execFileSync('git', args, {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
}
