import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'android-mobile-control-custody');
const policySourcePath = join(outputDir, '01-screen-derived-policy-source.json');
const custodyArtifactPath = join(outputDir, '02-android-mobile-control-custody.json');
const proofPath = join(outputDir, 'proof-summary.json');
const snapshotPath = join(outputDir, '00-android-mobile-control-custody.md');
const commandsPath = join(outputDir, '10-validation-commands.log');

const sourceArtifacts = {
  blockPolicySource: 'output/screen-ai-pipeline-proof/block-action-dispatch/00-screen-block-source.json',
  adapterReadinessReadModel: 'output/screen-ai-pipeline-proof/adapter-readiness/read-model.json',
  androidCaptureCapability: 'output/screen-plan-proof/android/proof-summary.json',
  androidMediaProjectionProof: 'output/screen-plan-proof/android-mediaprojection/proof-summary.json',
  adapterBlockerLedger: 'output/screen-ai-pipeline-proof/adapter-blocker-ledger/proof-summary.json',
  screenAiPipelineChecklist: 'docs/plans/screen-ai-pipeline-plan/implementation-checklist.md',
};

const failures = [];
const blockPolicySource = readJson(sourceArtifacts.blockPolicySource);
const adapterReadinessReadModel = readJson(sourceArtifacts.adapterReadinessReadModel);
const androidCaptureCapability = readJson(sourceArtifacts.androidCaptureCapability);
const androidMediaProjectionProof = readJson(sourceArtifacts.androidMediaProjectionProof);
const adapterBlockerLedger = readJson(sourceArtifacts.adapterBlockerLedger);
const screenAiPipelineChecklist = readText(sourceArtifacts.screenAiPipelineChecklist);
const androidReadinessRow = adapterReadinessReadModel.rows.find(
  (row) => row.rowId === 'screen-ai-android-mobile-control-manual-required'
);
const androidBlockerRow = adapterBlockerLedger.rows.find(
  (row) => row.rowId === 'screen-ai-android-mobile-control-manual-required'
);

assert(blockPolicySource.scenarioId === 'bypass-tool', 'Android custody proof must use the bypass-tool block source');
assert(blockPolicySource.sourcePolicyAction === 'block', 'Android custody proof must use a block policy action');
assert(blockPolicySource.sourcePolicyDryRun === true, 'Android custody proof source policy must remain dry-run');
assert(
  blockPolicySource.rawImageDeletedBeforeDispatch === true,
  'Android custody proof requires deleted image before adapter handoff'
);
assert(Boolean(androidReadinessRow), 'missing Android mobile control readiness row');
assert(androidReadinessRow?.platform === 'android', 'Android readiness row must target Android');
assert(androidReadinessRow?.readinessState === 'manual-required', 'Android readiness row must stay manual-required');
assert(androidReadinessRow?.actionExecutionState === 'skipped', 'Android row must not execute without device proof');
assert(androidReadinessRow?.adapterExecutionProofArtifact === null, 'Android row must not have execution proof yet');
assert(androidReadinessRow?.rawImageRetained === false, 'Android row must not retain raw image');
assert(androidReadinessRow?.rawImageDeletedBeforeAdapter === true, 'Android row must delete image before adapter');
assert(
  Object.values(androidReadinessRow?.claimFlags ?? {}).every((flag) => flag === false),
  'Android row must not upgrade any claim flags'
);
assert(Boolean(androidBlockerRow), 'missing Android adapter blocker ledger row');
assert(
  androidBlockerRow?.requiredProofArtifact ===
    'screen-derived mobile decision to Android DO/profile action with rollback, audit, and custody proof on real device or managed profile',
  'Android blocker ledger required artifact changed'
);
assert(
  androidCaptureCapability.gapStatus?.emulatorMediaProjectionProofExists === true,
  'Android emulator MediaProjection proof must be present'
);
assert(
  androidCaptureCapability.gapStatus?.physicalAndroidDeviceProofExists === false,
  'Android physical-device proof must remain missing before claim upgrade'
);
assert(
  androidCaptureCapability.gapStatus?.productAndroidCaptureReady === false,
  'Android product capture readiness must remain false'
);
assert(androidMediaProjectionProof.rawTempDeleted === true, 'Android emulator raw temp image must be deleted');
assert(
  androidMediaProjectionProof.nonClaims?.includes(
    'It does not claim physical-device parity unless the target serial is a physical device recorded in 00-device.json.'
  ),
  'Android MediaProjection proof must keep physical-device parity non-claim'
);
assert(
  screenAiPipelineChecklist.includes(
    '- [ ] Browser, network, mobile, and broad block adapters proven from screen-derived decisions before product-complete action claims.'
  ),
  'product-complete adapter row must remain open'
);

