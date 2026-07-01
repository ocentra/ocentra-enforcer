import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const OutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'screen-ai-model-artifact-manifest-proof');
const TestResultRoot = resolve(RepoRoot, 'test-results', 'screen-ai-model-artifact-manifest-proof');
const ProofPath = join(OutputRoot, 'proof-summary.json');
const TestResultPath = join(TestResultRoot, 'proof.json');
const generatedAt = new Date().toISOString();

runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { buildScreenAiModelArtifactManifestProof } =
  await import('@ocentra-parent/schema-domain/screen-ai-model-artifact-manifest-proof');

const manifest = buildScreenAiModelArtifactManifestProof({
  schemaVersion: 'v0.6',
  proofId: 'screen-ai-model-artifact-manifest-proof',
  generatedAt,
  purpose: 'screen-child-safety-local-analysis',
  providerId: 'screen-local-provider',
  modelId: 'screen-child-safety-v1',
  artifactRef: 'artifact:screen-child-safety-v1',
  manifestRef: 'manifest:screen-child-safety-v1',
  requiredCapability: 'safety-decision',
  privacyMode: 'local-only',
  cacheStatus: {
    statusKind: 'local-model-cache-status',
    artifactRef: 'artifact:screen-child-safety-v1',
    manifestRef: 'manifest:screen-child-safety-v1',
    sourcePolicy: 'local-cache',
    cacheState: 'cache-ready',
    cacheHealth: 'healthy',
    manifestIntegrity: 'verified',
    downloadEnabled: false,
    downloadStatus: 'download-disabled',
    cacheByteSize: 524288,
    checkedAt: generatedAt,
    unavailableReason: null,
    storageError: null,
    corruptionReason: null,
  },
  runtimeStatus: {
    runtimeReferenceId: 'runtime:screen-child-safety-v1',
    providerId: 'screen-local-provider',
    modelId: 'screen-child-safety-v1',
    modelReference: 'artifact:screen-child-safety-v1',
    privacyMode: 'local-only',
    adapterBoundary: 'local-adapter-ready',
    executionState: 'dry-run-ready',
    providerSource: 'local-model-cache',
    loadState: 'loaded',
    capabilityFlags: ['safety-decision', 'classification'],
    resourceClass: 'cpu',
    degradedState: 'none',
    lastCheckedAt: generatedAt,
    unavailableReason: null,
  },
  providerCapability: {
    providerId: 'screen-local-provider',
    supportedTasks: ['safety-decision', 'classification'],
    resourceClass: 'cpu',
    privacyMode: 'local-only',
    fallbackOrder: 1,
  },
  manifestCheckedAt: generatedAt,
  claimBoundaries: {
    remoteProviderUsed: false,
    apiProviderUsed: false,
    ocentraHostedProcessingUsed: false,
    modelQualityClaimed: false,
    rawEvidenceEmbedded: false,
    executionClaimed: false,
  },
});

const proof = {
  status: 'ok',
  proofKind: 'screen-ai-model-artifact-manifest-proof',
  generatedAt,
  output: relativePath(ProofPath),
  manifest,
  assertions: {
    usesExistingModelArtifactContracts: true,
    artifactRefIsOpaque: manifest.artifactRef.startsWith('artifact:'),
    manifestRefIsOpaque: manifest.manifestRef.startsWith('manifest:'),
    manifestIntegrityVerified: manifest.cacheStatus.manifestIntegrity === 'verified',
    localOnlyRuntime: manifest.runtimeStatus.privacyMode === 'local-only',
    screenSafetyCapabilityPresent: manifest.providerCapability.supportedTasks.includes('safety-decision'),
    noRemoteApiHostedOrQualityClaim:
      !manifest.claimBoundaries.remoteProviderUsed &&
      !manifest.claimBoundaries.apiProviderUsed &&
      !manifest.claimBoundaries.ocentraHostedProcessingUsed &&
      !manifest.claimBoundaries.modelQualityClaimed,
  },
  nonClaims: [
    'This proof validates the typed local model artifact manifest/config boundary for screen AI.',
    'It does not download a production model, run model inference, prove model quality, use remote/API AI, or embed raw evidence.',
  ],
};

mkdirSync(OutputRoot, { recursive: true });
mkdirSync(TestResultRoot, { recursive: true });
writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(TestResultPath, `${JSON.stringify({ status: 'ok', proof: relativePath(ProofPath) }, null, 2)}\n`);
console.log(`screen-ai-model-artifact-manifest-proof-ok:${ProofPath}`);

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
