/** Returns the single mutable proof-evidence cache record owned by one Cargo crate. */
export function crateEvidenceCacheRecord(crateEvidenceCache, cargoRoot) {
  if (!crateEvidenceCache || !cargoRoot) return null;
  const cacheKey = `evidence:${cargoRoot}`;
  let record = crateEvidenceCache.get(cacheKey);
  if (!record) {
    record = { sources: null, context: null };
    crateEvidenceCache.set(cacheKey, record);
  }
  return record;
}
