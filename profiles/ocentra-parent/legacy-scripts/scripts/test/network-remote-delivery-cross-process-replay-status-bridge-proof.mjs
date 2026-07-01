import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10s-remote-delivery-cross-process-replay-status-bridge');
const testRoot = join('test-results', 'network-remote-delivery-cross-process-replay-status-bridge-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-cross-process-replay-status-bridge-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-protocol/src/network_flow.rs',
  'crates/agent-protocol/tests/contract/network_flow_tests.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime_cross_process_replay_tests.rs',
  'crates/agent-service/src/network_remote_delivery_status_cross_process.rs',
  'crates/agent-service/src/network_remote_delivery_status_payload.rs',
  'crates/agent-service/tests/unit/network_remote_delivery_status_service_tests.rs',
  'packages/schema-domain/src/agent-protocol-defaults.ts',
  'packages/schema-domain/src/network-remote-delivery-status.ts',
  'packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts',
  'packages/agent-protocol-domain/tests/unit/generated-agent-protocol-contracts.test.ts',
  'crates/agent-protocol/README.md',
  'crates/agent-service/README.md',
  'packages/agent-protocol-domain/README.md',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/workpacks/README.md',
];
const sourceShapeFiles = sourceFiles.filter(
  (sourceFile) =>
    !sourceFile.startsWith('docs/') &&
    !sourceFile.endsWith('/README.md') &&
    !sourceFile.includes('/generated/')
);

assertSourceContracts();

