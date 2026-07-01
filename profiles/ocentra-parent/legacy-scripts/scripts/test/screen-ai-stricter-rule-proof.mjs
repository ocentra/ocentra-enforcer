import { spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'stricter-rule');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');

await mkdir(outputRoot, { recursive: true });
runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { PolicyAction, PolicyDecisionHandoffState, PolicyDecisionSchema, selectStricterPolicyAction } =
  await import('@ocentra-parent/schema-domain/policy');

const evidenceReference = {
  evidenceReferenceId: 'screen-ai-stricter-rule-evidence',
  kind: 'activity-event',
  observedAt: '2026-06-03T20:35:00.000Z',
};

const cases = [
  strictnessCase('parent-block-ai-allow', PolicyAction.Block, PolicyAction.Allow, PolicyAction.Block),
  strictnessCase('parent-time-limit-ai-warn', PolicyAction.TimeLimit, PolicyAction.Warn, PolicyAction.TimeLimit),
  strictnessCase('parent-ask-ai-unknown', PolicyAction.AskParent, PolicyAction.Unknown, PolicyAction.AskParent),
  strictnessCase('parent-warn-ai-block-candidate', PolicyAction.Warn, PolicyAction.Block, PolicyAction.Block),
];

const failedCases = cases.filter((entry) => entry.selectedAction !== entry.expectedAction);
if (failedCases.length > 0) {
  throw new Error(`Stricter policy action proof failed: ${JSON.stringify(failedCases)}`);
}

const parentBlockDecision = PolicyDecisionSchema.parse({
  schemaVersion: 'v0.6',
  decisionId: 'screen-ai-stricter-parent-rule-decision',
  action: cases[0].selectedAction,
  reasonCodes: ['parent-screen-block-rule'],
  evidenceReferences: [evidenceReference],
  ruleIds: ['rule-screen-block-school-video'],
  localAiResultId: 'local-ai-screen-safe-allow-result',
  dryRun: true,
  enforcementHandoffState: PolicyDecisionHandoffState.Disabled,
  expiresAt: null,
});

if (parentBlockDecision.action !== PolicyAction.Block) {
  throw new Error(`Parent block was weakened to ${parentBlockDecision.action}`);
}

const summary = {
  status: 'ok',
  proofKind: 'screen-ai-stricter-parent-rule-policy-gate',
  artifact: artifactSummaryPath,
  decision: parentBlockDecision,
  cases,
  assertion: 'A local AI recommendation cannot weaken a stricter parent policy rule before policy handoff.',
  nonClaims: [
    'This proves policy candidate selection only.',
    'It does not claim a real enforcement adapter executed the final action.',
  ],
};

await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ai-stricter-rule-proof-ok ${artifactSummaryPath}`);

function strictnessCase(caseId, parentRuleAction, localAiAction, expectedAction) {
  const selectedAction = selectStricterPolicyAction(parentRuleAction, localAiAction);
  return {
    caseId,
    parentRuleAction,
    localAiAction,
    selectedAction,
    expectedAction,
    parentRuleWeakened: selectedAction !== parentRuleAction && expectedAction === parentRuleAction,
  };
}

function runCommand(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed\n${result.stdout}\n${result.stderr}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
