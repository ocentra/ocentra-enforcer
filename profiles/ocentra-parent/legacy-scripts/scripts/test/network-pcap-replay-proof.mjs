import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const crateRoot = join('crates', 'ocentra-network-evidence');
const crateDecisionRoot = join('output', 'network-plan-proof', '11-rust-crate-and-tooling-evaluation');
const replayRoot = join('output', 'network-plan-proof', '12-pcap-file-replay-harness');
mkdirSync(crateDecisionRoot, { recursive: true });
mkdirSync(join(replayRoot, 'fixtures'), { recursive: true });

writeFileSync(join(replayRoot, 'fixtures', 'dns-example.pcap'), dnsExamplePcap());
writeFileSync(
  join(replayRoot, 'expected-domain-evidence.json'),
  `${JSON.stringify(
    {
      queryName: 'video.example.test',
      sourceIp: '192.168.1.25',
      destinationIp: '1.1.1.1',
      sourcePort: 53000,
      destinationPort: 53,
      evidenceGrade: 'B',
      exactUrlAvailable: false,
      decryptedPayloadAvailable: false,
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(replayRoot, 'must-not-claim.json'),
  `${JSON.stringify(
    {
      networkOnlyEvidenceMustNotClaim: [
        'exact-url',
        'exact-video',
        'private-message',
        'search-query',
        'page-content',
        'screen-activity',
        'decrypted-payload',
      ],
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(crateDecisionRoot, 'tooling-decision.md'),
  [
    '# Network Rust Crate And Tooling Decision',
    '',
    'Decision: add `ocentra-network-evidence` as a reusable Rust workspace crate for deterministic metadata-only network proof fixtures.',
    '',
    'This slice does not select an external packet parsing crate or live capture binding. The local parser is intentionally bounded to classic PCAP, Ethernet, IPv4, UDP, and DNS question metadata for deterministic fixture replay.',
    '',
    'Rationale:',
    '',
    '- avoids claiming live Npcap/libpcap support before driver and permission proof exists;',
    '- keeps packet replay independent from `agent-protocol`, `agent-core`, `agent-service`, policy, adapters, and portal UI;',
    '- inherits workspace Rust license/version/lint policy;',
    '- leaves live capture, TCP/TLS/QUIC parsing, analyzer comparison, and adapter enforcement to later proof-gated workpacks.',
  ].join('\n') + '\n'
);

const commands = [
  {
    name: 'cargo-network-evidence-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence'],
    log: join(replayRoot, '01-contract-proof.log'),
  },
  {
    name: 'cargo-network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(replayRoot, '12-validation-commands.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(replayRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

const sourceSnapshot = [
  '# Network PCAP Replay Source Snapshot',
  '',
  `checkedAt: ${new Date().toISOString()}`,
  `branch: ${runText('git', ['branch', '--show-current']).trim()}`,
  `commit: ${runText('git', ['rev-parse', 'HEAD']).trim()}`,
  '',
  '```text',
  runText('git', ['status', '--short']),
  '```',
  '',
  'Inspected source paths:',
  '',
  `- ${crateRoot}`,
  '- docs/plans/network-plan/02-network-tests-proof-and-validation-blueprint.md',
  '- docs/plans/network-plan/workpacks/README.md',
  '',
  'Before-state gap: the network plan named PCAP fixture replay but the Rust workspace did not have a reusable network metadata replay crate or deterministic PCAP proof harness.',
];
writeFileSync(join(replayRoot, '00-source-snapshot.md'), `${sourceSnapshot.join('\n')}\n`);

const proof = {
  proof: 'network-pcap-replay',
  checkedAt: new Date().toISOString(),
  crateDecisionRoot,
  replayRoot,
  commands: commandResults,
  artifacts: {
    toolingDecision: join(crateDecisionRoot, 'tooling-decision.md'),
    sourceSnapshot: join(replayRoot, '00-source-snapshot.md'),
    fixture: join(replayRoot, 'fixtures', 'dns-example.pcap'),
    expectedDomainEvidence: join(replayRoot, 'expected-domain-evidence.json'),
    mustNotClaim: join(replayRoot, 'must-not-claim.json'),
  },
  provenRows: ['11 Rust crate and tooling evaluation', '12 PCAP file replay harness'],
  notClaimed: [
    'live Npcap/libpcap capture',
    'TCP stream reassembly, TLS SNI, QUIC, DoH/DoT, or analyzer comparison',
    'raw packet storage as normal product evidence',
    'exact URL, page content, search query, message, video, or decrypted payload visibility',
    'policy, adapter, or portal runtime integration',
  ],
};
writeFileSync(join(replayRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-pcap-replay-proof-ok:crate-tests,clippy,source-shape,dns-fixture');
console.log(`proof=${join(replayRoot, 'proof-summary.json')}`);

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

function dnsExamplePcap() {
  const dns = dnsPayload();
  const frame = ethernetIpv4UdpFrame(dns);
  return Buffer.concat([
    Buffer.from([0xd4, 0xc3, 0xb2, 0xa1]),
    le16(2),
    le16(4),
    le32(0),
    le32(0),
    le32(65535),
    le32(1),
    le32(1765000000),
    le32(123000),
    le32(frame.length),
    le32(frame.length),
    frame,
  ]);
}

function ethernetIpv4UdpFrame(dns) {
  const udpLength = 8 + dns.length;
  const ipLength = 20 + udpLength;
  return Buffer.concat([
    Buffer.from([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60]),
    be16(0x0800),
    Buffer.from([0x45, 0x00]),
    be16(ipLength),
    be16(0),
    be16(0),
    Buffer.from([64, 17]),
    be16(0),
    Buffer.from([192, 168, 1, 25, 1, 1, 1, 1]),
    be16(53000),
    be16(53),
    be16(udpLength),
    be16(0),
    dns,
  ]);
}

function dnsPayload() {
  return Buffer.concat([
    be16(0x1234),
    be16(0x0100),
    be16(1),
    be16(0),
    be16(0),
    be16(0),
    dnsLabel('video'),
    dnsLabel('example'),
    dnsLabel('test'),
    Buffer.from([0]),
    be16(1),
    be16(1),
  ]);
}

function dnsLabel(value) {
  const label = Buffer.from(value, 'ascii');
  return Buffer.concat([Buffer.from([label.length]), label]);
}

function le16(value) {
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16LE(value);
  return buffer;
}

function le32(value) {
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32LE(value);
  return buffer;
}

function be16(value) {
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16BE(value);
  return buffer;
}
