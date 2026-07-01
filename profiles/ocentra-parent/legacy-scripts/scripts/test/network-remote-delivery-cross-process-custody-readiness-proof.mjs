import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10q-remote-delivery-cross-process-custody-readiness');
const testRoot = join('test-results', 'network-remote-delivery-cross-process-custody-readiness-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-cross-process-custody-readiness-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-protocol/src/network_flow.rs',
  'crates/agent-protocol/tests/contract/network_flow_tests.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_custody_readiness.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_custody_readiness_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime/remote_delivery_cross_process_custody_readiness_tests.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_provider_child_readiness.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_provider_child_readiness_types.rs',
  'crates/agent-service/src/network_remote_delivery_status_cross_process.rs',
  'crates/agent-service/src/network_remote_delivery_status_payload.rs',
  'crates/agent-service/tests/unit/network_remote_delivery_status_service_tests.rs',
  'packages/schema-domain/src/agent-protocol-defaults.ts',
  'packages/schema-domain/src/network-remote-delivery-status.ts',
  'packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts',
  'packages/agent-protocol-domain/tests/unit/generated-agent-protocol-contracts.test.ts',
  'crates/agent-core/README.md',
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
  crossProcessCustodyReadinessRefs: [
    'network.remote-delivery.cross-process-custody-status.10q',
    'network.remote-delivery.cross-process-replay-readiness.10q',
    'network.remote-delivery.remote-retention-readiness.10q',
    'network.remote-delivery.remote-delete-custody-readiness.10q',
    'network.remote-delivery.remote-export-custody-readiness.10q',
  ],
  provenStates: [
    'crossProcessCustodyReadinessState=manual-required-unavailable',
    'crossProcessReplayReadinessRecordCount equals providerDeliveryReadinessRecordCount',
    'remoteRetentionReadinessRecordCount equals providerDeliveryReadinessRecordCount',
    'remoteDeleteCustodyReadinessRecordCount equals providerDeliveryReadinessRecordCount',
    'remoteExportCustodyReadinessRecordCount equals providerDeliveryReadinessRecordCount',
    'crossProcessReplayArtifactCount remains zero',
    'remoteRetentionArtifactCount remains zero',
    'remoteDeleteCustodyArtifactCount remains zero',
    'remoteExportCustodyArtifactCount remains zero',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
    'live remote acknowledgement',
    'remote provider delivery',
    'child-device delivery',
    'cross-process durable replay implementation',
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
writeJson(join(proofRoot, 'expected-cross-process-custody-readiness-status.json'), expectedStatus);

const commands = [
  {
    name: 'agent-core-cross-process-custody-readiness-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'cross_process_custody_readiness'],
    log: join(proofRoot, 'agent-core-cross-process-custody-readiness-test.log'),
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
    name: 'agent-core-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-parent-agent-core', '--lib', '--no-deps', '--', '-D', 'warnings'],
    log: join(proofRoot, 'agent-core-clippy.log'),
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
    'checkedAt=deterministic:network-remote-delivery-cross-process-custody-readiness-proof/v1',
    'asserted=remote delivery status carries row10q cross-process custody and replay readiness refs',
    'asserted=row10q readiness records match row10p provider/child readiness records',
    'asserted=cross-process replay, remote retention, remote delete custody, and remote export custody artifact counts remain zero',
    'asserted=crossProcessReplayImplemented and remoteDeleteExportPropagationImplemented remain false',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
    'asserted=no live broker/family-hub/provider/child-device delivery claim',
    'asserted=no actual remote delete/export propagation claim',
    'asserted=no policy authority, side-effect authority, adapter action, explicit host-filter claim, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-cross-process-custody-readiness-proof',
  proofRevision: 'network-remote-delivery-cross-process-custody-readiness-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-cross-process-custody-readiness-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  runContext: 'deterministic-row10q-cross-process-custody-readiness',
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedCrossProcessCustodyReadinessStatus: join(proofRoot, 'expected-cross-process-custody-readiness-status.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10q remote delivery cross-process custody readiness proof',
    'network-plan supplemental row 10p remote delivery provider child readiness proof',
    'network-plan supplemental row 10l remote delivery fixture transport receipt proof',
  ],
  provenBoundaries: [
    'schema-domain owns the row10q remote-delivery status payload fields and zero-claim counters',
    'Rust core turns row10p provider/child readiness records into row10q cross-process custody/replay readiness records without live remote transport artifacts',
    'Rust protocol serializes row10q cross-process custody refs, manual-required-unavailable state, and zero custody/replay artifact counts in the existing remote delivery status shape',
    'agent-service reports row10q cross-process custody readiness through agent.network.remote-delivery.status.reported',
    'Rust-generated TypeScript schema rejects stale row10q refs, nonzero custody/replay artifact counts, mismatched readiness counts, live/product-ready delivery, explicit host-filter claims, and content claims',
    'row10q readiness remains unavailable/manual-required and cannot authorize policy, adapter, host filtering, or enforcement commands',
  ],
  notClaimed: expectedStatus.noClaims,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log(
  'network-remote-delivery-cross-process-custody-readiness-proof-ok:core,protocol,service,ts,clippy,fmt,source-shape,diff-check'
);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const protocolShape = readText('crates/agent-protocol/src/network_flow.rs');
  const protocolTests = readText('crates/agent-protocol/tests/contract/network_flow_tests.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_custody_readiness.rs'
  );
  const coreTypes = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_cross_process_custody_readiness_types.rs'
  );
  const coreCustodyTests = readText(
    'crates/agent-core/tests/unit/network_event_runtime/remote_delivery_cross_process_custody_readiness_tests.rs'
  );
  const serviceCrossProcess = readText('crates/agent-service/src/network_remote_delivery_status_cross_process.rs');
  const servicePayload = readText('crates/agent-service/src/network_remote_delivery_status_payload.rs');
  const serviceTests = readText('crates/agent-service/tests/unit/network_remote_delivery_status_service_tests.rs');
  const tsDefaults = readText('packages/schema-domain/src/agent-protocol-defaults.ts');
  const schemaStatus = readText('packages/schema-domain/src/network-remote-delivery-status.ts');
  const generatedContracts = readText('packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts');
  const generatedContractTests = readText('packages/agent-protocol-domain/tests/unit/generated-agent-protocol-contracts.test.ts');
  const coreReadme = readText('crates/agent-core/README.md');
  const protocolReadme = readText('crates/agent-protocol/README.md');
  const serviceReadme = readText('crates/agent-service/README.md');
  const tsReadme = readText('packages/agent-protocol-domain/README.md');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_CROSS_PROCESS_CUSTODY_STATUS_REF'],
    [protocolShape, 'cross_process_custody_status_ref'],
    [protocolShape, 'cross_process_replay_readiness_record_count'],
    [protocolShape, 'host_filtering_claimed'],
    [protocolTests, 'crossProcessCustodyStatusRef'],
    [protocolTests, 'hostFilteringClaimed'],
    [coreRuntime, 'pub mod remote_delivery_cross_process_custody_readiness'],
    [coreProof, 'cross_process_replay_artifact_count: 0'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryCrossProcessCustodyReadinessReport'],
    [coreCustodyTests, 'preserves_provider_child_readiness_refs_without_cross_process_claims'],
    [serviceCrossProcess, 'cross_process_custody_readiness_state'],
    [serviceCrossProcess, 'RuntimeProviderChildReadinessState::ManualRequiredUnavailable'],
    [serviceCrossProcess, 'RuntimeCrossProcessCustodyReadinessState::ManualRequiredUnavailable'],
    [servicePayload, 'host_filtering_claimed = false'],
    [serviceTests, 'assert_remote_delivery_cross_process_custody_readiness_status'],
    [serviceTests, 'host_filtering_claimed'],
    [
      schemaStatus,
      'crossProcessCustodyReadinessState: AgentNetworkRemoteDeliveryCrossProcessCustodyReadinessStateSchema',
    ],
    [schemaStatus, 'crossProcessReplayArtifactCount: Schema.Literal(0)'],
    [schemaStatus, 'hostFilteringClaimed: Schema.Literal(false)'],
    [tsDefaults, 'CrossProcessCustodyStatusRef'],
    [generatedContracts, 'ParentAgentNetworkRemoteDeliveryStatusSchema'],
    [generatedContracts, 'hostFilteringClaimed'],
    [generatedContractTests, 'crossProcessCustodyStatusRef'],
    [generatedContractTests, 'hostFilteringClaimed'],
    [coreReadme, 'row10q cross-process custody readiness'],
    [protocolReadme, 'row10q cross-process custody'],
    [serviceReadme, 'row10q cross-process custody'],
    [tsReadme, 'Generated/thin protocol adapters'],
    [featureDoc, 'network-remote-delivery-cross-process-custody-readiness-proof'],
    [checklist, '10q-remote-delivery-cross-process-custody-readiness'],
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
