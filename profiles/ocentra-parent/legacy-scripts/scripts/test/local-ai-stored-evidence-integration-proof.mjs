import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const ContextProofPath = resolve(
  RepoRoot,
  'output',
  'ai-plan-proof',
  'local-ai-stored-evidence-context',
  'proof-summary.json'
);
const OutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'local-ai-stored-evidence-integration-proof');
const ProofPath = join(OutputRoot, 'proof-summary.json');
const generatedAt = new Date().toISOString();

runCommand(...npmCommand(['run', 'build:contracts']));

const { LocalAiEvaluationInputSchema } = await import('@ocentra-parent/schema-domain/local-ai');
const { runLocalAiTextInferenceDryRun } =
  await import('@ocentra-parent/schema-domain/local-ai-text-inference-dry-run-proof');

const contextProof = JSON.parse(readFileSync(ContextProofPath, 'utf8'));
const readyContextRow = contextProof.rows.find((row) => row.rowId === 'mixed-stored-evidence-ready');
if (readyContextRow === undefined || readyContextRow.contextState !== 'ready') {
  throw new Error('Expected a ready mixed stored-evidence context row');
}

const childProfile = { childProfileId: 'stored-evidence-integration-child', displayName: 'Sam' };
const device = {
  deviceId: 'stored-evidence-integration-device',
  childProfileId: childProfile.childProfileId,
  label: 'Sam Windows PC',
  platform: 'windows',
};
const modelRuntime = {
  runtimeReferenceId: readyContextRow.localModelRuntimeRefs[0],
  providerId: 'stored-evidence-integration-provider',
  modelId: 'stored-evidence-integration-model',
  modelReference: 'local-model-cache/stored-evidence-integration-model',
  privacyMode: 'local-only',
  adapterBoundary: 'local-adapter-ready',
  executionState: 'dry-run-ready',
  providerSource: 'local-model-cache',
  loadState: 'loaded',
  capabilityFlags: ['classification', 'safety-decision'],
  resourceClass: 'cpu',
  degradedState: 'none',
  lastCheckedAt: generatedAt,
  unavailableReason: null,
};
const evidenceReferences = readyContextRow.auditEvidenceRefs.map((evidenceReferenceId) => ({
  evidenceReferenceId,
  kind: 'query-store-summary',
  observedAt: generatedAt,
}));
const evaluationInput = LocalAiEvaluationInputSchema.parse({
  schemaVersion: 'v0.6',
  requestId: 'stored-evidence-integration-request',
  childProfile,
  device,
  currentObservation: {
    contextKind: 'page',
    evidence: evidenceReferences[0],
  },
  evidenceReferences,
  parentRuleReferences: readyContextRow.parentRuleReferences,
  recentActivityWindow: evidenceReferences,
  memoryReferences: [],
  graphReferences: [],
  modelRequest: {
    providerId: modelRuntime.providerId,
    modelId: modelRuntime.modelId,
    promptVersion: 'stored-evidence-integration-v1',
  },
});
const inferenceResult = runLocalAiTextInferenceDryRun({
  schemaVersion: 'v0.6',
  dryRunId: 'stored-evidence-integration-dry-run',
  evaluationInput,
  modelRuntime,
  rawPromptRetained: false,
});
const assertions = {
  storedContextReady: readyContextRow.contextState === 'ready',
  allStoredEvidenceRefsReachEvaluationInput: evidenceReferences.length === readyContextRow.auditEvidenceRefs.length,
  dryRunConsumesStoredEvidenceRefs: inferenceResult.evidenceReferenceCount === evidenceReferences.length,
  dryRunPreservesParentRuleRefs:
    inferenceResult.parentRuleReferenceCount === readyContextRow.parentRuleReferences.length,
  dryRunPreservesRuntimeRef:
    inferenceResult.result.modelRuntime.runtimeReferenceId === readyContextRow.localModelRuntimeRefs[0],
  localOnly: inferenceResult.localOnly && inferenceResult.result.modelRuntime.privacyMode === 'local-only',
  noRawPromptRetention: !inferenceResult.rawPromptRetained,
  noRemoteOrPolicyAuthority:
    !inferenceResult.remoteApiClaimed && !inferenceResult.policyAuthorityClaimed && !inferenceResult.enforcementClaimed,
};

if (Object.values(assertions).some((value) => value !== true)) {
  throw new Error(`Stored evidence integration assertions failed: ${JSON.stringify(assertions)}`);
}

const proof = {
  status: 'ok',
  proofKind: 'local-ai-stored-evidence-integration-proof',
  generatedAt,
  sourceProofs: {
    storedEvidenceContext: relativePath(ContextProofPath),
  },
  output: relativePath(ProofPath),
  contextRow: {
    rowId: readyContextRow.rowId,
    selectedEvidenceRefs: readyContextRow.selectedEvidenceRefs,
    auditEvidenceRefs: readyContextRow.auditEvidenceRefs,
    localModelRuntimeRefs: readyContextRow.localModelRuntimeRefs,
    parentRuleReferences: readyContextRow.parentRuleReferences,
    custodyLabels: readyContextRow.custodyLabels,
    validationGateSummary: readyContextRow.validationGateSummary,
  },
  evaluationInput: {
    requestId: evaluationInput.requestId,
    evidenceReferenceCount: evaluationInput.evidenceReferences.length,
    parentRuleReferenceCount: evaluationInput.parentRuleReferences.length,
    recentActivityReferenceCount: evaluationInput.recentActivityWindow.length,
    modelRequest: evaluationInput.modelRequest,
  },
  inferenceResult: {
    dryRunId: inferenceResult.dryRunId,
    state: inferenceResult.state,
    action: inferenceResult.result.action,
    confidence: inferenceResult.result.confidence,
    evidenceReferenceCount: inferenceResult.evidenceReferenceCount,
    parentRuleReferenceCount: inferenceResult.parentRuleReferenceCount,
    localOnly: inferenceResult.localOnly,
    dryRunOnly: inferenceResult.dryRunOnly,
    modelExecuted: inferenceResult.modelExecuted,
    remoteApiClaimed: inferenceResult.remoteApiClaimed,
    policyAuthorityClaimed: inferenceResult.policyAuthorityClaimed,
    enforcementClaimed: inferenceResult.enforcementClaimed,
    productionModelQualityClaimed: inferenceResult.productionModelQualityClaimed,
    rawPromptRetained: inferenceResult.rawPromptRetained,
  },
  assertions,
  nonClaims: [
    'This proof integrates existing stored-evidence context output with the local AI dry-run contract.',
    'It does not create fresh captures, execute a production model, prove model quality, render portal UI, or dispatch enforcement.',
    'It does not retain raw prompt text, raw model output, or raw screenshots, and it does not use remote/API AI.',
  ],
};

mkdirSync(OutputRoot, { recursive: true });
writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`local-ai-stored-evidence-integration-proof-ok:${relativePath(ProofPath)}`);

function relativePath(filePath) {
  return relative(RepoRoot, filePath).replaceAll('\\', '/');
}

function runCommand(command, args) {
  execFileSync(command, args, { cwd: RepoRoot, stdio: 'inherit' });
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
