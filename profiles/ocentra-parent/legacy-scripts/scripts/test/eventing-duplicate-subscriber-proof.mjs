import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '73-duplicate-subscriber');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'duplicate-subscriber-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'duplicate_subscriber_ids_are_rejected'],
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

const subscriberSource = readFileSync('crates/ocentra-eventing/src/bus/subscriber.rs', 'utf8');
const typedBoundaryTests = readFileSync('crates/ocentra-eventing/src/tests/typed_boundary.rs', 'utf8');
const insertSubscriberBlock = sliceBetween(subscriberSource, 'pub(super) fn insert_subscriber', 'fn remove_subscriber');

const assertions = [
  ['subscriber-policy-checks-id-per-event-type', insertSubscriberBlock.includes('subscriber.id == record.id')],
  ['subscriber-policy-returns-explicit-error', insertSubscriberBlock.includes('EventingError::DuplicateSubscriber')],
  ['subscriber-policy-does-not-replace-existing-handler', !insertSubscriberBlock.includes('remove_subscriber')],
  ['duplicate-policy-test-exists', typedBoundaryTests.includes('duplicate_subscriber_ids_are_rejected')],
  ['duplicate-policy-test-asserts-error', typedBoundaryTests.includes('Err(EventingError::DuplicateSubscriber')],
];

const failed = assertions.find(([, passed]) => !passed);
if (failed) {
  throw new Error(`eventing duplicate-subscriber assertion failed: ${failed[0]}`);
}

const proof = {
  proof: 'eventing-duplicate-subscriber',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: assertions.map(([name]) => name),
  provenRows: ['73 Duplicate subscriber registration policy is explicit and tested'],
  policy:
    'A duplicate subscriber id for the same event type is rejected with EventingError::DuplicateSubscriber; the existing handler remains registered.',
  notClaimed: [
    'constrained force/republish override',
    'SubscriptionPolicy replace/allow semantics',
    'PublishOverride or RepublishPolicy behavior',
    '69 Unity/TypeScript semantics conformance matrix',
    '70 event topology manifest',
    '72 contract registry generated docs',
    '74 shutdown/drain/test-clear lifecycle',
    '75 event-family wrapper proof',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-duplicate-subscriber-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function sliceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex);
  if (startIndex < 0 || endIndex < 0) {
    throw new Error(`Unable to slice source between ${start} and ${end}`);
  }
  return source.slice(startIndex, endIndex);
}
