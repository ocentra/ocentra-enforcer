import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const RealAnalysisProofPath = resolve(RepoRoot, 'output', 'ai-plan-proof', 'real-analysis', 'proof-summary.json');
const ScreenOcrProofPath = resolve(
  RepoRoot,
  'output',
  'ai-plan-proof',
  'screen-winrt-ocr-worker',
  'proof-summary.json'
);
const NetworkStorageProofPath = resolve(
  RepoRoot,
  'output',
  'network-plan-proof',
  '03a-live-capture-storage-proof',
  'proof-summary.json'
);
const OutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'local-ai-stored-evidence-context');
const ProofPath = join(OutputRoot, 'proof-summary.json');
const ObservedAtFallback = '2026-06-06T03:42:00.000Z';
const ChildProfile = { childProfileId: 'stored-context-child', displayName: 'Sam' };
const Device = {
  deviceId: 'stored-context-windows-device',
  childProfileId: ChildProfile.childProfileId,
  label: 'Sam Windows PC',
  platform: 'windows',
};
const RuntimeStatus = {
  runtimeReferenceId: 'stored-context-local-runtime',
  providerId: 'stored-context-local-provider',
  modelId: 'stored-context-safety-model',
  modelReference: 'local-model-cache/stored-context-safety-model',
  privacyMode: 'local-only',
  adapterBoundary: 'local-adapter-ready',
  executionState: 'dry-run-ready',
  providerSource: 'local-model-cache',
  loadState: 'loaded',
  capabilityFlags: ['classification', 'safety-decision'],
  resourceClass: 'cpu',
  degradedState: 'none',
  lastCheckedAt: ObservedAtFallback,
  unavailableReason: null,
};

runCommand(...npmCommand(['run', 'build:contracts']));

const { buildLocalAiEvidenceContext } = await import('@ocentra-parent/schema-domain/local-ai-context-builder');
const realAnalysisProof = readJson(RealAnalysisProofPath);
const screenOcrProof = readJson(ScreenOcrProofPath);
const networkStorageProof = readJson(NetworkStorageProofPath);

const browserScenario = scenarioById(realAnalysisProof, 'youtube-ordinary-video');
const appGameScenario = scenarioById(realAnalysisProof, 'native-game');
const screenScenario = screenOcrProof.proof.scenarios[0];
const readyInput = contextInput([
  realAnalysisEvidence(browserScenario, 'browser-ref-youtube-ordinary-video', 'browser'),
  realAnalysisEvidence(appGameScenario, 'app-game-ref-native-game', 'app-game'),
  networkEvidence(networkStorageProof),
  screenSummaryEvidence(screenScenario),
]);
const hostedInput = contextInput([
  hostedEvidence(realAnalysisEvidence(browserScenario, 'hosted-browser-ref', 'browser')),
]);
const missingNetworkInput = contextInput([
  realAnalysisEvidence(browserScenario, 'browser-ref-youtube-ordinary-video', 'browser'),
  realAnalysisEvidence(appGameScenario, 'app-game-ref-native-game', 'app-game'),
  screenSummaryEvidence(screenScenario),
]);
const rows = [
  rowFor('mixed-stored-evidence-ready', readyInput),
  rowFor('hosted-child-activity-rejected', hostedInput),
  rowFor('missing-network-flow-partial', missingNetworkInput),
];
const failures = rows.flatMap(validateRow);

if (failures.length > 0) {
  throw new Error(`Local AI stored-evidence context proof failed:\n${failures.join('\n')}`);
}

const proof = {
  status: 'ok',
  proofKind: 'local-ai-stored-evidence-context-proof',
  generatedAt: new Date().toISOString(),
  sourceProofs: {
    realAnalysis: relativePath(RealAnalysisProofPath),
    screenOcr: relativePath(ScreenOcrProofPath),
    networkStorage: relativePath(NetworkStorageProofPath),
  },
  output: relativePath(ProofPath),
  rows,
  summary: {
    readyRows: rows.filter((row) => row.contextState === 'ready').length,
    rejectedRows: rows.filter((row) => row.contextState === 'rejected').length,
    partialRows: rows.filter((row) => row.contextState === 'partial').length,
    selectedEvidenceRefs: rows.reduce((count, row) => count + row.selectedEvidenceRefs.length, 0),
    auditEvidenceRefs: rows.reduce((count, row) => count + row.auditEvidenceRefs.length, 0),
    remoteAiUsed: false,
    rawImagesRetained: false,
    failures: failures.length,
  },
  assertions: [
    'Stored browser, app/game, network-flow, and screen-summary evidence refs can build one ready local AI context.',
    'The ready context preserves child-device custody, runtime refs, parent-rule refs, and audit evidence refs before local model input.',
    'Ocentra-hosted non-activity custody is rejected before child-activity evidence can enter local model input.',
    'Missing required stored evidence degrades to a partial context with explicit missing evidence kinds.',
  ],
  nonClaims: [
    'This proof consumes existing proof artifacts; it does not create fresh browser, screen, app/game, or network captures.',
    'This proof does not execute a local model, prove production model quality, render portal UI, dispatch enforcement, or use remote/API AI.',
    'This proof does not claim network policy, network adapter authority, exact URL, page content, private message, search query, or decrypted payload coverage.',
  ],
};

