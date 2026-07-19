import path from "node:path";
import {
  escapeRegExp,
  firstLineMatching,
} from "./rust-rules-path-core.mjs";
import { cargoEvidenceSources } from "./rust-rules-source-test-evidence-cache.mjs";
import { crateRootForEvidence } from "./rust-rules-source-test-evidence-paths.mjs";

/** Checks for target-specific negative conversion evidence outside production source. */
export function hasExternalNegativeConversionEvidence(root, filePath, source, dtoName) {
  const crateRoot = crateRootForEvidence(root, filePath);
  if (!crateRoot) return false;
  const target = new RegExp(`\\b${escapeRegExp(dtoName)}\\b`, "u");
  return cargoEvidenceSources(root, filePath, source)
    .filter((candidate) => candidate.path !== filePath)
    .some((candidate) =>
      target.test(candidate.source) &&
      /\b(?:TryFrom|try_from)\b/u.test(candidate.source) &&
      /\b(?:expect_err|assert|matches)!?\s*\([^\n]*(?:is_err|Err|reject)/u.test(candidate.source),
    );
}

/** Checks crate-local evidence for one exact parser or constructor target. */
export function hasTargetEvidence(root, filePath, source, targetName, evidencePattern) {
  const target = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  return cargoEvidenceSources(root, filePath, source).some(
    (candidate) => target.test(candidate.source) && evidencePattern.test(candidate.source),
  );
}

/** Finds the source line for one parser or constructor target. */
export function evidenceLineForTarget(originalLines, targetName) {
  return firstLineMatching(
    originalLines,
    new RegExp(`^\\s*(?:pub\\s+)?(?:async\\s+)?fn\\s+${escapeRegExp(targetName)}\\s*\\(`, "u"),
  );
}

/** Checks crate-local property-test evidence for one public parser target. */
export function hasPropertyEvidence(root, filePath, source, targetName) {
  const target = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  const crateRoot = crateRootForEvidence(root, filePath);
  const relativeTarget = path
    .relative(crateRoot ?? path.dirname(path.dirname(filePath)), filePath)
    .replaceAll(path.sep, "/");
  const registered = new RegExp(
    `["']${escapeRegExp(`${relativeTarget}::${targetName}`)}["']\\s*=>`,
    "u",
  );
  return cargoEvidenceSources(root, filePath, source).some((candidate) =>
    /\b(?:proptest|quickcheck)!\s*\{/u.test(candidate.source) &&
    (target.test(candidate.source) || registered.test(candidate.source)));
}
