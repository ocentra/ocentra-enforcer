import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const requiredScenarioIds = [
  'managed-browser-exact-url-active-tab',
  'foreground-process-window',
  'network-domain-attribution',
  'app-game-duration',
  'screen-evidence-queue',
  'lan-smoke',
  'package-installed-service-autostart-gaps',
];

const allowedProofModes = new Set(['ci-mechanical-proof', 'manual-required', 'scaffold-gap']);

export function validateCheckpointScenarios(matrix, options = {}) {
  const repoRoot = options.repoRoot ?? process.cwd();

  if (!Array.isArray(matrix.checkpointScenarios) || matrix.checkpointScenarios.length === 0) {
    fail('checkpointScenarios must be a non-empty array.');
  }

  const claimIds = new Set(matrix.claims.map((claim) => claim.id));
  const scenarioIds = matrix.checkpointScenarios.map((scenario) => scenario.id);
  assertUnique(scenarioIds, 'checkpointScenarios.id');

  for (const requiredScenarioId of requiredScenarioIds) {
    if (!scenarioIds.includes(requiredScenarioId)) {
      fail(`Missing required real evidence checkpoint scenario: ${requiredScenarioId}`);
    }
  }

  let manualRequiredCount = 0;
  let scaffoldGapCount = 0;

  for (const scenario of matrix.checkpointScenarios) {
    validateScenario(scenario, claimIds, repoRoot);
    if (scenario.proofMode === 'manual-required') {
      manualRequiredCount += 1;
    }
    if (scenario.proofMode === 'scaffold-gap') {
      scaffoldGapCount += 1;
    }
  }

  if (manualRequiredCount === 0) {
    fail('At least one checkpoint scenario must require manual real-machine proof.');
  }

  if (scaffoldGapCount === 0) {
    fail('At least one checkpoint scenario must record a scaffold or packaging gap.');
  }

  return {
    manualRequiredCount,
    scaffoldGapCount,
    scenarioCount: matrix.checkpointScenarios.length,
  };
}

function validateScenario(scenario, claimIds, repoRoot) {
  assertNonEmptyString(scenario.id, 'checkpointScenario.id');
  assertNonEmptyString(scenario.title, `${scenario.id}.title`);
  assertStringArray(scenario.claimIds, `${scenario.id}.claimIds`);
  assertStringArray(scenario.realEvidencePath, `${scenario.id}.realEvidencePath`);
  assertStringArray(scenario.ciCommands, `${scenario.id}.ciCommands`);
  assertStringArray(scenario.requiredArtifacts, `${scenario.id}.requiredArtifacts`);
  assertStringArray(scenario.knownGaps, `${scenario.id}.knownGaps`);
  assertStringArray(scenario.notProof, `${scenario.id}.notProof`);

  if (!allowedProofModes.has(scenario.proofMode)) {
    fail(`${scenario.id}.proofMode has unsupported value: ${scenario.proofMode}`);
  }

  for (const claimId of scenario.claimIds) {
    if (!claimIds.has(claimId)) {
      fail(`${scenario.id} references unknown claim id: ${claimId}`);
    }
  }

  for (const command of scenario.ciCommands) {
    if (!command.startsWith('npm run ') && !command.startsWith('node ') && !command.startsWith('cargo ')) {
      fail(`${scenario.id}.ciCommands contains an unsupported command shape: ${command}`);
    }
  }

  for (const artifact of scenario.requiredArtifacts) {
    if (artifact.toLowerCase().includes('screenshot') && scenario.proofMode === 'ci-mechanical-proof') {
      fail(`${scenario.id} should not require screenshots for CI-only mechanical proof.`);
    }
  }

  if (scenario.proofMode === 'manual-required' && scenario.requiredArtifacts.length < 3) {
    fail(`${scenario.id} manual proof must name at least three required artifacts.`);
  }

  if (scenario.proofMode === 'scaffold-gap' && scenario.knownGaps.length < 2) {
    fail(`${scenario.id} scaffold-gap proof must name at least two known gaps.`);
  }

  for (const expectationFile of scenario.expectationFiles ?? []) {
    assertNonEmptyString(expectationFile, `${scenario.id}.expectationFiles[]`);
    const expectationPath = join(repoRoot, expectationFile);
    if (!existsSync(expectationPath)) {
      fail(`${scenario.id} references missing expectation file: ${expectationFile}`);
    }
  }
}

function assertNonEmptyString(value, label) {
  if (typeof value !== 'string' || value.trim().length === 0) {
    fail(`${label} must be a non-empty string.`);
  }
}

function assertStringArray(value, label) {
  if (!Array.isArray(value) || value.length === 0) {
    fail(`${label} must be a non-empty array.`);
  }

  for (const [index, item] of value.entries()) {
    assertNonEmptyString(item, `${label}[${index}]`);
  }
}

function assertUnique(values, label) {
  const seen = new Set();
  for (const value of values) {
    if (seen.has(value)) {
      fail(`${label} contains duplicate value: ${value}`);
    }
    seen.add(value);
  }
}

function fail(message) {
  throw new Error(message);
}

function readMatrix(repoRoot) {
  const matrixPath = join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json');
  return JSON.parse(readFileSync(matrixPath, 'utf8'));
}

function runStandalone() {
  const repoRoot = process.cwd();
  const matrix = readMatrix(repoRoot);
  const result = validateCheckpointScenarios(matrix, { repoRoot });
  console.log(
    `real-evidence-proof-checkpoint-ok: ${result.scenarioCount} scenarios checked; ` +
      `${result.manualRequiredCount} manual-required; ${result.scaffoldGapCount} scaffold-gap.`
  );
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    runStandalone();
  } catch (error) {
    console.error('real-evidence-proof-checkpoint-failed');
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
