import { matchesAnyGlob } from "./rust-rules-path-core.mjs";

export function isTestFile(rel, config) {
  return matchesAnyGlob(rel, config.testFileGlobs);
}

export function isRawTypeBoundary(rel, config) {
  return matchesAnyGlob(rel, config.rawTypeBoundaryGlobs);
}

export function isBoundaryModulePath(rel, config) {
  return (
    isTestFile(rel, config) ||
    isRawTypeBoundary(rel, config) ||
    /(?:^|\/)(?:boundary|boundaries|serde|transport|adapter|adapters)(?:\/|\.|-)/iu.test(
      rel,
    )
  );
}

export function isRawStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.rawStringOwnerGlobs);
}

export function isDomainPrimitiveOwner(rel, config) {
  return matchesAnyGlob(rel, config.domainPrimitiveOwnerGlobs);
}

export function isRuntimeStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.runtimeStringOwnerGlobs);
}

export function isSerializedDomainOwner(rel, config) {
  return matchesAnyGlob(rel, config.serializedDomainOwnerGlobs);
}
