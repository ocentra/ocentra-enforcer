import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10r-remote-delivery-cross-process-replay');
const testRoot = join('test-results', 'network-remote-delivery-cross-process-replay-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-cross-process-replay-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/lib.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime_cross_process_replay_tests.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_custody_readiness.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_custody_readiness_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope_types.rs',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/workpacks/README.md',
];

assertSourceContracts();

const expectedReplay = {
  replayRefs: [
    'network.remote-delivery.cross-process-replay.10r',
    'network.remote-delivery.cross-process-replay-store.10r',
    'network.remote-delivery.cross-process-replay-cursor.10r',
  ],
  provenStates: [
    'crossProcessReplayRecordCount equals sourceDurableEnvelopeCount',
    'crossProcessReplayStoreWriteCount equals crossProcessReplayRecordCount',
    'crossProcessReplayCursorNextSequence advances after the last durable envelope sequence',
    'replay records preserve event id, event type, correlation id, durable envelope refs, receipt refs, and row10q custody refs',
    'crossProcessReplayImplemented is true only for deterministic replay metadata records',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
    'remote acknowledgement delivery',
    'provider delivery',
    'child-device delivery',
    'actual remote delete/export propagation',
    'product-ready remote delivery',
    'policy authority',
    'side-effect authority',
    'adapter action execution',
    'enforcement command publication',
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
writeJson(join(proofRoot, 'expected-cross-process-replay.json'), expectedReplay);

const commands = [
  {
    name: 'agent-core-cross-process-replay-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'cross_process_replay'],
    log: join(proofRoot, 'agent-core-cross-process-replay-test.log'),
  },
  {
    name: 'agent-core-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-parent-agent-core', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'agent-core-clippy.log'),
  },
  {
    name: 'rust-format',
    command: 'cargo',
    args: ['fmt', '--all', '--check'],
    log: join(proofRoot, 'rust-format.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
  {
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check'],
    log: join(proofRoot, 'git-diff-check.log'),
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
    'checkedAt=deterministic:network-remote-delivery-cross-process-replay-proof/v1',
    'asserted=row10r replay records preserve durable envelope refs and row10q custody refs',
    'asserted=cross-process replay is implemented only as deterministic durable replay metadata',
    'asserted=no live broker/family-hub/provider/child-device delivery claim',
    'asserted=no actual remote delete/export propagation claim',
    'asserted=no product-ready delivery, policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-cross-process-replay-proof',
  proofRevision: 'network-remote-delivery-cross-process-replay-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-cross-process-replay-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  runContext: 'deterministic-row10r-cross-process-durable-replay',
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedCrossProcessReplay: join(proofRoot, 'expected-cross-process-replay.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10r remote delivery cross-process durable replay metadata proof',
    'network-plan supplemental row 10q remote delivery cross-process custody readiness proof',
    'network-plan supplemental row 10e remote delivery durable envelope proof',
  ],
  provenBoundaries: [
    'Rust core creates deterministic row10r cross-process replay records from row10e durable envelopes and row10q custody readiness records',
    'row10r replay records preserve sequence, event id, event type, correlation id, durable envelope/store refs, receipt refs, row10q custody status refs, and replay cursor refs',
    'source readiness that already claims cross-process replay is rejected before row10r records are built',
    'mismatched durable envelope and custody readiness inputs are rejected',
    'row10r keeps live broker/family-hub/provider/child delivery, actual remote delete/export propagation, product-ready delivery, policy authority, side-effect authority, adapter action, host filtering, exact content, raw PCAP, and enforcement-command publication unclaimed',
  ],
  notClaimed: expectedReplay.noClaims,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('network-remote-delivery-cross-process-replay-proof-ok:core,clippy,fmt,source-shape,diff-check');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreLib = readText('crates/agent-core/src/lib.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay.rs');
  const coreProofTests = readText('crates/agent-core/tests/unit/network_event_runtime_cross_process_replay_tests.rs');
  const coreTypes = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay_types.rs'
  );
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_REPLAY_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_REPLAY_STORE_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_REPLAY_CURSOR_REF'],
    [protocolConstants, 'ERROR_NETWORK_RUNTIME_REMOTE_CROSS_PROCESS_REPLAY'],
    [coreRuntime, 'remote_delivery_cross_process_replay'],
    [coreLib, 'prove_network_runtime_remote_delivery_cross_process_replay'],
    [coreLib, 'network_event_runtime_cross_process_replay_tests'],
    [coreProof, 'prove_network_runtime_remote_delivery_cross_process_replay'],
    [coreProof, 'cross_process_replay_records_match_durable_envelopes: true'],
    [coreProofTests, 'rejects_source_readiness_that_already_claims_cross_process_replay'],
    [coreProofTests, 'assert_cross_process_replay_no_delivery_or_enforcement_claims'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryCrossProcessReplayReport'],
    [coreTypes, 'DurableReplayRecorded'],
    [featureDoc, 'row10r cross-process durable replay metadata'],
    [checklist, '10r-remote-delivery-cross-process-replay'],
    [workpacks, '10r'],
  ];
  for (const [haystack, needle] of requiredSnippets) {
    assertIncludes(haystack, needle, `source contract snippet ${needle}`);
  }
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, normalizeCommandLog(entry.name, `${result.stdout ?? ''}${result.stderr ?? ''}`));
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

function sourceFingerprint() {
  const hash = createHash('sha256');
  for (const filePath of sourceFiles.filter((filePath) => !filePath.startsWith('scripts/test/'))) {
    hash.update(filePath);
    hash.update('\0');
    hash.update(readText(filePath));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function normalizeCommandLog(commandName, log) {
  const normalized = log.replaceAll(process.cwd(), '<repo>').replaceAll('\r\n', '\n').trimEnd();
  if (normalized.length === 0) {
    return `${commandName}\n`;
  }
  return `${commandName}\n${normalized}\n`;
}

function writeJson(filePath, value) {
  writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function readText(filePath) {
  return readFileSync(filePath, 'utf8');
}

function assertIncludes(haystack, needle, label) {
  if (!haystack.includes(needle)) {
    throw new Error(`${label}: missing ${needle}`);
  }
}
