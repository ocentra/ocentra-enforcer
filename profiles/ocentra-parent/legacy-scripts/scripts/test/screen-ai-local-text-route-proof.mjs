import { spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'local-text-route');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');

await mkdir(outputRoot, { recursive: true });
runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { LocalAiEvaluationInputSchema, LocalAiSafetyResultSchema } =
  await import('@ocentra-parent/schema-domain/local-ai');
const { PolicyDecisionHandoffState, PolicyDecisionSchema } = await import('@ocentra-parent/schema-domain/policy');

const observedAt = '2026-06-03T20:55:00.000Z';
const evidenceReference = {
  evidenceReferenceId: 'screen-local-text-route-evidence',
  kind: 'activity-event',
  observedAt,
};
const childProfile = {
  childProfileId: 'screen-local-text-child',
  displayName: 'Sam',
};
const device = {
  deviceId: 'screen-local-text-windows-device',
  childProfileId: childProfile.childProfileId,
  label: 'Sam Windows PC',
  platform: 'windows',
};
const modelRequest = {
  providerId: 'local-text-safety-provider',
  modelId: 'local-text-safety-model',
  promptVersion: 'screen-local-text-route-v1',
};
const memoryReference = {
  memoryReferenceId: 'screen-local-text-memory',
  kind: 'recent-activity',
  sourceEvidenceReferences: [evidenceReference],
  sourcePolicyVersion: 'policy-screen-ai-v1',
  generatedAt: '2026-06-03T20:55:01.000Z',
  confidence: 0.86,
  derivedIndexVersion: 'screen-ai-memory-index-v1',
};
const graphReference = {
  graphReferenceId: 'screen-local-text-graph',
  kind: 'graph-entity',
  sourceEvidenceReferences: [evidenceReference],
  sourcePolicyVersion: 'policy-screen-ai-v1',
  generatedAt: '2026-06-03T20:55:01.000Z',
  confidence: 0.81,
  derivedIndexVersion: 'screen-ai-graph-index-v1',
};
const ruleId = 'screen-ai-chat-risk-rule';
const resultId = 'screen-local-text-safety-result';

const evaluationInput = LocalAiEvaluationInputSchema.parse({
  schemaVersion: 'v0.6',
  requestId: 'screen-local-text-route-request',
  childProfile,
  device,
  currentObservation: {
    contextKind: 'window',
    evidence: evidenceReference,
  },
  evidenceReferences: [evidenceReference],
  parentRuleReferences: [ruleId],
  recentActivityWindow: [evidenceReference],
  memoryReferences: [memoryReference],
  graphReferences: [graphReference],
  modelRequest,
});

const safetyResult = LocalAiSafetyResultSchema.parse({
  schemaVersion: 'v0.6',
  resultId,
  requestId: evaluationInput.requestId,
  action: 'ask-parent',
  confidence: 0.87,
  unknownState: 'none',
  degradedState: 'none',
  reasonCodes: ['screen-local-text-chat-risk'],
  explanationReference: 'screen-local-text-explanation-redacted',
  evidenceReferences: [evidenceReference],
  parentRuleReferences: [ruleId],
  memoryReferences: [memoryReference],
  graphReferences: [graphReference],
  modelRuntime: {
    runtimeReferenceId: 'screen-local-text-runtime',
    providerId: modelRequest.providerId,
    modelId: modelRequest.modelId,
    modelReference: 'local-model-cache/local-text-safety-model.gguf',
    privacyMode: 'local-only',
    adapterBoundary: 'local-adapter-ready',
    executionState: 'dry-run-ready',
    providerSource: 'local-model-cache',
    loadState: 'loaded',
    capabilityFlags: ['classification', 'safety-decision'],
    resourceClass: 'cpu',
    degradedState: 'none',
    lastCheckedAt: '2026-06-03T20:55:02.000Z',
    unavailableReason: null,
  },
  promptVersion: modelRequest.promptVersion,
  expiresAt: '2026-06-03T21:05:00.000Z',
});

const policyDecision = PolicyDecisionSchema.parse({
  schemaVersion: 'v0.6',
  decisionId: 'screen-local-text-policy-decision',
  action: safetyResult.action,
  reasonCodes: safetyResult.reasonCodes,
  evidenceReferences: safetyResult.evidenceReferences,
  ruleIds: safetyResult.parentRuleReferences,
  localAiResultId: safetyResult.resultId,
  dryRun: true,
  enforcementHandoffState: PolicyDecisionHandoffState.Disabled,
  expiresAt: safetyResult.expiresAt,
});

const summary = {
  status: 'ok',
  proofKind: 'screen-ai-local-text-typed-context-route',
  artifact: artifactSummaryPath,
  evaluationInput,
  safetyResult,
  policyDecision,
  assertions: [
    'Screen-derived typed activity evidence can enter LocalAiEvaluationInputSchema with policy, memory, and graph references.',
    'A local text safety result can carry local-only runtime status and cite the same evidence references.',
    'The local AI result can hand off to a schema-valid dry-run policy decision without final enforcement.',
  ],
  nonClaims: [
    'This is a local AI text route contract proof over typed screen context.',
    'It does not claim live model inference, model quality, image analysis, or final enforcement execution.',
  ],
};

await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ai-local-text-route-proof-ok ${artifactSummaryPath}`);

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
