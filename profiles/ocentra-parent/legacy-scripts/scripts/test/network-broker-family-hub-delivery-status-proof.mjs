import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10b-broker-family-hub-delivery-status');
const testRoot = join('test-results', 'network-broker-family-hub-delivery-status-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'network-remote-delivery-status-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery'],
    log: join(proofRoot, 'remote-delivery-status-tests.log'),
  },
  {
    name: 'network-broker-delivery-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_broker_delivery'],
    log: join(proofRoot, 'broker-delivery-tests.log'),
  },
  {
    name: 'eventing-delivery-decision-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'delivery_decision'],
    log: join(proofRoot, 'eventing-delivery-decision-tests.log'),
  },
  {
    name: 'agent-core-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-parent-agent-core', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'agent-core-clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);
assertSourceContracts();

const statusLog = [
  'network row10b broker/family-hub remote delivery status',
  '',
  'brokerStatus=fixture-requirements-recorded-but-not-implemented',
  'familyHubStatus=fixture-requirements-recorded-but-not-implemented',
  'brokerDeliveryImplemented=false',
  'familyHubDeliveryImplemented=false',
  'crossProcessReplayImplemented=false',
  'remoteRetentionDeleteExportPropagationImplemented=false',
  'policyAuthority=false',
  'sideEffectAuthority=false',
  'enforcementCommandEvents=0',
  'adapterActionsExecuted=0',
  '',
  ...commandResults.map((result) => `${result.name}: ${result.command} -> exit ${result.status}; log=${result.log}`),
  '',
];
writeFileSync(join(proofRoot, '10b-remote-delivery-status.log'), statusLog.join('\n'));

const proof = {
  proof: 'network-broker-family-hub-delivery-status',
  proofRevision: 'network-broker-family-hub-delivery-status-proof/v1',
  checkedAt: 'deterministic:network-broker-family-hub-delivery-status-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  mergeBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    statusLog: join(proofRoot, '10b-remote-delivery-status.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: [
    '10b Broker/family-hub remote delivery status proof',
    '10 NetworkActivityEvent reusable Rust eventing consumption',
  ],
  provenBehavior: [
    'broker and family-hub relay routes carry custody, auth, encryption, retention, replay, deletion, offset, dedupe, broker config, identity, and relay policy refs',
    'fixture-recorded broker and family-hub decisions remain explicit status rows rather than live transport claims',
    'local idempotency and overflow dead-letter evidence remains attached to the remote-delivery status',
    'subscriber filters remain scoped to the network event namespace',
    'duplicate and family-hub status proof does not publish enforcement command events or execute adapter actions',
  ],
  notClaimed: [
    'live broker delivery',
    'live family-hub delivery',
    'cross-process durable replay',
    'remote retention/delete/export propagation',
    'policy authority',
    'side-effect authority',
    'adapter execution',
    'host filtering',
    'full network-plan completion',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-broker-family-hub-delivery-status-proof-ok:rust,eventing,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, normalizeLogText(`${result.stdout ?? ''}${result.stderr ?? ''}`));
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

function assertSourceContracts() {
  const statusSource = readText('crates/agent-core/src/network_event_runtime/remote_delivery_status.rs');
  const statusTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const brokerSource = readText('crates/agent-core/src/network_event_runtime/broker_delivery.rs');
  const deliverySource = readText('crates/ocentra-eventing/src/delivery.rs');

  assertIncludes(
    statusSource,
    'NetworkRuntimeRemoteDeliveryState::FixtureRequirementsRecordedButNotImplemented',
    'remote delivery status exposes explicit not-implemented state'
  );
  assertIncludes(
    statusSource,
    'external_transport_delivery_claimed: false',
    'remote delivery status does not claim live external transport'
  );
  assertIncludes(
    statusSource,
    'external_relay_delivery_claimed: false',
    'remote delivery status does not claim live family-hub relay'
  );
  assertIncludes(
    statusSource,
    'cross_process_replay_implemented: false',
    'remote delivery status keeps cross-process replay unclaimed'
  );
  assertIncludes(
    statusSource,
    'remote_retention_delete_export_propagation_implemented: false',
    'remote delivery status keeps remote delete/export propagation unclaimed'
  );
  assertIncludes(
    statusTests,
    'network_runtime_remote_delivery_status_preserves_broker_family_hub_refs_without_transport',
    'remote delivery tests preserve broker and family-hub refs without transport'
  );
  assertIncludes(statusTests, 'adapter_action_executed_count, 0', 'remote delivery tests assert no adapter execution');
  assertIncludes(
    brokerSource,
    'prove_network_runtime_broker_delivery_semantics',
    'remote delivery status composes the broker semantics proof'
  );
  assertIncludes(
    deliverySource,
    'ExternalRelayRouteRequirementsSatisfied',
    'generic eventing delivery decision owns relay route requirement state'
  );
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function sourceFingerprint() {
  const sourceFiles = [
    'scripts/test/network-broker-family-hub-delivery-status-proof.mjs',
    'crates/agent-core/src/network_event_runtime/remote_delivery_status.rs',
    'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
    'crates/agent-core/src/network_event_runtime/broker_delivery.rs',
    'crates/agent-core/tests/unit/network_event_runtime_broker_delivery_tests.rs',
    'crates/ocentra-eventing/src/delivery.rs',
    'crates/ocentra-eventing/src/tests/delivery.rs',
  ];
  const hash = createHash('sha256');
  for (const filePath of sourceFiles) {
    hash.update(filePath);
    hash.update('\0');
    hash.update(readText(filePath));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function mergeBase() {
  return runText('git', ['merge-base', 'HEAD', 'origin/main']).trim();
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function normalizeLogText(text) {
  const normalizedLines = sortConsecutiveTestLines(
    text
      .replace(/\r\n/g, '\n')
      .split('\n')
      .filter((line) => !line.includes('Blocking waiting for'))
      .filter((line) => !line.trimStart().startsWith('Compiling '))
      .filter((line) => !line.trimStart().startsWith('Checking '))
      .map((line) =>
        line
          .replace(/finished in [0-9.]+s/g, 'finished in <duration>')
          .replace(/target\(s\) in [0-9.]+s/g, 'target(s) in <duration>')
      )
  );
  const trimmed = normalizedLines
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return trimmed.length === 0 ? '' : `${trimmed}\n`;
}

function sortConsecutiveTestLines(lines) {
  const sortedLines = [];
  let testLineBuffer = [];
  for (const line of lines) {
    if (line.startsWith('test ') && line.endsWith(' ... ok')) {
      testLineBuffer.push(line);
      continue;
    }
    if (testLineBuffer.length > 0) {
      sortedLines.push(...testLineBuffer.sort());
      testLineBuffer = [];
    }
    sortedLines.push(line);
  }
  if (testLineBuffer.length > 0) {
    sortedLines.push(...testLineBuffer.sort());
  }
  return sortedLines;
}
