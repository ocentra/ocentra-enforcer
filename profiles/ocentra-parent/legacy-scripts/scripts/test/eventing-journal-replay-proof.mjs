import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const proofRoot = join('output', 'eventing-plan-proof', '36-41-journal-replay');
mkdirSync(proofRoot, { recursive: true });

const commands = [
  {
    name: 'eventing-journal-replay-tests',
    command: 'cargo',
    args: ['test', '--quiet', '-p', 'ocentra-eventing', 'journal_replay'],
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

const journalSource = readFileSync('crates/ocentra-eventing/src/journal.rs', 'utf8');
const journalPolicySource = readFileSync('crates/ocentra-eventing/src/journal/policy.rs', 'utf8');
const ndjsonSource = readFileSync('crates/ocentra-eventing/src/journal/ndjson.rs', 'utf8');
const hashChainSource = readFileSync('crates/ocentra-eventing/src/journal/hash_chain.rs', 'utf8');
const replaySource = readFileSync('crates/ocentra-eventing/src/replay.rs', 'utf8');
const busSource = readFileSync('crates/ocentra-eventing/src/bus/journaling.rs', 'utf8');
const publishSource = readFileSync('crates/ocentra-eventing/src/bus/publish.rs', 'utf8');
const libSource = readFileSync('crates/ocentra-eventing/src/lib.rs', 'utf8');
const journalTests = [
  'crates/ocentra-eventing/tests/journal_replay.rs',
  'crates/ocentra-eventing/tests/journal_replay/file.rs',
  'crates/ocentra-eventing/tests/journal_replay/bus_policy.rs',
  'crates/ocentra-eventing/tests/journal_replay/replay.rs',
  'crates/ocentra-eventing/tests/journal_replay/fixtures.rs',
  'crates/ocentra-eventing/tests/journal_replay/support.rs',
]
  .map((path) => readFileSync(path, 'utf8'))
  .join('\n');

const sourceAssertions = [
  ['event-journal-trait', journalSource.includes('pub trait EventJournal')],
  ['ndjson-journal', ndjsonSource.includes('pub struct NdjsonEventJournal')],
  ['tokio-file-io', ndjsonSource.includes('tokio::') && ndjsonSource.includes('OpenOptions')],
  ['one-object-per-line', ndjsonSource.includes("line.push(b'\\n')")],
  ['hash-chain-enabled', ndjsonSource.includes('JournalHashChain::Enabled')],
  ['stable-sha256-hash-chain', hashChainSource.includes('Sha256::digest')],
  ['hash-chain-recovery-verification', ndjsonSource.includes('verify_hash_chain_entry')],
  ['hash-chain-replay-verification', replaySource.includes('verify_hash_chain_entry')],
  ['journal-selector-types', journalPolicySource.includes('EventTypes') && journalPolicySource.includes('Namespaces')],
  ['journal-allowlist', journalPolicySource.includes('ContractAllowlist')],
  ['journal-policy-modes', journalPolicySource.includes('BeforeAndAfterDispatch')],
  ['bus-before-dispatch-hook', publishSource.includes('JournalDispatchPhase::BeforeDispatch')],
  ['bus-after-dispatch-hook', publishSource.includes('JournalDispatchPhase::AfterDispatch')],
  ['replay-cursor', replaySource.includes('pub struct ReplayCursor')],
  ['replay-filter', replaySource.includes('pub struct ReplayFilter')],
  ['projection-only-mode', replaySource.includes('ProjectionOnly')],
  ['action-mode-explicit', replaySource.includes('ActionHandlersAllowed')],
  ['projection-safety-gate', busSource.includes('ReplayActionNotAllowed')],
  ['public-exports', libSource.includes('EventJournal') && libSource.includes('ReplayFilter')],
  ['ndjson-test', journalTests.includes('ndjson_journal_appends_one_object_per_line_with_hash_chain')],
  ['selector-test', journalTests.includes('bus_journal_policy_honors_before_after_and_selected_journaling')],
  ['replay-filter-test', journalTests.includes('replay_cursor_and_filters_read_ordered_projection_records')],
  ['corrupt-line-test', journalTests.includes('replay_corrupt_line_is_reported_explicitly')],
  ['tamper-replay-test', journalTests.includes('replay_rejects_tampered_hash_chain_payload')],
  ['tamper-recovery-test', journalTests.includes('ndjson_journal_reopen_rejects_tampered_hash_chain_payload')],
  ['projection-gate-test', journalTests.includes('projection_replay_cannot_run_handlers_without_action_mode')],
];

const failedAssertion = sourceAssertions.find(([, passed]) => !passed);
if (failedAssertion) {
  throw new Error(`eventing journal/replay assertion failed: ${failedAssertion[0]}`);
}

const proof = {
  proof: 'eventing-journal-replay',
  proofRoot,
  checkedAt: new Date().toISOString(),
  commands: commandResults,
  assertions: sourceAssertions.map(([name]) => name),
  provenRows: [
    '36 EventJournal trait',
    '37 NDJSON append implementation',
    '38 stable hash-chain journal option with recovery/replay tamper verification',
    '39 replay cursor and filters',
    '40 projection-only replay safety gate',
    '41 journal-before/after dispatch modes',
    '77 selected journaling by event type, namespace/family, and allowlist',
  ],
  notClaimed: [
    '42-62 Parent/controller/child-agent event contracts and runtime boundaries',
    '66-67 ownership/interior-mutability and lock-held-await source gates',
    '71 manual clock deterministic timeout proof',
    '72 contract registry generated documentation',
    '73 duplicate subscription policy override',
    '74 shutdown/drain/test-clear lifecycle',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`eventing-journal-replay-proof-ok:${proof.assertions.join(',')}`);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);
