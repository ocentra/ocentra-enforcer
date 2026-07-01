import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', '40-detector-prompt-packs-and-schema-tests');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');

await main();

async function main() {
  runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const promptPackModule = await import('../../packages/schema-domain/dist/screen-evidence-detector-prompt-pack.js');
  const promptPackValuesModule =
    await import('../../packages/schema-domain/dist/screen-evidence-detector-prompt-pack-values.js');
  const screenEvidence = {
    ScreenDetectorPromptDefinitionSchema: promptPackModule.ScreenDetectorPromptDefinitionSchema,
    ScreenDetectorPromptOutputSchema: promptPackModule.ScreenDetectorPromptOutputSchema,
    ScreenDetectorPromptPackSchema: promptPackModule.ScreenDetectorPromptPackSchema,
    ScreenDetectorPromptPackSchemaVersion: promptPackValuesModule.ScreenDetectorPromptPackSchemaVersion,
    ScreenDetectorRequiredIds: promptPackValuesModule.ScreenDetectorRequiredIds,
  };
  const pack = screenEvidence.ScreenDetectorPromptPackSchema.parse(promptPack(screenEvidence));
  const validOutput = screenEvidence.ScreenDetectorPromptOutputSchema.parse(detectorOutput(screenEvidence));
  const invalidRows = {
    duplicateDetectorRejected: !screenEvidence.ScreenDetectorPromptPackSchema.safeParse({
      ...promptPack(screenEvidence),
      detectors: [...promptPack(screenEvidence).detectors, promptDefinition('socialVideo')],
    }).success,
    openEndedPromptRejected: !screenEvidence.ScreenDetectorPromptDefinitionSchema.safeParse({
      ...promptDefinition('chatMessaging'),
      openEndedDescriptionAllowed: true,
    }).success,
    privateOutputRejected: !screenEvidence.ScreenDetectorPromptOutputSchema.safeParse({
      ...detectorOutput(screenEvidence),
      privateMessageTextIncluded: true,
    }).success,
    lowConfidenceWithoutUncertaintyRejected: !screenEvidence.ScreenDetectorPromptOutputSchema.safeParse({
      ...detectorOutput(screenEvidence),
      confidence: 0.31,
      uncertaintyReasons: [],
    }).success,
    policyAuthorityRejected: !screenEvidence.ScreenDetectorPromptOutputSchema.safeParse({
      ...detectorOutput(screenEvidence),
      finalPolicyActionClaimed: true,
    }).success,
  };

  assert.deepEqual(
    pack.detectors.map((detector) => detector.detectorId),
    Array.from(screenEvidence.ScreenDetectorRequiredIds)
  );
  assert.equal(validOutput.privateMessageTextIncluded, false);
  assert.equal(Object.values(invalidRows).every(Boolean), true);

  const proof = {
    schemaVersion: screenEvidence.ScreenDetectorPromptPackSchemaVersion,
    proofKind: 'screen-detector-prompt-pack-proof',
    generatedAt: new Date().toISOString(),
    artifact: artifactSummaryPath,
    detectorIds: pack.detectors.map((detector) => detector.detectorId),
    outputShapeFields: pack.detectors[0].outputFields,
    invalidRows,
    assertions: [
      'active prompt pack includes social/video/chat/game/school/bypass/adult/violence/payment/signup detectors exactly once',
      'detector prompts are schema-bound and reject open-ended describe-screen prompts',
      'detector prompts forbid private message text, names, credentials, full OCR text, raw prompt text, and raw screenshot refs',
      'detector output must expose uncertainty for low-confidence or unknown classifications',
      'detector output cannot claim final policy authority or enforcement action',
    ],
    nonClaims: [
      'No production model quality is claimed.',
      'No live model invocation is claimed.',
      'No policy/action/enforcement execution is claimed.',
    ],
  };

  await mkdir(outputRoot, { recursive: true });
  await writeFile(artifactSummaryPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`screen-detector-prompt-pack-proof-ok: ${artifactSummaryPath}`);
}

