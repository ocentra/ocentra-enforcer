import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'ai-plan-proof', 'local-ai-policy-enforcement-consumption-proof');
const proofPath = join(outputDir, 'proof-summary.json');
const actionDispatchProofPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'action-dispatch',
  'proof-summary.json'
);
const handoffGuardProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'screen-ai-enforcement-handoff-guard',
  'proof-summary.json'
);
const childAuthorityProofPath = join(
  repoRoot,
  'output',
  'ai-plan-proof',
  'child-agent-ai-policy-authority-proof',
  'proof-summary.json'
);

const actionDispatchProof = readJson(actionDispatchProofPath);
const handoffGuardProof = readJson(handoffGuardProofPath);
const childAuthorityProof = readJson(childAuthorityProofPath);
const assertions = {
  actionDispatchConsumesPolicyDecision:
    actionDispatchProof.adapterCommandPolicyDecisionId === actionDispatchProof.screenPolicyDecisionId,
  policyDecisionLinkedToAdapter: actionDispatchProof.policyDecisionLinkedToAdapter === true,
  localAiResultLinkedOnlyThroughPolicy: actionDispatchProof.localAiResultLinkedToScreenPolicy === true,
  evidenceRefsLinkedToAdapter: actionDispatchProof.evidenceRefsLinkedToAdapter === true,
  rawImageDeletedBeforeDispatch: actionDispatchProof.rawImageDeletedBeforeDispatch === true,
  handoffGuardRequiresDryRunPolicy: handoffGuardProof.parsed.handoffMode === 'dry-run',
  handoffGuardRejectsRawMaterial: handoffGuardProof.parsed.rawMaterialIncluded === false,
  providerCannotPublishPolicyOrEnforcement:
    childAuthorityProof.assertions.providerCannotPublishPolicyOrEnforcement === true,
  childAgentOwnsPolicyDecision: childAuthorityProof.assertions.childAgentOwnsPolicyDecision === true,
};

if (Object.values(assertions).some((value) => value !== true)) {
  throw new Error(`Local AI policy enforcement consumption proof failed: ${JSON.stringify(assertions)}`);
}

const proof = {
  proof: 'local-ai-policy-enforcement-consumption-proof',
  generatedAt: new Date().toISOString(),
  sourceProofs: {
    actionDispatch: relativePath(actionDispatchProofPath),
    handoffGuard: relativePath(handoffGuardProofPath),
    childAuthority: relativePath(childAuthorityProofPath),
  },
  policyDecisionPath: {
    childPolicyDecisionId: childAuthorityProof.childPolicyDecision.decisionId,
    childPolicyLocalAiResultId: childAuthorityProof.childPolicyDecision.localAiResultId,
    adapterCommandPolicyDecisionId: actionDispatchProof.adapterCommandPolicyDecisionId,
    screenPolicyDecisionId: actionDispatchProof.screenPolicyDecisionId,
    adapterResultCode: actionDispatchProof.adapterResultCode,
    handoffMode: handoffGuardProof.parsed.handoffMode,
  },
  assertions,
  nonClaims: [
    'This proof composes existing policy authority, handoff guard, and Windows action-dispatch artifacts.',
    'It proves enforcement consumes policy decision refs rather than raw AI output or raw screen pixels.',
    'It does not add broad/browser/network/mobile enforcement, execute a production model, or claim final product-complete enforcement.',
  ],
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`local-ai-policy-enforcement-consumption-proof-ok:${relativePath(proofPath)}`);

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Missing required source proof: ${relativePath(path)}`);
  }
  return JSON.parse(readFileSync(path, 'utf8'));
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
