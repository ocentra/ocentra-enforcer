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
  runScanner,
} from "./rust-rules-cargo-runner.mjs";
export {
  scanCargoManifest,
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
  workspacePackageNamesFromManifests,
} from "./rust-rules-cargo-manifest-dependencies.mjs";
export {
  commandExists,
  runCommand,
  configuredCargoCommand,
} from "./rust-rules-cargo-command.mjs";
export {
  shouldRunCargoForScope,
  cargoPackageArgs,
  cargoFmtArgs,
  strongestEnabledSeverity,
  runCargoGates,
} from "./rust-rules-cargo-gates.mjs";
