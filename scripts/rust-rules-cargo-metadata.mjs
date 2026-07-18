import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { commandExists } from "./rust-rules-cargo-command.mjs";
import { cargoMetadataScope } from "./rust-rules-cargo-metadata-scope.mjs";
import { cargoMetadataDependencyFindings } from "./rust-rules-cargo-metadata-dependencies.mjs";
import { cargoMetadataRegistryFindings } from "./rust-rules-cargo-metadata-registry.mjs";

function loadCargoMetadata(root) {
  const result = spawnSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1"],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
      shell: false,
    },
  );
  if (result.error) throw result.error;
  if ((result.status ?? 1) !== 0) return null;
  return JSON.parse(result.stdout);
}

function scanCargoMetadata(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!commandExists("cargo")) return violations;
  const metadata = loadCargoMetadata(root);
  if (!metadata) return violations;
  const metadataScope = cargoMetadataScope(root, config, scope, metadata);
  violations.push(...cargoMetadataDependencyFindings(root, config, metadataScope));
  violations.push(...cargoMetadataRegistryFindings(root, metadataScope.packages));

  return violations;
}

export { loadCargoMetadata, scanCargoMetadata };
