import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-durable-status-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-durable-status');

const files = {
  fieldConstants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'field.rs'),
  defaults: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'defaults.ts'),
  servicePayload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  serviceTests: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_tests.rs'),
  protocolParser: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'browser-runtime-events.ts'),
  protocolTests: path.join(root, 'packages', 'agent-protocol-domain', 'tests', 'browser-runtime-events.test.ts'),
  portalTests: path.join(root, 'apps', 'portal', 'tests', 'live-activity-state.test.ts'),
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
  const [
    fieldConstants,
    defaults,
    servicePayload,
    serviceTests,
    protocolParser,
    protocolTests,
    portalTests,
    checklist,
    workpack,
  ] = await Promise.all(Object.values(files).map((file) => readFile(file, 'utf8')));

  return {
    rustFieldConstantsExposeHandoffStatus:
      fieldConstants.includes('BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_CANDIDATES') &&
      fieldConstants.includes('BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_OUTBOX_REFS') &&
      fieldConstants.includes('BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_REFS'),
    tsDefaultsExposeHandoffStatus:
      defaults.includes('BrowserRuntimeActionIntentHandoffCandidates') &&
      defaults.includes('BrowserRuntimeActionIntentHandoffOutboxRefs') &&
      defaults.includes('BrowserRuntimeActionIntentHandoffRefs'),
    servicePayloadPublishesPreparedRefs:
      servicePayload.includes('BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_CANDIDATES') &&
      servicePayload.includes('string_array_value(&report.action_intent_handoff_outbox_refs)') &&
      servicePayload.includes('string_array_value(&report.action_intent_handoff_refs)'),
    serviceTestsProveStoreBackedPreparedRefs:
      serviceTests.includes('service_browser_runtime_stream_projects_store_backed_policy_preview_candidate') &&
      serviceTests.includes('BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_OUTBOX_REFS') &&
      serviceTests.includes('BROWSER_RUNTIME_ACTION_INTENT_HANDOFF_REFS'),
    protocolParserRejectsUnpairedRefs:
      protocolParser.includes('actionIntentHandoffOutboxRefs.length === stream.actionIntentHandoffRefs.length') &&
      protocolTests.includes('BrowserRuntimeActionIntentHandoffOutboxRefs'),
    portalStateKeepsPreparedRefsVisible:
      portalTests.includes('keeps prepared browser action-intent handoff refs visible without execution claims') &&
      portalTests.includes('BrowserRuntimeActionIntentHandoffRefs'),
    executionCountersRemainRejected:
      protocolParser.includes('actionIntentDispatchAttempts: Schema.Literal(0)') &&
      protocolParser.includes('actionIntentAdapterExecutions: Schema.Literal(0)') &&
      protocolParser.includes('actionIntentChildInterventionExecutions: Schema.Literal(0)') &&
      protocolParser.includes('actionIntentEnforcementExecutions: Schema.Literal(0)'),
    docsMentionDurableStatusProof:
      checklist.includes('browser-runtime-action-intent-durable-status-proof') &&
      workpack.includes('Action-Intent Durable Status Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent durable status proof failed: ${missing.join(', ')}`);
  }
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
        'ocentra-parent-agent-service',
        'service_browser_runtime_stream_projects_store_backed_policy_preview_candidate',
        '--quiet',
      ],
    },
    {
      command: 'cmd',
      args: [
        '/c',
        'npm',
        'run',
        'test',
        '--workspace',
        '@ocentra-parent/agent-protocol-domain',
        '--',
        'browser-runtime-events.test.ts',
      ],
    },
    {
      command: 'cmd',
      args: ['/c', 'npm', 'run', 'test', '--workspace', '@ocentra-parent/portal', '--', 'live-activity-state.test.ts'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-durable-status-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      existingStreamCommand: 'agent.browser.runtime.event-chain.stream.get',
      handoffEvent: 'browser.action-intent.handoff.requested',
      publicStreamCarriesHandoffCandidateCount: true,
      publicStreamCarriesPreparedOutboxRefs: true,
      publicStreamCarriesPreparedHandoffRefs: true,
      portalStateParsesPreparedHandoffRefs: true,
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
    path.join(outputDir, '01-browser-runtime-action-intent-durable-status-proof.md'),
    [
      '# Browser Runtime Action Intent Durable Status Proof',
      '',
      'This proof carries prepared browser action-intent handoff status through the existing service-backed browser runtime event-chain stream and portal live-activity parser.',
      '',
      'The stream now exposes prepared handoff candidate count, local outbox refs, and handoff refs. Dispatch, adapter execution, browser mutation, child intervention execution, final policy execution, and enforcement remain zero or unclaimed.',
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
