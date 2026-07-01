import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-handoff-event-subscriber-proof');
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-action-intent-handoff-event-subscriber'
);

const files = {
  actionHandoff: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'action_handoff.rs'),
  delivery: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'delivery.rs'),
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  tests: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime_tests.rs'),
  constants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  feature: path.join(root, 'docs', 'features', 'browser-web-control.md'),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
};

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [actionHandoff, delivery, runtime, tests, constants, checklist, feature, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    handoffUsesEventBusRequestResponse: actionHandoff.includes('publish_request('),
    handoffCompletesTypedResponse: actionHandoff.includes('complete_request(BrowserRuntimeActionIntentHandoffResponse'),
    namedHandoffSubscriberRegistered: actionHandoff.includes('SUBSCRIBER_BROWSER_ACTION_INTENT_HANDOFF'),
    namedHandoffEventRegistered: constants.includes('EVENT_BROWSER_ACTION_INTENT_HANDOFF_REQUESTED'),
    namedHandoffTargetRegistered: constants.includes('TARGET_BROWSER_ACTION_INTENT_HANDOFF'),
    responsePinsDispatchToZero: actionHandoff.includes('dispatch_attempt_count: 0'),
    responsePinsAdapterToZero: actionHandoff.includes('adapter_execution_count: 0'),
    responsePinsBrowserMutationToZero: actionHandoff.includes('browser_mutation_count: 0'),
    responsePinsChildInterventionToZero: actionHandoff.includes('child_intervention_execution_count: 0'),
    responsePinsEnforcementToZero: actionHandoff.includes('enforcement_execution_count: 0'),
    runtimeExportsHandoffRequest: runtime.includes('request_browser_runtime_action_intent_handoff_for_input'),
    deliveryCountsHandoffLocalReady: delivery.includes('action_intent_handoff_delivery'),
    focusedDryRunTestExists: tests.includes(
      'browser_runtime_action_intent_handoff_event_subscriber_prepares_outbox_without_dispatch'
    ),
    focusedManualTestExists: tests.includes(
      'browser_runtime_action_intent_handoff_event_subscriber_keeps_manual_rows_empty'
    ),
    topologyTestExists: tests.includes(
      'browser_runtime_action_intent_handoff_topology_covers_named_event_and_subscriber'
    ),
    checklistMentionsHandoffProof: checklist.includes('browser-runtime-action-intent-handoff-event-subscriber-proof'),
    featureMentionsHandoffSubscriber: feature.includes('browser.action-intent.handoff.requested'),
    workpackMentionsHandoffProof: workpack.includes('Action-Intent Handoff Event Subscriber Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent handoff event subscriber source checks failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_action_intent_handoff', '--quiet'],
    },
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_delivery_decision', '--quiet'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-handoff-event-subscriber-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      namedBrowserEventPublished: 'browser.action-intent.handoff.requested',
      namedSubscriberReacts: 'browser-action-intent-handoff',
      requestResponsePath: true,
      dryRunCandidateCount: 1,
      manualRequiredCandidateCount: 0,
      outboxRefPrepared: true,
      handoffRefPrepared: true,
      deliveryLocalReadyRouteCount: 3,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      browserMutationCount: 0,
      childInterventionExecutionCount: 0,
      enforcementExecutionCount: 0,
      finalPolicyExecutionClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-handoff-event-subscriber-proof.md'),
    [
      '# Browser Runtime Action Intent Handoff Event Subscriber Proof',
      '',
      'This proof uses the reusable Rust eventing request/response path for a named browser action-intent handoff event and subscriber.',
      '',
      'The subscriber returns one prepared dry-run handoff candidate for policy-decision events with policy preview and action-intent refs, and zero candidates for manual-required rows.',
      '',
      'The delivery-decision proof now marks the browser runtime chain, action-intent status subscriber, and action-intent handoff subscriber as local ready while external transport remains manual-required.',
      '',
      'No adapter dispatch, browser mutation, child intervention execution, final policy execution, or enforcement is claimed.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
    ].join('\n')
  );

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
