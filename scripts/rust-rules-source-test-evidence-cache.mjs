import path from "node:path";
import {
  collectRustEvidenceTree,
  crateRootForEvidence,
} from "./rust-rules-source-test-evidence-paths.mjs";

const cargoEvidenceCache = new Map();

/** Clears crate-local source and executable-evidence indexes between scans. */
export function clearProofEvidenceCache() {
  cargoEvidenceCache.clear();
}

function crateSourceIndex(root, filePath) {
  const crateRoot = crateRootForEvidence(root, filePath);
  if (!crateRoot) return null;
  const cacheKey = path.resolve(crateRoot).toLowerCase();
  let cached = cargoEvidenceCache.get(cacheKey);
  if (!cached) {
    const sourceFiles = [];
    const evidenceFiles = [];
    collectRustEvidenceTree(path.join(crateRoot, "src"), sourceFiles);
    collectRustEvidenceTree(path.join(crateRoot, "tests"), evidenceFiles);
    collectRustEvidenceTree(path.join(crateRoot, "fuzz"), evidenceFiles);
    cached = { sourceFiles, evidenceFiles };
    cargoEvidenceCache.set(cacheKey, cached);
  }
  return cached;
}

/** Returns the current file plus executable test and fuzz evidence. */
export function cargoEvidenceSources(root, filePath, source) {
  const cached = crateSourceIndex(root, filePath);
  if (!cached) return [{ path: filePath, source }];
  return [
    { path: filePath, source },
    ...cached.evidenceFiles.filter((candidate) => candidate.path !== filePath),
  ];
}

/** Returns all crate sources for transport graphs plus executable evidence. */
export function cargoCrateSources(root, filePath, source) {
  const cached = crateSourceIndex(root, filePath);
  if (!cached) return [{ path: filePath, source }];
  return [
    { path: filePath, source },
    ...cached.sourceFiles.filter((candidate) => candidate.path !== filePath),
    ...cached.evidenceFiles.filter((candidate) => candidate.path !== filePath),
  ];
}
