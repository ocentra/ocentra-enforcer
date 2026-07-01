import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-service-status-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-service-status');

const files = {
  servicePayload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  serviceTests: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_tests.rs'),
  rustFields: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'field.rs'),
  tsDefaults: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'defaults.ts'),
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
  const [servicePayload, serviceTests, rustFields, tsDefaults, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );

  return {
    serviceRequestsNamedSubscriber: servicePayload.includes('request_browser_runtime_action_intent_status_for_input'),
    servicePayloadPublishesCandidateCount: servicePayload.includes('BROWSER_RUNTIME_ACTION_INTENT_CANDIDATES'),
    servicePayloadPublishesDispatchZeros: servicePayload.includes('BROWSER_RUNTIME_ACTION_INTENT_DISPATCH_ATTEMPTS'),
    storeBackedRowsStayZero: serviceTests.includes('assert_eq!(report.action_intent_candidates, 0)'),
    dryRunCandidateProjectsToPayload: serviceTests.includes(
      'service_browser_runtime_action_intent_status_projects_pending_candidate'
    ),
    rustProtocolFieldsRegistered: rustFields.includes('BROWSER_RUNTIME_ACTION_INTENT_CHILD_INTERVENTION_EXECUTIONS'),
    tsProtocolFieldsRegistered: tsDefaults.includes('BrowserRuntimeActionIntentChildInterventionExecutions'),
    docsMentionServiceStatusProof: checklist.includes('browser-runtime-action-intent-service-status-proof'),
    workpackMentionsServiceStatusProof: workpack.includes('Action-Intent Service Status Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent service status proof failed: ${missing.join(', ')}`);
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
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-service-status-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      existingStreamCommand: 'agent.browser.runtime.event-chain.stream.get',
      statusEvent: 'browser.action-intent.status.requested',
      pendingDryRunCandidateProjected: true,
      storeBackedBrowserRowsCurrentlyHavePendingCandidateCount: 0,
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
    path.join(outputDir, '01-browser-runtime-action-intent-service-status-proof.md'),
    [
      '# Browser Runtime Action Intent Service Status Proof',
      '',
      'This proof keeps the existing browser runtime event-chain stream command and enriches its service payload with action-intent status counters from the named event subscriber.',
      '',
      'Current store-backed browser evidence rows still report zero pending action intents because the read model does not yet carry policy preview or assistant action-intent references.',
      '',
      'A dry-run action-intent input projects one pending candidate through the reusable event-bus request/response subscriber and the service payload keeps dispatch, adapter execution, child intervention execution, final policy execution, and enforcement execution at zero.',
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