if (failures.length > 0) {
  throw new Error(
    `Screen AI Android mobile control custody proof failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
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
  sourceImageDeletionState: androidReadinessRow.sourceImageDeletionState,
  rawImageRetained: androidReadinessRow.rawImageRetained,
  rawImageDeletedBeforeAdapter: androidReadinessRow.rawImageDeletedBeforeAdapter,
  sourceProofArtifact: sourceArtifacts.blockPolicySource,
};
const custodyArtifact = {
  schemaVersion: 'v0.6',
  custodyArtifactId: 'screen-ai-android-mobile-control-custody',
  generatedAt,
  sourceArtifacts,
  sourcePolicyDecisionId: blockPolicySource.policyDecisionId,
  adapterClass: 'android-device-owner-or-managed-profile',
  adapterRuntimeBoundary: androidReadinessRow.adapterRuntimeBoundary,
  platform: androidReadinessRow.platform,
  apply: {
    state: 'not-executed-manual-required',
    requestedAction: blockPolicySource.sourcePolicyAction,
    screenDerived: true,
    mobileControlAttempted: false,
    liveAndroidDeviceOwnerOrProfileProofProved: false,
    physicalDeviceProofRef: null,
    refusalReason: androidReadinessRow.refusalReason,
    requiredBeforeExecution: androidReadinessRow.manualProofRequirements,
  },
  rollback: {
    state: 'not-executed-no-mobile-apply',
    rollbackAttempted: false,
    rollbackRequiredBeforeProductComplete: true,
    rollbackReferenceState: androidReadinessRow.rollbackReferenceState,
  },
  audit: {
    state: 'custody-recorded-not-executed',
    auditReferenceState: androidReadinessRow.auditReferenceState,
    auditRef: 'screen-ai-android-mobile-control-custody-audit',
    sourceEvidenceReferences: blockPolicySource.evidenceReferences,
    custodyRefs: [
      sourceArtifacts.blockPolicySource,
      sourceArtifacts.adapterReadinessReadModel,
      sourceArtifacts.androidCaptureCapability,
      sourceArtifacts.androidMediaProjectionProof,
    ],
    rawImageRetained: false,
    rawImageDeletedBeforeAdapter: true,
  },
  claimBoundary:
    'This artifact records screen-derived Android mobile-control custody only; it does not claim Device Owner, managed-profile, UsageStats, Accessibility, VPN/DNS, or physical-device action execution.',
  finalAdapterCompletionClaimed: false,
};
const proof = {
  status: 'android-mobile-control-custody-artifact-written-final-execution-blocked',
  proofKind: 'screen-ai-android-mobile-control-custody-proof',
  generatedAt,
  sourceArtifacts,
  screenDerivedPolicySource: relativePath(policySourcePath),
  custodyArtifact: relativePath(custodyArtifactPath),
  closure: {
    screenDerivedBlockDecisionPreserved: true,
    androidEmulatorCapturePrerequisitePresent: true,
    androidRawImageDeletionPreserved: true,
    androidMobileApplyCustodyRecorded: true,
    androidMobileApplyExecuted: false,
    androidRollbackExecutionRecorded: false,
    androidAuditCustodyRecorded: true,
    finalAdapterCompletionClaimed: false,
    productCompleteAdapterRowStillOpen: true,
  },
  nonClaims: [
    'This proof does not execute Android Device Owner or managed-profile control.',
    'This proof does not execute Android rollback.',
    'This proof does not claim physical-device parity, silent Android background capture, Accessibility enforcement, UsageStats enforcement, or VPN/DNS enforcement.',
  ],
};

writeOutputs(screenDerivedPolicySource, custodyArtifact, proof);
console.log(`screen-ai-android-mobile-control-custody-proof-ok:${relativePath(proofPath)}`);

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
  return `# Screen AI Android Mobile Control Custody Proof\n\nGenerated: ${proof.generatedAt}\n\nStatus: ${proof.status}\n\n## Apply\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.apply, null, 2)}\n\`\`\`\n\n## Rollback\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.rollback, null, 2)}\n\`\`\`\n\n## Audit\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.audit, null, 2)}\n\`\`\`\n\n## Closure\n\n\`\`\`json\n${JSON.stringify(proof.closure, null, 2)}\n\`\`\`\n`;
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-ai-android-mobile-control-custody-proof.mjs',
    'node scripts/test/screen-ai-android-mobile-control-custody-proof.mjs',
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
