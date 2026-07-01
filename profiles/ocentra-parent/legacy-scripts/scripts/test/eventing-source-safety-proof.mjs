import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '66-76-source-safety');
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

const eventingSources = sourceFiles('crates/ocentra-eventing/src')
  .filter((path) => !path.includes(`${join('src', 'tests')}`))
  .map((path) => [path, readFileSync(path, 'utf8')]);

const publisherSource = readFileSync('crates/ocentra-eventing/src/bus/publisher.rs', 'utf8');
const testkitSource = readFileSync('crates/ocentra-eventing/src/testkit.rs', 'utf8');
const requestSource = readFileSync('crates/ocentra-eventing/src/request.rs', 'utf8');
const requestPayloadSource = sliceBetween(requestSource, 'pub(crate) struct RequestPayload', 'fn completion_report');

const publicContextBlock = sliceBetween(publisherSource, 'pub struct EventContext<E>', 'impl<E> EventContext<E>');

const sourceAssertions = [
  ['context-envelope-private', !publicContextBlock.includes('pub envelope')],
  ['context-publisher-private', !publicContextBlock.includes('pub publisher')],
  ['context-envelope-accessor', publisherSource.includes('pub fn envelope(&self) -> &EventEnvelope<E>')],
  ['context-payload-accessor', publisherSource.includes('pub fn payload(&self) -> &E')],
  ['context-publisher-accessor', publisherSource.includes('pub fn publisher(&self) -> &EventPublisher')],
  ['context-complete-uses-payload-accessor', publisherSource.includes('self.payload().request_id()?')],
  ['testkit-clones-envelope-through-accessor', testkitSource.includes('context.envelope().clone()')],
  ['no-handler-mut-payload-api', !eventingSources.some(([, source]) => /&mut\s+E\b/u.test(source))],
  ['no-payload-mut-accessor', !publisherSource.includes('payload_mut')],
  ['no-payload-carried-sender', !requestPayloadSource.includes('Sender')],
  ['no-payload-carried-receiver', !requestPayloadSource.includes('Receiver')],
  ['no-payload-carried-join-handle', !requestPayloadSource.includes('JoinHandle')],
  ['no-payload-carried-callback', !requestPayloadSource.includes('Fn(')],
  [
    'no-payload-carried-resource-handle',
    !requestPayloadSource.includes('Arc<') && !requestPayloadSource.includes('Mutex<'),
  ],
  ['request-registry-retains-local-sender', requestSource.includes('sender: Option<oneshot::Sender<RequestPayload>>')],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing source-safety assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-source-safety',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '66 Ownership, mutation, and interior-mutability guard for handler-facing context',
    '76 No payload-carried deferred, cancellation, handle, or resource source gate',
  ],
  notClaimed: [
    '67 full no-lock-held-await audit',
    '68 TypeScript/Rust branded fixture parity',
    '69 Unity/TypeScript semantics conformance matrix',
    '70 event topology manifest',
    '71 manual clock deterministic proof',
    '72 contract registry generated docs',
    '73 duplicate subscription override',
    '74 shutdown/drain/test-clear lifecycle',
    '75 event-family wrapper proof',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-source-safety-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function sourceFiles(root) {
  const files = [];
  for (const entry of readdirSync(root)) {
    const path = join(root, entry);
    if (statSync(path).isDirectory()) {
      files.push(...sourceFiles(path));
      continue;
    }
    if (path.endsWith('.rs')) {
      files.push(path);
    }
  }
  return files;
}

function sliceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex);
  if (startIndex < 0 || endIndex < 0) {
    throw new Error(`Unable to slice source between ${start} and ${end}`);
  }
  return source.slice(startIndex, endIndex);
}
