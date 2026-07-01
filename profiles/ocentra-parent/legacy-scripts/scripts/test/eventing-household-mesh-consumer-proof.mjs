import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'eventing-plan-proof', '12-household-mesh-consumer');
const testRoot = join('test-results', 'eventing-household-mesh-consumer-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const sourceFiles = [
  'scripts/test/eventing-household-mesh-consumer-proof.mjs',
  'crates/agent-protocol/src/constants/household_mesh.rs',
  'crates/agent-protocol/src/constants.rs',
  'crates/agent-core/src/household_mesh_event_bridge.rs',
  'crates/agent-core/tests/unit/household_mesh_event_bridge.rs',
  'crates/agent-core/src/lib.rs',
  'crates/agent-core/README.md',
  'crates/agent-service/src/lan_pairing/lan_ai_job.rs',
  'crates/agent-service/tests/unit/lan_pairing/lan_ai_job.rs',
  'docs/plans/eventing-plan/README.md',
  'docs/plans/eventing-plan/implementation-checklist.md',
  'docs/plans/eventing-plan/workpacks/README.md',
  'docs/features/remote-lan-mobile-platforms.md',
  'docs/features/local-ai-safety-evaluator.md',
];

assertSourceContracts();

const expectedBoundary = {
  proof: 'eventing-household-mesh-consumer-proof',
  selectedLocalEvents: [
    'household.mesh.local-event.device-discovery',
    'household.mesh.local-event.provider-advertisement',
    'household.mesh.local-event.provider-heartbeat',
    'household.mesh.local-event.provider-capability',
    'household.mesh.local-event.ai-work-offer',
    'household.mesh.local-event.ai-work-claim-request',
    'household.mesh.local-event.ai-work-claim-decision',
    'household.mesh.local-event.ai-work-lease-state',
    'household.mesh.local-event.ai-job-payload-transfer',
    'household.mesh.local-event.ai-result-return',
    'household.mesh.local-event.config-command',
    'household.mesh.local-event.approval-override-command',
    'household.mesh.local-event.read-model-query-request',
  ],
  rejectedLocalOnlyEvents: [
    'household.mesh.local-event.raw-capture-internal',
    'household.mesh.local-event.adapter-internal',
    'household.mesh.local-event.private-queue-mechanic',
    'household.mesh.local-event.policy-decision',
    'household.mesh.local-event.enforcement-command',
  ],
  incomingValidation: [
    'paired-trusted-device authentication is required before local republish',
    'direct remote publish into another runtime bus is rejected',
    'provider and parent-UI policy authority claims are rejected',
    'raw payload transfer is rejected',
    'unselected local refs and mismatched event/message refs are rejected',
    'child-agent-only AI policy authority is preserved on local republish',
  ],
  notClaimed: [
    'shared LAN-wide event bus',
    'remote direct publish into another runtime local bus',
    'provider-owned policy authority',
    'parent-UI-owned policy authority',
    'raw screenshot transfer by default',
    'raw capture payload transfer by default',
    'live physical household device provider execution',
    'production model quality',
    'portal UI',
    'enforcement command publication',
    'adapter execution',
  ],
};
writeJson(join(proofRoot, 'expected-household-mesh-consumer-boundary.json'), expectedBoundary);

const commands = [
  {
    name: 'agent-core-household-mesh-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-core', 'household_mesh'],
    log: join(proofRoot, 'agent-core-household-mesh-tests.log'),
  },
  {
    name: 'agent-service-lan-ai-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'lan_ai'],
    log: join(proofRoot, 'agent-service-lan-ai-tests.log'),
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
    args: ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing'],
    log: join(proofRoot, 'source-shape.log'),
  },
  {
    name: 'git-diff-check',
    command: 'git',
    args: ['diff', '--check', '--', '.', ':(exclude)output', ':(exclude)test-results'],
    log: join(proofRoot, 'git-diff-check.log'),
  },
];

const commandResults = commands.map(runCommand);

const validationLogPath = join(proofRoot, '12-validation-commands.log');
writeFileSync(
  validationLogPath,
  commandResults.map((entry) => `${entry.command} -> ${entry.status}`).join('\n') + '\n'
);

