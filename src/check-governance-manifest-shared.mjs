import { DEFAULT_ALLOWED_LICENSES } from "./check-metadata.mjs";
import {
  dependencySections as dependencySectionsImpl,
  isBoundedNodeEngine as isBoundedNodeEngineImpl,
  isDeterministicDependencyVersion as isDeterministicDependencyVersionImpl,
  isSuspiciousDependencyName as isSuspiciousDependencyNameImpl,
} from "./check-governance-manifest-values.mjs";
import {
  lineForJsonKey as lineForJsonKeyImpl,
  parsePackageManifest as parsePackageManifestImpl,
} from "./check-governance-manifest-json.mjs";
import { packageExportTargets as packageExportTargetsImpl } from "./check-governance-manifest-package.mjs";

export { DEFAULT_ALLOWED_LICENSES };

export function parsePackageManifest(packageJsonPath) {
  return parsePackageManifestImpl(packageJsonPath);
}

export function packageExportTargets(exportsField) {
  return packageExportTargetsImpl(exportsField);
}

export function dependencySections(manifest) {
  return dependencySectionsImpl(manifest);
}

export function isDeterministicDependencyVersion(value) {
  return isDeterministicDependencyVersionImpl(value);
}

export function isSuspiciousDependencyName(name) {
  return isSuspiciousDependencyNameImpl(name);
}

export function isBoundedNodeEngine(value) {
  return isBoundedNodeEngineImpl(value);
}

export function lineForJsonKey(filePath, key) {
  return lineForJsonKeyImpl(filePath, key);
}
