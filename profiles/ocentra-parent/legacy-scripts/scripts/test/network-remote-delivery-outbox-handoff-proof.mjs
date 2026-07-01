import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10g-remote-delivery-outbox-handoff');
const testRoot = join('test-results', 'network-remote-delivery-outbox-handoff-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-outbox-handoff-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
  'crates/agent-core/README.md',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/workpacks/README.md',
];

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'row10e durable envelope records',
    'row10e durable store refs',
    'row10d receipt ledger and local receipt acknowledgement refs',
    'row10f read-only remote delivery status bridge remains available for consumers',
  ],
  outboxHandoffRefs: [
    'network.remote-delivery.outbox.10g',
    'network.remote-delivery.outbox-handoff.10g',
    'network.remote-delivery.outbox-replay.10g',
    'network.remote-delivery.outbox-support-status.10g',
  ],
  renderedStates: [
    'outboxCandidatesMatchDurableEnvelopes=true',
    'outboxCandidatesMatchReceipts=true',
    'outboxCandidateCount equals sourceDurableEnvelopeCount',
    'preparedNotDispatchedCount equals outboxCandidateCount',
    'dispatchAttemptCount=0',
    'remoteAckCount=0',
    'duplicateDurableEnvelopeRejected=true',
    'remoteDeleteExportPropagationImplemented=false',
    'productReadyRemoteDelivery=false',
  ],
  parserInvariants: [
    'outbox refs must all cite row10g',
    'outbox candidates must preserve durable envelope sequence, event id, event type, correlation id, durable-envelope refs, durable-store refs, receipt-ledger refs, and local receipt-ack refs',
    'duplicate durable envelope candidates must be rejected before outbox preparation',
    'outbox handoff cannot dispatch live transport',
    'outbox handoff cannot carry remote acknowledgement, exact content, host filtering, or adapter-action claims',
  ],
  noClaims: [
    'live broker delivery',
    'live family-hub relay delivery',
    'transport dispatch attempt',
    'remote acknowledgement',
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
writeJson(join(proofRoot, 'expected-remote-delivery-outbox-handoff-status.json'), expectedStatus);

const commands = [
  {
    name: 'agent-core-remote-delivery-outbox-handoff-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_outbox_handoff'],
    log: join(proofRoot, 'agent-core-remote-delivery-outbox-handoff-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-outbox-handoff-proof/v1',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP without custody claim',
    'asserted=no live broker/family-hub delivery claim',
    'asserted=no transport dispatch attempt claim',
    'asserted=no remote acknowledgement claim',
    'asserted=no remote provider or child-device delivery claim',
    'asserted=no remote delete/export propagation implementation claim',
    'asserted=no product-ready remote delivery claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-outbox-handoff-proof',
  proofRevision: 'network-remote-delivery-outbox-handoff-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-outbox-handoff-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  mergeBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryOutboxHandoffStatus: join(proofRoot, 'expected-remote-delivery-outbox-handoff-status.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10g remote delivery outbox/handoff readiness status',
    'network-plan supplemental row 10f remote delivery status bridge',
    'network-plan supplemental row 10e remote delivery durable envelope/store status',
    'network-plan supplemental row 10d remote delivery receipt ledger/local ack status',
  ],
  provenBoundaries: [
    'agent-core builds ordered outbox handoff candidates from row10e durable envelope records without regenerating durable, receipt, or local-ack refs',
    'outbox candidates preserve durable envelope sequence, event id, event type, correlation id, durable-envelope refs, durable-store refs, receipt-ledger refs, and local receipt-ack refs',
    'duplicate durable envelope candidates are rejected before outbox preparation',
    'row10g refs mark outbox, handoff, replay, and support-status boundaries that future broker/family-hub delivery can consume',
    'the proof keeps dispatch attempts, remote acknowledgements, broker delivery, family-hub relay delivery, provider delivery, child-device delivery, remote delete/export propagation, product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement commands, and host filtering false',
    'the proof keeps raw PCAP, exact URL, decrypted payload, page content, video content, private-message content, and search-query content unavailable from network-only outbox records',
  ],
  notClaimed: [
    'live broker delivery',
    'live family-hub relay delivery',
    'transport dispatch attempt',
    'remote acknowledgement',
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
  ],
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('network-remote-delivery-outbox-handoff-proof-ok:core,clippy,source-shape,diff-check');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff.rs');
  const coreTypes = readText('crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff_types.rs');
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const coreReadme = readText('crates/agent-core/README.md');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_OUTBOX_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_OUTBOX_HANDOFF_REF'],
    [coreRuntime, 'prove_network_runtime_remote_delivery_outbox_handoff'],
    [coreProof, 'outbox_candidates_from_durable_records'],
    [coreProof, 'dispatch_attempt_count: 0'],
    [coreProof, 'remote_ack_count: 0'],
    [coreProof, 'duplicate_durable_envelope_rejected'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryOutboxHandoffReport'],
    [coreTests, 'network_runtime_remote_delivery_outbox_handoff_preserves_durable_refs_without_dispatch'],
    [coreTests, 'network_runtime_remote_delivery_outbox_handoff_rejects_dispatch_ack_action_and_content_claims'],
    [coreReadme, 'remote outbox handoff proof'],
    [featureDoc, 'network-remote-delivery-outbox-handoff-proof'],
    [checklist, '10g-remote-delivery-outbox-handoff'],
    [workpacks, '10g'],
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

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function sourceFingerprint() {
  const hash = createHash('sha256');
  for (const filePath of fingerprintSourceFiles()) {
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
    .filter((line) => fingerprintSourceFiles().some((filePath) => line.startsWith(filePath)))
    .sort();
  const passedLine = normalized.includes('Source shape guard passed.') ? 'Source shape guard passed.' : '';
  return (
    ['Source shape warnings scoped to row10g source refs:', ...scopedWarnings, passedLine]
      .filter((line) => line.length > 0)
      .join('\n') + '\n'
  );
}

function normalizeLogText(text) {
  const normalizedLines = sortSourceShapeWarningLines(
    sortConsecutiveTestLines(
      normalizeWorkspacePaths(text)
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
            .replace(/duration_ms: [0-9.]+/g, 'duration_ms: <duration>')
            .replace(/\b[0-9.]+(?:ms|s)\b/g, '<duration>')
        )
    )
  );
  const trimmed = normalizedLines
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return trimmed.length === 0 ? '' : `${trimmed}\n`;
}

function fingerprintSourceFiles() {
  return sourceFiles.filter((filePath) => !filePath.startsWith('scripts/test/'));
}

function normalizeWorkspacePaths(text) {
  const workspacePath = process.cwd();
  const workspacePathForward = workspacePath.replace(/\\/g, '/');
  return text
    .replace(new RegExp(escapeRegExp(workspacePath), 'g'), '<workspace>')
    .replace(new RegExp(escapeRegExp(workspacePathForward), 'g'), '<workspace>');
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
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
