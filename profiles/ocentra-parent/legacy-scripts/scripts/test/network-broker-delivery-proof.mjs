import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10a-broker-delivery-proof');
const testRoot = join('test-results', 'network-broker-delivery-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'network-broker-delivery-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_broker_delivery'],
    log: join(proofRoot, 'broker-delivery-tests.log'),
  },
  {
    name: 'network-queue-idempotency-tests',
    command: 'cargo',
    args: [
      'test',
      '-p',
      'ocentra-parent-agent-core',
      'network_runtime_queue_idempotency_rejects_queued_and_completed_duplicates',
    ],
    log: join(proofRoot, 'queue-idempotency-tests.log'),
  },
  {
    name: 'network-queue-overflow-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_queue_overflow_dead_letters_oldest_flow'],
    log: join(proofRoot, 'queue-overflow-tests.log'),
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
    args: [
      'scripts/check-source-shape.mjs',
      'scripts/test/network-broker-delivery-proof.mjs',
      'crates/agent-core/src/network_event_runtime/broker_delivery.rs',
      'crates/agent-core/tests/unit/network_event_runtime_broker_delivery_tests.rs',
      'crates/agent-core/src/network_event_runtime/queue.rs',
      'crates/agent-core/tests/unit/network_event_runtime_queue_tests.rs',
      'crates/ocentra-eventing/src/delivery.rs',
      'crates/ocentra-eventing/tests/contract/delivery.rs',
    ],
    log: join(proofRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);
assertSourceContracts();

const brokerDeliveryLog = [
  'network row10a broker delivery semantics',
  '',
  'semantic=local-idempotency-queue-proof',
  'brokerDeliveryImplemented=false',
  'relayHubDeliveryImplemented=false',
  'duplicateDetection=queued-and-completed-idempotency-rejection',
  'replay=broker-replay-plan-ref-preserved',
  'droppedEventAudit=queue-overflow-dead-letter-count-preserved',
  'adapterAction=zero-enforcement-command-events-and-zero-adapter-action-executed',
  '',
  ...commandResults.map((result) => `${result.name}: ${result.command} -> exit ${result.status}; log=${result.log}`),
  '',
];
writeFileSync(join(proofRoot, '10a-broker-delivery-proof.log'), brokerDeliveryLog.join('\n'));

const proof = {
  proof: 'network-broker-delivery',
  proofRevision: 'network-broker-delivery-proof/v1',
  checkedAt: 'deterministic:network-broker-delivery-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  mergeBase: mergeBase(),
  sourceStatusShort: sourceStatusShort(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    brokerDeliveryProofLog: join(proofRoot, '10a-broker-delivery-proof.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['10a Broker delivery semantics proof'],
  provenBehavior: [
    'broker route requirements can be satisfied while live broker delivery remains false',
    'local network queue idempotency rejects queued and completed duplicate events',
    'queue overflow creates dropped-event dead-letter audit evidence',
    'broker replay, dropped-event audit, and adapter-action ledger refs are preserved',
    'duplicate broker routes do not create duplicate enforcement command events or adapter actions',
  ],
  notClaimed: [
    'live broker delivery',
    'relay-hub delivery',
    'cross-process broker transport',
    'production retention/delete/export propagation',
    'adapter execution',
    'enforcement command publication from duplicate broker events',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-broker-delivery-proof-ok:tests,clippy,source-shape');
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
  const brokerSource = readText('crates/agent-core/src/network_event_runtime/broker_delivery.rs');
  const brokerTestsSource = readText('crates/agent-core/tests/unit/network_event_runtime_broker_delivery_tests.rs');
  const unitHarness = readText('crates/agent-core/tests/unit.rs');
  const deliverySource = readText('crates/ocentra-eventing/src/delivery.rs');
  const deliveryTestsSource = readText('crates/ocentra-eventing/tests/contract/delivery.rs');
  const eventingContractHarness = readText('crates/ocentra-eventing/tests/contract.rs');

  assertIncludes(
    brokerSource,
    'prove_network_runtime_broker_delivery_semantics',
    'network broker delivery proof helper exists'
  );
  assertIncludes(
    brokerSource,
    'external_transport_delivery_implemented: delivery_decision',
    'broker proof exposes the live external transport implementation state'
  );
  assertIncludes(
    brokerSource,
    'external_relay_delivery_implemented: delivery_decision.external_relay_delivery_implemented',
    'broker proof exposes the live relay implementation state'
  );
  assertIncludes(
    brokerSource,
    'adapter_action_executed_count',
    'broker proof counts adapter-action execution instead of assuming none'
  );
  assertIncludes(
    brokerTestsSource,
    'network_runtime_broker_delivery_semantics_preserve_refs_without_live_broker',
    'network broker tests preserve the no-live-transport claim boundary'
  );
  assertIncludes(
    unitHarness,
    '#[path = "unit/network_event_runtime_broker_delivery_tests.rs"]',
    'network broker tests live under the real tests/unit folder'
  );
  assertIncludes(
    brokerTestsSource,
    'TEST_BROKER_REPLAY_PLAN_REF',
    'network broker tests assert replay/dead-letter/adapter ledger refs'
  );
  assertIncludes(
    brokerTestsSource,
    'adapter_action_executed_count, 0',
    'network broker tests assert duplicate broker routes do not duplicate adapter actions'
  );
  assertIncludes(
    deliverySource,
    'external_transport_delivery_claimed',
    'generic eventing delivery decision owns external transport claim input'
  );
  assertIncludes(
    deliveryTestsSource,
    'delivery_decision_preserves_satisfied_external_transport_requirements_without_live_transport',
    'generic eventing delivery tests prove external transport requirement satisfaction'
  );
  assertIncludes(
    eventingContractHarness,
    '#[path = "contract/delivery.rs"]',
    'generic eventing delivery tests live under the real tests/contract folder'
  );
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function sourceStatusShort() {
  return runText('git', [
    'status',
    '--short',
    '--',
    '.',
    ':(exclude)output/network-plan-proof/10a-broker-delivery-proof',
    ':(exclude)test-results/network-broker-delivery-proof',
  ]);
}

function sourceFingerprint() {
  const sourceFiles = [
    'scripts/test/network-broker-delivery-proof.mjs',
    'crates/agent-core/src/network_event_runtime/broker_delivery.rs',
    'crates/agent-core/tests/unit/network_event_runtime_broker_delivery_tests.rs',
    'crates/agent-core/src/network_event_runtime/queue.rs',
    'crates/agent-core/tests/unit/network_event_runtime_queue_tests.rs',
    'crates/ocentra-eventing/src/delivery.rs',
    'crates/ocentra-eventing/tests/contract/delivery.rs',
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
