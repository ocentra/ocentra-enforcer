import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '74-lifecycle-clear');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'lifecycle-clear-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'lifecycle_clear'],
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

const busSource = readFileSync('crates/ocentra-eventing/src/bus.rs', 'utf8');
const busLifecycleSource = readFileSync('crates/ocentra-eventing/src/bus/lifecycle.rs', 'utf8');
const queueSource = readFileSync('crates/ocentra-eventing/src/queue/state.rs', 'utf8');
const requestSource = readFileSync('crates/ocentra-eventing/src/request.rs', 'utf8');
const lifecycleTests = readFileSync('crates/ocentra-eventing/src/tests/lifecycle_clear.rs', 'utf8');

const assertions = [
  ['event-bus-clear-report-exists', busSource.includes('pub struct EventBusClearReport')],
  ['event-bus-clear-method-exists', busLifecycleSource.includes('pub async fn clear_for_test')],
  ['clear-reports-subscriptions', busLifecycleSource.includes('subscription_count')],
  [
    'clear-reports-journal-and-dead-letters',
    busLifecycleSource.includes('stored_journal_count') && busLifecycleSource.includes('dead_letter_count'),
  ],
  ['clear-reports-aggregate-gates', busLifecycleSource.includes('aggregate_gate_count')],
  ['queue-clear-method-exists', queueSource.includes('pub(crate) fn clear_for_test')],
  ['queue-clear-removes-queued-events', queueSource.includes('state.queued.clear()')],
  ['queue-clear-removes-idempotency-state', queueSource.includes('state.completed_keys.clear()')],
  ['request-clear-method-exists', requestSource.includes('pub(crate) fn clear_for_test')],
  ['request-clear-counts-pending', requestSource.includes('pending_request_count')],
  ['request-clear-drops-senders', requestSource.includes('entries.clear()')],
  ['lifecycle-test-resets-state', lifecycleTests.includes('clear_for_test_reports_and_resets_local_bus_state')],
  ['lifecycle-test-cancels-request', lifecycleTests.includes('clear_for_test_cancels_pending_request_completion')],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing lifecycle-clear assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-lifecycle-clear',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  provenRows: [
    '74 Shutdown/clear lifecycle drains, dead-letters, cancels, or test-clears state according to documented policy',
  ],
  policy:
    'EventBus::clear_for_test is a deterministic test lifecycle, not a production shutdown claim; it clears local bus state and cancels pending local requests by dropping completion senders.',
  notClaimed: [
    'production shutdown drain',
    'broker-backed delivery cancellation',
    '69 Unity/TypeScript semantics conformance matrix',
    '70 event topology manifest',
    '72 contract registry generated docs',
    '75 event-family wrapper proof',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-lifecycle-clear-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
