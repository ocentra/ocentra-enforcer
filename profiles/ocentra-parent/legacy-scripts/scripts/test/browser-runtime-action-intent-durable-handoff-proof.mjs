import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-durable-handoff-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-durable-handoff');

const files = {
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  durableModule: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime', 'action_handoff_durable.rs'),
  durableTypes: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime',
    'action_handoff_durable_types.rs'
  ),
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
  feature: path.join(root, 'docs', 'features', 'browser-web-control.md'),
};

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [runtime, durableModule, durableTypes, tests, constants, checklist, workpack, feature] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );

  return {
    runtimeExportsDurableHandoff:
      runtime.includes('mod action_handoff_durable;') &&
      runtime.includes('prove_browser_runtime_action_intent_durable_handoff'),
    durableModuleUsesNamedHandoffSubscriber:
      durableModule.includes('request_browser_runtime_action_intent_handoff_for_input') &&
      durableModule.includes('duplicate_request_event_rejected') &&
      durableModule.includes('has_unsupported_claims'),
    durableTypesKeepPreparedState:
      durableTypes.includes('BrowserRuntimeActionIntentDurableHandoffReadModelState') &&
      durableTypes.includes('PreparedNotDispatched') &&
      durableTypes.includes('durable_result_ref') &&
      durableTypes.includes('read_model_ref'),
    protocolConstantsNameDurableRefs:
      constants.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_DURABLE_RESULT_REF') &&
      constants.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_DURABLE_STORE_REF') &&
      constants.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_READ_MODEL_REF') &&
      constants.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_SUPPORT_STATUS_REF'),
    testsProveRefsAndNoExecution:
      tests.includes('browser_runtime_action_intent_durable_handoff_preserves_refs_without_execution') &&
      tests.includes('final_policy_execution_count'),
    docsMentionProof:
      checklist.includes('browser-runtime-action-intent-durable-handoff-proof') &&
      workpack.includes('Action-Intent Durable Handoff Result Addendum') &&
      feature.includes('durable handoff result/read-model proof'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent durable handoff proof failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_action_intent_durable_handoff', '--quiet'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-durable-handoff-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      handoffEvent: 'browser.action-intent.handoff.requested',
      durableResultRefProjected: true,
      durableStoreRefProjected: true,
      readModelRefProjected: true,
      supportStatusRefProjected: true,
      duplicateRequestEventRejected: true,
      rowMatchesHandoffResponse: true,
      rowMatchesRequestEvent: true,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      browserMutationCount: 0,
      childInterventionExecutionCount: 0,
      finalPolicyExecutionCount: 0,
      enforcementExecutionCount: 0,
      externalTransportImplemented: false,
      finalPolicyExecutionClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-durable-handoff-proof.md'),
    [
      '# Browser Runtime Action Intent Durable Handoff Proof',
      '',
      'This proof carries the named browser action-intent handoff subscriber result into a durable result/read-model row without creating dispatch, browser mutation, child-intervention execution, final policy execution, or enforcement claims.',
      '',
      'The row preserves the source handoff event, policy preview id, parent action-intent id, local outbox ref, local handoff ref, durable result ref, durable store ref, read-model ref, and support-status ref. Duplicate request event ids are rejected before projection.',
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
