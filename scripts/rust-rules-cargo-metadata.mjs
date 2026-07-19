import fs from "node:fs";
import path from "node:path";
import { addViolation } from "./rust-rules-path-core.mjs";
import { commandExists } from "./rust-rules-cargo-command.mjs";
import { cargoMetadataScope } from "./rust-rules-cargo-metadata-scope.mjs";
import { cargoMetadataDependencyFindings } from "./rust-rules-cargo-metadata-dependencies.mjs";
import { cargoMetadataRegistryFindings } from "./rust-rules-cargo-metadata-registry.mjs";
import {
  cargoLockNeedsUpdate,
  loadLockedCargoMetadata,
} from "./rust-rules-cargo-metadata-load.mjs";

/** Compatibility API for callers that need only successful locked metadata. */
function loadCargoMetadata(root) {
  return loadLockedCargoMetadata(root).metadata;
}

function scanCargoMetadata(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!commandExists("cargo")) return violations;
  const loaded = loadLockedCargoMetadata(root);
  if (!loaded.metadata) {
    if (!loaded.unavailable && cargoLockNeedsUpdate(loaded.output)) {
      addViolation(
        violations,
        root,
        fs.existsSync(path.join(root, "Cargo.lock"))
          ? path.join(root, "Cargo.lock")
          : path.join(root, "Cargo.toml"),
        1,
        "RR-9.25",
        "Cargo.lock is not current for the workspace manifests.",
        loaded.output.slice(0, 4000),
      );
    }
    return violations;
  }
  const metadata = loaded.metadata;
  const metadataScope = cargoMetadataScope(root, config, scope, metadata);
  violations.push(...cargoMetadataDependencyFindings(root, config, metadataScope));
  violations.push(...cargoMetadataRegistryFindings(root, metadataScope.packages));

  return violations;
}

export { loadCargoMetadata, scanCargoMetadata };
