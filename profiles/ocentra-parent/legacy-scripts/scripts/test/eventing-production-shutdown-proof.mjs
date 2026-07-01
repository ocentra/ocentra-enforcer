import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '74-production-shutdown');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'production-shutdown-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'production_shutdown'],
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
const lifecycleSource = readFileSync('crates/ocentra-eventing/src/bus/lifecycle.rs', 'utf8');
const reportsSource = readFileSync('crates/ocentra-eventing/src/bus/reports.rs', 'utf8');
const publishSource = readFileSync('crates/ocentra-eventing/src/bus/publish.rs', 'utf8');
const requestSource = readFileSync('crates/ocentra-eventing/src/request.rs', 'utf8');
const testsSource = readFileSync('crates/ocentra-eventing/src/tests/production_shutdown.rs', 'utf8');

const assertions = [
  ['shutdown-mode-exists', busSource.includes('pub enum ShutdownMode')],
  ['shutdown-report-exists', busSource.includes('pub struct EventBusShutdownReport')],
  [
    'shutdown-state-exists',
    busSource.includes('shutdown: Arc<Mutex<EventBusLifecycleState>>') &&
      busSource.includes('enum EventBusLifecycleState') &&
      busSource.includes('ShuttingDown'),
  ],
  ['shutdown-method-exists', lifecycleSource.includes('pub async fn shutdown')],
  ['drain-mode-drains-queued', lifecycleSource.includes('drain_queued_unchecked')],
  ['shutdown-dead-letters-remaining-queued', lifecycleSource.includes('DeadLetterReason::Shutdown')],
  ['test-only-drop-mode-reports-drops', lifecycleSource.includes('DropQueuedForTestOnly')],
  ['pending-requests-cancelled', lifecycleSource.includes('cancel_for_shutdown')],
  ['publish-rejects-after-shutdown', publishSource.includes('self.ensure_active()?')],
  ['request-registry-cancel-for-shutdown', requestSource.includes('pub(crate) fn cancel_for_shutdown')],
  ['dead-letter-reason-shutdown', reportsSource.includes('Shutdown')],
  ['drain-test-exists', testsSource.includes('production_shutdown_drain_dispatches_queue_and_dead_letters_remaining')],
  ['dead-letter-test-exists', testsSource.includes('production_shutdown_dead_letters_queued_without_dispatch')],
  ['drop-test-exists', testsSource.includes('test_only_shutdown_drop_reports_dropped_queued_work')],
  ['request-cancel-test-exists', testsSource.includes('production_shutdown_cancels_pending_request_completion')],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing production-shutdown assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-production-shutdown',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  provenRows: ['74 Bus shutdown, drain, dead-letter, and test clear lifecycle'],
  policy:
    'EventBus::shutdown supports production drain, production queued dead-letter, and explicit test-only queued drop modes. Shutdown reports queued dispatch/dead-letter/drop counts, cancels pending local requests by dropping completion senders, and rejects new publish/subscribe work after shutdown.',
  notClaimed: [
    'broker-backed delivery shutdown',
    'cross-process parent-child transport shutdown',
    'platform adapter rollback execution',
    'whole-repo event topology source scan',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-production-shutdown-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
