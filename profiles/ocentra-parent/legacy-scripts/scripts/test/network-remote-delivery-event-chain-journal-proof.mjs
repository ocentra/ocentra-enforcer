import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10c-remote-delivery-event-chain-journal-status');
const testRoot = join('test-results', 'network-remote-delivery-event-chain-journal-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'local network runtime event-chain payloads',
    'row10b broker/family-hub delivery requirement refs',
    'ocentra-eventing NDJSON journal and projection replay records',
  ],
  eventChainJournalRefs: [
    'network.remote-delivery.event-chain-journal.10c',
    'network.remote-delivery.event-chain-replay.10c',
    'network.remote-delivery.event-chain-export.10c',
    'network.remote-delivery.event-chain.support-status.10c',
  ],
  renderedStates: [
    'projectionReplayMode=ProjectionOnly',
    'journalEntryCount equals replayRecordCount',
    'exportableRemoteEnvelopeCount equals journalEntryCount',
    'unavailableEventCount equals journalEntryCount',
  ],
  parserInvariants: [
    'event-chain journal refs must all cite row10c',
    'projection replay cannot dispatch action handlers',
    'event-chain journal/export boundary cannot carry live broker/family-hub delivery claims',
    'event-chain journal/export boundary cannot carry exact content or adapter-action claims',
  ],
  noClaims: [
    'live broker delivery',
    'live family-hub relay delivery',
    'remote provider delivery',
    'child-device delivery',
    'product-ready remote delivery',
    'policy authority',
    'side-effect authority',
    'enforcement command publication',
    'adapter action execution',
    'exact URL from network-only evidence',
    'decrypted payload',
    'page content',
    'host filtering',
  ],
};
writeFileSync(
  join(proofRoot, 'expected-remote-delivery-event-chain-journal-status.json'),
  `${JSON.stringify(expectedStatus, null, 2)}\n`
);

const commands = [
  {
    name: 'eventing-journal-replay-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'journal_replay'],
    log: join(proofRoot, 'eventing-journal-replay-test.log'),
  },
  {
    name: 'agent-core-remote-delivery-rust-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_status'],
    log: join(proofRoot, 'agent-core-remote-delivery-rust-test.log'),
  },
  {
    name: 'agent-core-remote-event-chain-journal-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_event_chain_journal'],
    log: join(proofRoot, 'agent-core-remote-event-chain-journal-test.log'),
  },
  {
    name: 'agent-core-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-parent-agent-core', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'agent-core-clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
];

const commandResults = commands.map(runCommand);

const validationLogPath = join(proofRoot, '12-validation-commands.log');
writeFileSync(
  validationLogPath,
  commandResults.map((entry) => `${entry.command} -> ${entry.status}`).join('\n') + '\n'
);

const securityLogPath = join(proofRoot, '09-security-negative-proof.log');
writeFileSync(
  securityLogPath,
  [
    'checkedAt=deterministic:network-remote-delivery-event-chain-journal-proof/v1',
    'asserted=no exact URL/page/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP without custody claim',
    'asserted=no live broker/family-hub delivery claim',
    'asserted=no remote provider or child-device delivery claim',
    'asserted=no product-ready remote delivery claim',
    'asserted=no action replay, policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication claim',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-event-chain-journal-proof',
  proofRevision: 'network-remote-delivery-event-chain-journal-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-event-chain-journal-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  mergeBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryEventChainJournalStatus: join(
      proofRoot,
      'expected-remote-delivery-event-chain-journal-status.json'
    ),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10c remote delivery event-chain journal/export boundary status',
    'network-plan supplemental row 10b broker/family-hub remote delivery status',
    'network-plan workpack 45 eventing delivery-decision and journal/replay consumption',
  ],
  provenBoundaries: [
    'agent-core materializes local network runtime event-chain payloads through ocentra-eventing NDJSON journal records',
    'projection replay reads the remote-delivery event-chain journal as exportable stored envelopes without action-handler dispatch',
    'row10c refs mark journal, replay, export, and support-status boundaries that future broker/family-hub transport can consume',
    'the proof keeps provider delivery, child-device delivery, product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement commands, and host filtering false',
    'the proof keeps exact URL, decrypted payload, and page content unavailable from network-only event-chain envelopes',
  ],
  notClaimed: [
    'live broker delivery',
    'live family-hub relay delivery',
    'remote provider delivery',
    'child-device delivery',
    'product-ready remote delivery',
    'cross-process transport implementation',
    'remote retention/delete/export propagation implementation',
    'policy authority',
    'side-effect authority',
    'adapter execution',
    'host filtering',
    'full network-plan completion',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-remote-delivery-event-chain-journal-proof-ok:eventing-journal,core,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_journal.rs');
  const coreStore = readText('crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_store.rs');
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_EVENT_CHAIN_JOURNAL_REF'],
    [coreRuntime, 'prove_network_runtime_remote_event_chain_journal'],
    [coreProof, 'projection_replay_mode: projection.mode'],
    [coreStore, 'NdjsonEventJournal::with_options'],
    [coreStore, 'JournalPolicy::after_dispatch(JournalSelector::All)'],
    [coreTests, 'network_runtime_remote_event_chain_journal_preserves_export_boundary_without_transport'],
    [coreTests, 'broker_delivery_implemented'],
  ];
  for (const [haystack, needle] of requiredSnippets) {
    assertIncludes(haystack, needle, `source contract snippet ${needle}`);
  }
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, normalizeLogText(`${result.stdout ?? ''}${result.stderr ?? ''}`));
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function sourceFingerprint() {
  const sourceFiles = [
    'scripts/test/network-remote-delivery-event-chain-journal-proof.mjs',
    'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_journal.rs',
    'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_journal_types.rs',
    'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_store.rs',
    'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
    'crates/agent-core/src/network_event_runtime/remote_delivery_status.rs',
    'crates/agent-core/src/network_event_runtime/broker_delivery.rs',
    'crates/ocentra-eventing/src/journal.rs',
    'crates/ocentra-eventing/src/tests/journal_replay.rs',
  ];
  const hash = createHash('sha256');
  for (const filePath of sourceFiles) {
    hash.update(filePath);
    hash.update('\0');
    hash.update(readText(filePath));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function mergeBase() {
  return runText('git', ['merge-base', 'HEAD', 'origin/main']).trim();
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function normalizeLogText(text) {
  const normalizedLines = sortConsecutiveTestLines(
    text
      .replace(/\r\n/g, '\n')
      .split('\n')
      .filter((line) => !line.includes('Blocking waiting for'))
      .filter((line) => !line.trimStart().startsWith('Compiling '))
      .filter((line) => !line.trimStart().startsWith('Checking '))
      .map((line) =>
        line
          .replace(/finished in [0-9.]+s/g, 'finished in <duration>')
          .replace(/target\(s\) in [0-9.]+s/g, 'target(s) in <duration>')
      )
  );
  const trimmed = normalizedLines
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return trimmed.length === 0 ? '' : `${trimmed}\n`;
}

function sortConsecutiveTestLines(lines) {
  const sortedLines = [];
  let testLineBuffer = [];
  for (const line of lines) {
    if (line.startsWith('test ') && line.endsWith(' ... ok')) {
      testLineBuffer.push(line);
      continue;
    }
    if (testLineBuffer.length > 0) {
      sortedLines.push(...testLineBuffer.sort());
      testLineBuffer = [];
    }
    sortedLines.push(line);
  }
  if (testLineBuffer.length > 0) {
    sortedLines.push(...testLineBuffer.sort());
  }
  return sortedLines;
}
