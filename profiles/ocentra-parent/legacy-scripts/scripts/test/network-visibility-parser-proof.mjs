import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const tlsRoot = join('output', 'network-plan-proof', '16-tls-clienthello-sni-parser');
const httpRoot = join('output', 'network-plan-proof', '17-http-host-parser');
const quicRoot = join('output', 'network-plan-proof', '18-quic-http3-limitation-detector');
const encryptedDnsRoot = join('output', 'network-plan-proof', '19-doh-dot-detector');
for (const root of [tlsRoot, httpRoot, quicRoot, encryptedDnsRoot]) {
  mkdirSync(root, { recursive: true });
}

writeFileSync(
  join(tlsRoot, 'expected-sni-visibility.json'),
  `${JSON.stringify(
    {
      visibleSni: 'video.example.test',
      hiddenSni: null,
      exactUrlAvailable: false,
      decryptedPayloadAvailable: false,
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(httpRoot, 'expected-http-host.json'),
  `${JSON.stringify(
    {
      plaintextHost: 'video.example.test',
      httpsPayloadHostClaim: null,
      exactUrlAvailable: false,
      decryptedPayloadAvailable: false,
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(quicRoot, 'expected-quic-limitation.json'),
  `${JSON.stringify(
    {
      likelyQuic: true,
      exactDomainAvailable: false,
      decryptedPayloadAvailable: false,
    },
    null,
    2
  )}\n`
);
writeFileSync(
  join(encryptedDnsRoot, 'must-not-claim.json'),
  `${JSON.stringify(
    {
      encryptedDnsCandidateMustNotClaim: [
        'visited-domain',
        'search-query',
        'page-content',
        'exact-url',
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
    name: 'tls-sni-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'tls_parser'],
    log: join(tlsRoot, 'tls-sni-tests.log'),
  },
  {
    name: 'http-host-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'http_host_parser'],
    log: join(httpRoot, 'http-host-tests.log'),
  },
  {
    name: 'quic-limitation-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'quic_limitation'],
    log: join(quicRoot, 'quic-limitation-tests.log'),
  },
  {
    name: 'encrypted-dns-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'encrypted_dns_detector'],
    log: join(encryptedDnsRoot, 'encrypted-dns-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(encryptedDnsRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(encryptedDnsRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-visibility-parser',
  checkedAt: new Date().toISOString(),
  roots: { tlsRoot, httpRoot, quicRoot, encryptedDnsRoot },
  commands: commandResults,
  artifacts: {
    tlsSni: join(tlsRoot, 'expected-sni-visibility.json'),
    httpHost: join(httpRoot, 'expected-http-host.json'),
    quicLimitation: join(quicRoot, 'expected-quic-limitation.json'),
    encryptedDnsMustNotClaim: join(encryptedDnsRoot, 'must-not-claim.json'),
  },
  provenRows: [
    '16 TLS ClientHello/SNI parser',
    '17 HTTP Host parser',
    '18 QUIC/HTTP3 limitation detector',
    '19 DoH/DoT detector',
  ],
  notClaimed: [
    'decrypted HTTPS payloads or page contents',
    'exact URLs from TLS SNI or HTTP Host evidence',
    'visited domains behind QUIC, DoH, or DoT',
    'policy, adapter, or portal runtime integration',
  ],
};
writeFileSync(join(encryptedDnsRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-visibility-parser-proof-ok:tls,http,quic,doh-dot,clippy,source-shape');
console.log(`proof=${join(encryptedDnsRoot, 'proof-summary.json')}`);

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
