import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-service-stream-eventing-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-service-stream-eventing');

const files = {
  cargo: path.join(root, 'crates', 'agent-service', 'Cargo.toml'),
  browserConstants: path.join(root, 'crates', 'agent-protocol', 'src', 'constants', 'browser.rs'),
  main: path.join(root, 'crates', 'agent-service', 'src', 'main.rs'),
  api: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_api.rs'),
  request: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_request.rs'),
  payload: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_payload.rs'),
  events: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_events.rs'),
  tests: path.join(root, 'crates', 'agent-service', 'src', 'browser_runtime_stream_tests.rs'),
  eventingTests: path.join(
    root,
    'crates',
    'agent-service',
    'src',
    'browser_runtime_stream_tests',
    'browser_runtime_service_stream_eventing_tests.rs'
  ),
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
  const [cargo, browserConstants, main, api, request, payload, events, tests, eventingTests, checklist, workpack] =
    await Promise.all(Object.values(files).map((file) => readFile(file, 'utf8')));

  return {
    serviceConsumesReusableEventingCrate: cargo.includes('ocentra-eventing = { path = "../ocentra-eventing" }'),
    browserEventNameIsProtocolOwned:
      browserConstants.includes('EVENT_BROWSER_RUNTIME_STREAM_REPORT_REQUESTED') &&
      browserConstants.includes('browser.runtime.stream.report.requested') &&
      browserConstants.includes('SUBSCRIBER_BROWSER_RUNTIME_STREAM_REPORT') &&
      browserConstants.includes('TARGET_BROWSER_RUNTIME_STREAM_REPORT'),
    requestModuleRegistered: main.includes('mod browser_runtime_stream_request;'),
    websocketRouteUsesEventedRequest:
      api.includes('request_browser_runtime_service_stream_report') &&
      !api.includes('stream_browser_runtime_event_chain_for_read_model_with_policy_preview('),
    requestPublishesNamedLocalRequest:
      request.includes('BrowserRuntimeServiceStreamReportRequest') &&
      request.includes('publish_request') &&
      request.includes('EVENT_BROWSER_RUNTIME_STREAM_REPORT_REQUESTED') &&
      request.includes('SUBSCRIBER_BROWSER_RUNTIME_STREAM_REPORT') &&
      request.includes('stream_browser_runtime_event_chain_for_read_model_with_policy_preview'),
    responseBoundaryCanRoundTrip:
      payload.includes('Serialize, Deserialize') &&
      events.includes("impl<'de> Deserialize<'de> for BrowserRuntimeServiceStreamEntry") &&
      events.includes('constants::field::EVENT_TYPE') &&
      events.includes('constants::field::EVENT_REF'),
    focusedTestCoversEquivalentProjection:
      tests.includes('browser_runtime_service_stream_eventing_tests') &&
      eventingTests.includes('service_browser_runtime_stream_uses_named_event_request_boundary') &&
      eventingTests.includes('assert_eq!(evented_report, direct_report)') &&
      eventingTests.includes('action_intent_dispatch_attempts, 0') &&
      eventingTests.includes('action_intent_enforcement_executions, 0'),
    docsMentionProof:
      checklist.includes('browser-runtime-service-stream-eventing-proof') &&
      workpack.includes('Service Stream Eventing Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser runtime service stream eventing proof failed: ${missing.join(', ')}`);
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
        'service_browser_runtime_stream_uses_named_event_request_boundary',
        '--quiet',
      ],
    },
    {
      command: 'cargo',
      args: ['test', '-p', 'ocentra-parent-agent-service', 'service_browser_runtime_stream', '--quiet'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-service-stream-eventing-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      existingPortalCommand: 'agent.browser.runtime.event-chain.stream.get',
      internalServiceEvent: 'browser.runtime.stream.report.requested',
      requestResponseBoundary: true,
      directProjectionAndEventedProjectionMatch: true,
      portalWireContractChanged: false,
      adapterDispatchClaimed: false,
      browserMutationClaimed: false,
      childInterventionExecutionClaimed: false,
      finalPolicyExecutionClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-service-stream-eventing-proof.md'),
    [
      '# Browser Runtime Service Stream Eventing Proof',
      '',
      'This proof moves the service-side browser runtime stream projection behind a named local eventing request/subscriber boundary.',
      '',
      'The existing portal command remains `agent.browser.runtime.event-chain.stream.get`, but the Rust service now publishes `browser.runtime.stream.report.requested` internally and completes the response through the reusable `ocentra-eventing` request/response path.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
      'No-claim boundary:',
      '- No portal wire contract change.',
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
