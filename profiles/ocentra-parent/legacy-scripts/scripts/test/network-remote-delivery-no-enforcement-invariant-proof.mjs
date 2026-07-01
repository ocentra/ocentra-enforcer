import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10j-remote-delivery-no-enforcement-invariant');
const testRoot = join('test-results', 'network-remote-delivery-no-enforcement-invariant-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-no-enforcement-invariant-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_no_enforcement_invariant.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_no_enforcement_invariant_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_dispatch_readiness.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_dispatch_readiness_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_durable_envelope.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_receipt_ledger.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_status.rs',
  'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/workpacks/README.md',
];

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'row10b broker and family-hub requirement refs',
    'row10c event-chain journal/export refs',
    'row10d receipt-ledger and local-ack refs',
    'row10e durable envelope/store/delete-export refs',
    'row10g outbox handoff refs',
    'row10i dispatch-readiness refs',
  ],
  invariantRefs: [
    'network.remote-delivery.no-enforcement-invariant.10j',
    'network.remote-delivery.available-metadata.10j',
  ],
  provenStates: [
    'state=AvailableMetadataNonEnforcing',
    'remoteMetadataStageCount=6',
    'availableMetadataRefCount>=31',
    'manualRequiredCandidateCount equals row10g outbox candidate count',
    'dispatchReadyCandidateCount=0',
    'dispatchAttemptCount=0',
    'remoteAckCount=0',
  ],
  parserInvariants: [
    'available remote metadata cannot publish enforcement commands',
    'available remote metadata cannot execute adapters',
    'available remote metadata cannot claim policy or side-effect authority',
    'available remote metadata cannot claim exact content from network-only evidence',
  ],
  noClaims: [
    'live broker dispatch',
    'live family-hub relay dispatch',
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
writeJson(join(proofRoot, 'expected-remote-delivery-no-enforcement-invariant-status.json'), expectedStatus);

const commands = [
  {
    name: 'agent-core-remote-delivery-no-enforcement-invariant-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_no_enforcement_invariant'],
    log: join(proofRoot, 'agent-core-remote-delivery-no-enforcement-invariant-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-no-enforcement-invariant-proof/v1',
    'asserted=available remote metadata remains non-enforcing across row10b-row10i',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP claim',
    'asserted=no live broker/family-hub dispatch claim',
    'asserted=no transport dispatch attempt claim',
    'asserted=no remote acknowledgement claim',
    'asserted=no remote provider or child-device delivery claim',
    'asserted=no remote delete/export propagation implementation claim',
    'asserted=no product-ready remote delivery claim',
    'asserted=no policy authority, side-effect authority, adapter action, host filtering, or enforcement command publication',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'network-remote-delivery-no-enforcement-invariant-proof',
  proofRevision: 'network-remote-delivery-no-enforcement-invariant-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-no-enforcement-invariant-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  sourceBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryNoEnforcementInvariantStatus: join(
      proofRoot,
      'expected-remote-delivery-no-enforcement-invariant-status.json'
    ),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10j remote delivery no-enforcement invariant',
    'network-plan supplemental row 10i remote delivery dispatch readiness',
    'network-plan supplemental row 10h remote delivery outbox status bridge',
    'network-plan supplemental row 10g remote delivery outbox handoff',
    'network-plan supplemental rows 10b through 10f remote delivery refs',
  ],
  provenBoundaries: [
    'agent-core composes row10b through row10i available remote metadata refs into one non-enforcing invariant',
    'broker/family-hub requirement refs, event-chain journal/export refs, receipt-ledger/local-ack refs, durable envelope/store/delete-export refs, outbox handoff refs, and dispatch-readiness refs remain metadata-only',
    'dispatch-ready candidates, dispatch attempts, and remote acknowledgements stay zero',
    'the invariant rejects policy authority, side-effect authority, adapter execution, enforcement command publication, live broker/family-hub delivery, remote acknowledgement, provider delivery, child-device delivery, remote delete/export propagation, and product-ready remote delivery claims',
    'the invariant rejects raw PCAP, exact URL, decrypted payload, page content, video content, private-message content, and search-query content claims from network-only evidence',
  ],
  notClaimed: [
    'live broker dispatch',
    'live family-hub relay dispatch',
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
console.log('network-remote-delivery-no-enforcement-invariant-proof-ok:core,clippy,fmt,source-shape,diff-check');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_no_enforcement_invariant.rs');
  const coreTypes = readText(
    'crates/agent-core/src/network_event_runtime/remote_delivery_no_enforcement_invariant_types.rs'
  );
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_NO_ENFORCEMENT_INVARIANT_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_AVAILABLE_METADATA_REF'],
    [coreRuntime, 'prove_network_runtime_remote_delivery_no_enforcement_invariant'],
    [coreProof, 'has_unsupported_claims'],
    [coreProof, 'available_metadata_refs'],
    [coreProof, 'AvailableMetadataNonEnforcing'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryNoEnforcementInvariantReport'],
    [coreTests, 'network_runtime_remote_delivery_no_enforcement_invariant_accepts_available_metadata'],
    [coreTests, 'network_runtime_remote_delivery_no_enforcement_invariant_rejects_remote_action_claims'],
    [featureDoc, 'network-remote-delivery-no-enforcement-invariant-proof'],
    [checklist, '10j-remote-delivery-no-enforcement-invariant'],
    [workpacks, '10j'],
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

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function assertIncludes(haystack, needle, label) {
  if (!haystack.includes(needle)) {
    throw new Error(`${label} missing`);
  }
}

function normalizeCommandLog(name, text) {
  const normalized = normalizeCargoTestOrder(text)
    .replaceAll(process.cwd().replaceAll('\\', '/'), '<repo>')
    .replaceAll(process.cwd(), '<repo>')
    .replace(/finished in \d+\.\d+s/g, 'finished in <duration>')
    .replace(/Finished `[^`]+` profile.*target\(s\) in \d+\.\d+s/g, 'Finished <cargo-profile> in <duration>')
    .replace(/^\s+Compiling .*$/gm, '')
    .replace(/^\s+Checking .*$/gm, '')
    .replace(/duration_ms: \d+(\.\d+)?/g, 'duration_ms: <duration>')
    .replace(/Start at\s+\d{2}:\d{2}:\d{2}/g, 'Start at <time>')
    .replace(/Duration\s+\d+(\.\d+)?[a-z]+/g, 'Duration <duration>');
  if (name !== 'source-shape') {
    return normalized;
  }
  const scopedLines = normalized
    .split(/\r?\n/)
    .filter(
      (line) =>
        sourceFiles.some((sourceFile) => line.includes(sourceFile)) || line.includes('Source shape guard passed')
    );
  return `Source shape warnings scoped to row10j source refs:\n${scopedLines.join('\n')}\n`;
}

function normalizeCargoTestOrder(text) {
  const lines = text.split(/\r?\n/);
  const testLines = lines.filter((line) => line.startsWith('test ')).sort();
  let testIndex = 0;
  return lines
    .map((line) => {
      if (!line.startsWith('test ')) {
        return line;
      }
      const sortedLine = testLines[testIndex];
      testIndex += 1;
      return sortedLine;
    })
    .join('\n');
}
