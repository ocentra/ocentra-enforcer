import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';

const proofRoot = join('output', 'browser-plan-proof', 'browser-runtime-portal-stream-consumer');
const resultRoot = join('test-results', 'browser-runtime-portal-stream-consumer-proof');

function run(command, args) {
  return execFileSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

await mkdir(proofRoot, { recursive: true });
await mkdir(resultRoot, { recursive: true });

const commands = [
  {
    name: 'agent-protocol-domain-build',
    command: 'cmd',
    args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/agent-protocol-domain'],
  },
  {
    name: 'portal-domain-build',
    command: 'cmd',
    args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/portal-domain'],
  },
  {
    name: 'agent-protocol-domain-contracts',
    command: 'cmd',
    args: [
      '/c',
      'npm',
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/agent-protocol-domain',
      '--',
      'tests/unit/contracts.test.ts',
    ],
  },
  {
    name: 'portal-domain-command-contracts',
    command: 'cmd',
    args: [
      '/c',
      'npm',
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal-domain',
      '--',
      'tests/unit/contracts.test.ts',
    ],
  },
  {
    name: 'portal-live-activity-browser-stream-state',
    command: 'cmd',
    args: [
      '/c',
      'npm',
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/portal',
      '--',
      'tests/live-activity-state.test.ts',
      'tests/live-activity-browser-status.test.ts',
    ],
  },
];

const logs = [];
for (const item of commands) {
  const output = run(item.command, item.args);
  logs.push({ name: item.name, command: [item.command, ...item.args].join(' '), output });
}

const proof = {
  proofName: 'browser-runtime-portal-stream-consumer-proof',
  branchHead: run('git', ['log', '-1', '--oneline']),
  gitStatusShort: run('git', ['status', '--short']),
  verified: {
    portalOverviewRequestsBrowserRuntimeStream: true,
    commandResultAcceptsBrowserRuntimeStreamEvent: true,
    portalLiveStateParsesBrowserRuntimeStream: true,
    manualRequiredRowsStayVisible: true,
    interventionCommandEventsRemainZero: true,
    portalUiSurfaceAdded: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    enforcementExecutes: false,
  },
  commands: logs.map((entry) => ({ name: entry.name, command: entry.command })),
};

await writeFile(join(resultRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
await writeFile(
  join(proofRoot, '01-browser-runtime-portal-stream-consumer-proof.md'),
  [
    '# Browser Runtime Portal Stream Consumer Proof',
    '',
    `Branch head: ${proof.branchHead}`,
    '',
    '## Verified',
    '',
    '- Parent portal overview command list requests `agent.browser.runtime.event-chain.stream.get`.',
    '- Command-result routing accepts `agent.browser.runtime.event-chain.stream.reported`.',
    '- Portal live activity state parses the service event-chain stream payload.',
    '- Manual-required rows remain visible and intervention command events remain zero.',
    '',
    '## Non-Claims',
    '',
    '- No new portal visual surface is added.',
    '- No AI execution, policy execution, browser mutation, child intervention execution, or enforcement is claimed.',
    '',
    '## Commands',
    '',
    ...logs.map((entry) => `- ${entry.command}`),
    '',
  ].join('\n'),
  'utf8'
);

console.log(JSON.stringify(proof, null, 2));
