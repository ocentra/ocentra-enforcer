import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-child-status-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-child-status');

const files = {
  runtimeExport: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  childStatus: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime',
    'action_handoff_child_status.rs'
  ),
  childStatusTypes: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime',
    'action_handoff_child_status_types.rs'
  ),
  runtimeTests: path.join(
    root,
    'crates',
    'agent-core',
    'src',
    'browser_event_runtime_tests',
    'browser_event_runtime_child_status_tests.rs'
  ),
  runtimeTestModule: path.join(root, 'crates', 'agent-core', 'src', 'browser_event_runtime_tests.rs'),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
};

const commands = [
  {
    command: 'cargo',
    args: [
      'test',
      '-p',
      'ocentra-parent-agent-core',
      'browser_runtime_action_intent_child_status_links_durable_handoff_to_child_acceptance',
      '--quiet',
    ],
  },
];

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [runtimeExport, childStatus, childStatusTypes, runtimeTests, runtimeTestModule, workpack, checklist] =
    await Promise.all(Object.values(files).map((file) => readFile(file, 'utf8')));

  return {
    proofModuleExported:
      runtimeExport.includes('mod action_handoff_child_status;') &&
      runtimeExport.includes('prove_browser_runtime_action_intent_child_status'),
    durableHandoffComposed:
      childStatus.includes('prove_browser_runtime_action_intent_durable_handoff') &&
      childStatus.includes('BrowserRuntimeActionIntentChildStatusReport'),
    parentChildSequenceComposed:
      childStatus.includes('publish_parent_child_runtime_for_validated_intent') &&
      childStatus.includes('ParentChildRuntimeInput::browser_action_intent_handoff_fixture()') &&
      childStatus.includes('ChildCommandKind::BrowserActionIntentHandoff'),
    statusRowCarriesChildAcceptanceRefs:
      childStatusTypes.includes('child_command_received_event_ref') &&
      childStatusTypes.includes('child_command_accepted_event_ref') &&
      childStatusTypes.includes('parent_read_model_projected_event_ref'),
    serviceStreamStatusStaysNoObservationOnly:
      childStatus.includes('public_stream_field_registry_ready: true') &&
      !runtimeExport.includes('BROWSER_RUNTIME_ACTION_INTENT_CHILD_ACCEPTED_REFS'),
    noExecutionClaims:
      childStatus.includes('dispatch_attempt_count > 0') &&
      childStatus.includes('adapter_execution_count > 0') &&
      childStatus.includes('browser_mutation_count > 0') &&
      childStatus.includes('child_intervention_execution_count > 0') &&
      childStatus.includes('final_policy_execution_count > 0') &&
      childStatus.includes('enforcement_execution_count > 0'),
    rustTestProvesRefsAndNoClaims:
      runtimeTestModule.includes('mod browser_event_runtime_child_status_tests;') &&
      runtimeTests.includes('browser_runtime_action_intent_child_status_links_durable_handoff_to_child_acceptance') &&
      runtimeTests.includes('child_command_accepted_event_ref') &&
      runtimeTests.includes('public_stream_field_registry_ready'),
    docsMentionChildStatusProof:
      workpack.includes('Action-Intent Child Status Addendum') &&
      workpack.includes('Action-Intent Child Status Public Stream Addendum') &&
      checklist.includes('browser-runtime-action-intent-child-status-proof'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent child status proof failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-child-status-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      durableHandoffResultComposed: true,
      parentChildCommandKind: 'browser-action-intent-handoff',
      childReceivedStatusVisible: true,
      childAcceptedStatusVisible: true,
      parentReadModelProjectedStatusVisible: true,
      publicStreamFieldRegistryReady: true,
      serviceStreamChildStatusBoundary: 'input-driven-parent-child-status',
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      browserMutationCount: 0,
      childInterventionExecutionCount: 0,
      finalPolicyExecutionCount: 0,
      enforcementExecutionCount: 0,
    },
    remainingGap: {
      reason:
        'The service stream exposes input-driven parent-child child-status fields, but it still does not call the fixture-backed proof from runtime state.',
      requiredFollowUp:
        'Add adapter execution, browser mutation, final policy, and enforcement proof before claiming the accepted child command executed a browser intervention.',
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-child-status-proof.md'),
    [
      '# Browser Runtime Action Intent Child Status Proof',
      '',
      'This proof composes the durable browser action-intent handoff record with the existing parent/controller to child-agent event sequence.',
      '',
      'It verifies the browser action-intent id reaches a named `browser-action-intent-handoff` child command, records child received/accepted refs, and projects a parent-visible read-model row while preserving zero execution counters.',
      '',
      'The public service stream now exposes child-status fields through an input-driven parent-child handoff status request. The fixture-backed proof remains separate and must not be called from service runtime state.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
      'No-claim boundary:',
      '- No adapter dispatch.',
      '- No browser mutation.',
      '- No child intervention execution.',
      '- No final policy execution.',
      '- No enforcement.',
      '- No unmanaged exact URL support.',
      '',
    ].join('\n')
  );

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
