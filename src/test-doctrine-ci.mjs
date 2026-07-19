/*
 * CI-gating detection: does the target project's CI config actually INVOKE and
 * BLOCK ON each detected test category, or does the category only exist
 * locally with nothing forcing it to run/fail the build? Distinguishes a step
 * that runs but is `continue-on-error: true` (informational, non-blocking)
 * from one that actually gates merges.
 */
import { readTextSafe } from "./test-doctrine-fs.mjs";
import { stepBlocks, stripCommentLines } from "./test-doctrine-ci-text.mjs";

const COVERAGE_THRESHOLD_RE = /--cov-fail-under|fail_under\s*=|coverageThreshold/i;

const CI_CONFIG_RE = /(^|\/)\.github\/workflows\/.+\.ya?ml$|(^|\/)\.gitlab-ci\.ya?ml$|(^|\/)azure-pipelines\.ya?ml$|(^|\/)Jenkinsfile$|(^|\/)\.circleci\/config\.ya?ml$/i;

const CATEGORY_CI_PATTERNS = {
  unit: [/\bpytest\b/i, /\bnpm (run )?test\b/i, /\bvitest\b/i, /\bnpx vitest\b/i, /\bcargo test\b/i],
  integration: [/\bpytest\b/i], // refined further by service-container presence in analyzeBlock
  e2e: [/playwright test/i, /cypress run/i],
  contract: [/pact[-_ ]?(broker|verify)/i, /schemathesis run/i, /prism m[o]ck/i],
  mutation: [/stryker run/i, /mutmut run/i, /cargo mutants/i],
  propertyFuzzing: [/schemathesis run/i, /\bhypothesis\b/i],
  security: [/\bbandit\b/i, /\bsemgrep\b/i, /\bgitleaks\b/i, /codeql/i, /npm audit\b/i, /uv audit\b/i, /\btrivy\b/i],
  loadPerformance: [/\bk6 run\b/i, /artillery run/i, /\blocust\b/i],
  coverageTooling: [/--cov\b/i, /coverage run/i, /codecov/i],
};

const SERVICE_CONTAINER_RE = /services:\s*\n\s*(postgres|redis|mysql|rabbitmq|mongo)/i;
const NON_BLOCKING_RE = /continue-on-error:\s*true|\|\|\s*true\b|allow_failure:\s*true/i;

function evaluateCategory(category, blocks, wholeText, manifestText) {
  const patterns = CATEGORY_CI_PATTERNS[category];
  if (!patterns) return { wired: false, blocking: null, evidence: [] };
  const evidence = [];
  let anyBlocking = false;
  for (const block of blocks) {
    const codeOnly = stripCommentLines(block);
    if (!patterns.some((re) => re.test(codeOnly))) continue;
    if (category === "integration" && !SERVICE_CONTAINER_RE.test(wholeText)) continue;
    const blocking = !NON_BLOCKING_RE.test(codeOnly);
    anyBlocking = anyBlocking || blocking;
    const firstLine = block.trim().split("\n")[0].trim();
    evidence.push({ step: firstLine, blocking });
  }
  if (category === "coverageTooling" && evidence.length === 0 && COVERAGE_THRESHOLD_RE.test(manifestText)) {
    evidence.push({ step: "coverage threshold enforced via test-runner config (addopts/coverageThreshold), not a CI-visible flag", blocking: true });
    anyBlocking = true;
  }
  // Show blocking examples first so a 3-item slice can't misrepresent an overall
  // blocking=true verdict with only non-blocking evidence.
  evidence.sort((a, b) => Number(b.blocking) - Number(a.blocking));
  return { wired: evidence.length > 0, blocking: evidence.length > 0 ? anyBlocking : null, evidence: evidence.slice(0, 3) };
}

function trackedPathsFallback(relPaths) {
  return new Set(relPaths);
}

// Blocks and the whole-file text must never cross a file boundary â€” a step's
// accumulated lines otherwise bleed into the next unrelated CI file when
// texts are naively concatenated, misattributing that file's steps.
function evaluateAll(fileTexts, manifestText) {
  const blocks = fileTexts.flatMap((text) => stepBlocks(text));
  const combinedText = fileTexts.join("\n");
  const perCategory = {};
  for (const category of Object.keys(CATEGORY_CI_PATTERNS)) {
    perCategory[category] = evaluateCategory(category, blocks, combinedText, manifestText);
  }
  return perCategory;
}

function analyzeCiGating(files, relPaths, { root, manifestText = "" } = {}) {
  const ciFiles = [];
  for (let i = 0; i < files.length; i += 1) {
    if (CI_CONFIG_RE.test(relPaths[i])) ciFiles.push({ relPath: relPaths[i], file: files[i] });
  }
  if (ciFiles.length === 0) {
    return { ciConfigFilesFound: [], perCategory: {}, perCategoryIncludingUntracked: {}, hasUntrackedCiFiles: false };
  }
  const tracked = trackedPathsFallback(ciFiles.map((f) => f.relPath));
  const ciConfigFilesFound = ciFiles.map((f) => ({ path: f.relPath, tracked: tracked.has(f.relPath) }));
  const trackedTexts = ciFiles.filter((f) => tracked.has(f.relPath)).map((f) => readTextSafe(f.file));
  const allTexts = ciFiles.map((f) => readTextSafe(f.file));
  return {
    ciConfigFilesFound,
    hasUntrackedCiFiles: ciConfigFilesFound.some((f) => !f.tracked),
    perCategory: evaluateAll(trackedTexts, manifestText),
    perCategoryIncludingUntracked: evaluateAll(allTexts, manifestText),
  };
}

export { analyzeCiGating };
