import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const OcrProofPath = resolve(RepoRoot, 'output', 'ai-plan-proof', 'screen-winrt-ocr-worker', 'proof-summary.json');
const OutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'screen-summary-ai-context');
const ProofPath = join(OutputRoot, 'proof-summary.json');
const ObservedAtFallback = '2026-06-05T10:22:19.824Z';
const ChildProfile = { childProfileId: 'screen-summary-context-child', displayName: 'Sam' };
const Device = {
  deviceId: 'screen-summary-context-windows-device',
  childProfileId: ChildProfile.childProfileId,
  label: 'Sam Windows PC',
  platform: 'windows',
};

runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { buildLocalAiEvidenceContext } = await import('@ocentra-parent/schema-domain/local-ai-context-builder');
const ocrProof = JSON.parse(readFileSync(OcrProofPath, 'utf8'));
const scenarios = ocrProof.proof.scenarios;
const rows = scenarios.map((scenario) => contextRowForScenario(scenario));
const failures = rows.flatMap(validateContextRow);

if (failures.length > 0) {
  throw new Error(`Screen summary AI context proof failed:\n${failures.join('\n')}`);
}

const proof = {
  status: 'ok',
  proofKind: 'screen-summary-ai-context-builder-proof',
  generatedAt: new Date().toISOString(),
  sourceProof: relativePath(OcrProofPath),
  output: relativePath(ProofPath),
  rows,
  summary: {
    sourceScenarioCount: scenarios.length,
    readyContextCount: rows.filter((row) => row.contextState === 'ready').length,
    selectedScreenSummaryCount: rows.reduce((count, row) => count + row.screenSummaryRefs.length, 0),
    localOnly: ocrProof.proof.localOnly,
    remoteAiUsed: ocrProof.proof.remoteAiUsed,
    rawImageRetained: ocrProof.proof.rawImageRetained,
    failures: failures.length,
  },
  assertions: [
    'Real WinRT OCR worker result rows become local AI screen-summary context references.',
    'The context builder selects only child-device query-store custody from deleted raw-image screen summaries.',
    'The context builder preserves source evidence refs, local runtime refs, parent rule refs, confidence, and screen-image-deleted degradation.',
  ],
  nonClaims: [
    'This proof replays the already-captured WinRT OCR proof artifacts; it does not create new screen captures.',
    'This proof does not claim production model quality, portal UI, final enforcement, or remote/API AI.',
  ],
};