mkdirSync(OutputRoot, { recursive: true });
writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`local-ai-stored-evidence-context-proof-ok:${ProofPath}`);

function contextInput(evidenceReferences) {
  const evidenceRefIds = evidenceReferences.map((reference) => reference.evidenceRefId);
  return {
    contextId: 'stored-evidence-context',
    request: {
      schemaVersion: 'v0.6',
      requestId: 'stored-evidence-context-request',
      requestedAt: ObservedAtFallback,
      childProfile: ChildProfile,
      device: Device,
      requestedEvaluationKind: 'mixed-context',
      requiredEvidenceKinds: ['browser', 'app-game', 'network-flow', 'screen-summary'],
      parentRuleContextReferences: [parentRuleContext(evidenceRefIds)],
      modelTaskRequirements: ['classification', 'safety-decision'],
      allowedCustody: ['child-device-query-store', 'child-device-journal'],
      promptVersion: 'stored-evidence-context-v1',
    },
    evidenceReferences,
    runtimeReferences: [RuntimeStatus],
    memoryReferences: [],
    graphReferences: [],
  };
}

function parentRuleContext(targetEvidenceRefs) {
  return {
    parentRuleRefId: 'stored-evidence-parent-rule-context',
    policyVersion: 'stored-evidence-policy-v1',
    family: { familyId: 'stored-evidence-family' },
    childProfile: ChildProfile,
    device: Device,
    rule: {
      ruleId: 'stored-evidence-safety-rule',
      target: {
        targetId: 'stored-evidence-mixed-context-target',
        targetType: 'category',
        targetValue: 'mixed-context',
      },
      action: 'warn',
      scheduleId: null,
      priority: 10,
      reasonCode: 'stored-evidence-context-safety',
      createdBy: { actorId: 'parent-1', role: 'parent' },
      enabled: true,
      effectiveFrom: null,
      effectiveUntil: null,
    },
    targetEvidenceRefs,
    custody: 'parent-device-cache',
    updatedAt: ObservedAtFallback,
    expiresAt: null,
  };
}

function realAnalysisEvidence(scenario, evidenceRefId, evidenceKind) {
  const observedAt = ObservedAtFallback;
  return {
    evidenceRefId,
    evidence: {
      evidenceReferenceId: `${scenario.scenarioId}-query-store-summary`,
      kind: 'query-store-summary',
      observedAt,
    },
    evidenceKind,
    sourceSchemaVersion: 'v0.6',
    observedAt,
    ingestedAt: observedAt,
    freshUntil: null,
    sourceId: scenario.scenarioId,
    adapterId: scenario.fixtureKind,
    device: Device,
    childProfile: ChildProfile,
    custody: 'child-device-query-store',
    retentionState: scenario.rawImagesDeletedAfterAnalysis ? 'deleted-source' : 'temporary',
    confidence: scenario.confidence,
    confidenceKind: 'classifier',
    capabilityStatus: scenario.schemaValidated ? 'available' : 'degraded',
    degradedReasons: scenario.rawImagesDeletedAfterAnalysis
      ? ['screen-image-deleted']
      : ['screen-deletion-unconfirmed'],
    unknownReasons: scenario.primaryCategory === 'unknown' ? ['missing-evidence'] : [],
    sourceEvidenceReferences: [
      {
        evidenceReferenceId: `screen-evidence-${scenario.scenarioId}`,
        kind: 'journal-event',
        observedAt,
      },
    ],
  };
}

function networkEvidence(proof) {
  const observedAt = proof.checkedAt ?? ObservedAtFallback;
  return {
    evidenceRefId: 'network-flow-ref-live-capture-storage',
    evidence: {
      evidenceReferenceId: 'network-live-capture-storage-query-row',
      kind: 'query-store-summary',
      observedAt,
    },
    evidenceKind: 'network-flow',
    sourceSchemaVersion: 'v0.6',
    observedAt,
    ingestedAt: observedAt,
    freshUntil: null,
    sourceId: proof.proof,
    adapterId: 'network-live-capture-storage-proof',
    device: Device,
    childProfile: ChildProfile,
    custody: 'child-device-query-store',
    retentionState: 'local',
    confidence: null,
    confidenceKind: null,
    capabilityStatus: proof.statusShort === '' ? 'available' : 'degraded',
    degradedReasons: [],
    unknownReasons: [],
    sourceEvidenceReferences: [
      {
        evidenceReferenceId: proof.artifacts.storageBoundary,
        kind: 'journal-event',
        observedAt,
      },
    ],
  };
}

