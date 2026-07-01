import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'ios-mobile-control-custody');
const policySourcePath = join(outputDir, '01-screen-derived-policy-source.json');
const custodyArtifactPath = join(outputDir, '02-ios-mobile-control-custody.json');
const proofPath = join(outputDir, 'proof-summary.json');
const snapshotPath = join(outputDir, '00-ios-mobile-control-custody.md');
const commandsPath = join(outputDir, '10-validation-commands.log');

const sourceArtifacts = {
  blockPolicySource: 'output/screen-ai-pipeline-proof/block-action-dispatch/00-screen-block-source.json',
  adapterReadinessReadModel: 'output/screen-ai-pipeline-proof/adapter-readiness/read-model.json',
  iosCaptureCapability: 'output/screen-plan-proof/ios/proof-summary.json',
  adapterBlockerLedger: 'output/screen-ai-pipeline-proof/adapter-blocker-ledger/proof-summary.json',
  screenAiPipelineChecklist: 'docs/plans/screen-ai-pipeline-plan/implementation-checklist.md',
};

const failures = [];
const blockPolicySource = readJson(sourceArtifacts.blockPolicySource);
const adapterReadinessReadModel = readJson(sourceArtifacts.adapterReadinessReadModel);
const iosCaptureCapability = readJson(sourceArtifacts.iosCaptureCapability);
const adapterBlockerLedger = readJson(sourceArtifacts.adapterBlockerLedger);
const screenAiPipelineChecklist = readText(sourceArtifacts.screenAiPipelineChecklist);
const iosReadinessRow = adapterReadinessReadModel.rows.find(
  (row) => row.rowId === 'screen-ai-ios-mobile-control-manual-required'
);
const iosBlockerRow = adapterBlockerLedger.rows.find(
  (row) => row.rowId === 'screen-ai-ios-mobile-control-manual-required'
);

assert(blockPolicySource.scenarioId === 'bypass-tool', 'iOS custody proof must use the bypass-tool block source');
assert(blockPolicySource.sourcePolicyAction === 'block', 'iOS custody proof must use a block policy action');
assert(blockPolicySource.sourcePolicyDryRun === true, 'iOS custody proof source policy must remain dry-run');
assert(
  blockPolicySource.rawImageDeletedBeforeDispatch === true,
  'iOS custody proof requires deleted image before adapter handoff'
);
assert(Boolean(iosReadinessRow), 'missing iOS mobile control readiness row');
assert(iosReadinessRow?.platform === 'ios', 'iOS readiness row must target iOS');
assert(iosReadinessRow?.readinessState === 'manual-required', 'iOS readiness row must stay manual-required');
assert(iosReadinessRow?.actionExecutionState === 'skipped', 'iOS row must not execute without device proof');
assert(iosReadinessRow?.adapterExecutionProofArtifact === null, 'iOS row must not have execution proof yet');
assert(iosReadinessRow?.rawImageRetained === false, 'iOS row must not retain raw image');
assert(iosReadinessRow?.rawImageDeletedBeforeAdapter === true, 'iOS row must delete image before adapter');
assert(
  Object.values(iosReadinessRow?.claimFlags ?? {}).every((flag) => flag === false),
  'iOS row must not upgrade any claim flags'
);
assert(Boolean(iosBlockerRow), 'missing iOS adapter blocker ledger row');
assert(
  iosBlockerRow?.requiredProofArtifact ===
    'screen-derived mobile decision to iOS Family Controls or DeviceActivity action with rollback, audit, and custody proof',
  'iOS blocker ledger required artifact changed'
);
assert(
  iosCaptureCapability.gapStatus?.physicalIosDeviceReplayKitProofExists === false,
  'iOS physical-device ReplayKit proof must remain missing before claim upgrade'
);
assert(
  iosCaptureCapability.gapStatus?.replayKitDeletionProofExists === false,
  'iOS deletion proof must remain missing before claim upgrade'
);
assert(
  iosCaptureCapability.gapStatus?.productIosCaptureReady === false,
  'iOS product capture readiness must remain false'
);
assert(
  iosCaptureCapability.nonClaims?.includes(
    'This proof does not claim physical iOS ReplayKit execution, live iOS pixels, or iOS deletion proof.'
  ),
  'iOS capture proof must keep physical ReplayKit execution non-claim'
);
assert(
  screenAiPipelineChecklist.includes(
    '- [ ] Browser, network, mobile, and broad block adapters proven from screen-derived decisions before product-complete action claims.'
  ),
  'product-complete adapter row must remain open'
);

