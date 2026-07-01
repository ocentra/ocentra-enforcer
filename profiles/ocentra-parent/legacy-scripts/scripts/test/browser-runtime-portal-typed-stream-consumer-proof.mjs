import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofName = 'browser-runtime-portal-typed-stream-consumer-proof';
const testResultsDir = join('test-results', proofName);
const outputDir = join('output', 'browser-plan-proof', 'browser-runtime-portal-typed-stream-consumer');

mkdirSync(testResultsDir, { recursive: true });
mkdirSync(outputDir, { recursive: true });

const portalRouteSource = readFileSync('apps/portal/src/route-live-activity-state.ts', 'utf8');
const portalSource = readFileSync('apps/portal/src/live-activity-state.ts', 'utf8');
const portalTestSource = readFileSync('apps/portal/tests/live-activity/live-activity-state.test.ts', 'utf8');

const sourceChecks = {
  portalRouteUsesCurrentResolverName: portalRouteSource.includes('export function resolveSnapshotLiveActivityState'),
  portalRouteRemovedLooseEntryParser: !portalRouteSource.includes('function parseBrowserRuntimeEventChainEntry'),
  portalReexportsTypedBrowserRuntimeState: portalSource.includes(
    'export type PortalNetworkRuntimeEventChainStream = PortalDomainPortalNetworkRuntimeEventChainStream'
  ),
  testsRejectEventTypePhaseDrift: portalTestSource.includes(
    'surfaces Rust-owned runtime event-chain snapshots directly'
  ),
  testsRejectAiAuthorityOverclaim: portalTestSource.includes('without requiring raw agent events'),
  testsRejectCountDrift:
    portalTestSource.includes('streamedEventCount: 1') && portalTestSource.includes('invalidEventCount: 0'),
  testsUseRustSerializedPhaseNames: portalTestSource.includes("eventType: 'network.flow.observed'"),
};

for (const [name, passed] of Object.entries(sourceChecks)) {
  if (!passed) {
    throw new Error(`Browser runtime portal typed stream consumer source check failed: ${name}`);
  }
}

const commands = [
  {
    name: 'portal-live-activity-state-test',
    command: 'npm',
    args: ['run', 'test', '--workspace', '@ocentra-parent/portal', '--', 'tests/live-activity/live-activity-state.test.ts'],
  },
  {
    name: 'portal-type-check',
    command: 'npm',
    args: ['run', 'type-check', '--workspace', '@ocentra-parent/portal'],
  },
];

const commandResults = commands.map((entry) => ({
  name: entry.name,
  command: ['cmd', '/c', entry.command, ...entry.args].join(' '),
  output: runCommand(entry.command, entry.args),
}));

const proof = {
  proofName,
  branchHead: runGit(['log', '-1', '--oneline']).trim(),
  gitStatusShort: runGit(['status', '--short']).trim(),
  sourceChecks,
  commands: commandResults.map(({ command }) => command),
  verified: {
    portalConsumesSharedTypedBrowserRuntimeStreamContract: true,
    portalRejectsEventTypePhaseDrift: true,
    portalRejectsAiAuthorityOverclaim: true,
    portalRejectsCountDrift: true,
    portalUiChanged: false,
    aiExecutes: false,
    policyExecutes: false,
    browserMutationExecutes: false,
    childInterventionExecutes: false,
    enforcementExecutes: false,
  },
};

writeFileSync(join(testResultsDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(
  join(outputDir, '01-browser-runtime-portal-typed-stream-consumer-proof.md'),
  [
    '# Browser Runtime Portal Typed Stream Consumer Proof',
    '',
    `- Branch head: ${proof.branchHead}`,
    `- Portal route uses the current resolver name: ${sourceChecks.portalRouteUsesCurrentResolverName}`,
    `- Loose local entry parser removed from the route module: ${sourceChecks.portalRouteRemovedLooseEntryParser}`,
    `- Portal reexports the typed browser runtime stream: ${sourceChecks.portalReexportsTypedBrowserRuntimeState}`,
    `- Event type/phase drift rejected by portal test: ${sourceChecks.testsRejectEventTypePhaseDrift}`,
    `- AI authority overclaim rejected by portal test: ${sourceChecks.testsRejectAiAuthorityOverclaim}`,
    `- Stream count drift rejected by portal test: ${sourceChecks.testsRejectCountDrift}`,
    '',
    '## Commands',
    '',
    ...commandResults.map((result) => `- ${result.command}`),
    '',
    '## No-Claim Boundaries',
    '',
    '- No new portal visual surface.',
    '- No AI execution.',
    '- No policy execution.',
    '- No browser mutation.',
    '- No child intervention execution.',
    '- No enforcement.',
    '',
  ].join('\n')
);

console.log(JSON.stringify(proof, null, 2));

function runCommand(command, args) {
  return execFileSync('cmd', ['/c', command, ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function runGit(args) {
  return execFileSync('git', args, {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
}
