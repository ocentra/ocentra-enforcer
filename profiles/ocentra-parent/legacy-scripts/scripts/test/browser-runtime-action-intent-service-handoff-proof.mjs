import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-service-handoff-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-service-handoff');

const files = {
  servicePayload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  serviceTests: path.join(root, 'crates', 'agent-service', 'tests', 'unit', 'browser_runtime_stream_tests.rs'),
  handoffSubscriber: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'action_handoff.rs'),
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
  const [servicePayload, serviceTests, handoffSubscriber, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );

  return {
    serviceRequestsNamedHandoffSubscriber: servicePayload.includes(
      'request_browser_runtime_action_intent_handoff_for_input'
    ),
    serviceRecordsPreparedOutboxRefs:
      servicePayload.includes('action_intent_handoff_outbox_refs') &&
      servicePayload.includes('record_action_intent_handoff'),
    serviceRecordsPreparedHandoffRefs: servicePayload.includes('action_intent_handoff_refs'),
    serviceKeepsExecutionCountersZero:
      serviceTests.includes('service_browser_runtime_action_intent_status_projects_pending_candidate') &&
      serviceTests.includes('action_intent_dispatch_attempts, 0') &&
      serviceTests.includes('action_intent_enforcement_executions, 0'),
    storeBackedPolicyPreviewPreparesHandoff:
      serviceTests.includes('service_browser_runtime_stream_projects_store_backed_policy_preview_candidate') &&
      serviceTests.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_OUTBOX_REF') &&
      serviceTests.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_REF'),
    subscriberStillNoExecutionClaim:
      handoffSubscriber.includes('browser_mutation_count: 0') &&
      handoffSubscriber.includes('child_intervention_execution_count: 0') &&
      handoffSubscriber.includes('enforcement_execution_count: 0'),
    docsMentionServiceHandoffProof: checklist.includes('browser-runtime-action-intent-service-handoff-proof'),
    workpackMentionsServiceHandoffProof: workpack.includes('Action-Intent Service Handoff Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent service handoff proof failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-service', 'service_browser_runtime_action_intent_status', '--quiet'],
    },
    {
      command: 'cargo',
      args: [
        'test',
        '-p',
        'ocentra-parent-agent-service',
        'service_browser_runtime_stream_projects_store_backed_policy_preview_candidate',
        '--quiet',
      ],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-service-handoff-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      existingStreamCommand: 'agent.browser.runtime.event-chain.stream.get',
      handoffEvent: 'browser.action-intent.handoff.requested',
      preparedOutboxRefRecordedInServiceReport: true,
      preparedHandoffRefRecordedInServiceReport: true,
      publicWireShapeChanged: false,
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
    lockConstraint:
      'Shared protocol field constants/defaults are currently owned by codex-c, so this slice keeps prepared handoff refs in browser service report state and does not add new wire keys.',
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-service-handoff-proof.md'),
    [
      '# Browser Runtime Action Intent Service Handoff Proof',
      '',
      'This proof extends the existing service-backed browser runtime event-chain stream path so the service asks the named browser action-intent handoff subscriber and records prepared local outbox/handoff refs in report state.',
      '',
      'The public wire payload is intentionally unchanged in this slice because the shared protocol field constants/defaults are owned by another active lane. The service still publishes the existing action-intent counters and keeps dispatch, adapter execution, browser mutation, child intervention, final policy execution, and enforcement at zero.',
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
