import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(new URL('..', import.meta.url)), '..');
const outputRoot = join(repoRoot, 'output', 'ai-plan-proof', 'lan-ai-provider-heartbeat-runtime-proof');
const testResultRoot = join(repoRoot, 'test-results', 'lan-ai-provider-heartbeat-runtime-proof');
mkdirSync(outputRoot, { recursive: true });
mkdirSync(testResultRoot, { recursive: true });

const commands = [
  ['cargo', ['test', '-p', 'ocentra-parent-agent-service', 'lan_ai', '--', '--nocapture']],
  ['cargo', ['test', '-p', 'ocentra-parent-agent-service', 'provider_selection_read_model', '--', '--nocapture']],
];

const logs = [];
for (const [command, args] of commands) {
  const result = spawnSync(command, args, { cwd: repoRoot, encoding: 'utf8' });
  logs.push(`$ ${command} ${args.join(' ')}`);
  logs.push(result.stdout);
  logs.push(result.stderr);
  if (result.status !== 0) {
    writeFileSync(join(testResultRoot, 'cargo-test.log'), logs.filter(Boolean).join('\n'));
    throw new Error(`LAN AI provider heartbeat runtime command failed: ${command} ${args.join(' ')}`);
  }
}
writeFileSync(join(testResultRoot, 'cargo-test.log'), logs.filter(Boolean).join('\n'));

const sourceFiles = [
  join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing_runtime_state', 'provider_heartbeat.rs'),
  join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing_runtime_state', 'provider_routing.rs'),
  join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing_runtime_state', 'device_roles.rs'),
  join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing', 'lan_ai_provider_heartbeat_tests.rs'),
  join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing_provider_selection_read_model_tests.rs'),
];
const combinedSource = sourceFiles.map((filePath) => readFileSync(filePath, 'utf8')).join('\n');
for (const required of [
  'LanAiProviderHeartbeatState',
  'lan_ai_provider_heartbeat_reachability',
  'lan_ai_provider_heartbeat_allows_routing',
  'mark_lan_ai_provider_heartbeat_stale_for_test',
  'mark_lan_ai_provider_heartbeat_offline_for_test',
  'LAN_AI_PROVIDER_STATUS_DEGRADED',
  'LAN_AI_PROVIDER_STATUS_UNAVAILABLE',
  'DegradeProviderUnavailable',
]) {
  if (!combinedSource.includes(required)) {
    throw new Error(`missing LAN AI provider heartbeat requirement: ${required}`);
  }
}

const proof = {
  proofKind: 'lan-ai-provider-heartbeat-runtime-proof',
  proofTier: 'P2_SERVICE_RUNTIME_STATE',
  commands: commands.map(([command, args]) => `${command} ${args.join(' ')}`),
  result: 'pass',
  sourceBehavior: [
    'LanPairingRuntime now owns LAN AI provider heartbeat reachability state.',
    'Stale provider heartbeat degrades provider status/routing and keeps screen-derived LAN AI jobs out of completed provider-result state.',
    'Offline provider heartbeat marks provider status/routing unavailable and degrades the job without raw screen transfer.',
    'Provider selection read model no longer selects a stale-heartbeat provider as an authorized route.',
  ],
  sourceFiles: sourceFiles.map((filePath) => relative(repoRoot, filePath)),
  testEvidence: {
    log: relative(repoRoot, join(testResultRoot, 'cargo-test.log')),
    asserts: [
      'stale heartbeat produces lan-ai-provider-degraded and degraded routing',
      'offline heartbeat produces lan-ai-provider-unavailable and unavailable routing',
      'job state is degraded rather than completed for stale/offline heartbeat',
      'raw LAN AI markers are absent from service event payloads',
      'provider selection read model omits selected_provider_route_id for stale-heartbeat provider',
    ],
  },
  nonClaims: [
    'Does not implement physical LAN sockets, mDNS, multicast, or provider gossip transport.',
    'Does not execute a production model or claim model quality.',
    'Does not grant provider policy or enforcement authority.',
    'Does not transfer raw screenshots to a provider.',
    'Does not close authenticated-account social, physical Android, iOS, macOS, or final product-complete gates.',
  ],
};

const proofPath = join(outputRoot, 'proof-summary.json');
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testResultRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`lan-ai-provider-heartbeat-runtime-proof-ok:${relative(repoRoot, proofPath)}`);
