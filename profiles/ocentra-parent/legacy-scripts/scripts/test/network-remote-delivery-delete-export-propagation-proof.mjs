import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10m-remote-delivery-delete-export-propagation');
const testRoot = join('test-results', 'network-remote-delivery-delete-export-propagation-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-delete-export-propagation-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/lib.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_delete_export_propagation.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_delete_export_propagation_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_fixture_transport.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_fixture_transport_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime_delete_export_propagation_tests.rs',
  'crates/agent-core/README.md',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/workpacks/README.md',
];

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'row10l fixture transport records',
    'row10g prepared outbox refs',
    'row10e durable envelope delete/export readiness refs',
    'row10d receipt-ledger refs',
  ],
  deleteExportReadinessRefs: [
    'network.remote-delivery.delete-export-propagation-readiness.10m',
    'network.remote-delivery.remote-delete-readiness.10m',
    'network.remote-delivery.remote-export-readiness.10m',
  ],
  provenStates: [
    'propagationReadinessRecordCount equals sourceFixtureRecordCount',
    'remoteDeleteReadyCount equals sourceFixtureRecordCount',
    'remoteExportReadyCount equals sourceFixtureRecordCount',
    'propagationRecordsMatchFixtureRecords=true',
    'propagationState=ReadinessRecordedNotPropagated',
  ],
  parserInvariants: [
    'readiness records preserve event id, event type, correlation id, outbox ref, handoff ref, and fixture ack ref',
    'remote delete/export readiness is recorded only as a local proof boundary',
    'remote delete/export readiness does not upgrade broker/family-hub/provider/child-device delivery',
    'unsupported product-ready, policy, adapter, enforcement, exact-content, or host-filter claims are rejected',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
    'remote provider delivery',
    'child-device delivery',
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
writeJson(join(proofRoot, 'expected-remote-delivery-delete-export-propagation.json'), expectedStatus);

const commands = [
  {
    name: 'agent-core-remote-delivery-delete-export-propagation-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_delete_export_propagation'],
    log: join(proofRoot, 'agent-core-remote-delivery-delete-export-propagation-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-delete-export-propagation-proof/v1',
    'asserted=remote delete/export propagation readiness records are derived from row10l fixture acknowledgements',
    'asserted=readiness records preserve outbox refs, handoff refs, and fixture acknowledgement refs',
    'asserted=remote delete/export propagation remains not implemented and product support remains false',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
    'asserted=no live broker/family-hub/provider/child-device delivery claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-delete-export-propagation-proof',
  proofRevision: 'network-remote-delivery-delete-export-propagation-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-delete-export-propagation-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  sourceBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryDeleteExportPropagation: join(
      proofRoot,
      'expected-remote-delivery-delete-export-propagation.json'
    ),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10m remote delivery delete export propagation readiness proof',
    'network-plan supplemental row 10l remote delivery fixture transport receipt proof',
    'network-plan supplemental row 10g remote delivery outbox handoff',
    'network-plan supplemental rows 10b through 10f remote delivery refs',
  ],
  provenBoundaries: [
    'agent-core consumes row10l fixture transport records and records remote delete/export propagation readiness for each fixture acknowledgement',
    'readiness records preserve event id, event type, correlation id, outbox refs, handoff refs, and fixture acknowledgement refs',
    'remote delete readiness and remote export readiness counts equal the source fixture record count',
    'the proof rejects product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement command publication, live broker/family-hub delivery, provider delivery, child-device delivery, remote delete/export propagation implementation, exact-content, and host-filter claims',
    'remote delete/export readiness remains a local proof boundary and does not upgrade the service remote-delivery status payload or production transport support',
  ],
  notClaimed: expectedStatus.noClaims,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('network-remote-delivery-delete-export-propagation-proof-ok:core,clippy,fmt,source-shape,diff-check');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreLib = readText('crates/agent-core/src/lib.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_delete_export_propagation.rs'
  );
  const coreTypes = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_delete_export_propagation_types.rs'
  );
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_delete_export_propagation_tests.rs');
  const coreReadme = readText('crates/agent-core/README.md');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_DELETE_EXPORT_PROPAGATION_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_REMOTE_DELETE_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_REMOTE_EXPORT_REF'],
    [protocolConstants, 'ERROR_NETWORK_RUNTIME_REMOTE_DELETE_EXPORT_PROPAGATION'],
    [coreLib, 'prove_network_runtime_remote_delivery_delete_export_propagation'],
    [coreRuntime, 'remote_delivery_delete_export_propagation'],
    [coreRuntime, 'NetworkRuntimeRemoteDeliveryDeleteExportPropagationReport'],
    [coreProof, 'propagation_records_match_fixture_records'],
    [coreProof, 'has_unsupported_claims'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryDeleteExportPropagationState'],
    [coreTests, 'network_runtime_remote_delivery_delete_export_propagation_records_readiness_refs'],
    [coreTests, 'network_runtime_remote_delivery_delete_export_propagation_stays_proof_only'],
    [coreTests, 'network_runtime_remote_delivery_delete_export_propagation_rejects_product_claims'],
    [coreReadme, 'Network remote delete/export propagation readiness proof'],
    [featureDoc, 'network-remote-delivery-delete-export-propagation-proof'],
    [checklist, '10m-remote-delivery-delete-export-propagation'],
    [workpacks, '10m'],
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
    ['Source shape warnings scoped to row10m source refs:', ...scopedWarnings, passedLine]
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
