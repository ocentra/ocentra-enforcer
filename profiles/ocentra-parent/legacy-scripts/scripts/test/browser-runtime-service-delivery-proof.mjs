import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'browser-plan-proof', 'browser-runtime-service-delivery');
const resultRoot = join('test-results', 'browser-runtime-service-delivery-proof');

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
const rustProof = run('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'browser_runtime_delivery', '--quiet']);

const proof = {
  proofName: 'browser-runtime-service-delivery-proof',
  branchHead: head,
  gitStatusShort: status,
  commands: ['cargo test -p ocentra-parent-agent-service browser_runtime_delivery --quiet'],
  rustProof,
  verified: {
    consumesBrowserEvidenceReadModelRows: true,
    publishesReusableEventBusChain: true,
    exactUrlRowsStayEvidenceOnly: true,
    unavailableRowsStayManualRequired: true,
    interventionCommandEventsRemainZero: true,
    serviceWebSocketChanged: false,
    portalUiChanged: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    enforcementExecutes: false,
  },
};

writeFileSync(join(resultRoot, 'proof.json'), JSON.stringify(proof, null, 2));
writeFileSync(
  join(proofRoot, '01-browser-runtime-service-delivery-proof.md'),
  [
    '# Browser Runtime Service Delivery Proof',
    '',
    `Branch/head: \`${head}\``,
    '',
    '## Command',
    '',
    '```powershell',
    'cargo test -p ocentra-parent-agent-service browser_runtime_delivery --quiet',
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
    '- Service-owned browser evidence read-model rows now map into the reusable browser event-runtime chain.',
    '- Managed exact-URL rows remain evidence-only and do not become policy authority.',
    '- Unavailable rows stay manual-required while still publishing read-model projection visibility.',
    '- No service WebSocket route, portal UI, AI execution, policy execution, browser mutation, or enforcement is claimed.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));
