import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-child-status-public-stream-proof');
const outputDir = path.join(
  root,
  'output',
  'browser-plan-proof',
  'browser-runtime-action-intent-child-status-public-stream'
);

const childStatusFieldNames = [
  'browserRuntimeActionIntentChildAcceptedRows',
  'browserRuntimeActionIntentChildCommandRefs',
  'browserRuntimeActionIntentChildAcceptedEventRefs',
  'browserRuntimeActionIntentParentReadModelRefs',
];

const childStatusConstantNames = [
  'BROWSER_RUNTIME_ACTION_INTENT_CHILD_ACCEPTED_ROWS',
  'BROWSER_RUNTIME_ACTION_INTENT_CHILD_COMMAND_REFS',
  'BROWSER_RUNTIME_ACTION_INTENT_CHILD_ACCEPTED_EVENT_REFS',
  'BROWSER_RUNTIME_ACTION_INTENT_PARENT_READ_MODEL_REFS',
];

const childStatusPropertyNames = [
  'actionIntentChildAcceptedRows',
  'actionIntentChildCommandRefs',
  'actionIntentChildAcceptedEventRefs',
  'actionIntentParentReadModelRefs',
];

const files = {
  rustFieldConstants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'field.rs'),
  servicePayload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  serviceTests: path.join(root, 'crates', 'agent-service', 'tests', 'unit', 'browser_runtime_stream_tests.rs'),
  serviceTestAssertions: path.join(
    root,
    'crates',
    'agent-service',
    'tests',
    'unit',
    'browser_runtime_stream_tests',
    'browser_runtime_stream_test_assertions.rs'
  ),
  protocolDefaults: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'defaults.ts'),
  protocolParser: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'browser-runtime-events.ts'),
  protocolTests: path.join(root, 'packages', 'agent-protocol-domain', 'tests', 'browser-runtime-events.test.ts'),
  portalTests: path.join(root, 'apps', 'portal', 'tests', 'live-activity-state.test.ts'),
  featureDoc: path.join(root, 'docs', 'features', 'browser-web-control.md'),
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

const commands = [
  {
    command: 'cargo',
    args: [
      'test',
      '-p',
      'ocentra-parent-agent-service',
      'service_browser_runtime_action_intent_status_projects_pending_candidate',
      '--quiet',
    ],
  },
  {
    command: 'cargo',
    args: [
      'test',
      '-p',
      'ocentra-parent-agent-service',
      'websocket_browser_runtime_stream_command_reports_store_backed_chain',
      '--quiet',
    ],
  },
  {
    command: 'cmd',
    args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/agent-protocol-domain'],
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

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [
    rustFieldConstants,
    servicePayload,
    serviceTests,
    serviceTestAssertions,
    protocolDefaults,
    protocolParser,
    protocolTests,
    portalTests,
    featureDoc,
    checklist,
    workpack,
  ] = await Promise.all(Object.values(files).map((file) => readFile(file, 'utf8')));
  const serviceTestSources = `${serviceTests}\n${serviceTestAssertions}`;

  return {
    rustFieldConstantsAdded: childStatusFieldNames.every((fieldName) => rustFieldConstants.includes(fieldName)),
    servicePayloadPublishesInputDrivenChildStatusFields:
      childStatusConstantNames.every((constantName) => servicePayload.includes(constantName)) &&
      servicePayload.includes('action_intent_child_status_from_handoff') &&
      servicePayload.includes('publish_parent_child_runtime_for_validated_intent') &&
      servicePayload.includes('record_action_intent_child_status') &&
      servicePayload.includes('action_intent_child_accepted_rows') &&
      servicePayload.includes('action_intent_child_command_refs') &&
      servicePayload.includes('action_intent_child_accepted_event_refs') &&
      servicePayload.includes('action_intent_parent_read_model_refs'),
    servicePayloadDoesNotCallFixtureProof:
      !servicePayload.includes('prove_browser_runtime_action_intent_child_status') &&
      servicePayload.includes('BrowserRuntimeActionIntentChildStatusResponse'),
    serviceTestsProveInputDrivenPublicFieldsAndZeroFallback:
      serviceTestSources.includes('assert_child_status_payload_empty') &&
      serviceTestSources.includes('action_intent_child_accepted_rows, 1') &&
      childStatusConstantNames.every((constantName) => serviceTestSources.includes(constantName)),
    protocolDefaultsAdded: childStatusFieldNames.every((fieldName) => protocolDefaults.includes(fieldName)),
    protocolParserValidatesHonestCounts:
      childStatusPropertyNames.every((propertyName) => protocolParser.includes(propertyName)) &&
      protocolParser.includes('browserRuntimeActionIntentChildStatusIsHonest') &&
      protocolParser.includes('Expected browser runtime child status refs to match observed child accepted rows'),
    protocolTestsRejectOverclaims:
      protocolTests.includes('specifyActionIntentChildStatusOverclaimRejections') &&
      childStatusPropertyNames.every((propertyName) => protocolTests.includes(propertyName)),
    portalTestsRejectOverclaims:
      portalTests.includes('rejects browser runtime child status refs when accepted row counts drift') &&
      childStatusPropertyNames.every((propertyName) => portalTests.includes(propertyName)),
    docsRecordPublicStreamBoundary:
      featureDoc.includes('public browser action-intent child-status') &&
      featureDoc.includes('service-backed parent-child event path') &&
      checklist.includes('browser-runtime-action-intent-child-status-public-stream-proof') &&
      workpack.includes('Action-Intent Child Status Public Stream Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent child-status public stream proof failed: ${missing.join(', ')}`);
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
    proofName: 'browser-runtime-action-intent-child-status-public-stream-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], {
      cwd: root,
      encoding: 'utf8',
    }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      publicChildStatusFieldsAdded: true,
      serviceStreamChildAcceptedRows: 1,
      serviceStreamChildCommandRefs: 'input-driven parent-child command refs',
      serviceStreamChildAcceptedEventRefs: 'input-driven child accepted event refs',
      serviceStreamParentReadModelRefs: 'input-driven parent read-model refs',
      mismatchedChildStatusCountsRejected: true,
      fixtureChildStatusProofNotCalledByService: true,
      externalTransportImplemented: false,
      adapterDispatchClaimed: false,
      browserMutationClaimed: false,
      childInterventionExecutionClaimed: false,
      finalPolicyExecutionClaimed: false,
      enforcementClaimed: false,
    },
    requiredFollowUp: {
      condition:
        'Report child accepted rows only from the input-driven parent-child handoff path; execution/enforcement still requires separate adapter and policy proof.',
      forbiddenShortcut:
        'Do not call the fixture-backed child-status proof from agent-service or portal runtime state.',
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-child-status-public-stream-proof.md'),
    [
      '# Browser Runtime Action Intent Child Status Public Stream Proof',
      '',
      'This proof carries the browser action-intent child-status boundary into public service stream fields through the input-driven parent-child handoff path.',
      '',
      'The service stream reports child accepted rows and child command, accepted-event, and parent read-model refs only for a dry-run action handoff candidate. Normal/manual rows remain zero/empty. The shared protocol parser and portal state tests reject mismatched nonzero/empty combinations.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
      'No-claim boundary:',
      '- No fixture-backed child-status proof call in service or portal runtime state.',
      '- No external child transport implementation.',
      '- No adapter dispatch.',
      '- No browser mutation.',
      '- No child intervention execution.',
      '- No final policy execution.',
      '- No enforcement.',
      '',
    ].join('\n')
  );

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