function screenSummaryEvidence(scenario) {
  const observedAt = scenario.analyzedAt ?? ObservedAtFallback;
  return {
    evidenceRefId: 'screen-summary-ref-live-ocr',
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

function hostedEvidence(reference) {
  return {
    ...reference,
    custody: 'ocentra-hosted-non-activity',
  };
}

function rowFor(rowId, input) {
  const result = buildLocalAiEvidenceContext(input);
  const context = result.context;
  return {
    rowId,
    contextState: result.state,
    selectedEvidenceRefs: context?.evidenceReferences.map((reference) => reference.evidenceRefId) ?? [],
    browserEvidenceRefs: context?.browserEvidenceRefs ?? [],
    appGameEvidenceRefs: context?.appGameEvidenceRefs ?? [],
    networkFlowEvidenceRefs: context?.networkFlowEvidenceRefs ?? [],
    screenSummaryRefs: context?.screenSummaryRefs ?? [],
    custodyLabels: context?.custodyLabels ?? [],
    localModelRuntimeRefs: context?.localModelRuntimeRefs ?? [],
    parentRuleReferences: context?.parentRuleReferences ?? [],
    degradedReasons: context?.degradedReasons ?? [],
    missingEvidenceKinds: result.missingEvidenceKinds,
    rejectedFields: result.rejectedFields,
    custodyBoundarySummary: result.custodyBoundarySummary,
    validationGateSummary: result.validationGateSummary,
    auditEvidenceRefs: result.auditEvidenceReferences.map((reference) => reference.evidenceReferenceId),
  };
}

function validateRow(row) {
  if (row.rowId === 'mixed-stored-evidence-ready') {
    return validateReadyRow(row);
  }
  if (row.rowId === 'hosted-child-activity-rejected') {
    return validateHostedRejectedRow(row);
  }
  return validateMissingNetworkRow(row);
}

function validateReadyRow(row) {
  const failures = [];
  if (row.contextState !== 'ready') {
    failures.push(`${row.rowId} state was ${row.contextState}`);
  }
  for (const [label, refs] of [
    ['browser', row.browserEvidenceRefs],
    ['app-game', row.appGameEvidenceRefs],
    ['network-flow', row.networkFlowEvidenceRefs],
    ['screen-summary', row.screenSummaryRefs],
  ]) {
    if (refs.length !== 1) {
      failures.push(`${row.rowId} selected ${refs.length} ${label} refs`);
    }
  }
  if (!row.custodyLabels.includes('child-device-query-store')) {
    failures.push(`${row.rowId} omitted child-device query-store custody`);
  }
  if (row.localModelRuntimeRefs.length !== 1) {
    failures.push(`${row.rowId} selected ${row.localModelRuntimeRefs.length} runtime refs`);
  }
  if (row.parentRuleReferences.length !== 1) {
    failures.push(`${row.rowId} selected ${row.parentRuleReferences.length} parent rules`);
  }
  if (row.auditEvidenceRefs.length !== 4) {
    failures.push(`${row.rowId} emitted ${row.auditEvidenceRefs.length} audit evidence refs`);
  }
  return failures;
}

function validateHostedRejectedRow(row) {
  const failures = [];
  if (row.contextState !== 'rejected') {
    failures.push(`${row.rowId} state was ${row.contextState}`);
  }
  if (!row.rejectedFields.includes('evidenceReferences')) {
    failures.push(`${row.rowId} did not reject evidenceReferences`);
  }
  if (row.selectedEvidenceRefs.length !== 0) {
    failures.push(`${row.rowId} selected hosted evidence`);
  }
  return failures;
}

function validateMissingNetworkRow(row) {
  const failures = [];
  if (row.contextState !== 'partial') {
    failures.push(`${row.rowId} state was ${row.contextState}`);
  }
  if (!row.missingEvidenceKinds.includes('network-flow')) {
    failures.push(`${row.rowId} did not report missing network-flow`);
  }
  if (row.auditEvidenceRefs.length !== 3) {
    failures.push(`${row.rowId} emitted ${row.auditEvidenceRefs.length} audit evidence refs`);
  }
  return failures;
}

function scenarioById(proof, scenarioId) {
  const scenario = proof.scenarios.find((entry) => entry.scenarioId === scenarioId);
  if (!scenario) {
    throw new Error(`Missing real-analysis scenario ${scenarioId}`);
  }
  return scenario;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
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
