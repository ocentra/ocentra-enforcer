import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10-service-runtime-delivery');
const testRoot = join('test-results', 'eventing-network-service-runtime-delivery-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'service-network-runtime-delivery-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'network_runtime_delivery'],
    log: join(proofRoot, 'network-runtime-delivery-tests.log'),
  },
  {
    name: 'service-network-payload-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'network_flow_payload'],
    log: join(proofRoot, 'network-flow-payload-tests.log'),
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
  proof: 'eventing-network-service-runtime-delivery',
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
    serviceRuntimeDeliveryTests: 'crates/agent-service/tests/unit/network_runtime_delivery_tests.rs',
    serviceNetworkBridgeHarness: 'crates/agent-service/tests/network_bridge_runtime.rs',
  },
  provenRows: ['network-plan row 10 service runtime delivery sub-slice'],
  claimsProved: [
    'the service network read-model command path calls the local network runtime publisher for each stored network row',
    'service delivery reports observed, delivered, failed, stored, dead-letter, manual-required, and enforcement-command event counts in the read-model event payload',
    'full metadata rows publish the local runtime chain and preserve dry-run enforcement-command visibility',
    'partial metadata rows stay manual-required and publish no enforcement-command events',
    'empty service read models do not invent network runtime events',
  ],
  claimsNotProved: [
    'broker-backed delivery, cross-process durable replay, production retention/delete/export, or relay-hub delivery',
    'live packet capture, live analyzer/model execution, policy execution, adapter execution, or host filtering',
    'TypeScript network event package parity beyond generated/thin edge contracts',
    'portal UI changes or product-ready network/domain blocking',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-network-service-runtime-delivery-proof-ok:service-tests,payload-tests,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const activityApi = readText('crates/agent-service/src/activity_api.rs');
  const deliverySource = readText('crates/agent-service/src/network_runtime_delivery.rs');
  const deliveryTests = readText('crates/agent-service/tests/unit/network_runtime_delivery_tests.rs');
  const runtimeHarness = readText('crates/agent-service/tests/network_bridge_runtime.rs');
  const payloadSource = readText('crates/agent-service/src/activity_network_flow_payload.rs');
  const fieldConstants = readText('crates/agent-protocol/src/constants/field.rs');

  assertIncludes(
    activityApi,
    'deliver_network_runtime_for_read_model(&read_model).await',
    'service command publishes local runtime delivery before reporting payload'
  );
  assertIncludes(
    deliverySource,
    'publish_network_runtime_chain_for_observation',
    'service delivery uses agent-core runtime publisher'
  );
  assertIncludes(
    deliverySource,
    'EVENT_ENFORCEMENT_COMMAND_ISSUED',
    'service delivery counts enforcement command events'
  );
  assertIncludes(
    deliveryTests,
    'service_network_read_model_keeps_partial_metadata_manual_required',
    'service delivery tests prove manual-required path'
  );
  assertIncludes(
    deliveryTests,
    'empty_service_network_read_model_does_not_invent_runtime_events',
    'service delivery tests prove no invented events'
  );
  assertIncludes(
    runtimeHarness,
    '#[path = "unit/network_runtime_delivery_tests.rs"]',
    'service delivery tests live under the real tests/unit folder'
  );
  assertIncludes(payloadSource, 'NETWORK_RUNTIME_DELIVERED_ROWS', 'service payload exposes runtime delivery counts');
  assertIncludes(
    fieldConstants,
    'NETWORK_RUNTIME_ENFORCEMENT_COMMAND_EVENTS',
    'protocol constants own runtime delivery field ids'
  );
  assertDoesNotInclude(
    activityApi,
    'EVENT_ENFORCEMENT_COMMAND_ISSUED',
    'service command path does not publish direct enforcement command'
  );
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertDoesNotInclude(text, unexpected, label) {
  if (text.includes(unexpected)) {
    throw new Error(`${label}: found ${unexpected}`);
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
