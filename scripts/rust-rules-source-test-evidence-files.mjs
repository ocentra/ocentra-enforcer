import fs from "node:fs";
import path from "node:path";
import { crateEvidenceCacheRecord } from "./rust-rules-source-test-evidence-cache.mjs";
import { nearestCargoRoot, rustFilesUnder } from "./rust-rules-source-test-evidence-paths.mjs";

/** Collects cached evidence sources associated with a Rust crate. */
export function crateEvidenceSources(root, filePath, source, crateEvidenceCache = null) {
  const cargoRoot = nearestCargoRoot(root, filePath);
  if (!cargoRoot) return [source];
  const record = crateEvidenceCacheRecord(crateEvidenceCache, cargoRoot);
  let evidenceSources = record?.sources;
  if (!evidenceSources) {
    evidenceSources = [
      ...rustFilesUnder(path.join(cargoRoot, "src")),
      ...rustFilesUnder(path.join(cargoRoot, "tests")),
    ].map((evidenceFile) => fs.readFileSync(evidenceFile, "utf8"));
    if (record) record.sources = evidenceSources;
  }
  return evidenceSources.includes(source) ? evidenceSources : [source, ...evidenceSources];
}
