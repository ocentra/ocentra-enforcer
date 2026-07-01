import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

export const RepoRoot = process.cwd();
export const OutputRoot = resolve(RepoRoot, 'output', 'screen-ai-pipeline-proof', 'final-product-path');
export const ProofPath = join(OutputRoot, 'proof-summary.json');
export const SnapshotPath = join(OutputRoot, '00-source-snapshot.md');
export const CommandsPath = join(OutputRoot, '14-validation-commands.log');

export const LiveScenarioIds = [
  'youtube-ordinary-video',
  'youtube-education-video',
  'vimeo-video',
  'facebook-social-surface',
  'browser-game',
  'shopping-page',
  'school-productivity',
  'native-app',
  'protected-unsupported-state',
];

export const BrowserScenarioIds = new Set([
  'youtube-ordinary-video',
  'youtube-education-video',
  'vimeo-video',
  'facebook-social-surface',
  'browser-game',
  'shopping-page',
  'school-productivity',
]);

export const SourcePaths = {
  actionDispatch: 'output/screen-ai-pipeline-proof/action-dispatch/proof-summary.json',
  blockActionDispatch: 'output/screen-ai-pipeline-proof/block-action-dispatch/proof-summary.json',
  deletionRetentionCustody: 'output/screen-ai-pipeline-proof/deletion-retention-custody/proof-summary.json',
  aiPlanClosure: 'output/ai-plan-proof/local-ai-plan-closure-audit/proof-summary.json',
  adapterBlockerLedger: 'output/screen-ai-pipeline-proof/adapter-blocker-ledger/proof-summary.json',
  adapterDependencyHandoff: 'output/screen-ai-pipeline-proof/adapter-dependency-handoff/proof-summary.json',
  finalAdapterAudit: 'output/screen-ai-pipeline-proof/final-adapter-dependency-audit/proof-summary.json',
  childAgentPolicyAuthority: 'output/ai-plan-proof/child-agent-ai-policy-authority-proof/proof-summary.json',
  householdMeshEventBridge: 'output/ai-plan-proof/household-mesh-event-bridge-proof/proof-summary.json',
  householdMeshScreenAi: 'output/screen-ai-pipeline-proof/household-mesh-screen-ai/proof-summary.json',
  householdProviderRouteSelection:
    'output/ai-plan-proof/household-ai-provider-route-selection-proof/proof-summary.json',
  householdProviderResultValidation: 'output/ai-plan-proof/household-ai-provider-result-validation/proof-summary.json',
  liveOperator: 'output/screen-ai-pipeline-proof/live-operator/proof-summary.json',
  liveOperatorArtifactGate: 'output/screen-ai-pipeline-proof/live-operator-artifact-gate/proof-summary.json',
  liveOperatorEvidenceBundle: 'output/screen-ai-pipeline-proof/live-operator-evidence-bundle/proof-summary.json',
  liveOperatorAi: 'output/ai-plan-proof/live-operator/proof-summary.json',
  mobileDormantProvider: 'output/ai-plan-proof/mobile-dormant-ai-provider-proof/proof-summary.json',
  noRawScreenTransferMesh: 'output/ai-plan-proof/no-raw-screen-transfer-mesh/proof-summary.json',
  portalChain: 'output/screen-ai-pipeline-proof/portal-chain/proof-summary.json',
  protectedSurface: 'output/screen-ai-pipeline-proof/protected-surface/proof-summary.json',
  readModel: 'output/ai-plan-proof/screen-summary-parent-explanation-read-model/proof-summary.json',
  retentionSweeper: 'output/screen-ai-pipeline-proof/service-retention-sweeper/proof-summary.json',
  screenPlanClosure: 'output/screen-plan-proof/screen-plan-closure-audit/proof-summary.json',
  serviceReadModel: 'output/ai-plan-proof/screen-summary-parent-explanation-service-read-model/proof-summary.json',
  serviceAnalysisRowReady: 'output/screen-ai-pipeline-proof/screen-service-analysis-row-ready/proof-summary.json',
  serviceCaptureEventProducer:
    'output/screen-ai-pipeline-proof/screen-service-capture-event-producer/proof-summary.json',
  serviceDeletionEventProducer:
    'output/screen-ai-pipeline-proof/screen-service-deletion-event-producer/proof-summary.json',
  serviceEventBridge: 'output/screen-ai-pipeline-proof/screen-service-event-bridge/proof-summary.json',
  serviceEventSubscription: 'output/screen-ai-pipeline-proof/screen-service-event-subscription/proof-summary.json',
  servicePolicyRefProducer: 'output/screen-ai-pipeline-proof/screen-service-policy-ref-producer/proof-summary.json',
  serviceWinRtOcrPolicy: 'output/screen-ai-pipeline-proof/service-winrt-ocr-policy/proof-summary.json',
};

export function readJson(path, assert) {
  const absolute = resolve(RepoRoot, path);
  assert(existsSync(absolute), `missing artifact ${path}`);
  return JSON.parse(readFileSync(absolute, 'utf8'));
}

export function existsPath(path) {
  return Boolean(path) && existsSync(path);
}

export function repoPath(path) {
  return resolve(RepoRoot, path);
}

export function writeProofOutputs(proof) {
  mkdirSync(OutputRoot, { recursive: true });
  writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
  writeFileSync(SnapshotPath, sourceSnapshot(proof));
  writeFileSync(CommandsPath, validationCommands());
}

function sourceSnapshot(proof) {
  const rows = Object.entries(SourcePaths)
    .map(([name, path]) => `- ${name}: \`${path}\``)
    .join('\n');
  return `# Screen AI Final Product Path Proof\n\nGenerated: ${proof.generatedAt}\n\n## Source Artifacts\n\n${rows}\n\n## Closure\n\n\`\`\`json\n${JSON.stringify(proof.closure, null, 2)}\n\`\`\`\n`;
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-ai-live-operator-evidence-bundle.mjs',
    'node scripts/test/screen-ai-live-operator-evidence-bundle.mjs',
    'node --check scripts/test/screen-ai-final-product-path-proof.mjs',
    'node scripts/test/screen-ai-final-product-path-proof.mjs',
    'git diff --check',
    'npm run lanes:guard',
    'npm run hub:guard',
    '',
  ].join('\n');
}
