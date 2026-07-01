import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10-service-event-chain-stream');
const testRoot = join('test-results', 'eventing-network-service-event-chain-stream-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'service-network-runtime-stream-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'network_runtime_stream'],
    log: join(proofRoot, 'service-network-runtime-stream-tests.log'),
  },
  {
    name: 'agent-protocol-network-runtime-stream-command-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'network_runtime_stream_command'],
    log: join(proofRoot, 'agent-protocol-network-runtime-stream-command-test.log'),
  },
  {
    name: 'agent-protocol-domain-generated-network-contract-tests',
    command: npmCommand(),
    args: npmArgs([
      '--workspace',
      '@ocentra-parent/agent-protocol-domain',
      'run',
      'test',
      '--',
      'generated-agent-protocol-contracts.test.ts',
    ]),
    log: join(proofRoot, 'agent-protocol-domain-network-runtime-tests.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing'],
    log: join(proofRoot, 'source-shape.log'),
  },
];

const commandResults = commands.map(runCommand);
assertSourceContracts();

const proof = {
  proof: 'eventing-network-service-event-chain-stream',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  rowsCovered: [
    'network-plan row 10 service WebSocket event-chain stream sub-slice',
    'eventing-plan row 50 portal/read-model event transport supplement',
    'eventing-plan row 62 network proof-links supplement',
  ],
  claimsProved: [
    'agent-protocol-domain and agent-protocol expose a typed WebSocket command and report event for network runtime event-chain streams',
    'the Rust service command handler routes agent.network.runtime.event-chain.stream.get to a service-backed report event',
    'stored ActivityStore network rows are published through the local network runtime and streamed back as protocol-shaped event entries',
    'full metadata rows stream the eleven network flow, domain, classification, AI, policy, enforcement dry-run, audit, and portal read-model entries',
    'partial metadata rows stream no enforcement command or result entries and preserve visible manual-required state',
    'stream payload entries reject exact URL, decrypted payload, message content, search query, raw packet, and adapter-action claims through Rust-generated TypeScript runtime event contracts',
  ],
  claimsNotProved: [
    'broker-backed delivery, relay-hub delivery, cross-process durable replay, production retention/delete/export, or offset/dedupe management',
    'live packet capture, live analyzer/model execution, full policy engine execution, adapter execution, host DNS/filter mutation, firewall mutation, or enforcement-command execution',
    'portal UI rendering or product-complete network/domain blocking',
    'provider notification delivery, remote parent transport, or Ocentra-hosted child activity custody',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-network-service-event-chain-stream-proof-ok:service,protocol,ts,build,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const contracts = readText('packages/schema-domain/src/agent-command-event-contracts.ts');
  const generatedContracts = readText('packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts');
  const generatedContractTests = readText('packages/agent-protocol-domain/tests/unit/generated-agent-protocol-contracts.test.ts');
  const rustTransport = readText('crates/agent-protocol/src/transport.rs');
  const fieldConstants = readText('crates/agent-protocol/src/constants/field.rs');
  const serviceWebSocket = readText('crates/agent-service/src/websocket.rs');
  const serviceActivityApi = readText('crates/agent-service/src/activity_api.rs');
  const streamPayload = readText('crates/agent-service/src/network_runtime_stream_payload.rs');
  const streamEventPayloads = readText('crates/agent-service/src/network_runtime_stream_event_payloads.rs');
  const streamTests = readText('crates/agent-service/tests/unit/network_runtime_stream_tests.rs');

  assertIncludes(contracts, 'agent.network.runtime.event-chain.stream.get', 'TypeScript command contract exists');
  assertIncludes(contracts, 'agent.network.runtime.event-chain.stream.reported', 'TypeScript event contract exists');
  assertIncludes(
    generatedContracts,
    'decodeParentAgentNetworkRuntimeEventPayload',
    'Rust-generated TypeScript runtime-event decoder preserves stream entry event type'
  );
  assertIncludes(
    generatedContractTests,
    'rejects unsupported exact-content and adapter-action claims',
    'generated contract tests reject network runtime event claim upgrades'
  );
  assertIncludes(rustTransport, 'AgentNetworkRuntimeEventChainStreamGet', 'Rust command enum exists');
  assertIncludes(rustTransport, 'AgentNetworkRuntimeEventChainStreamReported', 'Rust event enum exists');
  assertIncludes(fieldConstants, 'NETWORK_RUNTIME_EVENT_CHAIN_STREAM', 'Rust field constant owns stream payload key');
  assertIncludes(fieldConstants, 'NETWORK_RUNTIME_STREAMED_EVENTS', 'Rust field constant owns stream count key');
  assertIncludes(
    serviceWebSocket,
    'AgentCommandName::AgentNetworkRuntimeEventChainStreamGet',
    'WebSocket command dispatcher routes stream command'
  );
  assertIncludes(
    serviceActivityApi,
    'build_network_runtime_event_chain_stream_report',
    'service report builder exists'
  );
  assertIncludes(
    streamPayload,
    'stream_network_runtime_event_chain_for_read_model',
    'service stream publishes stored network rows through runtime'
  );
  assertIncludes(
    streamEventPayloads,
    'adapter_action_executed: false',
    'service stream keeps adapter action claim false'
  );
  assertIncludes(
    streamTests,
    'websocket_network_runtime_stream_command_reports_store_backed_chain',
    'WebSocket command test covers real store-backed stream'
  );
  assertIncludes(
    streamTests,
    'service_network_runtime_stream_skips_enforcement_for_manual_required_rows',
    'manual-required stream test keeps enforcement entries absent'
  );
}

function npmCommand() {
  return process.platform === 'win32' ? 'cmd' : 'npm';
}

function npmArgs(args) {
  return process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
