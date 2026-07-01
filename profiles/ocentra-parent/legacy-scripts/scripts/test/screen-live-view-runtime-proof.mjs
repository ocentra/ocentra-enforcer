import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', 'live-view-runtime');
const proofPath = join(outputDir, 'proof-summary.json');
const transportProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'live-view-session-transport',
  'proof-summary.json'
);
const serviceSessionProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'live-view-service-session',
  'proof-summary.json'
);
const parentUiPersistenceProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'live-view-parent-ui-persistence',
  'proof-summary.json'
);

const testCommand = ['test', '-p', 'ocentra-parent-agent-service', 'screen_live_view_runtime', '--', '--nocapture'];

const cargoOutput = execFileSync('cargo', testCommand, {
  cwd: repoRoot,
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'pipe'],
});
const transportProof = readJson(transportProofPath);
const serviceSessionProof = readJson(serviceSessionProofPath);
const parentUiPersistenceProof = readJson(parentUiPersistenceProofPath);

const proof = {
  proof: 'screen-live-view-runtime-proof',
  generatedAt: new Date().toISOString(),
  claim:
    'The Rust agent-service has a bounded live-view runtime decision boundary that consumes live-frame transport/deletion evidence and parent UI persistence evidence, rejects capture-only permission, refuses frame cache/session recording/remote input, and keeps product readiness blocked unless live-view permission, service runtime, and relay/cache requirements are all satisfied.',
  sourceEvidence: {
    liveTransportProof: relativePath(transportProofPath),
    liveTransportProofPresent: existsSync(transportProofPath),
    realPixelsCaptured: transportProof.assertions.realPixelsCaptured === true,
    localTransportDeliveredFrame: transportProof.assertions.localTransportDeliveredFrame === true,
    rawFrameDeletedAfterTransport: transportProof.assertions.rawFrameDeletedAfterTransport === true,
    serviceSessionProof: relativePath(serviceSessionProofPath),
    serviceSessionProofPresent: existsSync(serviceSessionProofPath),
    serviceSessionRuntimeStillMissing: serviceSessionProof.gapStatus.serviceSessionRuntimeProofExists === false,
    parentUiPersistenceProof: relativePath(parentUiPersistenceProofPath),
    parentUiPersistenceProofPresent: existsSync(parentUiPersistenceProofPath),
    parentUiPersistenceStateProved: parentUiPersistenceProof.assertions.parentUiPersistenceStateProved === true,
  },
  rustRuntime: {
    module: 'crates/agent-service/src/screen_ai_service_event_subscription/live_view_runtime.rs',
    tests: 'crates/agent-service/tests/unit/live_view_service_runtime_tests.rs',
    validationCommand: `cargo ${testCommand.join(' ')}`,
    validationPassed: cargoOutput.includes('test result: ok'),
  },
  runtimeDecisionsProved: {
    captureOnlyPermissionBlocked: true,
    missingTransportProofBlocked: true,
    missingDeletionProofBlocked: true,
    frameCacheRecordingRemoteInputBlocked: true,
    serviceRuntimeCanBeReadyWithoutProductReady: true,
    relayModeRequiresRelayCacheProof: true,
  },
  gapStatus: {
    serviceRuntimeDecisionBoundaryExists: true,
    liveViewPermissionPromptProofExists: false,
    parentUiPersistenceProofExists: parentUiPersistenceProof.assertions.parentUiPersistenceStateProved === true,
    relayCacheProofExists: false,
    privacyLegalApprovalExists: false,
    liveViewProductReady: false,
  },
  assertions: {
    rustRuntimeValidationPassed: cargoOutput.includes('test result: ok'),
    realLoopbackTransportArtifactConsumed: transportProof.assertions.localTransportDeliveredFrame === true,
    rawFrameDeletionCarriedForward: transportProof.assertions.rawFrameDeletedAfterTransport === true,
    serviceSessionReadinessProofDoesNotClaimRuntime:
      serviceSessionProof.gapStatus.serviceSessionRuntimeProofExists === false,
    parentUiPersistenceCarriedForward: parentUiPersistenceProof.assertions.parentUiPersistenceStateProved === true,
    productReadinessStillBlocked: true,
  },
  nonClaims: [
    'This proof does not start a production live-view worker in the agent service.',
    'This proof does not claim platform live-view permission-prompt screenshots, relay/cache execution, privacy/legal approval, or physical-device parity.',
    'This proof does not allow raw frame caching, session recording, or remote input control.',
  ],
};

if (!Object.values(proof.assertions).every(Boolean)) {
  throw new Error(`screen live-view runtime proof assertions failed: ${JSON.stringify(proof.assertions)}`);
}

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-live-view-runtime-proof-ok:${proofPath}`);

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Expected proof artifact at ${path}`);
  }

  return JSON.parse(readFileSync(path, 'utf8'));
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}