if (failures.length > 0) {
  throw new Error(
    `Screen AI iOS mobile control custody proof failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
  );
}

const generatedAt = new Date().toISOString();
const screenDerivedPolicySource = {
  schemaVersion: 'v0.6',
  sourcePolicyDecisionId: blockPolicySource.policyDecisionId,
  sourcePolicyAction: blockPolicySource.sourcePolicyAction,
  sourcePolicyDryRun: blockPolicySource.sourcePolicyDryRun,
  sourceScreenCategory: blockPolicySource.sourceScreenCategory,
  sourceEvidenceReferences: blockPolicySource.evidenceReferences,
  sourceImageDeletionState: iosReadinessRow.sourceImageDeletionState,
  rawImageRetained: iosReadinessRow.rawImageRetained,
  rawImageDeletedBeforeAdapter: iosReadinessRow.rawImageDeletedBeforeAdapter,
  sourceProofArtifact: sourceArtifacts.blockPolicySource,
};
const custodyArtifact = {
  schemaVersion: 'v0.6',
  custodyArtifactId: 'screen-ai-ios-mobile-control-custody',
  generatedAt,
  sourceArtifacts,
  sourcePolicyDecisionId: blockPolicySource.policyDecisionId,
  adapterClass: 'ios-family-controls-device-activity',
  adapterRuntimeBoundary: iosReadinessRow.adapterRuntimeBoundary,
  platform: iosReadinessRow.platform,
  apply: {
    state: 'not-executed-manual-required',
    requestedAction: blockPolicySource.sourcePolicyAction,
    screenDerived: true,
    mobileControlAttempted: false,
    liveIosFamilyControlsOrDeviceActivityProofProved: false,
    physicalDeviceProofRef: null,
    refusalReason: iosReadinessRow.refusalReason,
    requiredBeforeExecution: iosReadinessRow.manualProofRequirements,
  },
  rollback: {
    state: 'not-executed-no-mobile-apply',
    rollbackAttempted: false,
    rollbackRequiredBeforeProductComplete: true,
    rollbackReferenceState: iosReadinessRow.rollbackReferenceState,
  },
  audit: {
    state: 'custody-recorded-not-executed',
    auditReferenceState: iosReadinessRow.auditReferenceState,
    auditRef: 'screen-ai-ios-mobile-control-custody-audit',
    sourceEvidenceReferences: blockPolicySource.evidenceReferences,
    custodyRefs: [
      sourceArtifacts.blockPolicySource,
      sourceArtifacts.adapterReadinessReadModel,
      sourceArtifacts.iosCaptureCapability,
    ],
    rawImageRetained: false,
    rawImageDeletedBeforeAdapter: true,
  },
  claimBoundary:
    'This artifact records screen-derived iOS mobile-control custody only; it does not claim Family Controls, DeviceActivity, Network Extension, ReplayKit physical execution, live iOS pixels, deletion proof, or rollback execution.',
  finalAdapterCompletionClaimed: false,
};
const proof = {
  status: 'ios-mobile-control-custody-artifact-written-final-execution-blocked',
  proofKind: 'screen-ai-ios-mobile-control-custody-proof',
  generatedAt,
  sourceArtifacts,
  screenDerivedPolicySource: relativePath(policySourcePath),
  custodyArtifact: relativePath(custodyArtifactPath),
  closure: {
    screenDerivedBlockDecisionPreserved: true,
    iosReplayKitSourceDocPrerequisitePresent: true,
    iosRawImageRetentionBlocked: true,
    iosMobileApplyCustodyRecorded: true,
    iosMobileApplyExecuted: false,
    iosRollbackExecutionRecorded: false,
    iosAuditCustodyRecorded: true,
    finalAdapterCompletionClaimed: false,
    productCompleteAdapterRowStillOpen: true,
  },
  nonClaims: [
    'This proof does not execute iOS Family Controls or DeviceActivity control.',
    'This proof does not execute iOS rollback.',
    'This proof does not claim physical iOS ReplayKit capture, live iOS pixels, iOS deletion proof, Network Extension enforcement, or mobile-control product readiness.',
  ],
};

writeOutputs(screenDerivedPolicySource, custodyArtifact, proof);
console.log(`screen-ai-ios-mobile-control-custody-proof-ok:${relativePath(proofPath)}`);

function readJson(path) {
  return JSON.parse(readText(path));
}

function readText(path) {
  const absolute = resolve(repoRoot, path);
  assert(existsSync(absolute), `missing source artifact ${path}`);
  return readFileSync(absolute, 'utf8');
}

function writeOutputs(screenDerivedPolicySource, custodyArtifact, proof) {
  mkdirSync(outputDir, { recursive: true });
  writeFileSync(policySourcePath, `${JSON.stringify(screenDerivedPolicySource, null, 2)}\n`);
  writeFileSync(custodyArtifactPath, `${JSON.stringify(custodyArtifact, null, 2)}\n`);
  writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  writeFileSync(snapshotPath, markdownSnapshot(proof, custodyArtifact));
  writeFileSync(commandsPath, validationCommands());
}

function markdownSnapshot(proof, custodyArtifact) {
  return `# Screen AI iOS Mobile Control Custody Proof\n\nGenerated: ${proof.generatedAt}\n\nStatus: ${proof.status}\n\n## Apply\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.apply, null, 2)}\n\`\`\`\n\n## Rollback\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.rollback, null, 2)}\n\`\`\`\n\n## Audit\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.audit, null, 2)}\n\`\`\`\n\n## Closure\n\n\`\`\`json\n${JSON.stringify(proof.closure, null, 2)}\n\`\`\`\n`;
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-ai-ios-mobile-control-custody-proof.mjs',
    'node scripts/test/screen-ai-ios-mobile-control-custody-proof.mjs',
    'git diff --check',
    'npm run lanes:guard',
    'npm run hub:guard',
    '',
  ].join('\n');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}
