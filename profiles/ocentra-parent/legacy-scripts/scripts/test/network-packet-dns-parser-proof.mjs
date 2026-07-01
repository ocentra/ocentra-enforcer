import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const packetRoot = join('output', 'network-plan-proof', '14-packet-parser');
const dnsRoot = join('output', 'network-plan-proof', '15-dns-query-response-parser');
mkdirSync(packetRoot, { recursive: true });
mkdirSync(dnsRoot, { recursive: true });

writeFileSync(
  join(packetRoot, 'expected-packet-fixtures.json'),
  `${JSON.stringify(
    {
      ethernet: {
        sourceMac: '10:20:30:40:50:60',
        destinationMac: 'aa:bb:cc:dd:ee:ff',
        etherType: '0x0800',
      },
      ipv4: {
        udpDns: { sourceIp: '192.168.1.25', destinationIp: '1.1.1.1' },
        tcpSyn: { sourcePort: 53001, destinationPort: 443 },
        icmpEcho: { icmpType: 8, code: 0 },
      },
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(dnsRoot, 'expected-dns-response.json'),
  `${JSON.stringify(
    {
      transactionId: '0x1234',
      queryName: 'video.example.test',
      queryType: 'A',
      answer: { recordType: 'A', address: '203.0.113.7', ttlSeconds: 300 },
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(dnsRoot, 'must-not-claim.json'),
  `${JSON.stringify(
    {
      parserOnlyEvidenceMustNotClaim: [
        'live-packet-capture',
        'exact-url',
        'page-content',
        'decrypted-payload',
        'policy-action',
        'adapter-apply',
      ],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'packet-parser-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'packet_parser'],
    log: join(packetRoot, 'packet-parser-tests.log'),
  },
  {
    name: 'dns-parser-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'dns_parser'],
    log: join(dnsRoot, 'dns-parser-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(dnsRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(dnsRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-packet-dns-parser',
  checkedAt: new Date().toISOString(),
  packetRoot,
  dnsRoot,
  commands: commandResults,
  artifacts: {
    packetExpectations: join(packetRoot, 'expected-packet-fixtures.json'),
    dnsResponseExpectations: join(dnsRoot, 'expected-dns-response.json'),
    mustNotClaim: join(dnsRoot, 'must-not-claim.json'),
  },
  provenRows: ['14 Packet parser', '15 DNS query/response parser'],
  notClaimed: [
    'live Npcap/libpcap capture',
    'TLS, QUIC, DoH, HTTP host, analyzer comparison, or process correlation',
    'exact URL, page content, message, search query, or decrypted payload visibility',
    'policy, adapter, or portal runtime integration',
  ],
};
writeFileSync(join(dnsRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-packet-dns-parser-proof-ok:packet-fixtures,dns-response,clippy,source-shape');
console.log(`proof=${join(dnsRoot, 'proof-summary.json')}`);

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
