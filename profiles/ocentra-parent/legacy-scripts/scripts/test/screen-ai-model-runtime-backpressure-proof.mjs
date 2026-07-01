import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const PipelineOutputRoot = resolve(RepoRoot, 'output', 'screen-ai-pipeline-proof', 'model-runtime-backpressure');
const AiPlanOutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'model-runtime-backpressure');
const TestResultRoot = resolve(RepoRoot, 'test-results', 'screen-ai-model-runtime-backpressure-proof');
const PipelineProofPath = join(PipelineOutputRoot, 'proof-summary.json');
const AiPlanProofPath = join(AiPlanOutputRoot, 'proof-summary.json');
const TestResultPath = join(TestResultRoot, 'proof.json');
const generatedAt = new Date().toISOString();
const successfulCommands = [];

runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { buildScreenAiModelRuntimeBackpressureProof, screenAiModelRuntimeBackpressureSummary } =
  await import('@ocentra-parent/schema-domain/screen-ai-model-runtime-backpressure-proof');

const physicalDeviceId = 'child-laptop-physical-1';
const runtimeReferenceId = 'runtime:screen-child-safety-vlm';
const modelId = 'screen-child-safety-vlm';
const maxQueueDepth = 2;

const proof = buildScreenAiModelRuntimeBackpressureProof({
  schemaVersion: 'v0.6',
  proofId: 'screen-ai-model-runtime-backpressure-proof',
  generatedAt,
  maxQueueDepth,
  rows: [
    makeRow({
      jobNumber: 1,
      priority: 'policy-blocking',
      jobStatus: 'running',
      jobState: 'running',
      queuePosition: null,
      queuedHeavyJobCount: 0,
      backpressureAction: 'run-now',
      degradedState: 'none',
    }),
    makeRow({
      jobNumber: 2,
      priority: 'foreground',
      jobStatus: 'queued',
      jobState: 'queued',
      queuePosition: 1,
      queuedHeavyJobCount: 1,
      backpressureAction: 'enqueue',
      degradedState: 'none',
    }),
    makeRow({
      jobNumber: 3,
      priority: 'background-summary',
      jobStatus: 'queued',
      jobState: 'queued',
      queuePosition: 2,
      queuedHeavyJobCount: 2,
      backpressureAction: 'enqueue',
      degradedState: 'none',
    }),
    makeRow({
      jobNumber: 4,
      priority: 'cadence',
      jobStatus: 'degraded',
      jobState: 'overflow-degraded',
      queuePosition: null,
      queuedHeavyJobCount: 2,
      backpressureAction: 'reject-overload',
      degradedState: 'overloaded',
    }),
  ],
});
const summary = screenAiModelRuntimeBackpressureSummary(proof);

const proofSummary = {
  status: 'ok',
  proofKind: 'screen-ai-model-runtime-backpressure-proof',
  generatedAt,
  output: relativePath(PipelineProofPath),
  claimsProven: [
    'one heavy local screen AI model job can run per physical child device',
    'queued heavy screen AI jobs remain bounded by max queue depth',
    'policy-blocking child-safety work owns the active runtime lane ahead of lower-priority cadence/background work',
    'overflowed screen AI work becomes overloaded/degraded and cannot reach policy',
    'flood control does not fall back to remote/API providers',
    'flood control does not retain raw screenshot images',
  ],
  validationCommands: successfulCommands,
  nonClaims: [
    'This proof validates model-runtime flood-control/backpressure contracts only.',
    'It does not run live model inference, prove production model quality, render portal UI, dispatch enforcement, or claim physical household family-hub runtime.',
  ],
  summary,
  proof,
};

mkdirSync(PipelineOutputRoot, { recursive: true });
mkdirSync(AiPlanOutputRoot, { recursive: true });
mkdirSync(TestResultRoot, { recursive: true });
const serialized = `${JSON.stringify(proofSummary, null, 2)}\n`;
writeFileSync(PipelineProofPath, serialized);
writeFileSync(AiPlanProofPath, serialized);
writeFileSync(TestResultPath, `${JSON.stringify({ status: 'ok', proof: relativePath(PipelineProofPath) }, null, 2)}\n`);
console.log(`screen-ai-model-runtime-backpressure-proof-ok:${PipelineProofPath}`);

function makeRow({
  jobNumber,
  priority,
  jobStatus,
  jobState,
  queuePosition,
  queuedHeavyJobCount,
  backpressureAction,
  degradedState,
}) {
  return {
    jobId: `screen-model-job-${jobNumber}`,
    physicalDeviceId,
    sourceEncryptedQueueRef: `queue:encrypted-frame-${jobNumber}`,
    captureDigestRef: `digest:frame-${jobNumber}`,
    priority,
    requestedAt: generatedAt,
    modelId,
    runtimeReferenceId,
    providerDecision: {
      physicalDeviceId,
      jobClass: 'child-safety',
      jobStatus,
      selectedRuntimeReferenceId: runtimeReferenceId,
      queuePosition,
      unavailableReason: null,
      duplicateRuntimeBlocked: true,
    },
    jobState,
    queuePosition,
    maxQueueDepth,
    activeHeavyRuntimeCount: 1,
    queuedHeavyJobCount,
    backpressureAction,
    degradedState,
    unavailableReason: null,
    policyEligible: false,
    remoteProviderUsed: false,
    rawImageRetained: false,
  };
}

function relativePath(filePath) {
  return relative(RepoRoot, filePath).replaceAll('\\', '/');
}

function runCommand(command, args) {
  execFileSync(command, args, { cwd: RepoRoot, stdio: 'inherit' });
  successfulCommands.push(`${command} ${args.join(' ')}`);
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
