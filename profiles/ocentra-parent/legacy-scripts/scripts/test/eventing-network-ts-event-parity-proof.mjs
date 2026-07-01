import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '04-typescript-event-parity');
const testRoot = join('test-results', 'eventing-network-ts-event-parity-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const commands = [
  {
    name: 'agent-protocol-domain-test',
    command: npmCommand(),
    args: npmArgs(['run', 'test', '--workspace', '@ocentra-parent/agent-protocol-domain']),
    log: join(proofRoot, 'agent-protocol-domain-test.log'),
  },
  {
    name: 'agent-protocol-domain-build',
    command: npmCommand(),
    args: npmArgs(['run', 'build', '--workspace', '@ocentra-parent/agent-protocol-domain']),
    log: join(proofRoot, 'agent-protocol-domain-build.log'),
  },
  {
    name: 'schema-generated-agent-protocol-artifact-test',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-schema', 'generated_agent_protocol_domain_artifact_stays_checked_in'],
    log: join(proofRoot, 'schema-generated-agent-protocol-artifact-test.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing'],
    log: join(proofRoot, 'source-shape.log'),
  },
];

const commandResults = commands.map(runCommand);
assertSourceContracts();
const publicImport = await assertPublicImport();

const proof = {
  proof: 'eventing-network-ts-event-parity',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  publicImport,
  rowsCovered: [
    'network-plan row 04 TypeScript package parity sub-slice',
    'network-plan row 10 TypeScript event parity sub-slice',
    'eventing-plan rows 45-50 TypeScript public parity supplement',
  ],
  claimsProved: [
    'agent-protocol-domain publicly exports Rust-generated/thin network runtime event DTO contracts',
    'generated TypeScript edge decoders cover the eleven Rust protocol-facing network runtime event payload shapes',
    'event-type constants match the Rust network flow event type constants including portal.read_model.updated',
    'generated network claim-boundary parsing rejects exact URL, decrypted payload, message content, search query, and adapter-action claims',
    'generated network AI request parsing rejects raw packet payload inclusion',
    'generated network enforcement result parsing rejects adapter-action execution claims',
  ],
  claimsNotProved: [
    'broker-backed delivery, relay-hub delivery, or service WebSocket streaming of runtime event chains',
    'host DNS/filter mutation, firewall mutation, WFP/NetworkExtension/VpnService/nftables adapter execution, or enforcement-command execution',
    'portal UI rendering of network runtime event-chain payloads',
    'production retention, replay, delete/export, offset, dedupe, or cross-process durable event delivery',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-network-ts-event-parity-proof-ok:tests,build,public-import,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function assertSourceContracts() {
  const packageJson = readText('packages/agent-protocol-domain/package.json');
  const source = readText('packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts');
  const generator = readText('crates/schema/src/parent_agent_protocol_bridge_ts.rs');
  const rustContracts = readText('crates/agent-protocol/src/network_flow.rs');
  const rustConstants = readText('crates/agent-protocol/src/constants/network_flow.rs');

  assertIncludes(packageJson, '"./generated/agent-protocol-contracts"', 'public generated contract export exists');
  assertIncludes(source, 'export type ParentAgentNetworkFlowObservedEvent', 'network flow observed generated type exists');
  assertIncludes(source, 'export type ParentAgentNetworkDomainObservedEvent', 'network domain observed generated type exists');
  assertIncludes(source, 'export type ParentAgentNetworkActivityClassifiedEvent', 'network classification generated type exists');
  assertIncludes(source, 'export type ParentAgentNetworkAiAnalysisRequestedEvent', 'network AI request generated type exists');
  assertIncludes(source, 'export type ParentAgentNetworkPolicyDecisionCompletedEvent', 'network policy decision generated type exists');
  assertIncludes(
    source,
    'export type ParentAgentNetworkEnforcementCommandIssuedEvent',
    'network enforcement command generated type exists'
  );
  assertIncludes(source, 'export type ParentAgentNetworkAuditEntryCommittedEvent', 'network audit generated type exists');
  assertIncludes(source, 'export type ParentAgentNetworkPortalReadModelUpdatedEvent', 'network portal generated type exists');
  assertIncludes(source, 'decodeParentAgentNetworkRuntimeEventPayload', 'generated runtime decoder exists');
  assertIncludes(source, '__ParentAgentNetworkRuntimeReadRequiredBoolean', 'generated false-claim guard helper exists');
  assertIncludes(source, 'PortalReadModelUpdated', 'portal event type parity is asserted');
  assertIncludes(generator, 'decode{prefix}NetworkRuntimeEventPayload', 'Rust generator owns runtime decoder output');
  assertIncludes(rustContracts, 'pub struct NetworkPortalReadModelUpdatedEvent', 'Rust portal contract exists');
  assertIncludes(rustConstants, 'EVENT_PORTAL_READ_MODEL_UPDATED', 'Rust portal event constant exists');
}

async function assertPublicImport() {
  const publicModule = await import('@ocentra-parent/agent-protocol-domain/generated/agent-protocol-contracts');
  const eventType = publicModule.ParentAgentNetworkRuntimeEventType.NetworkFlowObserved;
  const eventTypeResult = publicModule.ParentAgentNetworkRuntimeEventTypeSchema.safeParse(eventType);
  const parsed = publicModule.decodeParentAgentNetworkRuntimeEventPayload(eventType, {
    schemaVersion: 1,
    flowEventRef: 'event.network.flow.observed.import-proof',
    observedAt: '2026-06-05T06:40:00Z',
    deviceRef: 'device.child.windows-import-proof',
    flowEvidenceRef: 'evidence.network.flow.import-proof',
    custody: 'child-device-query-store',
    evidenceGrade: 'A',
    claimBoundary: {
      exactUrlAvailable: false,
      decryptedHttpsPayloadAvailable: false,
      messageContentAvailable: false,
      searchQueryAvailable: false,
      adapterActionExecuted: false,
    },
  });
  if (!eventTypeResult.success || eventType !== 'network.flow.observed') {
    throw new Error('public generated network runtime event type import did not parse network.flow.observed');
  }
  if (parsed.flowEventRef !== 'event.network.flow.observed.import-proof') {
    throw new Error('public generated network runtime decoder did not preserve the flow event ref');
  }
  return {
    exportPath: '@ocentra-parent/agent-protocol-domain/generated/agent-protocol-contracts',
    eventType,
    parsed: eventTypeResult.success,
  };
}

function npmCommand() {
  return process.platform === 'win32' ? 'cmd' : 'npm';
}

function npmArgs(args) {
  return process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
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
