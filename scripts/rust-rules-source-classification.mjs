import { matchesAnyGlob } from "./rust-rules-path-core.mjs";

/** Determines whether a relative Rust path is a configured test file. */
export function isTestFile(rel, config) {
  return matchesAnyGlob(rel, config.testFileGlobs);
}

/** Determines whether a path permits a raw type at its boundary. */
export function isRawTypeBoundary(rel, config) {
  return matchesAnyGlob(rel, config.rawTypeBoundaryGlobs);
}

/** Determines whether a path is a configured boundary module. */
export function isBoundaryModulePath(rel, config) {
  return (
    isTestFile(rel, config) ||
    isRawTypeBoundary(rel, config) ||
    /(?:^|[\/_-])(?:boundary|boundaries|serde|transport|adapter|adapters)(?:\/|\.|-|_)/iu.test(
      rel,
    )
  );
}

/** Identifies a serde-backed transport record name. */
export function isTransportRecordName(name, hasSerdeDerive) {
  if (/(?:Request|Response)$/u.test(name)) return true;
  return hasSerdeDerive && /(?:Dto|DTO|Wire)$/u.test(name);
}

/** Determines whether a path is a configuration boundary module. */
export function isConfigurationBoundaryModulePath(rel) {
  return /(?:^|\/)[^/]*(?:config|configuration|settings)[^/]*(?:\/|$)/iu.test(rel);
}

/** Determines whether a path owns raw string representations. */
export function isRawStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.rawStringOwnerGlobs);
}

/** Determines whether a path owns domain primitive definitions. */
export function isDomainPrimitiveOwner(rel, config) {
  return matchesAnyGlob(rel, config.domainPrimitiveOwnerGlobs);
}

/** Determines whether a path owns runtime string values. */
export function isRuntimeStringOwner(rel, config) {
  return matchesAnyGlob(rel, config.runtimeStringOwnerGlobs);
}

/** Determines whether a path owns serialized domain values. */
export function isSerializedDomainOwner(rel, config) {
  return matchesAnyGlob(rel, config.serializedDomainOwnerGlobs);
}
