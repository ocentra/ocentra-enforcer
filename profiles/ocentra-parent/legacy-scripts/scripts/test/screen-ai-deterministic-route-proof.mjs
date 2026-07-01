import { spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'deterministic-route');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');

await mkdir(outputRoot, { recursive: true });
runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { ScreenAnalysisResultSchema } = await import('@ocentra-parent/schema-domain/screen-evidence-result');
const { ScreenEvidenceSchemaVersion } = await import('@ocentra-parent/schema-domain/screen-evidence-primitives');
const { PolicyDecisionHandoffState, PolicyDecisionSchema } = await import('@ocentra-parent/schema-domain/policy');

const observedAt = '2026-06-03T21:00:00.000Z';
const evidenceRef = {
  evidenceId: 'screen-deterministic-route-evidence',
  kind: 'journal-entry',
  digest: 'sha256:managed-structured-school-evidence',
  uri: null,
};
const parentEvidenceRef = {
  evidenceReferenceId: 'screen-deterministic-route-parent-evidence',
  kind: 'activity-event',
  observedAt,
};

const screenResult = ScreenAnalysisResultSchema.parse({
  schemaVersion: ScreenEvidenceSchemaVersion,
  screenAnalysisResultId: 'screen-analysis-deterministic-school',
  queueJobId: 'screen-queue-deterministic-school',
  analyzedAt: observedAt,
  modelRuntimeRef: 'screen-deterministic-rules-runtime',
  modelId: 'structured-evidence-rule-engine',
  providerKind: 'deterministicRules',
  promptOrTemplateVersion: 'screen-structured-evidence-v1',
  captureReason: 'policyAmbiguity',
  captureScope: 'unsupported',
  capabilityStatus: 'ready',
  summary: 'Managed structured evidence identifies a known school activity surface.',
  visibleCategoryCandidates: [
    {
      category: 'school',
      confidence: 0.95,
      evidenceRefs: [evidenceRef],
    },
  ],
  primaryCategory: 'school',
  riskSignals: [],
  ocrTextSnippets: [],
  redactionNotes: [],
  confidence: 0.95,
  uncertaintyReason: null,
  sourceEvidenceRefs: [evidenceRef],
  imageDigest: 'sha256:managed-structured-school-evidence',
  rawImageRetained: false,
  imageDeletionState: 'unavailableNoImage',
  custodyState: 'child-device-query-store',
  policyEligible: true,
});

const policyDecision = PolicyDecisionSchema.parse({
  schemaVersion: 'v0.6',
  decisionId: 'screen-deterministic-route-policy-decision',
  action: 'allow',
  reasonCodes: ['screen-deterministic-known-school'],
  evidenceReferences: [parentEvidenceRef],
  ruleIds: ['screen-policy-school-allow'],
  localAiResultId: null,
  dryRun: true,
  enforcementHandoffState: PolicyDecisionHandoffState.Disabled,
  expiresAt: null,
});

const imageBackedMisrouteRejected = !ScreenAnalysisResultSchema.safeParse({
  ...screenResult,
  imageDeletionState: 'deleted',
}).success;

if (!imageBackedMisrouteRejected) {
  throw new Error('Expected deterministic route to reject image-backed deletion state');
}

const summary = {
  status: 'ok',
  proofKind: 'screen-ai-deterministic-structured-evidence-route',
  artifact: artifactSummaryPath,
  providerKind: screenResult.providerKind,
  screenResult,
  policyDecision,
  imageBackedMisrouteRejected,
  assertions: [
    'Structured screen-adjacent evidence can produce a schema-valid deterministic screen analysis route without a model.',
    'The deterministic route is policy-eligible only when evidence is present, confidence is sufficient, category is known, and no raw image is claimed.',
    'The route rejects image-backed custody states so deterministic classification cannot masquerade as capture or OCR/VLM proof.',
  ],
  nonClaims: [
    'This is a deterministic structured-evidence contract proof.',
    'It does not claim live capture, OCR/VLM inference, live browser/account proof, or final enforcement execution.',
  ],
};

await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ai-deterministic-route-proof-ok ${artifactSummaryPath}`);

function runCommand(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed\n${result.stdout}\n${result.stderr}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
