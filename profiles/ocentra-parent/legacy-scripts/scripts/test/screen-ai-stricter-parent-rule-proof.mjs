import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const SourceProofPath = resolve(
  RepoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-winrt-ocr-policy',
  'proof-summary.json'
);
const SourceDecisionPath = resolve(
  RepoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-winrt-ocr-policy',
  'policy-decision.json'
);
const OutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'screen-ai-stricter-parent-rule-proof');
const TestResultRoot = resolve(RepoRoot, 'test-results', 'screen-ai-stricter-parent-rule-proof');
const ProofPath = join(OutputRoot, 'proof-summary.json');
const TestResultPath = join(TestResultRoot, 'proof.json');
const ClaimBoundaries = {
  localAiAuthorityClaimed: false,
  remoteAiUsed: false,
  apiAiUsed: false,
  rawImageRetained: false,
  enforcementClaimed: false,
};

runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { buildScreenAiStricterParentRuleProof } =
  await import('@ocentra-parent/schema-domain/screen-ai-stricter-parent-rule-proof');
const sourceProof = JSON.parse(readFileSync(SourceProofPath, 'utf8'));
const sourceDecision = JSON.parse(readFileSync(SourceDecisionPath, 'utf8'));
const generatedAt = new Date().toISOString();
const stricterParentRule = parentRuleFor(sourceProof.sourceAnalysisRow.primaryCategory);
const proofSnapshot = buildScreenAiStricterParentRuleProof({
  schemaVersion: 'v0.6',
  proofId: 'screen-ai-stricter-parent-rule-proof',
  generatedAt,
  sourceProof: relativePath(SourceProofPath),
  sourceDecision,
  stricterParentRule,
  expectedFinalAction: stricterParentRule.action,
  claimBoundaries: ClaimBoundaries,
});

if (proofSnapshot.finalDecision.action !== stricterParentRule.action) {
  throw new Error('screen AI stricter parent rule proof failed: final action did not preserve parent rule');
}

const proof = {
  status: 'ok',
  proofKind: 'screen-ai-stricter-parent-rule-proof',
  generatedAt,
  sourceProof: relativePath(SourceProofPath),
  sourceDecision: relativePath(SourceDecisionPath),
  output: relativePath(ProofPath),
  sourceLiveSurface: sourceProof.sourceLiveSurface,
  proof: proofSnapshot,
  assertions: {
    consumedRealServiceOcrPolicyArtifact: sourceProof.assertions.sourceUsedLivePublicBrowserPixels === true,
    sourcePolicyDecisionParsedByParentDomain: sourceProof.assertions.policyDecisionParsedByParentDomain === true,
    localAiSuggestedAllow: proofSnapshot.sourceLocalAiAction === 'allow',
    stricterParentRuleSelected: proofSnapshot.finalAction === stricterParentRule.action,
    evidenceRefsPreserved:
      proofSnapshot.finalDecision.evidenceReferences.length === sourceDecision.evidenceReferences.length,
    localAiResultRefPreserved: proofSnapshot.finalDecision.localAiResultId === sourceDecision.localAiResultId,
    dryRunOnly:
      proofSnapshot.finalDecision.dryRun && proofSnapshot.finalDecision.enforcementHandoffState === 'disabled',
    noLocalAiAuthorityClaimed: !proofSnapshot.claimBoundaries.localAiAuthorityClaimed,
  },
  nonClaims: [
    'This proof consumes the existing real service WinRT OCR policy decision and proves stricter parent rules win over local AI output.',
    'It does not rerun live capture, enable enforcement, retain raw screenshots, use remote/API AI, or make local AI policy authority.',
  ],
};

mkdirSync(OutputRoot, { recursive: true });
mkdirSync(TestResultRoot, { recursive: true });
writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(TestResultPath, `${JSON.stringify({ status: 'ok', proof: relativePath(ProofPath) }, null, 2)}\n`);
console.log(`screen-ai-stricter-parent-rule-proof-ok:${ProofPath}`);

function parentRuleFor(category) {
  return {
    ruleId: `screen-ai-parent-rule-block-${category}`,
    target: {
      targetId: `screen-ai-${category}-category-target`,
      targetType: 'category',
      targetValue: category,
    },
    action: 'block',
    scheduleId: null,
    priority: 100,
    reasonCode: `screen-ai-parent-rule-block-${category}-over-ai-allow`,
    createdBy: {
      actorId: 'parent-policy-author',
      role: 'parent',
    },
    enabled: true,
    effectiveFrom: null,
    effectiveUntil: null,
  };
}

function runCommand(command, args) {
  execFileSync(command, args, { cwd: RepoRoot, stdio: 'inherit' });
}

function relativePath(path) {
  return relative(RepoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
