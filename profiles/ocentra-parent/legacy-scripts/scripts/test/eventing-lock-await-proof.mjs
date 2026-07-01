import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '67-lock-await');
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

const productionSources = sourceFiles('crates/ocentra-eventing/src')
  .filter((path) => !path.includes(`${join('src', 'tests')}`))
  .map((path) => [path, readFileSync(path, 'utf8')]);
const sourceByPath = new Map(productionSources);
const ndjsonSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'journal', 'ndjson.rs'));
const publishSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'bus', 'publish.rs'));
const aggregateGateSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'bus', 'aggregate_gate.rs'));
const busSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'bus.rs'));
const queueSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'queue', 'state.rs'));
const requestSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'request.rs'));
const subscriberSource = sourceByPath.get(join('crates', 'ocentra-eventing', 'src', 'bus', 'subscriber.rs'));
const insertSubscriberBlock = sliceBetween(subscriberSource, 'pub(super) fn insert_subscriber', 'fn remove_subscriber');
const removeSubscriberBlock = subscriberSource.slice(subscriberSource.indexOf('fn remove_subscriber'));

const sourceAssertions = [
  ['production-source-has-no-async-mutex-lock-await', noProductionSourceIncludes('.lock().await')],
  ['journal-state-uses-sync-lock', ndjsonSource.includes('sync::{Arc, Mutex}')],
  ['journal-append-has-semaphore-gate', ndjsonSource.includes('append_gate: Arc<Semaphore>')],
  [
    'journal-reserves-state-before-await',
    /let append = \{\s*let (?:mut )?state = self\.state\.lock\(\)/s.test(ndjsonSource),
  ],
  [
    'journal-writes-after-state-block',
    /let append = \{[\s\S]*?\n        \};\s*self\.write_entry\(&append, envelope, phase\)\.await\?;/s.test(
      ndjsonSource
    ),
  ],
  [
    'journal-commits-state-after-file-await',
    /self\.write_entry\(&append, envelope, phase\)\.await\?;[\s\S]*previous_hash = append\.current_hash\.clone\(\);/s.test(
      ndjsonSource
    ),
  ],
  [
    'aggregate-gate-uses-semaphore-map',
    busSource.includes('aggregate_gates: Arc<Mutex<BTreeMap<AggregateKey, Arc<Semaphore>>>>'),
  ],
  [
    'aggregate-order-uses-owned-semaphore-permit',
    publishSource.includes('acquire_owned()') && publishSource.includes('aggregate ordering gate remains open'),
  ],
  [
    'aggregate-order-has-no-async-mutex',
    !publishSource.includes('AsyncMutex') &&
      !aggregateGateSource.includes('AsyncMutex') &&
      !busSource.includes('AsyncMutex'),
  ],
  ['queue-state-remains-sync-only', !queueSource.includes('async fn') && !queueSource.includes('.await')],
  ['request-registry-remains-sync-only', !requestSource.includes('async fn') && !requestSource.includes('.await')],
  ['subscriber-insert-registry-has-no-await', !insertSubscriberBlock.includes('.await')],
  ['subscriber-remove-registry-has-no-await', !removeSubscriberBlock.includes('.await')],
  [
    'publisher-clones-subscribers-before-dispatch-await',
    publishSource.includes('let subscribers = self.subscribers_for(&stored);'),
  ],
  [
    'aggregate-map-lock-only-returns-gate',
    /fn aggregate_gate[\s\S]*Arc::clone\([\s\S]*or_insert_with\(\|\| Arc::new\(Semaphore::new\(1\)\)\)/s.test(
      aggregateGateSource
    ),
  ],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing lock-await assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-lock-await',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '67 Borrow/await and no lock-held-await source audit for registry, queue, request, journal state, and aggregate ordering',
  ],
  explicitSemaphores: [
    'NdjsonEventJournal.append_gate serializes append file writes without holding journal state mutex guards across await',
    'EventBus.aggregate_gates serialize aggregate dispatch without AsyncMutex guards across handler awaits',
  ],
  notClaimed: [
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
console.log(`eventing-lock-await-proof-ok:${proof.assertions.join(',')}`);
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

function noProductionSourceIncludes(pattern) {
  return productionSources.every(([, source]) => !source.includes(pattern));
}

function sliceBetween(source, start, end) {
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end, startIndex);
  if (startIndex < 0 || endIndex < 0) {
    throw new Error(`Unable to slice source between ${start} and ${end}`);
  }
  return source.slice(startIndex, endIndex);
}