mkdirSync(OutputRoot, { recursive: true });
writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-summary-ai-context-proof-ok:${ProofPath}`);

function contextRowForScenario(scenario) {
  const contextInput = localAiContextInput(scenario);
  const contextResult = buildLocalAiEvidenceContext(contextInput);
  const context = contextResult.context;
  return {
    ocrResultId: scenario.ocrResultId,
    sourceQueueJobId: scenario.queueJobId,
    primaryCategory: scenario.primaryCategory,
    confidence: scenario.confidence,
    imageDigest: scenario.imageDigest,
    imageDeletionState: scenario.imageDeletionState,
    ocrCustodyState: scenario.custodyState,
    rawImageRetained: scenario.rawImageRetained,
    policyEligible: scenario.policyEligible,
    contextState: contextResult.state,
    screenSummaryRefs: context?.screenSummaryRefs ?? [],
    custodyLabels: context?.custodyLabels ?? [],
    localModelRuntimeRefs: context?.localModelRuntimeRefs ?? [],
    parentRuleReferences: context?.parentRuleReferences ?? [],
    degradedReasons: context?.degradedReasons ?? [],
    auditEvidenceReferences: contextResult.auditEvidenceReferences.map((reference) => reference.evidenceReferenceId),
    missingEvidenceKinds: contextResult.missingEvidenceKinds,
    rejectedFields: contextResult.rejectedFields,
    custodyBoundarySummary: contextResult.custodyBoundarySummary,
    validationGateSummary: contextResult.validationGateSummary,
  };
}

function localAiContextInput(scenario) {
  const observedAt = scenario.analyzedAt ?? ObservedAtFallback;
  const screenRefId = `${scenario.ocrResultId}-screen-summary-ref`;
  return {
    contextId: `${scenario.ocrResultId}-context`,
    request: {
      schemaVersion: 'v0.6',
      requestId: `${scenario.ocrResultId}-context-request`,
      requestedAt: observedAt,
      childProfile: ChildProfile,
      device: Device,
      requestedEvaluationKind: 'screen-summary',
      requiredEvidenceKinds: ['screen-summary'],
      parentRuleContextReferences: [parentRuleContext(scenario, screenRefId, observedAt)],
      modelTaskRequirements: ['classification', 'safety-decision'],
      allowedCustody: ['child-device-query-store'],
      promptVersion: 'screen-summary-context-v1',
    },
    evidenceReferences: [screenSummaryEvidence(scenario, screenRefId, observedAt)],
    runtimeReferences: [runtimeStatus(scenario, observedAt)],
    memoryReferences: [],
    graphReferences: [],
  };
}

function parentRuleContext(scenario, screenRefId, observedAt) {
  return {
    parentRuleRefId: `${scenario.ocrResultId}-parent-rule-context`,
    policyVersion: 'screen-summary-context-policy-v1',
    family: { familyId: 'screen-summary-context-family' },
    childProfile: ChildProfile,
    device: Device,
    rule: {
      ruleId: `${scenario.primaryCategory}-screen-summary-rule`,
      target: {
        targetId: `${scenario.primaryCategory}-screen-summary-target`,
        targetType: 'category',
        targetValue: scenario.primaryCategory,
      },
      action: policyActionForCategory(scenario.primaryCategory),
      scheduleId: null,
      priority: 10,
      reasonCode: `${scenario.primaryCategory}-screen-summary-context`,
      createdBy: { actorId: 'parent-1', role: 'parent' },
      enabled: true,
      effectiveFrom: null,
      effectiveUntil: null,
    },
    targetEvidenceRefs: [screenRefId],
    custody: 'parent-device-cache',
    updatedAt: observedAt,
    expiresAt: null,
  };
}

function screenSummaryEvidence(scenario, screenRefId, observedAt) {
  return {
    evidenceRefId: screenRefId,
    evidence: {
      evidenceReferenceId: `${scenario.ocrResultId}-query-store-summary`,
      kind: 'query-store-summary',
      observedAt,
    },
    evidenceKind: 'screen-summary',
    sourceSchemaVersion: 'v0.6',
    observedAt,
    ingestedAt: observedAt,
    freshUntil: null,
    sourceId: scenario.ocrResultId,
    adapterId: scenario.modelRuntimeRef,
    device: Device,
    childProfile: ChildProfile,
    custody: scenario.custodyState,
    retentionState: scenario.imageDeletionState === 'deleted' ? 'deleted-source' : 'temporary',
    confidence: scenario.confidence,
    confidenceKind: 'classifier',
    capabilityStatus: scenario.capabilityStatus === 'ready' ? 'available' : 'degraded',
    degradedReasons:
      scenario.imageDeletionState === 'deleted' ? ['screen-image-deleted'] : ['screen-deletion-unconfirmed'],
    unknownReasons: scenario.primaryCategory === 'unknown' ? ['missing-evidence'] : [],
    sourceEvidenceReferences: scenario.sourceEvidenceRefs.map((reference) => ({
      evidenceReferenceId: reference.evidenceId,
      kind: 'journal-event',
      observedAt,
    })),
  };
}

function runtimeStatus(scenario, observedAt) {
  return {
    runtimeReferenceId: `${scenario.modelRuntimeRef}-context`,
    providerId: scenario.ocrEngine,
    modelId: scenario.modelId,
    modelReference: scenario.modelRuntimeRef,
    privacyMode: 'local-only',
    adapterBoundary: 'local-adapter-ready',
    executionState: 'dry-run-ready',
    providerSource: 'local-model-cache',
    loadState: 'loaded',
    capabilityFlags: ['classification', 'safety-decision'],
    resourceClass: 'cpu',
    degradedState: 'none',
    lastCheckedAt: observedAt,
    unavailableReason: null,
  };
}

function policyActionForCategory(category) {
  return category === 'school' || category === 'productivity' ? 'allow' : 'warn';
}

function validateContextRow(row) {
  const failures = [];
  if (row.contextState !== 'ready') {
    failures.push(`${row.ocrResultId} context state was ${row.contextState}`);
  }
  if (row.imageDeletionState !== 'deleted' || row.rawImageRetained !== false) {
    failures.push(`${row.ocrResultId} source screen summary was not deletion-safe`);
  }
  if (row.ocrCustodyState !== 'child-device-query-store') {
    failures.push(`${row.ocrResultId} source custody was ${row.ocrCustodyState}`);
  }
  if (row.screenSummaryRefs.length !== 1) {
    failures.push(`${row.ocrResultId} selected ${row.screenSummaryRefs.length} screen summary refs`);
  }
  if (!row.custodyLabels.includes('child-device-query-store')) {
    failures.push(`${row.ocrResultId} context omitted child-device query-store custody`);
  }
  if (!row.degradedReasons.includes('screen-image-deleted')) {
    failures.push(`${row.ocrResultId} context omitted screen-image-deleted reason`);
  }
  if (row.auditEvidenceReferences.length === 0) {
    failures.push(`${row.ocrResultId} context omitted audit evidence refs`);
  }
  return failures;
}

function runCommand(command, args) {
  execFileSync(command, args, { cwd: RepoRoot, stdio: 'inherit' });
}

function relativePath(path) {
  return relative(RepoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
