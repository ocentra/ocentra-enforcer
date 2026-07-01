import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '25-30-queue-policy');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'eventing-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing'],
  },
  {
    name: 'eventing-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-eventing', '--all-targets', '--', '-D', 'warnings'],
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing'],
  },
];

const commandResults = commands.map((entry) => {
  const result = spawnSync(entry.command, entry.args, {
    encoding: 'utf8',
    shell: false,
  });
  const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  writeFileSync(join(proofRoot, `${entry.name}.log`), output);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: join(proofRoot, `${entry.name}.log`),
  };
});

const queueRootSource = readFileSync('crates/ocentra-eventing/src/queue.rs', 'utf8');
const queuePolicySource = readFileSync('crates/ocentra-eventing/src/queue/policy.rs', 'utf8');
const queueStateSource = readFileSync('crates/ocentra-eventing/src/queue/state.rs', 'utf8');
const queueDrainSource = readFileSync('crates/ocentra-eventing/src/bus/queue_drain.rs', 'utf8');
const reportsSource = readFileSync('crates/ocentra-eventing/src/bus/reports.rs', 'utf8');
const queueTests = readFileSync('crates/ocentra-eventing/src/tests/queue.rs', 'utf8');

const sourceAssertions = [
  ['queue-module-split', queueRootSource.includes('mod policy') && queueRootSource.includes('mod state')],
  ['event-queue-policy', queuePolicySource.includes('pub struct EventQueuePolicy')],
  ['no-subscriber-queue-policy', queuePolicySource.includes('NoSubscriberQueuePolicy::Queue')],
  ['bounded-drop-oldest-overflow-policy', queuePolicySource.includes('QueueOverflowPolicy::DropOldestAndDeadLetter')],
  ['bounded-dead-letter-rejected-policy', queuePolicySource.includes('DeadLetterRejected')],
  ['queue-ttl-expiry', queueDrainSource.includes('is_expired(now, self.queue.policy().ttl())')],
  ['in-flight-duplicate-guard', queueStateSource.includes('DuplicateInFlight')],
  ['idempotency-registry', queueStateSource.includes('completed_keys')],
  ['queue-drain-report', reportsSource.includes('pub struct QueueDrainReport')],
  ['dead-letter-event', reportsSource.includes('pub struct DeadLetterEvent')],
  ['no-subscriber-queue-test', queueTests.includes('no_subscriber_queue_drains_after_subscriber_registers')],
  [
    'overflow-drop-oldest-dead-letter-test',
    queueTests.includes('bounded_queue_overflow_dead_letters_oldest_event_and_keeps_newest'),
  ],
  ['queue-ttl-test', queueTests.includes('queued_event_expires_before_dispatch_when_ttl_elapsed')],
  ['in-flight-event-id-duplicate-test', queueTests.includes('in_flight_duplicate_guard_rejects_concurrent_event_id')],
  ['completed-idempotency-test', queueTests.includes('idempotency_registry_rejects_queued_and_completed_duplicates')],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing queue policy assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-queue-policy',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '25 no-subscriber queue policy',
    '26 bounded queue capacity and overflow policy',
    '27 TTL/deadline before dispatch and retry',
    '28 in-flight duplicate guard',
    '29 idempotency key registry for commands',
    '30 dead-letter record and event',
  ],
  notClaimed: [
    '31-35 request completion and durable result-event patterns',
    '36-41 durable journal and replay semantics',
    '42-56 parent/controller, child-agent, policy, enforcement, audit, and read-model event contracts',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-queue-policy-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
