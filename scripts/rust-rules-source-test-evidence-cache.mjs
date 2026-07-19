import path from "node:path";
import {
  collectRustEvidenceTree,
  crateRootForEvidence,
} from "./rust-rules-source-test-evidence-paths.mjs";

const cargoEvidenceCache = new Map();
let proofEvidenceIndexBuilds = 0;

/** Clears crate-local source and executable-evidence indexes between scans. */
export function clearProofEvidenceCache() {
  cargoEvidenceCache.clear();
}

/** Test-only visibility for cache invalidation and reuse assertions. */
export function proofEvidenceCacheStats() {
  return { indexBuilds: proofEvidenceIndexBuilds };
}

/** Resets cache telemetry before a deterministic cache-reuse test. */
export function resetProofEvidenceCacheStats() {
  clearProofEvidenceCache();
  proofEvidenceIndexBuilds = 0;
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
    cached = { sourceFiles, evidenceFiles, indexes: new Map() };
    cargoEvidenceCache.set(cacheKey, cached);
  }
  return cached;
}

/**
 * Reuses one derived evidence index for each current source snapshot in a crate
 * scan. The current file always comes from the caller, never the disk cache.
 */
export function withProofEvidenceIndex(root, filePath, source, buildIndex) {
  const cached = crateSourceIndex(root, filePath);
  if (!cached) {
    proofEvidenceIndexBuilds += 1;
    return buildIndex([{ path: filePath, source }]);
  }
  const sourceKey = `${path.resolve(filePath).toLowerCase()}\u0000${source}`;
  let index = cached.indexes.get(sourceKey);
  if (!index) {
    index = buildIndex(cargoCrateSources(root, filePath, source));
    cached.indexes.set(sourceKey, index);
    proofEvidenceIndexBuilds += 1;
  }
  return index;
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
