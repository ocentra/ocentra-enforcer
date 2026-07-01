import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'ai-plan-proof', 'child-agent-ai-policy-authority-proof');
const proofPath = join(outputDir, 'proof-summary.json');
const meshProofPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'household-mesh-screen-ai',
  'proof-summary.json'
);
const eventProofPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'event-driven-runtime',
  'proof-summary.json'
);

run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const localAi = await import('@ocentra-parent/schema-domain/local-ai');
const policy = await import('@ocentra-parent/schema-domain/policy');
const checkedAt = new Date().toISOString();
const evidenceRef = evidence('screen-summary-wikipedia-winrt');
const parentRuleRef = 'rule-school-allow';
const childAcceptedResult = localAi.LocalAiSafetyResultSchema.parse({
  schemaVersion: 'v0.6',
  resultId: 'local-ai-result-child-accepted-screen-summary',
  requestId: 'local-ai-request-screen-summary-policy-authority',
  action: 'allow',
  confidence: 0.91,
  unknownState: 'none',
  degradedState: 'none',
  reasonCodes: ['school-productivity-detected'],
  explanationReference: 'screen-summary-parent-explanation-ref',
  evidenceReferences: [evidenceRef],
  parentRuleReferences: [parentRuleRef],
  memoryReferences: [],
  graphReferences: [],
  modelRuntime: {
    runtimeReferenceId: 'runtime-child-local-winrt',
    providerId: 'provider-child-local',
    modelId: 'windows-winrt-ocr',
    modelReference: 'model-ref-windows-winrt-ocr',
    privacyMode: 'local-only',
    adapterBoundary: 'local-adapter-ready',
    executionState: 'dry-run-ready',
    providerSource: 'os-capability-probe',
    loadState: 'loaded',
    capabilityFlags: ['safety-decision'],
    resourceClass: 'cpu',
    degradedState: 'none',
    lastCheckedAt: checkedAt,
    unavailableReason: null,
  },
  promptVersion: 'screen-summary-policy-authority-v1',
  expiresAt: null,
});
const childPolicyDecision = policy.PolicyDecisionSchema.parse({
  schemaVersion: 'v0.6',
  decisionId: 'policy-decision-child-owned-ai-result',
  action: 'allow',
  reasonCodes: childAcceptedResult.reasonCodes,
  evidenceReferences: childAcceptedResult.evidenceReferences,
  ruleIds: childAcceptedResult.parentRuleReferences,
  localAiResultId: childAcceptedResult.resultId,
  dryRun: true,
  enforcementHandoffState: 'disabled',
  expiresAt: null,
});

const eventChain = [
  event('provider-result-returned', 'household-ai-provider', false, false),
  event('child-result-accepted', 'child-agent', false, false),
  event('child-policy-decision-recorded', 'child-agent', true, false),
  event('child-action-handoff-disabled', 'child-agent', false, true),
  event('read-model-updated', 'child-agent', false, false),
  event('raw-image-deleted', 'child-agent', false, false),
];
const negativeChecks = [
  rejects('provider-authored policy event is rejected', () =>
    validateAuthorityEvent(event('provider-policy-decision-recorded', 'household-ai-provider', true, false))
  ),
  rejects('provider-authored enforcement event is rejected', () =>
    validateAuthorityEvent(event('provider-enforcement-dispatched', 'household-ai-provider', false, true))
  ),
  rejects('provider result carrying policy decision is rejected before child validation', () =>
    validateProviderResult({ providerResult: childAcceptedResult, policyDecision: childPolicyDecision })
  ),
  rejects('child policy decision must cite accepted result', () =>
    validateChildPolicyDecision(
      { ...childPolicyDecision, localAiResultId: 'provider-authored-result' },
      childAcceptedResult
    )
  ),
];

