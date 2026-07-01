import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '20-24-metrics-testkit');
const testRoot = join('test-results', 'eventing-metrics-testkit-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'eventing-handler-policy-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'handler_policy'],
  },
  {
    name: 'eventing-metrics-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'metrics'],
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
  {
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check', '--', '.', ':(exclude)output', ':(exclude)test-results'],
  },
];

const commandResults = commands.map(runCommand);

const reportsSource = readFileSync('crates/ocentra-eventing/src/bus/reports.rs', 'utf8');
const busSource = readFileSync('crates/ocentra-eventing/src/bus.rs', 'utf8');
const queueSource = readFileSync('crates/ocentra-eventing/src/queue/state.rs', 'utf8');
const requestSource = readFileSync('crates/ocentra-eventing/src/request.rs', 'utf8');
const testkitSource = readFileSync('crates/ocentra-eventing/src/testkit.rs', 'utf8');
const handlerPolicyTests = readFileSync('crates/ocentra-eventing/src/tests/handler_policy.rs', 'utf8');
const metricsTests = readFileSync('crates/ocentra-eventing/src/tests/metrics.rs', 'utf8');

const sourceAssertions = [
  ['event-metrics-snapshot-struct', reportsSource.includes('pub struct EventMetricsSnapshot')],
  ['event-queue-metrics-struct', reportsSource.includes('pub struct EventQueueMetrics')],
  ['event-request-metrics-struct', reportsSource.includes('pub struct EventRequestMetrics')],
  ['metrics-snapshot-api', busSource.includes('pub async fn metrics_snapshot')],
  ['queue-metrics-counts', queueSource.includes('pub(crate) fn metrics')],
  ['request-metrics-counts', requestSource.includes('pub(crate) fn metrics')],
  ['event-trace-fields-struct', reportsSource.includes('pub struct EventTraceFields')],
  ['trace-event-id', reportsSource.includes('pub event_id: EventId')],
  ['trace-event-type', reportsSource.includes('pub event_type: EventType')],
  ['trace-correlation-id', reportsSource.includes('pub correlation_id: CorrelationId')],
  ['trace-target-handler', reportsSource.includes('pub target_handler: TargetHandler')],
  ['trace-outcome', reportsSource.includes('pub outcome: HandlerOutcome')],
  ['event-recorder-struct', testkitSource.includes('pub struct EventRecorder<E>')],
  ['event-recorder-attach', testkitSource.includes('pub async fn attach')],
  ['event-recorder-unsubscribe', testkitSource.includes('pub fn unsubscribe')],
  ['trace-field-test', handlerPolicyTests.includes('retry_policy_retries_failed_attempt_and_reports_trace_fields')],
  [
    'event-recorder-real-subscription-test',
    handlerPolicyTests.includes('event_recorder_uses_real_subscription_and_can_unsubscribe'),
  ],
  [
    'metrics-snapshot-test',
    metricsTests.includes('metrics_snapshot_reports_queue_dead_letter_journal_and_request_counts'),
  ],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing metrics/testkit assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-metrics-testkit',
  proofRoot,
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: ['20 metrics snapshot and tracing fields', '24 testkit bus construction and event recorder'],
  provenBehavior: [
    'EventBus::metrics_snapshot exposes exact stored event, dead-letter, queue, idempotency, and request-state counts',
    'handler reports expose trace fields for event id, event type, correlation id, target handler, and outcome',
    'EventRecorder attaches through a real typed subscription and can unsubscribe without mocks, fakes, stubs, or spies',
  ],
  notClaimed: [
    'external telemetry exporter integration',
    'broker or relay-hub delivery metrics',
    'portal business-event publishing',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-metrics-testkit-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const log = join(proofRoot, `${entry.name}.log`);
  writeFileSync(log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}; log=${log}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log,
  };
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
