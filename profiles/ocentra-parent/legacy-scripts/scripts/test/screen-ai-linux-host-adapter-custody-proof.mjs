import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'linux-host-adapter-custody');
const policySourcePath = join(outputDir, '01-screen-derived-policy-source.json');
const custodyArtifactPath = join(outputDir, '02-linux-host-adapter-custody.json');
const proofPath = join(outputDir, 'proof-summary.json');
const snapshotPath = join(outputDir, '00-linux-host-adapter-custody.md');
const commandsPath = join(outputDir, '10-validation-commands.log');

const sourceArtifacts = {
  blockPolicySource: 'output/screen-ai-pipeline-proof/block-action-dispatch/00-screen-block-source.json',
  adapterReadinessReadModel: 'output/screen-ai-pipeline-proof/adapter-readiness/read-model.json',
  linuxCaptureCapability: 'output/screen-plan-proof/linux/proof-summary.json',
  linuxWslgCapture: 'output/screen-plan-proof/linux-wslg/proof-summary.json',
  adapterBlockerLedger: 'output/screen-ai-pipeline-proof/adapter-blocker-ledger/proof-summary.json',
  screenAiPipelineChecklist: 'docs/plans/screen-ai-pipeline-plan/implementation-checklist.md',
};

const failures = [];
const blockPolicySource = readJson(sourceArtifacts.blockPolicySource);
const adapterReadinessReadModel = readJson(sourceArtifacts.adapterReadinessReadModel);
const linuxCaptureCapability = readJson(sourceArtifacts.linuxCaptureCapability);
const linuxWslgCapture = readJson(sourceArtifacts.linuxWslgCapture);
const adapterBlockerLedger = readJson(sourceArtifacts.adapterBlockerLedger);
const screenAiPipelineChecklist = readText(sourceArtifacts.screenAiPipelineChecklist);
const linuxReadinessRow = adapterReadinessReadModel.rows.find(
  (row) => row.rowId === 'screen-ai-linux-host-adapter-unavailable'
);
const linuxBlockerRow = adapterBlockerLedger.rows.find(
  (row) => row.rowId === 'screen-ai-linux-host-adapter-unavailable'
);

assert(blockPolicySource.scenarioId === 'bypass-tool', 'Linux custody proof must use the bypass-tool block source');
assert(blockPolicySource.sourcePolicyAction === 'block', 'Linux custody proof must use a block policy action');
assert(blockPolicySource.sourcePolicyDryRun === true, 'Linux custody proof source policy must remain dry-run');
assert(
  blockPolicySource.rawImageDeletedBeforeDispatch === true,
  'Linux custody proof requires deleted image before adapter handoff'
);
assert(Boolean(linuxReadinessRow), 'missing Linux host adapter readiness row');
assert(linuxReadinessRow?.platform === 'linux', 'Linux readiness row must target Linux');
assert(linuxReadinessRow?.readinessState === 'unavailable', 'Linux readiness row must stay unavailable');
assert(linuxReadinessRow?.actionExecutionState === 'skipped', 'Linux row must not execute without native proof');
assert(linuxReadinessRow?.adapterExecutionProofArtifact === null, 'Linux row must not have execution proof yet');
assert(linuxReadinessRow?.rawImageRetained === false, 'Linux row must not retain raw image');
assert(linuxReadinessRow?.rawImageDeletedBeforeAdapter === true, 'Linux row must delete image before adapter');
assert(
  Object.values(linuxReadinessRow?.claimFlags ?? {}).every((flag) => flag === false),
  'Linux row must not upgrade any claim flags'
);
assert(Boolean(linuxBlockerRow), 'missing Linux adapter blocker ledger row');
assert(
  linuxBlockerRow?.requiredProofArtifact ===
    'screen-derived Linux host decision to host action with rollback, audit, and custody proof',
  'Linux blocker ledger required artifact changed'
);
assert(
  linuxCaptureCapability.gapStatus?.wslgX11SelectedWindowProofExists === true,
  'Linux WSLg selected-window capture proof must be present'
);
assert(
  linuxCaptureCapability.gapStatus?.productLinuxCaptureReady === false,
  'Linux product capture readiness must remain false'
);
assert(linuxWslgCapture.custody?.rawImageDeleted === true, 'Linux WSLg raw image must be deleted');
assert(
  linuxWslgCapture.nonClaims?.includes(
    'It does not claim WSLg root display capture, native Linux Wayland portal capture, or macOS/iOS/Android physical parity.'
  ),
  'Linux WSLg proof must keep native/root-display non-claims'
);
assert(
  screenAiPipelineChecklist.includes(
    '- [ ] Browser, network, mobile, and broad block adapters proven from screen-derived decisions before product-complete action claims.'
  ),
  'product-complete adapter row must remain open'
);

