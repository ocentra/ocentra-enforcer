import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10-network-backpressure-depth');
const testRoot = join('test-results', 'eventing-network-backpressure-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'agent-core-network-runtime-queue-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_queue'],
    log: join(proofRoot, 'agent-core-network-runtime-queue-tests.log'),
  },
  {
    name: 'eventing-generic-queue-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'queue'],
    log: join(proofRoot, 'eventing-generic-queue-tests.log'),
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
  proof: 'eventing-network-backpressure-proof',
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
    'network-plan row 10 reusable Rust eventing backpressure depth sub-slice',
    'eventing-plan queue/retry/timeout proof consumed by Parent network runtime',
  ],
  claimsProved: [
    'network runtime flow events use the reusable ocentra-eventing no-subscriber queue path',
    'bounded network queue overflow records an explicit queue-overflow dead letter instead of silently dropping work',
    'network queued flow TTL expiry dead-letters before dispatch through ManualEventClock proof',
    'network flow idempotency rejects duplicate queued and completed observations',
    'stored network queue payloads remain metadata-only and do not claim exact URL, decrypted payload, or adapter action execution',
  ],
  claimsNotProved: [
    'broker-backed delivery or relay-hub transport',
    'service WebSocket streaming of the full runtime event chain',
    'production retention, replay, delete/export, offset, or dedupe behavior',
    'host DNS/filter, firewall, WFP, VpnService, NetworkExtension, nftables, eBPF, or TUN adapter execution',
    'policy execution, enforcement-command publication, or portal risk-budget UI rendering',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-network-backpressure-proof-ok:agent-core-queue,eventing-queue,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const queueSource = readText('crates/agent-core/src/network_event_runtime/queue.rs');
  const queueTestsSource = readText('crates/agent-core/tests/unit/network_event_runtime_queue_tests.rs');
  const unitHarness = readText('crates/agent-core/tests/unit.rs');
  const eventingQueueSource = readText('crates/ocentra-eventing/src/queue/state.rs');
  const eventingQueueTests = readText('crates/ocentra-eventing/tests/unit/queue.rs');
  const eventingUnitHarness = readText('crates/ocentra-eventing/tests/unit.rs');

  assertIncludes(
    queueSource,
    'queue_network_runtime_flow_overflow_dead_letters',
    'network overflow proof helper exists'
  );
  assertIncludes(queueSource, 'queue_network_runtime_flow_expires_before_drain', 'network ttl proof helper exists');
  assertIncludes(
    queueSource,
    'queue_network_runtime_flow_rejects_duplicate_idempotency',
    'network idempotency proof helper exists'
  );
  assertIncludes(queueSource, 'ManualEventClock', 'network ttl proof uses manual clock');
  assertIncludes(queueSource, 'with_idempotency_registry', 'network queue enables idempotency registry');
  assertIncludes(queueTestsSource, 'DeadLetterReason::QueueOverflow', 'network overflow dead-letter asserted');
  assertIncludes(
    unitHarness,
    '#[path = "unit/network_event_runtime_queue_tests.rs"]',
    'network queue tests live under the real tests/unit folder'
  );
  assertIncludes(queueTestsSource, 'DeadLetterReason::QueueExpired', 'network ttl dead-letter asserted');
  assertIncludes(queueTestsSource, 'DuplicateIdempotencyKey', 'network duplicate idempotency rejection asserted');
  assertIncludes(
    eventingQueueSource,
    'QueueOverflowPolicy::DropOldestAndDeadLetter',
    'generic eventing queue supports lineage-compatible drop-oldest dead-letter overflow policy'
  );
  assertIncludes(
    eventingQueueTests,
    'bounded_queue_overflow_dead_letters_oldest_event_and_keeps_newest',
    'generic eventing overflow test exists'
  );
  assertIncludes(
    eventingUnitHarness,
    '#[path = "unit/queue.rs"]',
    'generic eventing queue tests live under the real tests/unit folder'
  );
  assertIncludes(
    eventingQueueTests,
    'queued_event_expires_before_dispatch_when_ttl_elapsed',
    'generic eventing ttl test exists'
  );
  assertIncludes(
    eventingQueueTests,
    'idempotency_registry_rejects_queued_and_completed_duplicates',
    'generic eventing idempotency test exists'
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
