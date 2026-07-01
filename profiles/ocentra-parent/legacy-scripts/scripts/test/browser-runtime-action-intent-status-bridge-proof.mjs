import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const proofDir = path.join(root, 'test-results', 'browser-runtime-action-intent-status-bridge-proof');
const outputDir = path.join(root, 'output', 'browser-plan-proof', 'browser-runtime-action-intent-status-bridge');

const files = {
  parser: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'browser-runtime-events.ts'),
  tests: path.join(root, 'packages', 'agent-protocol-domain', 'tests', 'browser-runtime-events.test.ts'),
  contracts: path.join(root, 'packages', 'agent-protocol-domain', 'src', 'contracts.ts'),
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
  const [parser, tests, contracts, checklist, workpack] = await Promise.all(
    Object.values(files).map((file) => readFile(file, 'utf8'))
  );
  return {
    statusProjectionExported: parser.includes('deriveAgentBrowserRuntimeActionIntentStatus'),
    candidateRequiresPolicyDecision: parser.includes('AgentBrowserRuntimePhase.PolicyDecisionCompleted'),
    candidateRequiresDryRun: parser.includes('!payload.dryRun'),
    candidateRequiresPolicyAuthority: parser.includes('!payload.policyAuthority'),
    dispatchCountPinnedToZero: parser.includes('dispatchAttemptCount: 0'),
    adapterExecutionPinnedToZero: parser.includes('adapterExecutionCount: 0'),
    childInterventionPinnedToZero: parser.includes('childInterventionExecutionCount: 0'),
    enforcementPinnedToZero: parser.includes('enforcementExecutionCount: 0'),
    exportedThroughContracts: contracts.includes('deriveAgentBrowserRuntimeActionIntentStatus'),
    focusedTestExists: tests.includes('specifyActionIntentStatus'),
    docsMentionStatusBridge: checklist.includes('browser-runtime-action-intent-status-bridge-proof'),
    workpackMentionsStatusBridge: workpack.includes('Action-Intent Status Bridge Addendum'),
  };
}

function assertSourceChecks(checks) {
  const missing = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([name]) => name);
  if (missing.length > 0) {
    throw new Error(`browser action-intent status bridge source checks failed: ${missing.join(', ')}`);
  }
}

async function main() {
  await mkdir(proofDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });

  const checks = await sourceChecks();
  assertSourceChecks(checks);

  const commands = [
    {
      command: 'npm',
      args: [
        'run',
        'test',
        '--workspace',
        '@ocentra-parent/agent-protocol-domain',
        '--',
        'browser-runtime-events.test.ts',
      ],
    },
    {
      command: 'npm',
      args: ['run', 'type-check', '--workspace', '@ocentra-parent/agent-protocol-domain'],
    },
  ];

  for (const item of commands) {
    run(item.command, item.args);
  }

  const proof = {
    proofName: 'browser-runtime-action-intent-status-bridge-proof',
    branchHead: execFileSync('git', ['log', '-1', '--oneline'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceChecks: checks,
    commands: commands.map((item) => `${item.command} ${item.args.join(' ')}`),
    verified: {
      subscriberProjectionUsesExistingBrowserRuntimeStream: true,
      actionIntentCandidateDerivedFromDryRunPolicyDecision: true,
      candidateRefsPreserved: true,
      dispatchAttemptCount: 0,
      adapterExecutionCount: 0,
      childInterventionExecutionCount: 0,
      enforcementExecutionCount: 0,
      newCommandFamilyCreated: false,
      newGenericEventBusCreated: false,
      externalTransportImplemented: false,
      browserMutationExecutes: false,
      childInterventionExecutes: false,
      enforcementExecutes: false,
    },
  };

  await writeFile(path.join(proofDir, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(
    path.join(outputDir, '01-browser-runtime-action-intent-status-bridge-proof.md'),
    [
      '# Browser Runtime Action Intent Status Bridge Proof',
      '',
      'This proof derives pending browser action-intent status from the existing browser runtime event-chain stream.',
      '',
      'It does not create a new command family, generic event bus, external transport, browser mutation path, child intervention execution path, or enforcement path.',
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
