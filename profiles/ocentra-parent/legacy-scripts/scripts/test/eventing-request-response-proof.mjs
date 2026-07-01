import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '31-35-request-response');
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

const idsSource = readFileSync('crates/ocentra-eventing/src/ids.rs', 'utf8');
const requestSource = readFileSync('crates/ocentra-eventing/src/request.rs', 'utf8');
const publishSource = readFileSync('crates/ocentra-eventing/src/bus/publish.rs', 'utf8');
const publisherSource = readFileSync('crates/ocentra-eventing/src/bus/publisher.rs', 'utf8');
const requestTests = readFileSync('crates/ocentra-eventing/src/tests/request_response.rs', 'utf8');

const sourceAssertions = [
  ['request-id-newtype', idsSource.includes('text_identifier!(RequestId')],
  ['event-response-contract', requestSource.includes('pub trait EventResponseContract')],
  ['request-event-associated-response', requestSource.includes('type Response: EventResponseContract')],
  ['publish-request-associated-return', publishSource.includes('Result<RequestReport<E::Response>, EventingError>')],
  [
    'context-complete-request',
    publisherSource.includes('impl<E> EventContext<E>') && publisherSource.includes('E: RequestEvent'),
  ],
  ['local-request-registry', requestSource.includes('pub(crate) struct RequestRegistry')],
  ['timeout-state', requestSource.includes('RequestState::TimedOut')],
  ['duplicate-completion-outcome', requestSource.includes('RequestCompletionOutcome::Duplicate')],
  ['late-completion-outcome', requestSource.includes('RequestCompletionOutcome::Late')],
  ['typed-response-test', requestTests.includes('publish_request_resolves_associated_response_type')],
  ['validation-before-settle-test', requestTests.includes('invalid_response_validation_does_not_settle_request')],
  ['timeout-late-test', requestTests.includes('request_timeout_reports_late_response_without_mutating_result')],
  ['double-completion-test', requestTests.includes('double_completion_is_ignored_and_reported')],
  [
    'durable-result-event-test',
    requestTests.includes('durable_result_event_pattern_remains_separate_from_local_completion'),
  ],
  [
    'no-payload-sender',
    !requestTests.includes('oneshot::Sender') && !requestTests.includes('RequestCompletionReport>'),
  ],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing request-response assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-request-response',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '31 local request completion registry',
    '32 RequestEvent::Response typed response resolution',
    '33 timeout and late-response handling',
    '34 double-completion guard',
    '35 durable result-event pattern docs/tests',
    '65 RequestEvent associated response proof',
  ],
  notClaimed: [
    '36-41 durable journal and replay semantics',
    '66-67 ownership/interior-mutability and lock-held-await source gates',
    '71 manual clock deterministic timeout proof',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-request-response-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
