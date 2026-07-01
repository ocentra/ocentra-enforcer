import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10t-remote-delivery-external-cross-process-transport');
const testRoot = join('test-results', 'network-remote-delivery-external-cross-process-transport-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-external-cross-process-transport-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-protocol/src/network_flow.rs',
  'crates/agent-protocol/tests/contract/network_flow_tests.rs',
  'crates/agent-core/src/lib.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_external_cross_process_transport.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_external_cross_process_transport_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime/remote_delivery_external_cross_process_transport_tests.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_replay_types.rs',
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

const expectedTransport = {
  statusCommandAndEvent: {
    command: 'agent.network.remote-delivery.status.get',
    event: 'agent.network.remote-delivery.status.reported',
    payloadField: 'networkRemoteDeliveryStatus',
  },
  statusBridgeRef: 'network.remote-delivery.external-cross-process-transport-status.10t',
  transportRefs: [
    'network.remote-delivery.external-cross-process-transport.10t',
    'network.remote-delivery.external-cross-process-transport-envelope.10t',
    'network.remote-delivery.external-cross-process-transport-ack.10t',
  ],
  provenStates: [
    'externalCrossProcessTransportRecordCount equals crossProcessReplayRecordCount',
    'externalCrossProcessTransportEnvelopeCount equals externalCrossProcessTransportRecordCount',
    'externalCrossProcessTransportAckCount equals externalCrossProcessTransportRecordCount',
    'externalCrossProcessTransportRecordsMatchReplayRecords=true',
    'externalCrossProcessTransportAckRecordsMatchEnvelopes=true',
    'externalCrossProcessTransportImplemented=true for deterministic row10t transport-envelope/ack proof only',
    'dispatchAttemptCount and remoteAckCount remain zero for live remote delivery',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
    'product remote acknowledgement delivery',
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
writeJson(join(proofRoot, 'expected-external-cross-process-transport.json'), expectedTransport);

const commands = [
  {
    name: 'agent-core-external-cross-process-transport-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'external_cross_process_transport'],
    log: join(proofRoot, 'agent-core-external-cross-process-transport-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-external-cross-process-transport-proof/v1',
    'asserted=row10t creates deterministic external cross-process transport envelope and ack records from row10r replay metadata',
    'asserted=row10t transport records preserve sequence, event id, event type, correlation id, durable envelope refs, and row10r replay refs',
    'asserted=externalCrossProcessTransportImplemented is true only for the bounded row10t transport-envelope/ack proof',
    'asserted=dispatchAttemptCount and remoteAckCount remain zero for live remote delivery',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
    'asserted=no live broker/family-hub/provider/child-device delivery claim',
    'asserted=no actual remote delete/export propagation claim',
    'asserted=no product-ready remote delivery claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-external-cross-process-transport-proof',
  proofRevision: 'network-remote-delivery-external-cross-process-transport-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-external-cross-process-transport-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  runContext: 'deterministic-row10t-external-cross-process-transport-envelope-ack',
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedExternalCrossProcessTransport: join(proofRoot, 'expected-external-cross-process-transport.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10t remote delivery external cross-process transport proof',
    'network-plan supplemental row 10s remote delivery cross-process replay status bridge proof',
    'network-plan supplemental row 10r remote delivery cross-process durable replay metadata proof',
  ],
  provenBoundaries: [
    'Rust core consumes row10r replay records and creates deterministic row10t transport envelope and ack records',
    'row10t transport records preserve sequence, event id, event type, correlation id, durable envelope/store refs, row10r replay refs, and transport refs',
    'Rust protocol, service WebSocket payload, and Rust-generated TypeScript schema expose row10t status identity and transport envelope/ack counts in the existing remote-delivery status shape',
    'Rust-generated TypeScript schema rejects stale row10t refs, mismatched row10t transport counts, disabled row10t implementation, live remote ack counters, product delivery, policy authority, adapter execution, host filtering, and exact-content claims',
    'row10t keeps live broker/family-hub/provider/child delivery, product remote acknowledgement delivery, actual remote delete/export propagation, product-ready delivery, policy authority, side-effect authority, adapter action, host filtering, exact content, raw PCAP, and enforcement-command publication unclaimed',
  ],
  notClaimed: expectedTransport.noClaims,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log(
  'network-remote-delivery-external-cross-process-transport-proof-ok:core,protocol,service,ts,clippy,fmt,source-shape,diff-check'
);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const protocolShape = readText('crates/agent-protocol/src/network_flow.rs');
  const protocolTests = readText('crates/agent-protocol/tests/contract/network_flow_tests.rs');
  const coreLib = readText('crates/agent-core/src/lib.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_external_cross_process_transport.rs'
  );
  const coreTypes = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_external_cross_process_transport_types.rs'
  );
  const coreTransportTests = readText(
    'crates/agent-core/tests/unit/network_event_runtime/remote_delivery_external_cross_process_transport_tests.rs'
  );
  const serviceCrossProcess = readText('crates/agent-service/src/network_remote_delivery_status_cross_process.rs');
  const servicePayload = readText('crates/agent-service/src/network_remote_delivery_status_payload.rs');
  const serviceTests = readText('crates/agent-service/tests/unit/network_remote_delivery_status_service_tests.rs');
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
    [protocolConstants, 'TEST_REMOTE_DELIVERY_EXTERNAL_CROSS_PROCESS_TRANSPORT_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_EXTERNAL_CROSS_PROCESS_TRANSPORT_STATUS_REF'],
    [protocolConstants, 'ERROR_NETWORK_RUNTIME_REMOTE_EXTERNAL_CROSS_PROCESS_TRANSPORT'],
    [protocolShape, 'external_cross_process_transport_record_count'],
    [protocolShape, 'DeterministicEnvelopeAckRecorded'],
    [protocolTests, 'row10t_external_transport_status'],
    [coreLib, 'pub mod network_event_runtime'],
    [coreRuntime, 'pub mod remote_delivery_external_cross_process_transport'],
    [coreProof, 'external_cross_process_transport_records_match_replay_records: true'],
    [coreProof, 'external_cross_process_transport_implemented: true'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryExternalCrossProcessTransportReport'],
    [coreTransportTests, 'records_external_cross_process_transport_from_replay_records'],
    [serviceCrossProcess, 'apply_external_cross_process_transport_status'],
    [servicePayload, 'prove_network_runtime_remote_delivery_external_cross_process_transport'],
    [serviceTests, 'assert_remote_delivery_external_cross_process_transport_status'],
    [generatedContracts, 'ParentAgentNetworkRemoteDeliveryStatusSchema'],
    [schemaStatus, 'AgentNetworkRemoteDeliveryRow10tRefs'],
    [generatedContractTests, 'externalCrossProcessTransportRef'],
    [schemaStatus, 'externalCrossProcessTransportImplemented: Schema.Literal(true)'],
    [generatedContractTests, 'accepts row10 remote delivery status through the generated schema'],
    [protocolReadme, 'row10t'],
    [serviceReadme, 'row10t'],
    [tsReadme, 'Generated/thin protocol adapters'],
    [featureDoc, 'row10t external cross-process transport'],
    [checklist, '10t-remote-delivery-external-cross-process-transport'],
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
