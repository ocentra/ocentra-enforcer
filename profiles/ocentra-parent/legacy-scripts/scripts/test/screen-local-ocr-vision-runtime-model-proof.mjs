import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', '17-local-ocr-vision-runtime-model');
const requiredArtifacts = {
  scheduler: resolve(repoRoot, 'output', 'screen-plan-proof', 'local-ai-resource-scheduler', 'proof-summary.json'),
  winRtOcr: resolve(repoRoot, 'output', 'ai-plan-proof', 'screen-winrt-ocr-worker', 'proof-summary.json'),
  serviceWinRtOcr: resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'service-winrt-ocr', 'proof-summary.json'),
  vlmWorker: resolve(repoRoot, 'output', 'ai-plan-proof', 'screen-vlm-worker-contract-proof', 'proof-summary.json'),
  vlmReadiness: resolve(
    repoRoot,
    'output',
    'ai-plan-proof',
    'screen-vlm-execution-readiness-proof',
    'proof-summary.json'
  ),
};

const artifacts = Object.fromEntries(
  await Promise.all(
    Object.entries(requiredArtifacts).map(async ([name, path]) => [name, JSON.parse(await readFile(path, 'utf8'))])
  )
);

const scheduler = artifacts.scheduler;
const winRtOcr = artifacts.winRtOcr;
const serviceWinRtOcr = artifacts.serviceWinRtOcr;
const vlmWorker = artifacts.vlmWorker;
const vlmReadiness = artifacts.vlmReadiness;

const assertions = {
  localRuntimeStatusDefined: Boolean(
    scheduler.screenSchedulerProof.queueSnapshot.currentHeavyRuntimeRef &&
    vlmReadiness.assertions.completedStatusRequiresDeletedQueryStoreResult
  ),
  ocrWorkerInputOutputDefined: Boolean(
    winRtOcr.proof.localOnly === true &&
    winRtOcr.proof.remoteAiUsed === false &&
    winRtOcr.proof.rawImageRetained === false &&
    winRtOcr.proof.scenarios.every((row) => row.modelRuntimeRef && row.modelId && row.promptOrTemplateVersion)
  ),
  serviceOcrRuntimeProved: Boolean(
    serviceWinRtOcr.assertions.serviceAdapterRanWindowsWinRtOcr &&
    serviceWinRtOcr.assertions.rawImageNotRetainedInReadModel &&
    serviceWinRtOcr.assertions.adapterTemporaryImageDeleted
  ),
  vlmWorkerInputOutputDefined: Boolean(
    vlmWorker.assertions.guidedVlmWorkerContractImplemented &&
    vlmWorker.assertions.schemaBoundModelOutputFeedsAnalysis &&
    vlmWorker.assertions.localOnlyNoRemoteAi
  ),
  remoteUploadRejectedByDefault: Boolean(
    scheduler.screenSchedulerSummary.remoteAiAllowed === false &&
    winRtOcr.proof.rawImageRemoteUploadEnabled === false &&
    vlmWorker.assertions.localOnlyNoRemoteAi
  ),
  unavailableAndDegradedStatesDefined: Boolean(
    scheduler.screenSchedulerSummary.timedOutJobs > 0 &&
    scheduler.screenSchedulerSummary.skippedOrDegradedJobs > 0 &&
    vlmReadiness.statusRows.some((row) => row.status === 'queued') &&
    vlmReadiness.statusRows.some((row) => row.status === 'completed') &&
    vlmReadiness.statusRows.some((row) => row.status === 'manual-required' && row.degradedReasons.length > 0)
  ),
  modelMetadataPreserved: Boolean(
    vlmWorker.constants.modelRuntimeRef &&
    vlmWorker.constants.modelId &&
    vlmWorker.constants.promptOrTemplateVersion &&
    serviceWinRtOcr.analysisRow.modelRuntimeRef &&
    serviceWinRtOcr.analysisRow.modelId &&
    serviceWinRtOcr.analysisRow.promptOrTemplateVersion
  ),
  localOnlyProcessingProved: Boolean(
    scheduler.screenSchedulerSummary.rawImageRetained === false &&
    winRtOcr.proof.localOnly &&
    vlmWorker.assertions.localOnlyNoRemoteAi
  ),
};

const failedAssertions = Object.entries(assertions)
  .filter(([, passed]) => !passed)
  .map(([name]) => name);

if (failedAssertions.length > 0) {
  throw new Error(`WP17 aggregate proof failed: ${failedAssertions.join(', ')}`);
}

const proofSummary = {
  proof: 'screen-local-ocr-vision-runtime-model-proof',
  proofTier: 'P2_AGGREGATED_CONTRACT_RUNTIME_PROOF',
  artifact: 'output/screen-plan-proof/17-local-ocr-vision-runtime-model/proof-summary.json',
  assertions,
  sourceArtifacts: Object.fromEntries(
    Object.entries(requiredArtifacts).map(([name, path]) => [name, path.replace(`${repoRoot}\\`, '')])
  ),
  runtimeMetadata: {
    winRtOcr: {
      modelRuntimeRef: serviceWinRtOcr.analysisRow.modelRuntimeRef,
      modelId: serviceWinRtOcr.analysisRow.modelId,
      promptOrTemplateVersion: serviceWinRtOcr.analysisRow.promptOrTemplateVersion,
      providerKind: serviceWinRtOcr.analysisRow.providerKind,
    },
    vlmWorker: vlmWorker.constants,
  },
  nonClaims: [
    'This aggregates existing screen OCR/VLM contract and runtime proof; it does not run a new live model session.',
    'This does not claim production OCR/VLM model quality, cross-platform OCR/VLM parity, or authenticated-account coverage.',
    'This does not claim final enforcement or live-view runtime.',
  ],
  validationCommands: ['node scripts/test/screen-local-ocr-vision-runtime-model-proof.mjs'],
};

await mkdir(outputRoot, { recursive: true });
await writeFile(resolve(outputRoot, 'proof-summary.json'), `${JSON.stringify(proofSummary, null, 2)}\n`);

console.log(`screen-local-ocr-vision-runtime-model-proof-ok:${proofSummary.proofTier}`);
console.log(`artifact=${resolve(outputRoot, 'proof-summary.json')}`);
