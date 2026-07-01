import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const RepoRoot = process.cwd();
const SourceProofPath = resolve(
  RepoRoot,
  'output',
  'ai-plan-proof',
  'screen-summary-parent-explanation',
  'proof-summary.json'
);
const OutputRoot = resolve(RepoRoot, 'output', 'ai-plan-proof', 'screen-summary-parent-explanation-read-model');
const ProofPath = join(OutputRoot, 'proof-summary.json');
const ClaimBoundaries = {
  rawImageShown: false,
  rawImageRetained: false,
  remoteAiUsed: false,
  apiAiUsed: false,
  policyAuthorityClaimed: false,
  portalRuntimeClaimed: false,
  enforcementClaimed: false,
};

runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

const { buildScreenSummaryParentExplanationReadModelSnapshot } =
  await import('@ocentra-parent/schema-domain/local-ai-screen-summary-parent-explanation-read-model');
const sourceProof = JSON.parse(readFileSync(SourceProofPath, 'utf8'));
const generatedAt = new Date().toISOString();
const snapshot = buildScreenSummaryParentExplanationReadModelSnapshot({
  schemaVersion: 'v0.6',
  snapshotId: 'screen-summary-parent-explanation-read-model-snapshot',
  generatedAt,
  sourceProof: relativePath(SourceProofPath),
  sourceRows: sourceProof.rows,
  claimBoundaries: ClaimBoundaries,
});
const failures = snapshot.rows.filter(rowFailsValidation).map((row) => `${row.rowId} failed read-model invariants`);

if (failures.length > 0) {
  throw new Error(`Screen summary parent explanation read-model proof failed:\n${failures.join('\n')}`);
}

const proof = {
  status: 'ok',
  proofKind: 'screen-summary-parent-explanation-read-model-proof',
  generatedAt,
  sourceProof: relativePath(SourceProofPath),
  output: relativePath(ProofPath),
  snapshot,
  summary: {
    ...snapshot.summary,
    sourceProofStatus: sourceProof.status,
    sourceProofKind: sourceProof.proofKind,
    sourceRowCount: sourceProof.rows.length,
  },
  nonClaims: [
    'This proof converts existing screen-summary parent explanation proof rows into a parent-visible read model.',
    'This proof does not claim production portal rendering, remote/API AI, raw screenshot display, policy authority, or enforcement.',
  ],
};

mkdirSync(OutputRoot, { recursive: true });
writeFileSync(ProofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-summary-parent-explanation-read-model-proof-ok:${ProofPath}`);

function rowFailsValidation(row) {
  return (
    row.displayState !== 'ready-for-parent-explanation' ||
    row.screenSummaryRefs.length === 0 ||
    row.auditEvidenceRefs.length === 0 ||
    row.parentRuleRefs.length === 0 ||
    row.localModelRuntimeRefs.length === 0 ||
    !row.policyDryRun ||
    row.enforcementHandoffState === 'handed-off' ||
    !row.custodyLabels.includes('child-device-query-store') ||
    !row.deletionReasons.includes('screen-image-deleted') ||
    Object.values(row.claimBoundaries).some((claim) => claim !== false)
  );
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
