#!/usr/bin/env node
/*
 * Ocentra Enforcer Rust scan engine.
 */
import { scanRustFile } from "./rust-rules-source-scan.mjs";

export {
  collectFunctionSignatures,
  functionName,
  functionParams,
  normalizedNameTokens,
  isSuspiciousSerializedFieldName,
  braceDelta,
  hasStringLiteral,
} from "./rust-rules-source-helpers.mjs";
export {
  isTestFile,
  isRawTypeBoundary,
  isBoundaryModulePath,
  isRawStringOwner,
  isDomainPrimitiveOwner,
  isRuntimeStringOwner,
  isSerializedDomainOwner,
} from "./rust-rules-source-classification.mjs";
export { scanRustFile };
export {
  scanWorkspaceFiles,
  manifestPathsForScope,
  nearestCargoManifest,
  scanCargoManifest,
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
  workspacePackageNamesFromManifests,
  loadCargoMetadata,
  scanCargoMetadata,
  runScanner,
  commandExists,
  runCommand,
  shouldRunCargoForScope,
  cargoPackageArgs,
  configuredCargoCommand,
  strongestEnabledSeverity,
  runCargoGates,
} from "./rust-rules-cargo-scan.mjs";
