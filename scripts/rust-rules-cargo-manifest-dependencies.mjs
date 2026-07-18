import fs from "node:fs";
import { packageNameFromManifest } from "./rust-rules-path-core.mjs";
import { applyCargoManifestFinalChecks } from "./rust-rules-cargo-manifest-finalize.mjs";
import {
  workspaceDependencyJustifications,
  workspacePackageNamesFromManifests,
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
} from "./rust-rules-cargo-manifest-identity.mjs";
import { scanCargoManifestPackage } from "./rust-rules-cargo-manifest-package.mjs";
import { scanCargoManifestLines } from "./rust-rules-cargo-manifest-line-validation.mjs";

/** Scans one Cargo manifest for dependency-policy violations. */
export function scanCargoManifest(root, manifest, config, violations, workspacePackageNames = null) {
  const cargoText = fs.readFileSync(manifest, "utf8");
  scanCargoManifestPackage(root, manifest, cargoText, violations);
  const workspaceNames = workspacePackageNames ?? workspacePackageNamesFromManifests(root, config);
  const scanState = scanCargoManifestLines({
    root,
    manifest,
    config,
    violations,
    lines: cargoText.split(/\r?\n/u),
    workspacePackageNames: workspaceNames,
    currentPackageName: packageNameFromManifest(manifest),
    workspaceDependencyJustification: workspaceDependencyJustifications(root),
  });
  applyCargoManifestFinalChecks({
    root,
    manifest,
    config,
    violations,
    dependencyNamesBySection: scanState.namesBySection,
    dependencyRequirementsByName: scanState.requirementsByName,
  });
}

export {
  dependencyNameFromManifestLine,
  dependencyRequirementFromManifestLine,
  workspacePackageNamesFromManifests,
};
