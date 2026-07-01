import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10o-remote-delivery-fixture-transport-status-bridge');
const testRoot = join('test-results', 'network-remote-delivery-fixture-transport-status-bridge-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-fixture-transport-status-bridge-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-protocol/src/network_flow.rs',
  'crates/agent-protocol/tests/contract/network_flow_tests.rs',
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
  statusBridgeRef: 'network.remote-delivery.delete-export-status-bridge.10n',
  fixtureTransportRefs: [
    'network.remote-delivery.fixture-transport.10l',
    'network.remote-delivery.fixture-dispatch-attempt.10l',
    'network.remote-delivery.fixture-ack.10l',
  ],
  provenStates: [
    'fixtureSourceOutboxCandidateCount equals outboxCandidateCount',
    'fixtureDispatchAttemptCount equals outboxCandidateCount',
    'fixtureRemoteAckCount equals outboxCandidateCount',
    'fixtureRecordsMatchOutboxCandidates=true',
    'dispatchAttemptCount remains zero for live transport',
    'remoteAckCount remains zero for live transport',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
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
writeJson(join(proofRoot, 'expected-remote-delivery-fixture-transport-status-bridge.json'), expectedStatus);

const commands = [
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
    'checkedAt=deterministic:network-remote-delivery-fixture-transport-status-bridge-proof/v1',
    'asserted=remote delivery status carries row10l fixture transport refs and counts',
    'asserted=fixture dispatch/ack counts match row10g outbox candidates',
    'asserted=live dispatchAttemptCount and remoteAckCount remain zero',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
    'asserted=no live broker/family-hub/provider/child-device delivery claim',
    'asserted=no actual remote delete/export propagation claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-fixture-transport-status-bridge-proof',
  proofRevision: 'network-remote-delivery-fixture-transport-status-bridge-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-fixture-transport-status-bridge-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  sourceBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryFixtureTransportStatusBridge: join(
      proofRoot,
      'expected-remote-delivery-fixture-transport-status-bridge.json'
    ),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10o remote delivery fixture transport status bridge proof',
    'network-plan supplemental row 10l remote delivery fixture transport receipt proof',
    'network-plan supplemental row 10n remote delivery delete/export status bridge proof',
  ],
  provenBoundaries: [
    'Rust protocol serializes row10l fixture transport refs and counts inside the existing remote delivery status shape',
    'agent-service reports row10l fixture transport refs and counts from the existing nested core report instead of inventing a new transport path',
    'Rust-generated TypeScript schema rejects stale row10l fixture refs and fixture counts that do not match the source outbox candidate count',
    'fixture dispatch/ack proof counters stay separate from live dispatchAttemptCount and remoteAckCount, which remain zero',
    'the status bridge rejects product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement command publication, live broker/family-hub delivery, provider delivery, child-device delivery, exact-content, and host-filter claims',
  ],
  notClaimed: expectedStatus.noClaims,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log(
  'network-remote-delivery-fixture-transport-status-bridge-proof-ok:protocol,service,ts,clippy,fmt,source-shape,diff-check'
);
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const protocolShape = readText('crates/agent-protocol/src/network_flow.rs');
  const protocolTests = readText('crates/agent-protocol/tests/contract/network_flow_tests.rs');
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
    [protocolConstants, 'TEST_REMOTE_DELIVERY_FIXTURE_TRANSPORT_REF'],
    [protocolShape, 'fixture_transport_ref'],
    [protocolShape, 'fixture_dispatch_attempt_count'],
    [protocolTests, 'fixtureTransportRef'],
    [servicePayload, 'apply_fixture_transport_status'],
    [servicePayload, 'fixture_transport'],
    [serviceTests, 'assert_remote_delivery_fixture_transport_status'],
    [tsDefaults, 'FixtureTransportRef'],
    [generatedContracts, 'ParentAgentNetworkRemoteDeliveryStatusSchema'],
    [generatedContractTests, 'fixtureTransportRef'],
    [schemaStatus, 'dispatchAttemptCount: Schema.Literal(0)'],
    [protocolReadme, 'row10l fixture transport'],
    [serviceReadme, 'row10l fixture transport proof'],
    [tsReadme, 'Generated/thin protocol adapters'],
    [featureDoc, 'network-remote-delivery-fixture-transport-status-bridge-proof'],
    [checklist, '10o-remote-delivery-fixture-transport-status-bridge'],
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

function mergeBase() {
  const result = spawnSync('git', ['merge-base', 'HEAD', 'origin/main'], {
    encoding: 'utf8',
    shell: false,
  });
  if (result.status !== 0) {
    throw new Error(`git merge-base failed with exit ${result.status}`);
  }
  return result.stdout.trim();
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function normalizeCommandLog(name, text) {
  if (name === 'source-shape') {
    return normalizeSourceShapeLog(text);
  }
  return normalizeLogText(text);
}

function normalizeSourceShapeLog(text) {
  const normalized = normalizeLogText(text);
  const scopedWarnings = normalized
    .split('\n')
    .filter((line) =>
      sourceFiles
        .filter((filePath) => !filePath.startsWith('scripts/test/'))
        .some((filePath) => line.startsWith(filePath))
    )
    .sort();
  const passedLine = normalized.includes('Source shape guard passed.') ? 'Source shape guard passed.' : '';
  return (
    ['Source shape warnings scoped to row10o source refs:', ...scopedWarnings, passedLine]
      .filter((line) => line.length > 0)
      .join('\n') + '\n'
  );
}

function normalizeLogText(text) {
  const workspacePath = process.cwd();
  const workspacePathForward = workspacePath.replace(/\\/g, '/');
  const normalized = text
    .replace(new RegExp(escapeRegExp(workspacePath), 'g'), '<workspace>')
    .replace(new RegExp(escapeRegExp(workspacePathForward), 'g'), '<workspace>')
    .replace(/\r\n/g, '\n')
    .split('\n')
    .filter((line) => !line.includes('Blocking waiting for'))
    .filter((line) => !line.trimStart().startsWith('Compiling '))
    .filter((line) => !line.trimStart().startsWith('Checking '))
    .map((line) =>
      line
        .replace(/finished in [0-9.]+s/g, 'finished in <duration>')
        .replace(/target\(s\) in [0-9.]+s/g, 'target(s) in <duration>')
        .replace(/target\(s\) in [0-9]+m [0-9]+s/g, 'target(s) in <duration>')
        .replace(/Duration\s+[0-9.]+(?:ms|s)/g, 'Duration <duration>')
        .replace(/Start at\s+[0-9:]+/g, 'Start at <time>')
        .replace(/\b[0-9.]+(?:ms|s)\b/g, '<duration>')
    )
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return normalized.length === 0 ? '' : `${normalized}\n`;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