const expectedStatus = {
  statusCommandAndEvent: {
    command: 'agent.network.remote-delivery.status.get',
    event: 'agent.network.remote-delivery.status.reported',
    payloadField: 'networkRemoteDeliveryStatus',
  },
  currentStatusRef: 'network.remote-delivery.external-cross-process-transport-status.10t',
  replayStatusBridgeRef: 'network.remote-delivery.cross-process-replay-status.10s',
  replayMetadataRefs: [
    'network.remote-delivery.cross-process-replay.10r',
    'network.remote-delivery.cross-process-replay-store.10r',
    'network.remote-delivery.cross-process-replay-cursor.10r',
  ],
  provenStates: [
    'crossProcessReplayRecordCount equals outboxCandidateCount',
    'crossProcessReplayStoreWriteCount equals crossProcessReplayRecordCount',
    'crossProcessReplayCursorNextSequence equals crossProcessReplayRecordCount plus one',
    'crossProcessReplayRecordsMatchDurableEnvelopes=true',
    'crossProcessReplayRecordsMatchCustodyReadiness=true',
    'crossProcessReplayImplemented=true for deterministic row10r replay metadata only',
    'externalCrossProcessTransportImplemented is owned by row10t external transport proof',
    'dispatchAttemptCount and remoteAckCount remain zero for live transport',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
    'remote acknowledgement delivery',
    'remote provider delivery',
    'child-device delivery',
    'actual remote delete/export propagation',
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
writeJson(join(proofRoot, 'expected-cross-process-replay-status.json'), expectedStatus);

const commands = [
  {
    name: 'agent-core-cross-process-replay-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'cross_process_replay'],
    log: join(proofRoot, 'agent-core-cross-process-replay-test.log'),
  },
  {
    name: 'agent-protocol-remote-delivery-status-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'network_remote_delivery_status'],
    log: join(proofRoot, 'agent-protocol-remote-delivery-status-test.log'),
  },
  {
    name: 'agent-service-remote-delivery-status-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'network_remote_delivery_status'],
    log: join(proofRoot, 'agent-service-remote-delivery-status-test.log'),
  },
  {
    name: 'agent-protocol-domain-build',
    command: 'cmd',
    args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/agent-protocol-domain'],
    log: join(proofRoot, 'agent-protocol-domain-build.log'),
  },
  {
    name: 'agent-protocol-domain-generated-network-contract-tests',
    command: 'cmd',
    args: [
      '/c',
      'npm',
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/agent-protocol-domain',
      '--',
      'generated-agent-protocol-contracts.test.ts',
    ],
    log: join(proofRoot, 'agent-protocol-domain-generated-network-contract-tests.log'),
  },
  {
    name: 'agent-service-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-parent-agent-service', '--lib', '--no-deps', '--', '-D', 'warnings'],
    log: join(proofRoot, 'agent-service-clippy.log'),
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
    args: ['scripts/check-source-shape.mjs', '--files', ...sourceShapeFiles],
    log: join(proofRoot, 'source-shape.log'),
  },
  {
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check', '--', ...sourceFiles],
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
    'checkedAt=deterministic:network-remote-delivery-cross-process-replay-status-bridge-proof/v1',
    'asserted=remote delivery status carries row10r replay/store/cursor refs inside the current row10t status shape',
    'asserted=row10r replay record/store/cursor counts match the row10g outbox candidate count',
    'asserted=row10r replay records match durable envelopes and row10q custody readiness inputs',
    'asserted=crossProcessReplayImplemented is true only for deterministic replay metadata visibility',
    'asserted=external cross-process transport implementation belongs to row10t status proof',
    'asserted=live dispatchAttemptCount and remoteAckCount remain zero',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
    'asserted=no live broker/family-hub/provider/child-device delivery claim',
    'asserted=no actual remote delete/export propagation claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-cross-process-replay-status-bridge-proof',
  proofRevision: 'network-remote-delivery-cross-process-replay-status-bridge-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-cross-process-replay-status-bridge-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedCrossProcessReplayStatus: join(proofRoot, 'expected-cross-process-replay-status.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10s remote delivery cross-process replay status bridge proof',
    'network-plan supplemental row 10r remote delivery cross-process durable replay metadata proof',
    'network-plan supplemental row 10q remote delivery cross-process custody readiness proof',
  ],
  provenBoundaries: [
    'Rust protocol serializes row10r replay refs, counts, and match flags inside the current row10t remote delivery status shape',
    'agent-service builds the status snapshot from the row10r deterministic replay proof instead of inventing an external transport path',
    'Rust-generated TypeScript schema rejects stale row10t refs, stale row10r replay refs, mismatched replay/store/cursor counts, missing match flags, and invalid external transport claims',
    'row10s keeps deterministic replay metadata visibility separate from the row10t external cross-process transport envelope/ack proof',
    'the status bridge rejects product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement command publication, live broker/family-hub delivery, provider delivery, child-device delivery, exact-content, and host-filter claims',
  ],
  notClaimed: expectedStatus.noClaims,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log(
  'network-remote-delivery-cross-process-replay-status-bridge-proof-ok:core,protocol,service,ts,clippy,fmt,source-shape,diff-check'
);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const protocolShape = readText('crates/agent-protocol/src/network_flow.rs');
  const protocolTests = readText('crates/agent-protocol/tests/contract/network_flow_tests.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay.rs');
  const coreProofTests = readText('crates/agent-core/tests/unit/network_event_runtime_cross_process_replay_tests.rs');
  const serviceCrossProcess = readText('crates/agent-service/src/network_remote_delivery_status_cross_process.rs');
  const servicePayload = readText('crates/agent-service/src/network_remote_delivery_status_payload.rs');
  const serviceTests = readText('crates/agent-service/tests/unit/network_remote_delivery_status_service_tests.rs');
  const tsDefaults = readText('packages/schema-domain/src/agent-protocol-defaults.ts');
  const schemaStatus = readText('packages/schema-domain/src/network-remote-delivery-status.ts');
  const generatedContracts = readText('packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts');
  const generatedContractTests = readText('packages/agent-protocol-domain/tests/unit/generated-agent-protocol-contracts.test.ts');
  const protocolReadme = readText('crates/agent-protocol/README.md');
  const serviceReadme = readText('crates/agent-service/README.md');
  const tsReadme = readText('packages/agent-protocol-domain/README.md');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_REPLAY_STATUS_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_REPLAY_REF'],
    [protocolConstants, 'ERROR_NETWORK_RUNTIME_REMOTE_CROSS_PROCESS_REPLAY_STATUS_BRIDGE'],
    [protocolShape, 'cross_process_replay_record_count'],
    [protocolShape, 'external_cross_process_transport_implemented'],
    [protocolTests, 'network_remote_delivery_status_serializes_row10t_external_transport_status'],
    [protocolTests, 'externalCrossProcessTransportImplemented'],
    [coreProof, 'prove_network_runtime_remote_delivery_cross_process_replay'],
    [coreProofTests, 'assert_cross_process_replay_no_delivery_or_enforcement_claims'],
    [servicePayload, 'external_cross_process_transport.cross_process_replay'],
    [serviceCrossProcess, 'apply_cross_process_replay_status'],
    [serviceCrossProcess, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_REPLAY_STATUS_REF'],
    [servicePayload, 'prove_network_runtime_remote_delivery_external_cross_process_transport'],
    [serviceTests, 'assert_remote_delivery_cross_process_replay_status'],
    [tsDefaults, 'CrossProcessReplayRef'],
    [generatedContracts, 'ParentAgentNetworkRemoteDeliveryStatusSchema'],
    [schemaStatus, 'AgentNetworkRemoteDeliveryRow10tRefs'],
    [generatedContractTests, 'crossProcessReplayRef'],
    [schemaStatus, 'externalCrossProcessTransportImplemented: Schema.Literal(true)'],
    [generatedContractTests, 'accepts row10 remote delivery status through the generated schema'],
    [generatedContractTests, 'crossProcessReplayRecordCount'],
    [protocolReadme, 'row10s'],
    [protocolReadme, 'cross-process replay status'],
    [serviceReadme, 'row10s'],
    [serviceReadme, 'cross-process replay status bridge'],
    [tsReadme, 'Generated/thin protocol adapters'],
    [tsReadme, 'Rust-owned shapes'],
    [featureDoc, 'row10s cross-process replay status bridge'],
    [checklist, '10s-remote-delivery-cross-process-replay-status-bridge'],
    [workpacks, 'WORKPACK_INDEX.md'],
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
