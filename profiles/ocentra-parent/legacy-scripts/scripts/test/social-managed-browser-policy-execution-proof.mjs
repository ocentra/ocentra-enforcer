import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { isAbsolute, join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDirectory = join(repoRoot, 'output', 'browser-plan-proof', 'social-managed-browser-policy-execution-proof');
const resultDirectory = join(repoRoot, 'test-results', 'social-managed-browser-policy-execution-proof');
const proofPath = join(resultDirectory, 'proof.json');
const manifestPath = join(outputDirectory, '01-social-managed-browser-policy-execution-proof.md');

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  runManagedBrowserCompositedBlockProof();
  const managedEvidence = await latestManagedBrowserCompositedBlockEvidence();
  assertManagedEvidence(managedEvidence);

  const { buildSocialManagedBrowserPolicyExecution, summarizeSocialManagedBrowserPolicyExecution } = await importDist(
    'browser-domain',
    'social-managed-browser-policy-execution.js'
  );
  const { SocialParentPolicyCompilerInputSchema } = await importDist('schema-domain', 'social-policy-compiler.js');
  const { compileSocialParentPolicyCandidate } = await importDist(
    'browser-domain',
    'social-policy-candidate-compiler.js'
  );

  const decisionCandidate = compileSocialParentPolicyCandidate({
    input: SocialParentPolicyCompilerInputSchema.parse(policyInput()),
    decisionCandidateId: 'social-policy-decision-candidate-managed-youtube-block',
    decidedAt: managedEvidence.generatedAt,
    expiresAt: null,
    actionCandidate: 'block-candidate',
    reasonCodes: ['social-risk-high', 'video-safety-risk', 'parent-rule-match'],
    confidence: 'medium',
    fallbackUsed: false,
    parentApprovalRequired: false,
  });

  const execution = buildSocialManagedBrowserPolicyExecution({
    executionId: 'social-managed-browser-policy-execution',
    sourceDecisionCandidate: decisionCandidate,
    executionEvidenceRefs: [
      evidenceRef('managed-browser-composited-block', managedEvidence.evidencePath),
      evidenceRef('managed-browser-composited-block-screenshot', managedEvidence.screenshotPath),
      evidenceRef('social-policy-decision-candidate', decisionCandidate.decisionCandidateId),
    ],
    managedBrowserInterventionEvidenceRef: evidenceRef(
      'managed-browser-composited-block',
      managedEvidence.evidencePath
    ),
    childInterventionEndpointRef: evidenceRef('child-agent-intervention-endpoint', managedEvidence.childAgentEndpoint),
    targetUrlEvidenceRef: evidenceRef('managed-social-video-target', managedEvidence.requestedUrl),
    screenshotEvidenceRefs: [
      evidenceRef('managed-browser-composited-block-screenshot', managedEvidence.screenshotPath),
    ],
    createdAt: managedEvidence.generatedAt,
  });
  const summary = summarizeSocialManagedBrowserPolicyExecution(execution);
  const proof = {
    schemaVersion: 1,
    proofMode: 'social-managed-browser-policy-execution-proof',
    generatedAt: new Date().toISOString(),
    branch: git(['branch', '--show-current']),
    commit: git(['rev-parse', 'HEAD']),
    baseCommit: git(['rev-parse', 'origin/main']),
    sourceEvidence: {
      managedBrowserCompositedBlockProof: relativePath(managedEvidence.evidencePath),
      managedBrowserScreenshot: relativePath(managedEvidence.screenshotPath),
      childAgentEndpoint: managedEvidence.childAgentEndpoint,
      liveSurface: 'real-youtube-watch-page',
      rawUrlPersistedInThisProof: false,
      rawPageContentPersistedInThisProof: false,
    },
    execution,
    summary,
    noClaimBoundaries: {
      unmanagedBrowserClaimed: false,
      broadOsEnforcementClaimed: false,
      providerDeliveryAttempted: false,
      nativeAppControlClaimed: false,
      applePlatformClaimed: false,
      externalProviderDeliveryClaimed: false,
      cloudRoutingClaimed: false,
      androidPhysicalBrowserRoleClaimed: false,
      iosFamilyControlsClaimed: false,
    },
    validations: {
      managedBrowserHarnessAssertionsPassed: Object.values(managedEvidence.assertions).every(Boolean),
      policyDecisionRemainsNonFinalBeforeExecution: decisionCandidate.finalPolicyDecisionClaimed === false,
      executionClaimsManagedSessionOnly:
        summary.finalPolicyExecutionClaimed === true &&
        summary.browserMutationObserved === true &&
        summary.childInterventionExecuted === true &&
        summary.managedInterventionEnforced === true &&
        summary.broadOsEnforcementClaimed === false &&
        summary.unmanagedBrowserClaimed === false,
    },
  };

  if (!Object.values(proof.validations).every(Boolean)) {
    throw new Error(`Social managed browser policy execution proof failed: ${JSON.stringify(proof.validations)}`);
  }

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(manifestPath, `${markdownFor(proof)}\n`);

  console.log('social-managed-browser-policy-execution-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(manifestPath)}`);
  console.log(`managedBrowserEvidence=${relativePath(managedEvidence.evidencePath)}`);
  console.log(`screenshot=${relativePath(managedEvidence.screenshotPath)}`);
}

function runManagedBrowserCompositedBlockProof() {
  execFileSync(...npmCommand(['run', 'test:managed-browser-composited-block']), {
    cwd: repoRoot,
    stdio: 'inherit',
  });
}

async function latestManagedBrowserCompositedBlockEvidence() {
  const evidenceRoot = join(repoRoot, 'test-results', 'managed-browser-composited-block-proof');
  const entries = await readdir(evidenceRoot, { withFileTypes: true });
  const candidates = entries
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map((entry) => join(evidenceRoot, entry.name))
    .sort((left, right) => statSync(right).mtimeMs - statSync(left).mtimeMs);
  if (candidates.length === 0 || candidates[0] === undefined) {
    throw new Error('Managed browser composited block proof did not write an evidence JSON file');
  }
  const evidencePath = candidates[0];
  return {
    ...JSON.parse(await readFile(evidencePath, 'utf8')),
    evidencePath,
  };
}

function assertManagedEvidence(evidence) {
  const assertions = evidence.assertions ?? {};
  const failures = Object.entries({
    backdropRendered: assertions.backdropRendered === true,
    blockMarkerPresent: assertions.blockMarkerPresent === true,
    capturedTargetBeforeBlock: assertions.capturedTargetBeforeBlock === true,
    childAgentEndpointRendered: assertions.childAgentEndpointRendered === true,
    targetUrlShown: assertions.targetUrlShown === true,
    screenshotExists:
      typeof evidence.screenshotPath === 'string' && existsSync(resolveEvidencePath(evidence.screenshotPath)),
    endpointIsChildAgent:
      evidence.childAgentEndpoint === '/api/browser/intervention/page' &&
      typeof evidence.blockedPageUrl === 'string' &&
      evidence.blockedPageUrl.includes('/api/browser/intervention/page?target='),
  })
    .filter(([, passed]) => !passed)
    .map(([name]) => name);

  if (failures.length > 0) {
    throw new Error(`Managed browser composited block evidence is incomplete: ${failures.join(', ')}`);
  }
}

function policyInput() {
  return {
    schemaVersion: 'v0.6',
    compileRequestId: 'social-policy-compile-request-managed-youtube-block',
    familyId: 'family-main',
    childProfileId: 'child-profile-middle-school',
    deviceId: 'child-device-windows-managed-browser',
    requestedAt: '2026-06-08T22:30:00.000Z',
    policyVersionRef: 'policy-version-social-video-managed-block',
    targetKind: 'social-video',
    sourceEvidenceRefs: ['parent-evidence-social-video-route', 'parent-evidence-managed-browser-composited-block'],
    signalSetRefs: ['social-riskbenefit-signal-set-video'],
    parentRuleRefs: ['parent-rule-school-night-video'],
    scheduleContextRefs: ['schedule-context-school-night'],
    timeBudgetContextRefs: ['time-budget-context-social-video-daily'],
    scheduleState: 'outside-allowed-window',
    timeBudgetState: 'budget-low',
    compilerMode: 'contract-only',
    rawSignalPayloadIncluded: false,
    rawModelTextIncluded: false,
    activityDomainObjectIncluded: false,
    finalDecisionClaimedByInput: false,
    runtimeGateClaimedByInput: false,
    uiClaimedByInput: false,
    enforcementClaimedByInput: false,
    nativeAppControlClaimed: false,
    platformConnectorClaimed: false,
  };
}

function markdownFor(proof) {
  return [
    '# Social Managed Browser Policy Execution Proof',
    '',
    `Generated: ${proof.generatedAt}`,
    `Branch: ${proof.branch}`,
    `Commit: ${proof.commit}`,
    `Base: ${proof.baseCommit}`,
    '',
    `Managed browser evidence: ${proof.sourceEvidence.managedBrowserCompositedBlockProof}`,
    `Managed browser screenshot: ${proof.sourceEvidence.managedBrowserScreenshot}`,
    `Live surface: ${proof.sourceEvidence.liveSurface}`,
    `Child-agent endpoint: ${proof.sourceEvidence.childAgentEndpoint}`,
    '',
    `Final policy execution claimed: ${proof.summary.finalPolicyExecutionClaimed}`,
    `Browser mutation observed: ${proof.summary.browserMutationObserved}`,
    `Child intervention executed: ${proof.summary.childInterventionExecuted}`,
    `Managed intervention enforced: ${proof.summary.managedInterventionEnforced}`,
    '',
    'No-claim boundaries preserved:',
    `- Unmanaged browser claimed: ${proof.noClaimBoundaries.unmanagedBrowserClaimed}`,
    `- Broad OS enforcement claimed: ${proof.noClaimBoundaries.broadOsEnforcementClaimed}`,
    `- Provider delivery attempted: ${proof.noClaimBoundaries.providerDeliveryAttempted}`,
    `- Native app control claimed: ${proof.noClaimBoundaries.nativeAppControlClaimed}`,
    `- Apple platform claimed: ${proof.noClaimBoundaries.applePlatformClaimed}`,
    '',
    'This proof chains a schema-domain social policy decision candidate to a real',
    'managed-browser composited block run. The managed-browser harness loads a',
    'real YouTube watch page, captures it through CDP, renders the shared child',
    'intervention page through the Rust child-agent endpoint, and observes the',
    'browser tab on the intervention endpoint. The proof does not claim unmanaged',
    'browser control, broad OS enforcement, provider delivery, native app control,',
    'or Apple platform support.',
  ].join('\n');
}

function importDist(packageName, fileName) {
  return import(pathToFileURL(join(repoRoot, 'packages', packageName, 'dist', fileName)).href);
}

function evidenceRef(prefix, value) {
  return `${prefix}-${sha256(String(value)).slice(0, 16)}`;
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function resolveEvidencePath(path) {
  return isAbsolute(path) ? path : join(repoRoot, path);
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
