import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '24-vpn-proxy-tor-tunnel-classifier');
const testRoot = join('test-results', 'network-vpn-proxy-tunnel-classifier-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-vpn-proxy-tunnel-classifier.json'),
  `${JSON.stringify(
    {
      vpnAdapter: {
        kind: 'Vpn',
        basis: 'VpnAdapterIndicator',
        hiddenDestinationClaimed: false,
      },
      proxyPort: {
        kind: 'Proxy',
        basis: 'ProxyPortIndicator',
        hiddenDestinationClaimed: false,
      },
      torPriority: {
        kind: 'Tor',
        basis: 'TorIndicator',
        hiddenDestinationClaimed: false,
      },
      encryptedDnsNegative: {
        kind: 'Unknown',
        basis: 'EncryptedDnsOnlyNoTunnel',
        hiddenDestinationClaimed: false,
      },
      networkOnlyMustNotClaim: ['hidden-destination', 'exact-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-vpn-proxy-tunnel-classifier-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'tunnel_classifier'],
    log: join(proofRoot, 'tunnel-classifier-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-vpn-proxy-tor-tunnel-classifier',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedTunnelClassifier: join(proofRoot, 'expected-vpn-proxy-tunnel-classifier.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['24 VPN/proxy/Tor/tunnel classifier'],
  notClaimed: [
    'hidden destination identification from tunnel indicators',
    'visited URL, page content, message, search query, or decrypted payload visibility',
    'network adapter enforcement',
    'remote desktop, torrent, download classifier, AI, policy, broker, family-hub, or portal runtime integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-vpn-proxy-tunnel-classifier-proof-ok:tunnel-tests,clippy,source-shape');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

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
