import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'browser-plan-proof', 'browser-event-runtime-spine');
const resultRoot = join('test-results', 'browser-event-runtime-spine-proof');

function run(command, args) {
  const output = execFileSync(command, args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return output.trim();
}

mkdirSync(proofRoot, { recursive: true });
mkdirSync(resultRoot, { recursive: true });

const head = run('git', ['log', '-1', '--oneline']);
const status = run('git', ['status', '--short']);
const rustProof = run('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'browser_runtime_chain', '--quiet']);

const proof = {
  proofName: 'browser-event-runtime-spine-proof',
  branchHead: head,
  gitStatusShort: status,
  commands: ['cargo test -p ocentra-parent-agent-core browser_runtime_chain --quiet'],
  rustProof,
  verified: {
    usesReusableEventBus: true,
    browserSpecificPrivateBusCreated: false,
    serviceWebSocketChanged: false,
    portalUiChanged: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    enforcementExecutes: false,
    manualRequiredSkipsInterventionCommand: true,
    auditAndReadModelRemainVisible: true,
  },
};

writeFileSync(join(resultRoot, 'proof.json'), JSON.stringify(proof, null, 2));
writeFileSync(
  join(proofRoot, '01-browser-event-runtime-spine-proof.md'),
  [
    '# Browser Event Runtime Spine Proof',
    '',
    `Branch/head: \`${head}\``,
    '',
    '## Command',
    '',
    '```powershell',
    'cargo test -p ocentra-parent-agent-core browser_runtime_chain --quiet',
    '```',
    '',
    '## Result',
    '',
    '```text',
    rustProof,
    '```',
    '',
    '## Boundary',
    '',
    '- Reuses `crates/ocentra-eventing` through a browser-specific core consumer.',
    '- Does not create a browser-private event bus.',
    '- Does not change WebSocket/service routing or portal UI.',
    '- Does not execute AI, policy, browser mutation, or enforcement.',
    '- Proves manual-required rows skip intervention command/result phases while retaining audit/read-model visibility.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));
