import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(new URL('..', import.meta.url)), '..');
const outputRoot = join(repoRoot, 'output', 'ai-plan-proof', 'lan-ai-household-route-metadata-proof');
const testResultRoot = join(repoRoot, 'test-results', 'lan-ai-household-route-metadata-proof');
mkdirSync(outputRoot, { recursive: true });
mkdirSync(testResultRoot, { recursive: true });

const cargo = spawnSync(
  'cargo',
  ['test', '-p', 'ocentra-parent-agent-service', 'lan_ai_route_metadata', '--', '--nocapture'],
  { cwd: repoRoot, encoding: 'utf8' }
);

const testLog = [cargo.stdout, cargo.stderr].filter(Boolean).join('\n');
writeFileSync(join(testResultRoot, 'cargo-test.log'), testLog);
if (cargo.status !== 0) {
  throw new Error(`lan AI route metadata cargo test failed: ${cargo.status}`);
}

const sourcePath = join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing', 'lan_ai_route_metadata.rs');
const testPath = join(repoRoot, 'crates', 'agent-service', 'src', 'lan_pairing', 'lan_ai_route_metadata_tests.rs');
const source = readFileSync(sourcePath, 'utf8');
const tests = readFileSync(testPath, 'utf8');

for (const required of [
  'select_household_ai_provider_route',
  'POLICY_AUTHORITY_CHILD_AGENT_ONLY',
  'LAN_AI_RAW_SCREEN_TRANSFERRED',
  'LAN_AI_CHILD_VALIDATES_PROVIDER_RESULT',
  'LAN_AI_CLAIM_ID',
  'LAN_AI_LEASE_ID',
]) {
  if (!source.includes(required) && !tests.includes(required)) {
    throw new Error(`missing route metadata requirement: ${required}`);
  }
}

const proof = {
  proofKind: 'lan-ai-household-route-metadata-proof',
  proofTier: 'P2_SERVICE_RUNTIME_METADATA',
  command: 'cargo test -p ocentra-parent-agent-service lan_ai_route_metadata -- --nocapture',
  result: 'pass',
  sourceBehavior: [
    'AgentLanAiJobSubmit emits service-facing household provider route metadata after authorization.',
    'The event records selected provider peer, selected route reason, claim id, lease id, and child-agent-only policy authority.',
    'The event records providerCanPublishPolicy=false, rawScreenTransferred=false, and childValidatesProviderResult=true.',
  ],
  sourceFiles: [
    relative(repoRoot, sourcePath),
    relative(repoRoot, testPath),
    'crates/agent-service/src/lan_pairing/lan_ai_job.rs',
    'crates/agent-service/src/lan_pairing.rs',
    'crates/agent-protocol/src/constants/field.rs',
    'crates/agent-protocol/src/constants/lan_pairing.rs',
  ],
  testEvidence: {
    log: relative(repoRoot, join(testResultRoot, 'cargo-test.log')),
    asserts: [
      'selected provider peer is the local physical AI provider',
      'route reason is selected-desktop-provider',
      'policy authority remains child-agent-only',
      'provider policy publishing is false',
      'raw screen transfer is false',
      'child result validation is true',
      'raw evidence markers are absent',
    ],
  },
  nonClaims: [
    'Does not execute a physical household LAN socket or gossip transport.',
    'Does not execute a production model or claim model quality.',
    'Does not grant provider policy or enforcement authority.',
    'Does not transfer raw screenshots to a provider.',
    'Does not close the final product-complete screen AI gate.',
  ],
};

const proofPath = join(outputRoot, 'proof-summary.json');
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testResultRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log(`lan-ai-household-route-metadata-proof-ok:${relative(repoRoot, proofPath)}`);
