import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '71-manual-clock');
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

const clockSource = readFileSync('crates/ocentra-eventing/src/clock.rs', 'utf8');
const envelopeSource = readFileSync('crates/ocentra-eventing/src/envelope.rs', 'utf8');
const publishSource = readFileSync('crates/ocentra-eventing/src/bus/publish.rs', 'utf8');
const queueDrainSource = readFileSync('crates/ocentra-eventing/src/bus/queue_drain.rs', 'utf8');
const dispatchSource = readFileSync('crates/ocentra-eventing/src/bus/dispatch.rs', 'utf8');
const queueStateSource = readFileSync('crates/ocentra-eventing/src/queue/state.rs', 'utf8');
const manualClockTests = readFileSync('crates/ocentra-eventing/src/tests/clock_manual.rs', 'utf8');
const manualClockBlock = sliceBetween(clockSource, 'pub struct ManualEventClock', '#[derive(Default');

const assertions = [
  ['event-clock-trait-exists', clockSource.includes('pub trait EventClock')],
  ['system-event-clock-exists', clockSource.includes('pub struct SystemEventClock')],
  ['manual-event-clock-exists', clockSource.includes('pub struct ManualEventClock')],
  ['manual-clock-advance-exists', clockSource.includes('pub fn advance(&self, duration: Duration)')],
  ['manual-clock-pending-count-exists', clockSource.includes('pub fn pending_sleep_count(&self) -> usize')],
  ['manual-clock-has-no-wall-clock-sleep', !manualClockBlock.includes('tokio::time::sleep')],
  ['event-metadata-carries-deadline', envelopeSource.includes('pub deadline: Option<EventClockInstant>')],
  ['stored-envelope-checks-deadline', envelopeSource.includes('pub fn is_deadline_expired')],
  ['publish-request-uses-event-clock-sleep', publishSource.includes('self.clock.sleep(options.timeout())')],
  ['publish-path-dead-letters-expired-deadline', publishSource.includes('dead_letter_expired_deadline')],
  [
    'queued-drain-checks-deadline',
    queueDrainSource.includes('queued_expiration(') && queueDrainSource.includes('stored.is_deadline_expired(now)'),
  ],
  ['dispatch-retry-checks-deadline', dispatchSource.includes('HandlerOutcome::DeadlineExpired')],
  ['dispatch-timeout-uses-event-clock-sleep', dispatchSource.includes('clock.sleep(timeout)')],
  ['dispatch-has-no-tokio-timeout', !dispatchSource.includes('tokio::time::timeout')],
  ['publish-has-no-tokio-timeout', !publishSource.includes('tokio::time::timeout')],
  ['queue-state-uses-event-clock-instant', queueStateSource.includes('EventClockInstant')],
  ['queue-state-has-no-instant-now', !queueStateSource.includes('Instant::now')],
  ['manual-tests-have-no-wall-clock-sleep', !manualClockTests.includes('tokio::time::sleep')],
  ['manual-tests-cover-ttl', manualClockTests.includes('manual_clock_expires_queued_ttl')],
  ['manual-tests-cover-deadline', manualClockTests.includes('manual_clock_dead_letters_past_deadline')],
  ['manual-tests-cover-retry-deadline', manualClockTests.includes('manual_clock_stops_retry_when_deadline_expires')],
  ['manual-tests-cover-handler-timeout', manualClockTests.includes('manual_clock_drives_handler_timeout_retries')],
  ['manual-tests-cover-request-timeout', manualClockTests.includes('manual_clock_drives_request_timeout')],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing manual-clock assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-manual-clock',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  provenRows: ['71 Manual clock controls TTL, retry, deadline, queue expiry, and request timeout tests'],
  notClaimed: [
    '69 Unity/TypeScript semantics conformance matrix',
    '70 event topology manifest',
    '72 contract registry generated docs',
    '73 duplicate subscription override',
    '74 shutdown/drain/test-clear lifecycle',
    '75 event-family wrapper proof',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-manual-clock-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function sliceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex);
  if (startIndex < 0 || endIndex < 0) {
    throw new Error(`Unable to slice source between ${start} and ${end}`);
  }
  return source.slice(startIndex, endIndex);
}