function promptPack(screenEvidence) {
  return {
    schemaVersion: screenEvidence.ScreenDetectorPromptPackSchemaVersion,
    promptPackId: 'screen-detector-prompt-pack-v1',
    promptPackVersion: 'screen-detector-prompt-pack-2026-06-05',
    publishedAt: '2026-06-05T03:08:00.000Z',
    status: 'active',
    detectors: screenEvidence.ScreenDetectorRequiredIds.map((detectorId) => promptDefinition(detectorId)),
    degradedStates: [],
    auditEvidenceIds: ['screen-detector-prompt-pack-audit'],
  };
}

function promptDefinition(detectorId) {
  return {
    detectorId,
    promptPackId: 'screen-detector-prompt-pack-v1',
    promptPackVersion: 'screen-detector-prompt-pack-2026-06-05',
    promptHashRef: `screen-detector-prompt-hash-${detectorId}`,
    targetCategories: targetCategories(detectorId),
    targetRiskSignals: ['unknown'],
    allowedInputFields: ['sourceEvidenceRefs', 'ocrSnippets', 'visibleCategoryCandidates', 'safeImageCropRef'],
    outputFields: [
      'detectorId',
      'categoryCandidates',
      'riskSignals',
      'confidence',
      'uncertaintyReasons',
      'evidenceRefs',
      'redactionNotes',
      'childSafeSummary',
    ],
    forbiddenOutputFields: [
      'privateMessageText',
      'personName',
      'credentialText',
      'fullOcrText',
      'rawScreenshotRef',
      'rawPromptText',
      'accountIdentifier',
      'addressOrPhone',
    ],
    rawPromptTextIncluded: false,
    openEndedDescriptionAllowed: false,
    fullOcrTextAllowed: false,
    privateMessageTextAllowed: false,
    personalNamesAllowed: false,
    credentialTextAllowed: false,
    rawScreenshotRefAllowed: false,
    childSafetyOnly: true,
  };
}

function detectorOutput(screenEvidence) {
  return {
    schemaVersion: screenEvidence.ScreenDetectorPromptPackSchemaVersion,
    detectorId: 'browserGame',
    promptPackVersion: 'screen-detector-prompt-pack-2026-06-05',
    analyzedAt: '2026-06-05T03:09:00.000Z',
    sourceEvidenceIds: ['screen-detector-output-source'],
    primaryCategory: 'game',
    categoryCandidates: [{ category: 'game', confidence: 0.86, evidenceRefs: [evidenceRef()] }],
    riskSignals: [{ signal: 'unknown', confidence: 0.52, evidenceRefs: [evidenceRef()] }],
    ocrSnippets: [{ text: 'Start game', confidence: 0.73, evidenceRefs: [evidenceRef()] }],
    confidence: 0.86,
    uncertaintyReasons: [],
    redactionNotes: ['credentialLikeTextRedacted'],
    childSafeSummary: 'Visible screen signals match a browser game route.',
    privateMessageTextIncluded: false,
    personalNamesIncluded: false,
    credentialTextIncluded: false,
    fullOcrTextIncluded: false,
    rawScreenshotRefIncluded: false,
    finalPolicyActionClaimed: false,
    enforcementActionClaimed: false,
  };
}

function evidenceRef() {
  return {
    evidenceId: 'screen-detector-evidence-ref',
    kind: 'journal-entry',
    digest: 'screen-detector-evidence-digest',
    uri: null,
  };
}

function targetCategories(detectorId) {
  const categories = {
    socialVideo: ['video'],
    chatMessaging: ['chat'],
    browserGame: ['game'],
    schoolProductivity: ['school', 'productivity'],
    bypassTool: ['bypassTool'],
    adultContent: ['adultContent'],
    violenceSafety: ['violence'],
    shoppingPayment: ['shopping'],
    signupIdentity: ['unknown'],
  };
  return categories[detectorId];
}

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