if (failures.length > 0) {
  throw new Error(
    `Screen AI Linux host adapter custody proof failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
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
  sourceImageDeletionState: linuxReadinessRow.sourceImageDeletionState,
  rawImageRetained: linuxReadinessRow.rawImageRetained,
  rawImageDeletedBeforeAdapter: linuxReadinessRow.rawImageDeletedBeforeAdapter,
  sourceProofArtifact: sourceArtifacts.blockPolicySource,
};
const custodyArtifact = {
  schemaVersion: 'v0.6',
  custodyArtifactId: 'screen-ai-linux-host-adapter-custody',
  generatedAt,
  sourceArtifacts,
  sourcePolicyDecisionId: blockPolicySource.policyDecisionId,
  adapterClass: 'linux-host-control',
  adapterRuntimeBoundary: linuxReadinessRow.adapterRuntimeBoundary,
  platform: linuxReadinessRow.platform,
  apply: {
    state: 'not-executed-target-unavailable',
    requestedAction: blockPolicySource.sourcePolicyAction,
    screenDerived: true,
    hostMutationAttempted: false,
    liveLinuxHostMutationProved: false,
    nativeSessionProofRef: null,
    refusalReason: linuxReadinessRow.refusalReason,
    requiredBeforeExecution: linuxReadinessRow.manualProofRequirements,
  },
  rollback: {
    state: 'not-executed-no-host-apply',
    rollbackAttempted: false,
    rollbackRequiredBeforeProductComplete: true,
    rollbackReferenceState: linuxReadinessRow.rollbackReferenceState,
  },
  audit: {
    state: 'custody-recorded-not-executed',
    auditReferenceState: linuxReadinessRow.auditReferenceState,
    auditRef: 'screen-ai-linux-host-adapter-custody-audit',
    sourceEvidenceReferences: blockPolicySource.evidenceReferences,
    custodyRefs: [
      sourceArtifacts.blockPolicySource,
      sourceArtifacts.adapterReadinessReadModel,
      sourceArtifacts.linuxCaptureCapability,
      sourceArtifacts.linuxWslgCapture,
    ],
    rawImageRetained: false,
    rawImageDeletedBeforeAdapter: true,
  },
  claimBoundary:
    'This artifact records screen-derived Linux host adapter custody only; it does not claim native Linux host apply, rollback execution, root-display capture, or Wayland/PipeWire product readiness.',
  finalAdapterCompletionClaimed: false,
};
const proof = {
  status: 'linux-host-custody-artifact-written-final-execution-blocked',
  proofKind: 'screen-ai-linux-host-adapter-custody-proof',
  generatedAt,
  sourceArtifacts,
  screenDerivedPolicySource: relativePath(policySourcePath),
  custodyArtifact: relativePath(custodyArtifactPath),
  closure: {
    screenDerivedBlockDecisionPreserved: true,
    linuxCapturePrerequisitePresent: true,
    linuxRawImageDeletionPreserved: true,
    linuxHostApplyCustodyRecorded: true,
    linuxHostApplyExecuted: false,
    linuxRollbackExecutionRecorded: false,
    linuxAuditCustodyRecorded: true,
    finalAdapterCompletionClaimed: false,
    productCompleteAdapterRowStillOpen: true,
  },
  nonClaims: [
    'This proof does not execute native Linux host mutation.',
    'This proof does not execute Linux rollback.',
    'This proof does not claim Linux native X11 root-display, Wayland/PipeWire portal, or broad host-control product readiness.',
  ],
};

writeOutputs(screenDerivedPolicySource, custodyArtifact, proof);
console.log(`screen-ai-linux-host-adapter-custody-proof-ok:${relativePath(proofPath)}`);

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
  return `# Screen AI Linux Host Adapter Custody Proof\n\nGenerated: ${proof.generatedAt}\n\nStatus: ${proof.status}\n\n## Apply\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.apply, null, 2)}\n\`\`\`\n\n## Rollback\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.rollback, null, 2)}\n\`\`\`\n\n## Audit\n\n\`\`\`json\n${JSON.stringify(custodyArtifact.audit, null, 2)}\n\`\`\`\n\n## Closure\n\n\`\`\`json\n${JSON.stringify(proof.closure, null, 2)}\n\`\`\`\n`;
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-ai-linux-host-adapter-custody-proof.mjs',
    'node scripts/test/screen-ai-linux-host-adapter-custody-proof.mjs',
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