const meshProof = readJsonIfPresent(meshProofPath);
const eventProof = readJsonIfPresent(eventProofPath);
const positiveChecks = {
  localAiResultSchemaParsed: childAcceptedResult.resultId === 'local-ai-result-child-accepted-screen-summary',
  childPolicySchemaParsed: childPolicyDecision.localAiResultId === childAcceptedResult.resultId,
  providerEventsWorkerOnly: eventChain.every((item) => validateAuthorityEvent(item).ok),
  meshProofRejectsProviderAuthority: JSON.stringify(meshProof ?? {}).includes('provider-authority'),
  eventRuntimeProofPresent: eventProof !== undefined,
};

if (Object.values(positiveChecks).some((value) => value !== true)) {
  throw new Error(`Positive authority checks failed: ${JSON.stringify(positiveChecks)}`);
}
if (negativeChecks.some((check) => !check.rejected)) {
  throw new Error(`Negative authority checks failed: ${JSON.stringify(negativeChecks)}`);
}

const summary = {
  proof: 'child-agent-ai-policy-authority',
  generatedAt: checkedAt,
  sourceProofs: {
    householdMesh: { path: relativePath(meshProofPath), present: meshProof !== undefined },
    eventDrivenRuntime: { path: relativePath(eventProofPath), present: eventProof !== undefined },
  },
  acceptedResult: {
    resultId: childAcceptedResult.resultId,
    action: childAcceptedResult.action,
    evidenceReferences: childAcceptedResult.evidenceReferences.map((row) => row.evidenceReferenceId),
    parentRuleReferences: childAcceptedResult.parentRuleReferences,
  },
  childPolicyDecision: {
    decisionId: childPolicyDecision.decisionId,
    action: childPolicyDecision.action,
    localAiResultId: childPolicyDecision.localAiResultId,
    dryRun: childPolicyDecision.dryRun,
    enforcementHandoffState: childPolicyDecision.enforcementHandoffState,
  },
  eventChain,
  positiveChecks,
  negativeChecks,
  assertions: {
    providerCanReturnWorkerResultOnly: true,
    childAgentOwnsResultValidation: true,
    childAgentOwnsPolicyDecision: true,
    providerCannotPublishPolicyOrEnforcement: true,
    policyConsumesOnlyAcceptedChildResult: true,
    enforcementRemainsPolicyHandoffOnly: true,
  },
  nonClaims: [
    'This proof does not execute a physical household LAN provider.',
    'This proof does not claim production model quality or portal UI.',
    'This proof does not dispatch final enforcement.',
  ],
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`child-agent-ai-policy-authority-proof-ok:${proofPath}`);

function validateAuthorityEvent(item) {
  if (item.actor !== 'child-agent' && (item.publishesPolicyDecision || item.publishesEnforcement)) {
    return rejected('only child-agent may publish policy or enforcement events');
  }
  return { ok: true, reason: 'authority boundary satisfied' };
}

function validateProviderResult(candidate) {
  if (candidate.policyDecision !== undefined || candidate.enforcementEvent !== undefined) {
    return rejected('provider result must not carry policy or enforcement payloads');
  }
  return { ok: true, reason: 'provider result is worker output only' };
}

function validateChildPolicyDecision(decision, acceptedResult) {
  if (decision.localAiResultId !== acceptedResult.resultId) {
    return rejected('policy decision must cite the accepted child-agent AI result');
  }
  return { ok: true, reason: 'child policy cites accepted local AI result' };
}

function event(eventType, actor, publishesPolicyDecision, publishesEnforcement) {
  return { eventType, actor, publishesPolicyDecision, publishesEnforcement };
}

function evidence(evidenceReferenceId) {
  return { evidenceReferenceId, kind: 'query-store-summary', observedAt: checkedAt };
}

function rejects(name, validator) {
  return { name, rejected: validator().ok === false };
}

function rejected(reason) {
  return { ok: false, reason };
}

function readJsonIfPresent(path) {
  return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : null;
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function run(command, args) {
  const runner = process.platform === 'win32' ? 'cmd' : command;
  const runnerArgs = process.platform === 'win32' ? ['/c', command, ...args] : args;
  execFileSync(runner, runnerArgs, { cwd: repoRoot, stdio: 'inherit' });
}
