/*
 * Test-doctrine scanner: discovers what kinds of tests a target project actually
 * has (unit, integration, e2e, contract, mutation, property/fuzz, security,
 * snapshot, load) and what its "nature" implies it should have, then reports
 * the gap, including whether each category is actually wired to CI through a
 * blocking gate, or just present locally with nothing enforcing it. Signal
 * based (file names, config files, dependency manifests, CI step text) — the
 * signal-scanner approach used by generic regex scanners, not a language parser.
 *
 * Reusable across any target repo: `scanTestDoctrine({ root })`.
 */
import path from "node:path";
import { walk, readTextSafe, relFiles } from "./test-doctrine-fs.mjs";
import { detectNature } from "./test-doctrine-nature.mjs";
import { analyzeCiGating } from "./test-doctrine-ci.mjs";
import { buildReport } from "./test-doctrine-report.mjs";

function isManifestFile(relPath) {
  return /(^|\/)(package\.json|pyproject\.toml|requirements.*\.txt|Cargo\.toml|Pipfile)$/i.test(
    relPath,
  );
}

function collectManifestText(files, relPaths) {
  let text = "";
  for (let i = 0; i < files.length; i += 1) {
    if (!isManifestFile(relPaths[i])) continue;
    text += `\n${readTextSafe(files[i])}`;
  }
  return text;
}

function scanTestDoctrine({ root }) {
  const resolvedRoot = path.resolve(root);
  const files = walk(resolvedRoot);
  const relPaths = relFiles(files, resolvedRoot);
  const manifestText = collectManifestText(files, relPaths);
  const nature = detectNature(relPaths, manifestText);
  const ci = analyzeCiGating(files, relPaths, { root: resolvedRoot, manifestText });
  return buildReport({ root: resolvedRoot, relPaths, manifestText, nature, ci, files });
}

export { scanTestDoctrine };
