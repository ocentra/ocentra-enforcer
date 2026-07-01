import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10e-remote-delivery-durable-envelope-status');
const testRoot = join('test-results', 'network-remote-delivery-durable-envelope-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-durable-envelope-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime_state.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_receipt_ledger.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_receipt_ledger_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_event_chain_store.rs',
  'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
];

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'row10d local receipt ledger records',
    'row10d local receipt acknowledgement refs',
    'row10c event-chain projection replay records',
  ],
  durableEnvelopeRefs: [
    'network.remote-delivery.durable-envelope.10e',
    'network.remote-delivery.durable-envelope-store.10e',
    'network.remote-delivery.durable-envelope-replay.10e',
    'network.remote-delivery.durable-envelope-delete-export.10e',
    'network.remote-delivery.durable-envelope-support-status.10e',
  ],
  renderedStates: [
    'durableStoreReady=true',
    'durableReplayReady=true',
    'deleteExportReadinessRecorded=true',
    'durableRecordsMatchReceipts=true',
    'durableEnvelopeCount equals sourceReceiptRecordCount',
    'remoteDeleteExportPropagationImplemented=false',
    'productReadyRemoteDelivery=false',
  ],
  parserInvariants: [
    'durable records must preserve receipt sequence, event id, event type, and correlation id',
    'durable records must cite row10e durable envelope/store/delete-export refs',
    'durable envelope proof cannot claim live broker/family-hub delivery',
    'durable envelope proof cannot carry raw PCAP, exact content, or adapter-action claims',
  ],
  noClaims: [
    'live broker delivery',
    'live family-hub relay delivery',
    'remote provider delivery',
    'child-device delivery',
    'remote delivery acknowledgement implementation',
    'remote delete/export propagation implementation',
    'product-ready remote delivery',
    'policy authority',
    'side-effect authority',
    'enforcement command publication',
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
  join(proofRoot, 'expected-remote-delivery-durable-envelope-status.json'),
  `${JSON.stringify(expectedStatus, null, 2)}\n`
);

const commands = [
  {
    name: 'agent-core-remote-delivery-durable-envelope-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_durable_envelope'],
    log: join(proofRoot, 'agent-core-remote-delivery-durable-envelope-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-durable-envelope-proof/v1',
    'asserted=no exact URL/page/message/search claim from network-only evidence',
    'asserted=no video content, private-message content, or search-query content claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP without custody claim',
    'asserted=no live broker/family-hub delivery claim',
    'asserted=no remote acknowledgement implementation claim',
    'asserted=no remote provider or child-device delivery claim',
    'asserted=no remote delete/export propagation implementation claim',
    'asserted=no product-ready remote delivery claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-durable-envelope-proof',
  proofRevision: 'network-remote-delivery-durable-envelope-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-durable-envelope-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  mergeBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryDurableEnvelopeStatus: join(
      proofRoot,
      'expected-remote-delivery-durable-envelope-status.json'
    ),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10e remote delivery durable envelope/store status',
    'network-plan supplemental row 10d remote delivery receipt ledger/local ack status',
    'network-plan supplemental row 10c remote delivery event-chain journal/export boundary status',
  ],
  provenBoundaries: [
    'agent-core builds deterministic durable envelope records from row10d local receipt ledger records',
    'durable records preserve receipt sequence, event id, event type, and correlation id for every source receipt',
    'row10e refs mark durable envelope, durable store, replay, delete/export readiness, and support-status boundaries that future broker/family-hub delivery can consume',
    'the proof keeps broker delivery, family-hub relay delivery, remote acknowledgement implementation, provider delivery, child-device delivery, remote delete/export propagation, product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement commands, and host filtering false',
    'the proof keeps raw PCAP, exact URL, decrypted payload, page content, video content, private-message content, and search-query content unavailable from network-only durable envelope records',
  ],
  notClaimed: [
    'live broker delivery',
    'live family-hub relay delivery',
    'remote acknowledgement implementation',
    'remote provider delivery',
    'child-device delivery',
    'remote delete/export propagation implementation',
    'product-ready remote delivery',
    'cross-process transport implementation',
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
console.log('network-remote-delivery-durable-envelope-proof-ok:core,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreState = readText('crates/agent-core/src/network_event_runtime_state.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope.rs');
  const coreTypes = readText('crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope_types.rs');
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_DURABLE_ENVELOPE_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_DURABLE_DELETE_EXPORT_REF'],
    [coreRuntime, 'prove_network_runtime_remote_delivery_durable_envelope'],
    [coreState, 'raw_pcap_available'],
    [coreState, 'video_content_available'],
    [coreState, 'private_message_content_available'],
    [coreState, 'search_query_available'],
    [coreProof, 'durable_records_from_receipts'],
    [coreProof, 'remote_delete_export_propagation_implemented: false'],
    [coreProof, 'product_ready_remote_delivery: false'],
    [coreProof, 'search_query_available_count'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryDurableEnvelopeReport'],
    [coreTests, 'network_runtime_remote_delivery_durable_envelope_preserves_receipt_refs_without_transport'],
    [coreTests, 'remote_delete_export_propagation_implemented'],
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
