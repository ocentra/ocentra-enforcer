import fs from "node:fs";
import path from "node:path";
import { addViolation, toPosix } from "./rust-rules-path-core.mjs";
import { scanMetadataDependencies } from "./rust-rules-cargo-metadata-dependencies.mjs";
import {
  cargoLockNeedsUpdate,
  loadLockedCargoMetadata,
} from "./rust-rules-cargo-metadata-load.mjs";
import { scanMetadataRequirements } from "./rust-rules-cargo-metadata-requirements.mjs";

/** Compatibility API for consumers that need only locked metadata. */
export function loadCargoMetadata(root) {
  return loadLockedCargoMetadata(root).metadata;
}

function scopedPackages(metadata, scope, manifestPaths) {
  const scopedManifests = new Set(manifestPaths.map((manifest) =>
    toPosix(manifest)));
  return (metadata.packages ?? []).filter((packageInfo) => {
    if (scope.mode === "crate") return packageInfo.name === scope.crateName;
    if (scope.mode === "files" || scope.mode === "diff") {
      return scopedManifests.has(toPosix(packageInfo.manifest_path));
    }
    return true;
  });
}

function staleLockViolation(root, output) {
  const violations = [];
  if (!cargoLockNeedsUpdate(output)) return violations;
  const lockPath = path.join(root, "Cargo.lock");
  addViolation(
    violations,
    root,
    fs.existsSync(lockPath) ? lockPath : path.join(root, "Cargo.toml"),
    1,
    "RR-9.25",
    "Cargo.lock is not current for the workspace manifests.",
    output.slice(0, 4000),
  );
  return violations;
}

/** Applies locked Cargo metadata dependency policy without mutating the workspace. */
export function scanCargoMetadata(root, config, scope, manifestPaths) {
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return [];
  const loaded = loadLockedCargoMetadata(root);
  if (!loaded.metadata) {
    return loaded.unavailable ? [] : staleLockViolation(root, loaded.output);
  }
  const packages = scopedPackages(loaded.metadata, scope, manifestPaths);
  const workspacePackageNames = new Set(
    (loaded.metadata.packages ?? []).map((packageInfo) => packageInfo.name),
  );
  return [
    ...scanMetadataDependencies({
      root,
      config,
      packages,
      workspaceRoot: toPosix(loaded.metadata.workspace_root),
      workspacePackageNames,
    }),
    ...scanMetadataRequirements(root, packages),
  ];
}
