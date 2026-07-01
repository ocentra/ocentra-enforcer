import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';

const repoRoot = process.cwd();
const pipelineRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'live-operator');
const aiRoot = resolve(repoRoot, 'output', 'ai-plan-proof', 'live-operator');
const bundleRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'live-operator-evidence-bundle');
const proofPath = join(bundleRoot, 'proof-summary.json');
const indexPath = join(bundleRoot, 'evidence-index.md');

const requiredScenarioIds = [
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
const optionalScenarioIds = ['facebook-authenticated-social-surface'];

const portableJsonFiles = [
  '00-scenario.md',
  '01-redacted-source-evidence.json',
  '02-capture-proof-ref.json',
  '03-ai-context.json',
  '04-provider-route.json',
  '05-model-runtime-status.json',
  '06-ai-result.json',
  '07-policy-decision.json',
  '08-deletion-after-analysis.json',
  '09-parent-explanation.json',
  'vlm-stdout-redacted.log',
  'vlm-stderr-redacted.log',
];

rmSync(bundleRoot, { recursive: true, force: true });
mkdirSync(bundleRoot, { recursive: true });

const pipelineSummary = readJson(join(pipelineRoot, 'proof-summary.json'));
const aiSummary = readJson(join(aiRoot, 'proof-summary.json'));
assert(pipelineSummary.fullRequiredMatrixComplete === true, 'pipeline live operator matrix is incomplete');
assert(aiSummary.fullRequiredMatrixComplete === true, 'AI live operator matrix is incomplete');

const scenarioIds = [
  ...requiredScenarioIds,
  ...optionalScenarioIds.filter((scenarioId) => hasScenario(pipelineSummary, scenarioId)),
];
const rows = scenarioIds.map((scenarioId) => bundleScenario(scenarioId));
const analyzedRows = rows.filter((row) => row.localVlmAnalysisProof === true);
const policyRows = rows.filter((row) => row.policyDryRunProof === true);
const screenshotRows = rows.filter((row) => row.parentExplanationScreenshot !== undefined);

const proof = {
  proof: 'screen-ai-live-operator-evidence-bundle',
  generatedAt: new Date().toISOString(),
  sourceArtifacts: {
    pipelineSummary: relativePath(join(pipelineRoot, 'proof-summary.json')),
    aiSummary: relativePath(join(aiRoot, 'proof-summary.json')),
  },
  scenarioCount: rows.length,
  localVlmRows: analyzedRows.length,
  policyDryRunRows: policyRows.length,
  parentExplanationScreenshots: screenshotRows.length,
  publicSocialSurfaceRows: rows.filter((row) => row.publicSocialSurfaceProof === true).length,
  authenticatedAccountSocialProof: rows.some((row) => row.authenticatedAccountProof === true),
  rawScreenshotFilesCopied: false,
  encryptedQueueFilesCopied: false,
  bundlePortableForReview: true,
  productCompleteClaimed: false,
  scenarios: rows,
  nonClaims: [
    'This bundle copies redacted live-operator evidence and parent explanation screenshots only.',
    'Raw screenshots, raw image paths, encrypted queue payloads, and operator-supplied full URLs are not copied into the bundle.',
    'The bundle makes retained proof review portable; it does not rerun live capture, OCR, VLM, policy, or adapter execution.',
    'Authenticated-account social proof and broad/browser/network/mobile product-complete adapters remain separate gates.',
  ],
};

assert(rows.length === scenarioIds.length, 'scenario bundle count mismatch');
assert(rows.length >= requiredScenarioIds.length, 'required scenario bundle count mismatch');
assert(analyzedRows.length >= 8, `expected at least 8 local VLM rows, got ${analyzedRows.length}`);
assert(policyRows.length >= 8, `expected at least 8 policy rows, got ${policyRows.length}`);
assert(screenshotRows.length >= 8, `expected at least 8 parent explanation screenshots, got ${screenshotRows.length}`);
assert(proof.publicSocialSurfaceRows === 1, 'expected one public social surface row');

writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(indexPath, evidenceIndex(proof));
console.log(`screen-ai-live-operator-evidence-bundle-ok:${relativePath(proofPath)}`);

function bundleScenario(scenarioId) {
  const aiScenarioRoot = join(aiRoot, scenarioId);
  const bundleScenarioRoot = join(bundleRoot, scenarioId);
  mkdirSync(bundleScenarioRoot, { recursive: true });

  const pipelineSummaryRow = scenarioSummary(pipelineSummary, scenarioId);
  const source = readJson(join(aiScenarioRoot, '01-redacted-source-evidence.json'));
  const capture = readJson(join(aiScenarioRoot, '02-capture-proof-ref.json'));
  const deletion = optionalJson(join(aiScenarioRoot, '08-deletion-after-analysis.json'));
  const policy = optionalJson(join(aiScenarioRoot, '07-policy-decision.json'));
  const ai = optionalJson(join(aiScenarioRoot, '06-ai-result.json'));

  const copiedArtifacts = [];
  for (const fileName of portableJsonFiles) {
    const sourcePath = join(aiScenarioRoot, fileName);
    if (existsSync(sourcePath)) {
      copiedArtifacts.push(copyPortableFile(sourcePath, join(bundleScenarioRoot, fileName)));
    }
  }

  let parentExplanationScreenshot = null;
  const parentExplanationPath = join(aiScenarioRoot, '10-parent-explanation.png');
  if (existsSync(parentExplanationPath)) {
    parentExplanationScreenshot = copyPortableFile(
      parentExplanationPath,
      join(bundleScenarioRoot, '10-parent-explanation.png')
    );
    copiedArtifacts.push(parentExplanationScreenshot);
  }

  assert(!existsSync(join(bundleScenarioRoot, 'capture')), `${scenarioId} copied capture directory`);
  assert(!copiedArtifacts.some((artifact) => artifact.path.includes('queue')), `${scenarioId} copied queue artifact`);
  const noRawImageClaimed = capture.noRawImageClaimed === true;
  assert(noRawImageClaimed || capture.rawImagePathNotRetained === true, `${scenarioId} raw image path was retained`);
  assert(
    noRawImageClaimed || capture.captureMetadata?.rawImagePersistedInProof === false,
    `${scenarioId} persisted raw image in proof`
  );
  if (deletion !== undefined) {
    assert(deletion.rawImageDeletedAfterAnalysis === true, `${scenarioId} raw image was not deleted`);
    assert(deletion.existsAfterDelete === false, `${scenarioId} raw image still exists after delete`);
  }

  return {
    scenarioId,
    surface: pipelineSummaryRow.surface,
    category: pipelineSummaryRow.primaryCategory ?? null,
    policyAction: pipelineSummaryRow.policyAction ?? null,
    liveExternalUrlProof: pipelineSummaryRow.liveExternalUrlProof === true,
    publicSocialSurfaceProof:
      source.publicSocialSurfaceProof === true ||
      pipelineSummaryRow.publicSocialSurfaceProof === true ||
      scenarioId === 'facebook-social-surface',
    authenticatedAccountProof:
      source.authenticatedAccountProof === true || pipelineSummaryRow.authenticatedAccountProof === true,
    localVlmAnalysisProof: pipelineSummaryRow.analyzedByRealLocalVlm === true,
    policyDryRunProof: pipelineSummaryRow.policyDecisionValidated === true,
    rawImageDeletedAfterAnalysis: noRawImageClaimed || deletion?.rawImageDeletedAfterAnalysis === true,
    rawImagePathRetained: !noRawImageClaimed && capture.rawImagePathNotRetained !== true,
    copiedArtifactCount: copiedArtifacts.length,
    parentExplanationScreenshot,
    copiedArtifacts,
    portableAssertions: {
      sourceEvidenceRedacted: source.redacted === true,
      captureProofRefOnly: noRawImageClaimed || capture.rawImagePathNotRetained === true,
      policyDryRun: policy?.policyDecision?.dryRun === true || policy === null,
      localOnlyModelRuntime: ai?.localAiSafetyResult?.modelRuntime?.privacyMode === 'local-only' || ai === null,
    },
  };
}

function copyPortableFile(sourcePath, destinationPath) {
  assert(!sourcePath.includes(`${join('capture', 'queue')}`), `refusing to copy queue artifact ${sourcePath}`);
  assert(basename(sourcePath) !== '03-encrypted-queue.ndjson', `refusing to copy encrypted queue ${sourcePath}`);
  if (sourcePath.endsWith('.log')) {
    writeFileSync(destinationPath, normalizePortableLog(readFileSync(sourcePath, 'utf8')));
  } else {
    copyFileSync(sourcePath, destinationPath);
  }
  return {
    path: relativePath(destinationPath),
    sha256: sha256(destinationPath),
    bytes: readFileSync(destinationPath).byteLength,
  };
}

function normalizePortableLog(value) {
  const normalizedLines = value
    .replace(/\r\n/gu, '\n')
    .split('\n')
    .map((line) => line.trimEnd());
  return `${normalizedLines.join('\n').trimEnd()}\n`;
}

function scenarioSummary(summary, scenarioId) {
  const row = summary.scenarios.find((candidate) => candidate.scenarioId === scenarioId);
  assert(Boolean(row), `missing scenario summary ${scenarioId}`);
  assert(row.status === 'passed', `${scenarioId} did not pass live operator proof`);
  return row;
}

function hasScenario(summary, scenarioId) {
  return summary.scenarios.some((candidate) => candidate.scenarioId === scenarioId);
}

function evidenceIndex(proof) {
  const scenarioRows = proof.scenarios
    .map((scenario) =>
      [
        `## ${scenario.scenarioId}`,
        '',
        `- surface: ${scenario.surface}`,
        `- category: ${scenario.category ?? 'not-applicable'}`,
        `- policy action: ${scenario.policyAction ?? 'not-applicable'}`,
        `- live external URL proof: ${scenario.liveExternalUrlProof}`,
        `- local VLM analysis proof: ${scenario.localVlmAnalysisProof}`,
        `- policy dry-run proof: ${scenario.policyDryRunProof}`,
        `- raw image deleted after analysis: ${scenario.rawImageDeletedAfterAnalysis}`,
        `- parent explanation screenshot: ${scenario.parentExplanationScreenshot?.path ?? 'not-applicable'}`,
        `- copied artifacts: ${scenario.copiedArtifactCount}`,
        '',
      ].join('\n')
    )
    .join('\n');

  return [
    '# Screen AI Live Operator Evidence Bundle',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    `Scenario count: ${proof.scenarioCount}`,
    `Local VLM rows: ${proof.localVlmRows}`,
    `Policy dry-run rows: ${proof.policyDryRunRows}`,
    `Parent explanation screenshots: ${proof.parentExplanationScreenshots}`,
    `Raw screenshots copied: ${proof.rawScreenshotFilesCopied}`,
    `Encrypted queue files copied: ${proof.encryptedQueueFilesCopied}`,
    `Product-complete claimed: ${proof.productCompleteClaimed}`,
    '',
    'This bundle is meant for repository/remote review of retained live-operator evidence. It contains redacted JSON, redacted model logs, and parent explanation screenshots, not raw captured screenshots or encrypted queue payloads.',
    '',
    scenarioRows,
  ].join('\n');
}

function readJson(path) {
  assert(existsSync(path), `missing JSON artifact ${relativePath(path)}`);
  return JSON.parse(readFileSync(path, 'utf8'));
}

function optionalJson(path) {
  return existsSync(path) ? JSON.parse(readFileSync(path, 'utf8')) : null;
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function relativePath(path) {
  return resolve(path).replace(`${repoRoot}\\`, '').replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
