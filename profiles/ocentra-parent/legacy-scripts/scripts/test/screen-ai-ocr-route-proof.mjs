import { spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'ocr-route');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');

await mkdir(outputRoot, { recursive: true });
runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { ScreenAnalysisResultSchema } = await import('@ocentra-parent/schema-domain/screen-evidence-result');
const { ScreenEvidenceSchemaVersion } = await import('@ocentra-parent/schema-domain/screen-evidence-primitives');
const { PolicyDecisionHandoffState, PolicyDecisionSchema } = await import('@ocentra-parent/schema-domain/policy');

const evidenceRef = {
  evidenceId: 'screen-ocr-route-proof-evidence',
  kind: 'journal-entry',
  digest: 'sha256:screen-ocr-route-proof-typed-text',
  uri: null,
};
const parentEvidenceRef = {
  evidenceReferenceId: 'screen-ocr-route-proof-parent-evidence',
  kind: 'activity-event',
  observedAt: '2026-06-03T20:45:00.000Z',
};

const ocrText = 'VPN proxy bypass tool private tunnel unblock school network';
const screenResult = ScreenAnalysisResultSchema.parse({
  schemaVersion: ScreenEvidenceSchemaVersion,
  screenAnalysisResultId: 'screen-analysis-ocr-route-bypass-tool',
  queueJobId: 'screen-queue-ocr-route-bypass-tool',
  analyzedAt: parentEvidenceRef.observedAt,
  modelRuntimeRef: 'local-ocr-runtime-proof',
  modelId: 'local-ocr-visible-text-route',
  providerKind: 'localOcr',
  promptOrTemplateVersion: 'screen-ocr-visible-text-route-v1',
  captureReason: 'manualParentTestCapture',
  captureScope: 'selectedWindow',
  capabilityStatus: 'ready',
  summary: 'OCR text indicates a bypass tool surface.',
  visibleCategoryCandidates: [
    {
      category: 'bypassTool',
      confidence: 0.92,
      evidenceRefs: [evidenceRef],
    },
  ],
  primaryCategory: 'bypassTool',
  riskSignals: [
    {
      signal: 'possibleBypassTool',
      confidence: 0.92,
      evidenceRefs: [evidenceRef],
    },
  ],
  ocrTextSnippets: [
    {
      text: ocrText,
      confidence: 0.92,
      evidenceRefs: [evidenceRef],
    },
  ],
  redactionNotes: [],
  confidence: 0.92,
  uncertaintyReason: null,
  sourceEvidenceRefs: [evidenceRef],
  imageDigest: 'sha256:screen-ocr-route-proof-typed-text',
  rawImageRetained: false,
  imageDeletionState: 'deleted',
  custodyState: 'child-device-query-store',
  policyEligible: true,
});

const policyDecision = PolicyDecisionSchema.parse({
  schemaVersion: 'v0.6',
  decisionId: 'screen-ocr-route-policy-decision',
  action: 'block',
  reasonCodes: ['screen-ocr-bypass-tool'],
  evidenceReferences: [parentEvidenceRef],
  ruleIds: ['screen-policy-ocr-bypass-tool'],
  localAiResultId: null,
  dryRun: true,
  enforcementHandoffState: PolicyDecisionHandoffState.Disabled,
  expiresAt: null,
});

const summary = {
  status: 'ok',
  proofKind: 'screen-ai-ocr-visible-text-route',
  artifact: artifactSummaryPath,
  providerKind: screenResult.providerKind,
  promptOrTemplateVersion: screenResult.promptOrTemplateVersion,
  ocrText,
  screenResult,
  policyDecision,
  assertion:
    'Typed local OCR text evidence can produce schema-valid screen analysis and a policy dry-run without a vision model or retained raw image.',
  nonClaims: [
    'This is an OCR route contract proof over typed OCR evidence.',
    'It does not claim a production OCR adapter, live capture, or final enforcement execution.',
  ],
};

await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ai-ocr-route-proof-ok ${artifactSummaryPath}`);

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
