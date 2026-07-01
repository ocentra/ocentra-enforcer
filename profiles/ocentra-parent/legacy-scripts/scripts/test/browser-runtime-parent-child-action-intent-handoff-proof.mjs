import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-parent-child-action-intent-handoff-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-parent-child-action-intent-handoff');

const files = {
  childEvents: path.join(root, 'crates', 'agent-protocol', 'src', 'child_agent_events.rs'),
  childEventTests: path.join(root, 'crates', 'agent-protocol', 'src', 'child_agent_event_tests.rs'),
  browserConstants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
  runtime: path.join(root, 'crates', 'agent-core', 'src', 'parent_child_event_runtime.rs'),
  runtimeTests: path.join(root, 'crates', 'agent-core', 'src', 'parent_child_event_runtime_tests.rs'),
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
  const [childEvents, childEventTests, browserConstants, runtime, runtimeTests, checklist, workpack] =
    await Promise.all(Object.values(files).map((file) => readFile(file, 'utf8')));

  return {
    childCommandKindIsNamed:
      childEvents.includes('BrowserActionIntentHandoff') && childEventTests.includes('browser-action-intent-handoff'),
    parentIntentUsesBrowserActionIntentRef:
      runtime.includes('browser_action_intent_handoff_fixture') &&
      runtime.includes('TEST_BROWSER_RUNTIME_ACTION_INTENT_ID') &&
      runtime.includes('ChildCommandKind::BrowserActionIntentHandoff'),
    runtimeTestProvesParentChildHandoff:
      runtimeTests.includes('browser_action_intent_handoff_uses_parent_child_event_sequence_without_execution') &&
      runtimeTests.includes('ParentChildRuntimeInput::browser_action_intent_handoff_fixture') &&
      runtimeTests.includes('ChildCommandKind::BrowserActionIntentHandoff'),
    docsMentionProof:
      checklist.includes('browser-runtime-parent-child-action-intent-handoff-proof') &&
      workpack.includes('Parent-Child Action-Intent Handoff Addendum'),
    parentIntentRef: extractRustStringConstant(browserConstants, 'TEST_BROWSER_RUNTIME_ACTION_INTENT_ID'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([name, ok]) => name !== 'parentIntentRef' && !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser parent-child action-intent handoff proof failed: ${missing.join(', ')}`);
  }
  if (!checks.parentIntentRef) {
    throw new Error('browser parent-child action-intent handoff proof failed: missing parentIntentRef');
  }
}

function extractRustStringConstant(source, name) {
  const match = source.match(new RegExp(`pub const ${name}: &str = "([^"]+)";`));
  return match?.[1] ?? null;
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cargo',
      args: [
        'test',
        '-p',
        'ocentra-parent-agent-protocol',
        'child_agent_contracts_serialize_browser_action_intent_handoff_kind',
        '--quiet',
      ],
    },
    {
      command: 'cargo',
      args: [
        'test',
        '-p',
        'ocentra-parent-agent-core',
        'browser_action_intent_handoff_uses_parent_child_event_sequence_without_execution',
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
    proofName: 'browser-runtime-parent-child-action-intent-handoff-proof',
    checkedAt: new Date().toISOString(),
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], {
      cwd: root,
      encoding: 'utf8',
    }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      childCommandKind: 'browser-action-intent-handoff',
      parentIntentRef: checks.parentIntentRef,
      parentActionReceived: true,
      parentCommandValidated: true,
      parentChildForwardRequested: true,
      parentChildForwarded: true,
      childCommandReceived: true,
      childCommandAccepted: true,
      parentReadModelProjected: true,
      visibleToPortal: true,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      browserMutationCount: 0,
      childInterventionExecutionCount: 0,
      finalPolicyExecutionCount: 0,
      enforcementExecutionCount: 0,
    },
    notClaimed: [
      'external broker or relay delivery',
      'adapter dispatch',
      'browser mutation',
      'child intervention execution',
      'final policy execution',
      'enforcement execution',
      'unmanaged exact URL support',
    ],
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-parent-child-action-intent-handoff-proof.md'),
    [
      '# Browser Runtime Parent-Child Action-Intent Handoff Proof',
      '',
      'This proof carries the browser action-intent handoff into the existing parent/controller to child-agent event sequence by adding a named `browser-action-intent-handoff` child command kind.',
      '',
      'The proof validates parent action receipt, command validation, parent-child transport handoff, child command receive/acceptance, and parent read-model projection while keeping dispatch, adapter execution, browser mutation, child intervention, final policy execution, and enforcement counts at zero.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
      'Not claimed:',
      ...proof.notClaimed.map((claim) => `- ${claim}`),
      '',
    ].join('\n')
  );

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
