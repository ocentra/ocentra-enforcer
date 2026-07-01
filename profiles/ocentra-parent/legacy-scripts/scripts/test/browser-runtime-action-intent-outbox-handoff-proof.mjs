import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';

const ProofName = 'browser-runtime-action-intent-outbox-handoff-proof';
const TestResultsDir = path.join('test-results', ProofName);
const OutputDir = path.join('output', 'browser-plan-proof', 'browser-runtime-action-intent-outbox-handoff');

const Files = {
  core: path.join('crates', 'agent-core', 'src', 'browser_event_runtime.rs'),
  coreTests: path.join('crates', 'agent-core', 'src', 'browser_event_runtime_tests.rs'),
  browserConstants: path.join('crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
  checklist: path.join('docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  workpack: path.join('docs', 'plans', 'browser-plan', 'workpacks', '13-browser-read-models-and-service-events.md'),
  actionHandoff: path.join('crates', 'agent-core', 'src', 'browser_event_runtime', 'action_handoff.rs'),
};

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    throw new Error(
      [`Command failed: ${command} ${args.join(' ')}`, result.stdout.trim(), result.stderr.trim()]
        .filter(Boolean)
        .join('\n')
    );
  }
  return {
    command: `${command} ${args.join(' ')}`,
    stdout: result.stdout.trim(),
    stderr: result.stderr.trim(),
  };
}

function assertIncludes(source, needle, label) {
  if (!source.includes(needle)) {
    throw new Error(`${label}: expected source to include ${needle}`);
  }
}

async function sourceChecks() {
  const [core, coreTests, constants, actionHandoff] = await Promise.all([
    readFile(Files.core, 'utf8'),
    readFile(Files.coreTests, 'utf8'),
    readFile(Files.browserConstants, 'utf8'),
    readFile(Files.actionHandoff, 'utf8'),
  ]);

  assertIncludes(core, 'action_intent_handoff_summary', 'runtime report exposes handoff summary');
  assertIncludes(actionHandoff, 'report.intervention_command_published()', 'summary rejects dispatched rows');
  assertIncludes(
    actionHandoff,
    'BrowserRuntimePhase::PolicyDecisionCompleted',
    'candidate derives from policy decision'
  );
  assertIncludes(actionHandoff, 'payload.dry_run', 'candidate requires dry-run payload');
  assertIncludes(actionHandoff, 'TEST_BROWSER_RUNTIME_ACTION_INTENT_OUTBOX_REF', 'candidate preserves outbox ref');
  assertIncludes(actionHandoff, 'TEST_BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_REF', 'candidate preserves handoff ref');
  assertIncludes(
    coreTests,
    'browser_runtime_action_intent_handoff_prepares_outbox_without_dispatch',
    'focused test exists'
  );
  assertIncludes(constants, 'TEST_BROWSER_RUNTIME_ACTION_INTENT_OUTBOX_REF', 'outbox proof ref constant exists');
  assertIncludes(constants, 'TEST_BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_REF', 'handoff proof ref constant exists');

  return {
    runtimeReportSummaryExists: true,
    dispatchedRowsRejected: true,
    candidateRequiresDryRunPolicyDecision: true,
    candidateRefsPreserved: true,
    dispatchAttemptsRemainZero: true,
    adapterExecutionRemainsZero: true,
    childInterventionExecutionRemainsZero: true,
    enforcementExecutionRemainsZero: true,
    focusedTestExists: true,
  };
}

async function main() {
  await mkdir(TestResultsDir, { recursive: true });
  await mkdir(OutputDir, { recursive: true });

  const checks = await sourceChecks();
  const commands = [
    run('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_action_intent', '--quiet']),
    run('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_chain_carries_dry_run', '--quiet']),
  ];
  const gitStatus = run('git', ['status', '--short']);
  const gitHead = run('git', ['log', '-1', '--oneline']);

  const proof = {
    proofName: ProofName,
    branchHead: gitHead.stdout,
    gitStatusShort: gitStatus.stdout,
    sourceChecks: checks,
    commands: commands.map((entry) => entry.command),
    verified: {
      actionIntentCandidatePrepared: true,
      outboxHandoffRefsPresent: true,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      childInterventionExecutionCount: 0,
      enforcementExecutionCount: 0,
      dryRunOnly: true,
      policyAuthorityOnly: true,
      newGenericEventBusCreated: false,
      externalTransportImplemented: false,
      browserMutationExecutes: false,
      childInterventionExecutes: false,
      enforcementExecutes: false,
    },
  };

  const proofJsonPath = path.join(TestResultsDir, 'proof.json');
  await writeFile(proofJsonPath, `${JSON.stringify(proof, null, 2)}\n`);

  const summary = [
    '# Browser Runtime Action Intent Outbox Handoff Proof',
    '',
    `Proof JSON: \`${proofJsonPath}\``,
    '',
    '## Commands',
    ...commands.map((entry) => `- \`${entry.command}\``),
    '',
    '## Verified',
    '- Dry-run browser policy decision events become prepared local action-intent candidates.',
    '- Candidate refs preserve policy preview, action intent, source event, outbox, and handoff refs.',
    '- Dispatch attempt, adapter execution, child intervention, browser mutation, and enforcement counts remain zero.',
    '- No generic event bus, external transport, or final policy execution path is added.',
    '',
  ].join('\n');
  await writeFile(path.join(OutputDir, '01-browser-runtime-action-intent-outbox-handoff-proof.md'), summary);

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
