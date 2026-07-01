import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10d-remote-delivery-receipt-ledger-status');
const testRoot = join('test-results', 'network-remote-delivery-receipt-ledger-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-receipt-ledger-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime_state.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_receipt_ledger.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_receipt_ledger_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_store.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_journal.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_journal_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
];

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'row10c local network runtime event-chain projection replay records',
    'row10c exportable stored envelopes',
    'row10b broker/family-hub delivery requirement refs',
  ],
  receiptLedgerRefs: [
    'network.remote-delivery.event-chain.receipt-ledger.10d',
    'network.remote-delivery.event-chain.local-receipt-ack.10d',
    'network.remote-delivery.event-chain.receipt-replay.10d',
    'network.remote-delivery.event-chain.receipt-support-status.10d',
  ],
  renderedStates: [
    'receiptLedgerReady=true',
    'receiptReplayReady=true',
    'receiptRecordsMatchProjection=true',
    'receiptRecordCount equals sourceProjectionReplayRecordCount',
    'localReceiptAckCount equals receiptRecordCount',
    'remoteDeliveryAckImplemented=false',
  ],
  parserInvariants: [
    'receipt refs must all cite row10d',
    'receipt records must preserve replay sequence, event id, event type, and correlation id',
    'receipt ledger cannot claim live broker/family-hub delivery',
    'receipt ledger cannot carry raw PCAP, exact content, or adapter-action claims',
  ],
  noClaims: [
    'live broker delivery',
    'live family-hub relay delivery',
    'remote provider delivery',
    'child-device delivery',
    'family-hub delivery acknowledgement implementation',
    'product-ready remote delivery',
    'policy authority',
    'side-effect authority',
    'enforcement command publication in the row10d remote projection fixture',
    'adapter action execution',
    'raw PCAP',
    'exact URL from network-only evidence',
    'decrypted payload',
    'page content',
    'video content',
    'private-message content',
    'search-query content',
    'host filtering',
  ],
};
writeFileSync(
  join(proofRoot, 'expected-remote-delivery-receipt-ledger-status.json'),
  `${JSON.stringify(expectedStatus, null, 2)}\n`
);

const commands = [
  {
    name: 'agent-core-remote-event-chain-journal-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_event_chain_journal'],
    log: join(proofRoot, 'agent-core-remote-event-chain-journal-test.log'),
  },
  {
    name: 'agent-core-remote-delivery-receipt-ledger-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_receipt_ledger'],
    log: join(proofRoot, 'agent-core-remote-delivery-receipt-ledger-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-receipt-ledger-proof/v1',
    'asserted=no exact URL/page/message/search claim from network-only evidence',
    'asserted=no video content, private-message content, or search-query content claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP without custody claim',
    'asserted=no live broker/family-hub delivery claim',
    'asserted=no family-hub delivery acknowledgement implementation claim',
    'asserted=no remote provider or child-device delivery claim',
    'asserted=no product-ready remote delivery claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication in this row10d remote projection fixture',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-receipt-ledger-proof',
  proofRevision: 'network-remote-delivery-receipt-ledger-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-receipt-ledger-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  mergeBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryReceiptLedgerStatus: join(proofRoot, 'expected-remote-delivery-receipt-ledger-status.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10d remote delivery receipt ledger/local ack status',
    'network-plan supplemental row 10c remote delivery event-chain journal/export boundary status',
    'network-plan supplemental row 10b broker/family-hub remote delivery status',
  ],
  provenBoundaries: [
    'agent-core builds a deterministic local receipt ledger from row10c ocentra-eventing projection replay records',
    'receipt records preserve replay sequence, event id, event type, and correlation id for every exportable event-chain envelope',
    'row10d refs mark receipt ledger, local receipt ack, replay, and support-status boundaries that future broker/family-hub delivery can consume',
    'the proof keeps broker delivery, family-hub relay delivery, family-hub delivery acknowledgement implementation, provider delivery, child-device delivery, product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement commands, and host filtering false for this row10d remote projection fixture',
    'the proof keeps raw PCAP, exact URL, decrypted payload, page content, video content, private-message content, and search-query content unavailable from network-only receipt ledger records',
  ],
  notClaimed: [
    'live broker delivery',
    'live family-hub relay delivery',
    'family-hub delivery acknowledgement implementation',
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
    'available-metadata remote no-enforcement invariant',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-remote-delivery-receipt-ledger-proof-ok:core,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreState = readText('crates/agent-core/src/network_event_runtime_state.rs');
  const coreStore = readText('crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_store.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_receipt_ledger.rs');
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_EVENT_CHAIN_RECEIPT_LEDGER_REF'],
    [coreRuntime, 'prove_network_runtime_remote_delivery_receipt_ledger'],
    [coreState, 'raw_pcap_available'],
    [coreState, 'video_content_available'],
    [coreState, 'private_message_content_available'],
    [coreState, 'search_query_available'],
    [coreStore, 'publish_network_runtime_remote_event_chain_store'],
    [coreStore, 'raw_pcap_available_count'],
    [coreStore, 'private_message_content_available_count'],
    [coreProof, 'receipt_records_from_projection'],
    [coreProof, 'remote_delivery_ack_implemented: false'],
    [coreProof, 'search_query_available_count'],
    [coreTests, 'network_runtime_remote_delivery_receipt_ledger_preserves_local_ack_boundary_without_transport'],
    [coreTests, 'remote_delivery_ack_implemented'],
    [coreTests, 'video_content_available_count'],
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
  const normalizedLines = sortSourceShapeWarningLines(
    sortConsecutiveTestLines(
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
    )
  );
  const trimmed = normalizedLines
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return trimmed.length === 0 ? '' : `${trimmed}\n`;
}

function sortSourceShapeWarningLines(lines) {
  const warningHeaderIndex = lines.findIndex((line) => line.startsWith('Source shape warnings:'));
  if (warningHeaderIndex === -1) {
    return lines;
  }
  return [
    ...lines.slice(0, warningHeaderIndex + 1),
    ...lines
      .slice(warningHeaderIndex + 1)
      .filter((line) => line.trim().length > 0)
      .sort(),
  ];
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
