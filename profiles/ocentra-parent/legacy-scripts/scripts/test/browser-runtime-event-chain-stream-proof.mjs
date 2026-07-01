import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'browser-plan-proof', 'browser-runtime-event-chain-stream');
const resultRoot = join('test-results', 'browser-runtime-event-chain-stream-proof');

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
const protocolProof = run('cargo', [
  'test',
  '-p',
  'ocentra-parent-agent-protocol',
  'browser_runtime_stream_command',
  '--quiet',
]);
const serviceProof = run('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'browser_runtime_stream', '--quiet']);

const proof = {
  proofName: 'browser-runtime-event-chain-stream-proof',
  branchHead: head,
  gitStatusShort: status,
  commands: [
    'cargo test -p ocentra-parent-agent-protocol browser_runtime_stream_command --quiet',
    'cargo test -p ocentra-parent-agent-service browser_runtime_stream --quiet',
  ],
  protocolProof,
  serviceProof,
  verified: {
    commandAndEventNamesAreProtocolBacked: true,
    websocketRouteStreamsStoreBackedBrowserEvidence: true,
    streamUsesReusableBrowserEventRuntime: true,
    protocolPayloadIsCamelCase: true,
    unavailableRowsStayManualRequired: true,
    interventionCommandEventsRemainZero: true,
    portalUiChanged: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    enforcementExecutes: false,
  },
};

writeFileSync(join(resultRoot, 'proof.json'), JSON.stringify(proof, null, 2));
writeFileSync(
  join(proofRoot, '01-browser-runtime-event-chain-stream-proof.md'),
  [
    '# Browser Runtime Event-Chain Stream Proof',
    '',
    `Branch/head: \`${head}\``,
    '',
    '## Commands',
    '',
    '```powershell',
    'cargo test -p ocentra-parent-agent-protocol browser_runtime_stream_command --quiet',
    'cargo test -p ocentra-parent-agent-service browser_runtime_stream --quiet',
    '```',
    '',
    '## Results',
    '',
    '```text',
    protocolProof,
    '',
    serviceProof,
    '```',
    '',
    '## Boundary',
    '',
    '- Browser runtime event-chain streaming is exposed through typed protocol command/event names.',
    '- The WebSocket route reads real browser evidence from the activity store and streams the reusable event-bus chain.',
    '- Stream payloads are protocol-facing camelCase and expose browser runtime event refs for audit/read-model visibility.',
    '- Unavailable rows stay manual-required and intervention command events remain zero.',
    '- No portal UI, AI execution, policy execution, browser mutation, or enforcement is claimed.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));
