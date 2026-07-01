import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '13a-live-capture-service-readiness');
const testRoot = join('test-results', 'network-live-capture-service-readiness-proof');
const commandLogRoot = join(proofRoot, 'command-logs');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });
mkdirSync(commandLogRoot, { recursive: true });

const sourceRefs = [
  'crates/agent-protocol/src/constants.rs',
  'crates/agent-protocol/src/constants/network_flow.rs',
  'crates/agent-protocol/src/lib.rs',
  'crates/agent-protocol/src/network_flow.rs',
  'crates/agent-protocol/tests/contract/network_flow_tests.rs',
  'crates/agent-protocol/tests/contract/network_live_capture_status_tests.rs',
  'crates/agent-protocol/src/transport.rs',
  'crates/agent-service/src/main.rs',
  'crates/agent-service/src/network_live_capture_readiness_bridge.rs',
  'crates/agent-service/tests/unit/network_live_capture_readiness_bridge_tests.rs',
  'crates/agent-service/src/websocket.rs',
  'packages/agent-protocol-domain/package.json',
  'packages/agent-protocol-domain/src/generated/agent-protocol-contracts.ts',
  'packages/agent-protocol-domain/tests/unit/generated-agent-protocol-contracts.test.ts',
  'packages/schema-domain/src/agent-command-event-contracts.ts',
  'packages/schema-domain/src/agent-protocol-defaults.ts',
  'packages/schema-domain/src/network-live-capture-status.ts',
  'packages/agent-protocol-domain/README.md',
  'crates/agent-protocol/README.md',
  'crates/agent-service/README.md',
  'docs/features/network-domain-control.md',
  'docs/plans/network-plan/implementation-checklist.md',
  'scripts/test/network-live-capture-service-readiness-proof.mjs',
];
const testDoubleScopeRefs = sourceRefs.filter(
  (sourceRef) => !sourceRef.startsWith('docs/') && !sourceRef.endsWith('/README.md')
);
const sourceShapeScopeRefs = testDoubleScopeRefs.filter(
  (sourceRef) => !sourceRef.includes('/generated/')
);

const boundary = {
  reportRef: 'network-live-capture-service-readiness-row13a',
  command: 'agent.network.live-capture.status.get',
  event: 'agent.network.live-capture.status.reported',
  payloadField: 'networkLiveCaptureStatus',
  sourceRows: ['13 Live pcap/Npcap/libpcap capture adapter', '03a Live capture storage custody proof'],
  requiredLiveCaptureRefs: [
    'driver proof',
    'interface enumeration',
    'permission proof',
    'bounded capture proof',
    'clean stop proof',
    'quota rotation proof',
    'retention delete export proof',
    'custody proof',
    'private traffic exclusion proof',
  ],
  requiredRawStorageRefs: [
    'raw artifact manifest',
    'local encrypted storage location',
    'encryption-at-rest verification',
    'quota rotation',
    'retention policy',
    'delete export',
    'custody chain',
    'private traffic exclusion',
  ],
  supportedStates: ['proof-ready', 'manual-required', 'unavailable', 'degraded'],
  authorityBoundary:
    'The service reports deterministic row13 and row03a proof readiness over WebSocket; it does not invoke Npcap/libpcap, capture packets, create raw artifacts, or publish policy/adapter/enforcement commands.',
};

writeJson(join(proofRoot, 'live-capture-service-readiness-boundary.json'), boundary);

const commands = [
  {
    name: 'agent-protocol-network-live-capture-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'network_live_capture'],
  },
  {
    name: 'agent-service-network-live-capture-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'network_live_capture', '--', '--test-threads=1'],
  },
  {
    name: 'agent-protocol-domain-generated-network-contract-tests',
    ...npmWorkspaceCommand('@ocentra-parent/agent-protocol-domain', [
      'run',
      'test',
      '--',
      'generated-agent-protocol-contracts.test.ts',
    ]),
  },
  {
    name: 'no-test-doubles',
    command: 'node',
    args: ['scripts/check-no-test-doubles.mjs', '--files', ...testDoubleScopeRefs],
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs', '--files', ...sourceShapeScopeRefs],
  },
];

const commandResults = commands.map(runCommand);
writeFileSync(
  join(proofRoot, 'validation-commands.log'),
  `${commandResults.map((entry) => `${entry.command}\nlog=${entry.log}`).join('\n\n')}\n`
);

const proof = {
  schemaVersion: 1,
  proof: 'network-live-capture-service-readiness',
  checkedAt: 'deterministic:row13a-live-capture-service-readiness',
  sourceRefs,
  sourceFingerprint: `source-tree:${sourceFingerprint(sourceRefs)}`,
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    boundary: join(proofRoot, 'live-capture-service-readiness-boundary.json'),
    commandLog: join(proofRoot, 'validation-commands.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['13a Live-capture service readiness/status bridge'],
  provenBoundaries: [
    'typed Rust command/event and payload field for row13 live-capture status',
    'service WebSocket route returns proof-ready/manual-required/unavailable/degraded rows',
    'row03a raw capture custody readiness refs stay tied to proof-ready row13 status',
    'Rust-generated TypeScript schema rejects stale refs, count drift, missing required refs, and no-claim upgrades',
  ],
  notClaimed: [
    'live Npcap/libpcap invocation',
    'packet capture or raw artifact creation',
    'remote upload',
    'raw PCAP without custody',
    'exact URL, decrypted payload, page content, private message, or search query visibility',
    'policy authority',
    'adapter authority or execution',
    'enforcement command publication',
    'netstat metadata as a substitute for live capture proof',
    'host filtering',
  ],
};

writeJson(join(proofRoot, 'proof-summary.json'), proof);
writeJson(join(testRoot, 'proof.json'), proof);
console.log('network-live-capture-service-readiness-proof-ok:protocol,service,typescript,guards');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const log = join(commandLogRoot, `${safeName(entry.name)}.log`);
  writeFileSync(log, normalizeCommandOutput(`${result.stdout ?? ''}${result.stderr ?? ''}`));
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}; log=${log}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log,
  };
}

function npmWorkspaceCommand(workspaceName, args) {
  if (process.platform === 'win32') {
    return { command: 'cmd', args: ['/c', 'npm', '--workspace', workspaceName, ...args] };
  }
  return { command: 'npm', args: ['--workspace', workspaceName, ...args] };
}

function sourceFingerprint(paths) {
  const hash = createHash('sha256');
  for (const path of paths) {
    hash.update(path);
    hash.update('\0');
    hash.update(readFileSync(path, 'utf8').replace(/\r\n/gu, '\n'));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function safeName(value) {
  return value.replace(/[^a-zA-Z0-9_.-]/g, '-');
}

function normalizeCommandOutput(value) {
  return `${value
    .replace(/\r\n/gu, '\n')
    .replace(/\\/gu, '/')
    .replace(/target\/debug\/deps\/[^\s)]+/gu, 'target/debug/deps/<test-binary>')
    .replace(/\b\d+\.\d+s\b/gu, '<duration>s')
    .replace(/\b\d+\.\d{2}ms\b/gu, '<duration>ms')
    .replace(/target\(s\) in [^\n]+/gu, 'target(s) in <duration>')
    .replace(/finished in [^\n]+/giu, 'finished in <duration>')
    .trim()}\n`;
}
