import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '10i-remote-delivery-dispatch-readiness');
const testRoot = join('test-results', 'network-remote-delivery-dispatch-readiness-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/network-remote-delivery-dispatch-readiness-proof.mjs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-core/src/lib.rs',
  'crates/agent-core/src/network_event_runtime.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_dispatch_readiness.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_dispatch_readiness_types.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff.rs',
  'crates/agent-core/src/network_event_runtime/remote_delivery_outbox_handoff_types.rs',
  'crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'docs/plans/network-plan/workpacks/README.md',
];

assertSourceContracts();

const expectedStatus = {
  acceptedInputs: [
    'row10g prepared outbox candidates',
    'row10g outbox handoff refs',
    'row10f remote delivery status remains read-only',
    'row10b broker and family-hub fixture requirements',
  ],
  dispatchReadinessRefs: [
    'network.remote-delivery.dispatch-readiness.10i',
    'network.remote-delivery.transport-requirements.10i',
    'network.remote-delivery.broker-dispatch-gate.10i',
    'network.remote-delivery.family-hub-dispatch-gate.10i',
  ],
  renderedStates: [
    'state=ManualRequiredTransportNotImplemented',
    'fixtureRequirementsSatisfied=true',
    'transportImplemented=false',
    'dispatchReady=false',
    'manualRequired=true',
    'manualRequiredCandidateCount equals sourceOutboxCandidateCount',
    'dispatchReadyCandidateCount=0',
    'dispatchAttemptCount=0',
    'remoteAckCount=0',
  ],
  parserInvariants: [
    'dispatch readiness consumes row10g prepared candidates without mutating them',
    'broker and family-hub dispatch gates preserve eventing required-artifact refs',
    'fixture requirements alone do not make dispatch ready without transport implementation',
    'manual-required outbox candidates cannot publish enforcement commands',
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
writeJson(join(proofRoot, 'expected-remote-delivery-dispatch-readiness-status.json'), expectedStatus);

const commands = [
  {
    name: 'agent-core-remote-delivery-dispatch-readiness-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'network_runtime_remote_delivery_dispatch_readiness'],
    log: join(proofRoot, 'agent-core-remote-delivery-dispatch-readiness-test.log'),
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
    'checkedAt=deterministic:network-remote-delivery-dispatch-readiness-proof/v1',
    'asserted=no exact URL/page/video/message/search claim from network-only evidence',
    'asserted=no decrypted payload or raw PCAP without custody claim',
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
  proof: 'network-remote-delivery-dispatch-readiness-proof',
  proofRevision: 'network-remote-delivery-dispatch-readiness-proof/v1',
  checkedAt: 'deterministic:network-remote-delivery-dispatch-readiness-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  sourceBase: mergeBase(),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedRemoteDeliveryDispatchReadinessStatus: join(
      proofRoot,
      'expected-remote-delivery-dispatch-readiness-status.json'
    ),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'network-plan supplemental row 10i remote delivery dispatch readiness',
    'network-plan supplemental row 10h remote delivery outbox status bridge',
    'network-plan supplemental row 10g remote delivery outbox handoff',
    'network-plan supplemental row 10f remote delivery status bridge',
  ],
  provenBoundaries: [
    'agent-core consumes row10g prepared outbox candidates without mutating them or dispatching transport',
    'broker and family-hub dispatch gates preserve eventing required-artifact refs and fixture-satisfied state',
    'fixture requirements alone do not make dispatch ready while live transport implementation remains false',
    'manual-required candidates equal prepared candidates and dispatch-ready candidates stay zero',
    'the proof keeps dispatch attempts, remote acknowledgements, broker delivery, family-hub relay delivery, provider delivery, child-device delivery, remote delete/export propagation, product-ready remote delivery, policy authority, side-effect authority, adapter execution, enforcement commands, and host filtering false',
    'the proof keeps raw PCAP, exact URL, decrypted payload, page content, video content, private-message content, and search-query content unavailable from network-only dispatch readiness',
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
console.log('network-remote-delivery-dispatch-readiness-proof-ok:core,clippy,fmt,source-shape,diff-check');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const protocolConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');
  const coreLib = readText('crates/agent-core/src/lib.rs');
  const coreRuntime = readText('crates/agent-core/src/network_event_runtime.rs');
  const coreProof = readText('crates/agent-core/src/network_event_runtime/remote_delivery_dispatch_readiness.rs');
  const coreTypes = readText('crates/agent-core/src/network_event_runtime/remote_delivery_dispatch_readiness_types.rs');
  const coreTests = readText('crates/agent-core/tests/unit/network_event_runtime_remote_delivery_tests.rs');
  const featureDoc = readText('docs/features/network-domain-control.md');
  const checklist = readText('docs/plans/network-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/network-plan/workpacks/README.md');
  const requiredSnippets = [
    [protocolConstants, 'TEST_REMOTE_DELIVERY_DISPATCH_READINESS_REF'],
    [protocolConstants, 'TEST_REMOTE_DELIVERY_TRANSPORT_REQUIREMENTS_REF'],
    [coreLib, 'prove_network_runtime_remote_delivery_dispatch_readiness'],
    [coreRuntime, 'prove_network_runtime_remote_delivery_dispatch_readiness'],
    [coreProof, 'dispatch_ready_candidate_count: 0'],
    [coreProof, 'manual_required_candidate_count: candidate_count'],
    [coreProof, 'has_unsupported_claims'],
    [coreTypes, 'NetworkRuntimeRemoteDeliveryDispatchReadinessReport'],
    [coreTests, 'network_runtime_remote_delivery_dispatch_readiness_blocks_without_transport'],
    [coreTests, 'network_runtime_remote_delivery_dispatch_readiness_rejects_authority_and_content_claims'],
    [featureDoc, 'network-remote-delivery-dispatch-readiness-proof'],
    [checklist, '10i-remote-delivery-dispatch-readiness'],
    [workpacks, '10i'],
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
  return `Source shape warnings scoped to row10i source refs:\n${scopedLines.join('\n')}\n`;
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
