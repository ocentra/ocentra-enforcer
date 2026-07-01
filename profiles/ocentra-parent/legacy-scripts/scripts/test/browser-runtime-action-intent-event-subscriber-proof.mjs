import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-event-subscriber-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-event-subscriber');

const files = {
  actionStatus: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'action_status.rs'),
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  tests: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime_tests.rs'),
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

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [actionStatus, runtime, tests, constants, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    usesEventBusRequestResponse: actionStatus.includes('publish_request('),
    completesTypedResponse: actionStatus.includes('complete_request(BrowserRuntimeActionIntentStatusResponse'),
    namedSubscriberRegistered: actionStatus.includes('SUBSCRIBER_BROWSER_ACTION_INTENT_STATUS'),
    namedEventRegistered: constants.includes('EVENT_BROWSER_ACTION_INTENT_STATUS_REQUESTED'),
    namedTargetRegistered: constants.includes('TARGET_BROWSER_ACTION_INTENT_STATUS'),
    responsePinsDispatchToZero: actionStatus.includes('dispatch_attempt_count: 0'),
    responsePinsAdapterToZero: actionStatus.includes('adapter_execution_count: 0'),
    responsePinsChildInterventionToZero: actionStatus.includes('child_intervention_execution_count: 0'),
    responsePinsEnforcementToZero: actionStatus.includes('enforcement_execution_count: 0'),
    runtimeExportsRequest: runtime.includes('request_browser_runtime_action_intent_status_for_input'),
    focusedDryRunTestExists: tests.includes('browser_runtime_action_intent_event_subscriber_returns_pending_status'),
    focusedManualTestExists: tests.includes('browser_runtime_action_intent_event_subscriber_keeps_manual_rows_empty'),
    docsMentionSubscriberProof: checklist.includes('browser-runtime-action-intent-event-subscriber-proof'),
    workpackMentionsSubscriberProof: workpack.includes('Action-Intent Event Subscriber Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent event subscriber source checks failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_action_intent_event_subscriber', '--quiet'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-event-subscriber-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      namedBrowserEventPublished: 'browser.action-intent.status.requested',
      namedSubscriberReacts: 'browser-action-intent-status',
      requestResponsePath: true,
      dryRunCandidateCount: 1,
      manualRequiredCandidateCount: 0,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
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
    path.join(outputDir, '01-browser-runtime-action-intent-event-subscriber-proof.md'),
    [
      '# Browser Runtime Action Intent Event Subscriber Proof',
      '',
      'This proof uses the reusable Rust eventing request/response path for a named browser action-intent status event and subscriber.',
      '',
      'The subscriber returns one pending dry-run candidate for policy-decision events with policy preview and action-intent refs, and zero candidates for manual-required rows.',
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
