import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '14-24-runtime-lifecycle');
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
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check', '--', '.', ':(exclude)output', ':(exclude)test-results'],
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
const publishSource = readFileSync('crates/ocentra-eventing/src/bus/publish.rs', 'utf8');
const publisherSource = readFileSync('crates/ocentra-eventing/src/bus/publisher.rs', 'utf8');
const dispatchSource = readFileSync('crates/ocentra-eventing/src/bus/dispatch.rs', 'utf8');
const subscriberSource = readFileSync('crates/ocentra-eventing/src/bus/subscriber.rs', 'utf8');
const registrarSource = readFileSync('crates/ocentra-eventing/src/registrar.rs', 'utf8');
const lifecycleTestsSource = readFileSync('crates/ocentra-eventing/src/tests/lifecycle.rs', 'utf8');

const sourceAssertions = [
  ['ordered-dispatch-api', busSource.includes('OrderedByAggregateKey')],
  ['typed-context-api', publisherSource.includes('pub struct EventContext<E>')],
  ['nested-publisher-api', publisherSource.includes('pub struct EventPublisher')],
  ['detached-publish-api', publishSource.includes('publish_detached')],
  ['panic-isolation', dispatchSource.includes('catch_unwind')],
  ['subscription-handle', subscriberSource.includes('pub struct SubscriptionHandle')],
  ['registrar-lifecycle', registrarSource.includes('pub struct EventRegistrar')],
  ['no-hidden-global-bus', !busSource.includes('static EVENT_BUS')],
  ['aggregate-ordering-test', lifecycleTestsSource.includes('ordered_dispatch_serializes_same_aggregate_transitions')],
  ['nested-publish-test', lifecycleTestsSource.includes('nested_publish_uses_context_publisher_without_deadlock')],
  ['panic-isolation-test', lifecycleTestsSource.includes('panicking_handler_isolated_as_dead_letter_report')],
  ['registrar-dispose-test', lifecycleTestsSource.includes('registrar_dispose_removes_all_owned_subscriptions')],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing runtime lifecycle assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-runtime-lifecycle',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '14 aggregate-ordered dispatch',
    '15 nested publish through safe event context',
    '16 fire-and-forget publish mode',
    '17 publish-and-wait mode',
    '19 panic isolation and runtime survival',
    '21 EventRegistrar lifecycle',
    '22 subscription handle drop and idempotent unsubscribe',
  ],
  notClaimed: [
    '18 handler timeout and retry policy',
    '20 metrics and tracing fields',
    '24 testkit bus construction and event recorder',
    '31-41 request journal replay',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-runtime-lifecycle-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
