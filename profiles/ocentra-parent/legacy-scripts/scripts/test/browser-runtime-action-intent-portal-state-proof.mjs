import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-portal-state-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-portal-state');

const files = {
  parser: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'browser-runtime-events.ts'),
  parserTests: path.join(root, 'packages', 'agent-protocol-domain', 'tests', 'browser-runtime-events.test.ts'),
  portalTests: path.join(root, 'apps', 'portal', 'tests', 'live-activity-state.test.ts'),
  checklist: path.join(root, 'docs', 'plans', 'browser-plan', 'implementation-checklist.md'),
  workpack: path.join(
    root,
    'docs',
    'plans',
    'browser-plan',
    'workpacks',
    '13-browser-read-models-and-service-events.md'
  ),
};

function run(command, args) {
  execFileSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
}

async function sourceChecks() {
  const [parser, parserTests, portalTests, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );

  return {
    parserReadsCandidateCount: parser.includes('BrowserRuntimeActionIntentCandidates'),
    parserRejectsDispatchAttempt:
      parser.includes('BrowserRuntimeActionIntentDispatchAttempts') && parser.includes('Schema.Literal(0)'),
    parserRejectsAdapterExecution:
      parser.includes('BrowserRuntimeActionIntentAdapterExecutions') &&
      parser.includes('actionIntentAdapterExecutions'),
    parserRejectsChildInterventionExecution:
      parser.includes('BrowserRuntimeActionIntentChildInterventionExecutions') &&
      parser.includes('actionIntentChildInterventionExecutions'),
    parserRejectsEnforcementExecution:
      parser.includes('BrowserRuntimeActionIntentEnforcementExecutions') &&
      parser.includes('actionIntentEnforcementExecutions'),
    parserTestsCoverCounters:
      parserTests.includes('actionIntentCandidates') &&
      parserTests.includes('BrowserRuntimeActionIntentDispatchAttempts'),
    portalTestsCoverCounters:
      portalTests.includes('actionIntentCandidates') &&
      portalTests.includes('rejects browser runtime event-chain streams that claim action execution'),
    docsMentionPortalStateProof: checklist.includes('browser-runtime-action-intent-portal-state-proof'),
    workpackMentionsPortalStateProof: workpack.includes('Action-Intent Portal State Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent portal state proof failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const commands = [
    {
      command: 'cmd',
      args: ['/c', 'npm', 'run', 'build', '--workspace', '@ocentra-parent/agent-protocol-domain'],
    },
    {
      command: 'cmd',
      args: [
        '/c',
        'npm',
        'run',
        'test',
        '--workspace',
        '@ocentra-parent/agent-protocol-domain',
        '--',
        'browser-runtime-events.test.ts',
      ],
    },
    {
      command: 'cmd',
      args: ['/c', 'npm', 'run', 'test', '--workspace', '@ocentra-parent/portal', '--', 'live-activity-state.test.ts'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const proof = {
    proofName: 'browser-runtime-action-intent-portal-state-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      parserConsumesServiceCounters: true,
      portalStateConsumesServiceCounters: true,
      pendingCandidateCountAllowed: true,
      dispatchAttemptCountRequiredZero: true,
      adapterExecutionCountRequiredZero: true,
      childInterventionExecutionCountRequiredZero: true,
      enforcementExecutionCountRequiredZero: true,
      finalPolicyExecutionClaimed: false,
      browserMutationClaimed: false,
      childInterventionClaimed: false,
      enforcementClaimed: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-portal-state-proof.md'),
    [
      '# Browser Runtime Action Intent Portal State Proof',
      '',
      'This proof verifies that the shared TypeScript protocol parser and portal live-activity state consume browser runtime action-intent service counters from the existing event-chain stream payload.',
      '',
      'The parser allows pending candidate counts but rejects dispatch attempts, adapter execution, child intervention execution, and enforcement execution. The portal state test uses the same shared parser, so stale local shapes cannot bypass the protocol contract.',
      '',
      'No new portal visual surface, browser mutation, child intervention execution, final policy execution, or enforcement is claimed.',
      '',
      'Validation:',
      ...proof.commands.map((command) => `- \`${command}\``),
      '',
    ].join('\n')
  );

  console.log(JSON.stringify(proof, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
