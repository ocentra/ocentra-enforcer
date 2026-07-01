import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '18-24-handler-policy');
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

const executionSource = readFileSync('crates/ocentra-eventing/src/execution.rs', 'utf8');
const dispatchSource = readFileSync('crates/ocentra-eventing/src/bus/dispatch.rs', 'utf8');
const reportsSource = readFileSync('crates/ocentra-eventing/src/bus/reports.rs', 'utf8');
const testkitSource = readFileSync('crates/ocentra-eventing/src/testkit.rs', 'utf8');
const handlerPolicyTests = readFileSync('crates/ocentra-eventing/src/tests/handler_policy.rs', 'utf8');

const sourceAssertions = [
  ['handler-execution-policy', executionSource.includes('pub struct HandlerExecutionPolicy')],
  [
    'handler-timeout-wrapper',
    dispatchSource.includes('clock.sleep(timeout)') && dispatchSource.includes('AttemptOutcome::TimedOut'),
  ],
  ['handler-timeout-outcome', reportsSource.includes('TimedOut')],
  ['event-trace-fields', reportsSource.includes('pub struct EventTraceFields')],
  ['event-recorder-testkit', testkitSource.includes('pub struct EventRecorder<E>')],
  ['retry-policy-test', handlerPolicyTests.includes('retry_policy_retries_failed_attempt_and_reports_trace_fields')],
  ['timeout-policy-test', handlerPolicyTests.includes('timeout_policy_retries_then_dead_letters_final_timeout')],
  ['event-recorder-test', handlerPolicyTests.includes('event_recorder_uses_real_subscription_and_can_unsubscribe')],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing handler policy assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-handler-policy',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '18 handler timeout and retry policy',
    '20 metrics and tracing fields',
    '24 testkit bus construction and event recorder',
  ],
  notClaimed: [
    '31-35 request completion and durable result-event patterns',
    '36-41 durable journal and replay semantics',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-handler-policy-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