const securityLogPath = join(proofRoot, '08-security-negative-proof.log');
writeFileSync(
  securityLogPath,
  [
    'checkedAt=deterministic:eventing-household-mesh-consumer-proof/v1',
    'asserted=selected local events export as typed authenticated LAN messages',
    'asserted=local-only raw capture, adapter, private queue, policy, and enforcement events do not export',
    'asserted=incoming LAN messages validate authentication before local republish',
    'asserted=remote direct publish into another runtime bus is rejected',
    'asserted=provider and parent-UI policy-authority escalation is rejected',
    'asserted=raw payload transfer is rejected',
    'asserted=child-agent-only AI policy authority is preserved',
    'asserted=no shared LAN-wide event bus claim',
    'asserted=no physical household provider execution, portal UI, enforcement, or adapter execution claim',
  ].join('\n') + '\n'
);

const proof = {
  proof: 'eventing-household-mesh-consumer-proof',
  proofRevision: 'eventing-household-mesh-consumer-proof/v1',
  checkedAt: 'deterministic:eventing-household-mesh-consumer-proof/v1',
  sourceFingerprint: `source-tree:${sourceFingerprint()}`,
  sourceRefs: sourceFiles,
  runContext:
    'Branch, merge base, commit, pushed state, and validation command output are reported in worker handoffs; committed row12 proof artifacts stay deterministic.',
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedBoundary: join(proofRoot, 'expected-household-mesh-consumer-boundary.json'),
    securityNegativeLog: securityLogPath,
    validationCommands: validationLogPath,
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  coveredRows: [
    'eventing-plan proof-pack row 12 household mesh consumer proof',
    'eventing-plan full-plan proof consumer boundary extension',
  ],
  provenBoundaries: [
    'agent-core exports a consumer bridge that maps only selected local event refs into typed authenticated LAN message refs',
    'agent-core rejects local-only raw capture, adapter, private queue, policy, and enforcement event refs at export time',
    'incoming LAN messages must be paired-trusted-device authenticated before local republish',
    'incoming LAN messages cannot request direct remote publish into another runtime bus',
    'provider and parent UI policy authority claims are rejected; child-agent-only AI policy authority is preserved',
    'incoming raw payload transfer is rejected before local republish',
    'existing LAN AI service tests still prove provider status/job routing uses typed LAN AI rows without raw activity transfer',
  ],
  notClaimed: expectedBoundary.notClaimed,
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('eventing-household-mesh-consumer-proof-ok:agent-core,lan-ai,clippy,fmt,source-shape,diff-check');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const constants = readText('crates/agent-protocol/src/constants/household_mesh.rs');
  const constantsRoot = readText('crates/agent-protocol/src/constants.rs');
  const bridge = readText('crates/agent-core/src/household_mesh_event_bridge.rs');
  const bridgeTests = readText('crates/agent-core/tests/unit/household_mesh_event_bridge.rs');
  const coreLib = readText('crates/agent-core/src/lib.rs');
  const coreReadme = readText('crates/agent-core/README.md');
  const lanAiSource = readText('crates/agent-service/src/lan_pairing/lan_ai_job.rs');
  const lanAiTests = readText('crates/agent-service/tests/unit/lan_pairing/lan_ai_job.rs');
  const planReadme = readText('docs/plans/eventing-plan/README.md');
  const checklist = readText('docs/plans/eventing-plan/implementation-checklist.md');
  const workpacks = readText('docs/plans/eventing-plan/workpacks/README.md');
  const remoteFeature = readText('docs/features/remote-lan-mobile-platforms.md');
  const aiFeature = readText('docs/features/local-ai-safety-evaluator.md');
  const requiredSnippets = [
    [constants, 'LAN_MESSAGE_AI_RESULT_RETURN'],
    [constants, 'REJECTION_DIRECT_REMOTE_PUBLISH'],
    [constants, 'REJECTION_MISMATCHED_MESSAGE_REF'],
    [constants, 'POLICY_AUTHORITY_CHILD_AGENT_ONLY'],
    [constantsRoot, 'pub mod household_mesh'],
    [bridge, 'export_selected_local_event'],
    [bridge, 'validate_incoming_lan_message'],
    [bridge, 'HouseholdMeshPolicyAuthority::ChildAgentOnly'],
    [bridge, 'HouseholdMeshBridgeRejection::DirectRemotePublish'],
    [bridge, 'HouseholdMeshBridgeRejection::PolicyAuthorityEscalation'],
    [bridge, 'HouseholdMeshBridgeRejection::RawPayload'],
    [bridge, 'HouseholdMeshBridgeRejection::MismatchedMessageRef'],
    [bridgeTests, 'household_mesh_exports_all_selected_local_events'],
    [bridgeTests, 'household_mesh_rejects_unselected_local_events'],
    [bridgeTests, 'household_mesh_validates_incoming_before_local_republish'],
    [bridgeTests, 'household_mesh_rejects_direct_publish_and_policy_escalation'],
    [bridgeTests, 'household_mesh_rejects_raw_payload_and_invalid_refs'],
    [coreLib, 'household_mesh_event_bridge'],
    [coreReadme, 'Household Mesh Bridge consumer proof'],
    [lanAiSource, 'lan_ai_job_submit'],
    [lanAiTests, 'authorized_lan_ai_job_submit_routes_to_opted_in_provider'],
    [planReadme, 'eventing-household-mesh-consumer-proof.mjs'],
    [checklist, '12-household-mesh-consumer-proof.log'],
    [checklist, 'output/eventing-plan-proof/12-household-mesh-consumer/proof-summary.json'],
    [workpacks, 'Household Mesh consumer proof'],
    [remoteFeature, 'eventing-household-mesh-consumer-proof'],
    [aiFeature, 'eventing-household-mesh-consumer-proof'],
  ];
  for (const [haystack, needle] of requiredSnippets) {
    assertIncludes(haystack, needle, `source contract snippet ${needle}`);
  }
  assertDoesNotInclude(bridge, 'ocentra_eventing', 'bridge is a consumer boundary, not a generic bus change');
  assertDoesNotInclude(
    bridge,
    'NetworkRuntime',
    'bridge does not pull network runtime business logic into household mesh proof'
  );
  assertDoesNotInclude(bridge, 'EnforcementAdapter', 'bridge does not publish or execute enforcement adapter commands');
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

function assertDoesNotInclude(text, unexpected, label) {
  if (text.includes(unexpected)) {
    throw new Error(`${label}: unexpected ${unexpected}`);
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
    ['Source shape warnings scoped to row12 source refs:', ...scopedWarnings, passedLine]
      .filter((line) => line.length > 0)
      .join('\n') + '\n'
  );
}

function normalizeLogText(text) {
  const workspacePath = process.cwd();
  const workspacePathForward = workspacePath.replace(/\\/g, '/');
  const lines = text
    .replace(new RegExp(escapeRegExp(workspacePath), 'g'), '<workspace>')
    .replace(new RegExp(escapeRegExp(workspacePathForward), 'g'), '<workspace>')
    .replace(/\r\n/g, '\n')
    .replace(/\\/g, '/')
    .split('\n')
    .filter((line) => !line.includes('Blocking waiting for'))
    .filter((line) => !line.trimStart().startsWith('Compiling '))
    .filter((line) => !line.trimStart().startsWith('Checking '))
    .map((line) =>
      line
        .replace(/target\/debug\/deps\/[^\s)]+/g, 'target/debug/deps/<test-binary>')
        .replace(/finished in [0-9.]+s/g, 'finished in <duration>')
        .replace(/target\(s\) in [0-9.]+s/g, 'target(s) in <duration>')
        .replace(/target\(s\) in [0-9]+m [0-9]+s/g, 'target(s) in <duration>')
        .replace(/Duration\s+[0-9.]+(?:ms|s)/g, 'Duration <duration>')
        .replace(/Start at\s+[0-9:]+/g, 'Start at <time>')
        .replace(/; [0-9]+ filtered out;/g, '; <filtered> filtered out;')
        .replace(
          /file has \d+ lines; crossed \d+-line advisory band; maximum is \d+/g,
          'file has <lines> lines; crossed <band>-line advisory band; maximum is <max>'
        )
        .replace(
          /function has \d+ lines; warning starts at \d+ of \d+/g,
          'function has <lines> lines; warning starts at <warn> of <max>'
        )
        .replace(
          /file has \d+ functions; warning starts at \d+ of \d+/g,
          'file has <functions> functions; warning starts at <warn> of <max>'
        )
        .replace(
          /file has \d+ structs\/enums; warning starts at \d+ of \d+/g,
          'file has <structs-enums> structs/enums; warning starts at <warn> of <max>'
        )
        .replace(/\b[0-9.]+(?:ms|s)\b/g, '<duration>')
    );
  const normalized = stableRustTestLines(lines)
    .join('\n')
    .replace(/[ \t]+$/gm, '')
    .replace(/\s+$/u, '');
  return normalized.length === 0 ? '' : `${normalized}\n`;
}

function stableRustTestLines(lines) {
  const sortedTestLines = lines.filter(isRustTestLine).sort();
  let nextTestLine = 0;
  return lines.map((line) => {
    if (!isRustTestLine(line)) {
      return line;
    }
    const sortedLine = sortedTestLines[nextTestLine];
    nextTestLine += 1;
    return sortedLine;
  });
}

function isRustTestLine(line) {
  return /^test .+ \.\.\. ok$/u.test(line);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
