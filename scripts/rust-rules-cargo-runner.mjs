import fs from "node:fs";
import path from "node:path";
import {
  addViolation,
  findCargoManifests,
  normalizeRel,
} from "./rust-rules-path-core.mjs";
import { scanRustFile } from "./rust-rules-source-scan.mjs";
import { rustFileFindings } from "./rust-rules-workspace-scan.mjs";
import { manifestPathsForScope } from "./rust-rules-cargo-manifest-discovery.mjs";
import {
  scanCargoManifest,
  workspacePackageNamesFromManifests,
} from "./rust-rules-cargo-manifest-dependencies.mjs";
import { scanCargoMetadata } from "./rust-rules-cargo-metadata.mjs";

function scanWorkspaceFiles(root, config, scope) {
  const violations = [];
  const cargoToml = path.join(root, "Cargo.toml");
  if (!fs.existsSync(cargoToml) || !config.enforceWorkspaceFiles)
    return violations;

  const required = [
    ["rust-toolchain.toml", "RR-1.1"],
    ["Cargo.lock", "RR-1.2"],
    ["clippy.toml", "RR-1.3"],
    ["deny.toml", "RR-1.4"],
  ];
  for (const [fileName, ruleId] of required) {
    if (!fs.existsSync(path.join(root, fileName))) {
      addViolation(
        violations,
        root,
        path.join(root, fileName),
        1,
        ruleId,
        `${fileName} is missing.`,
      );
    }
  }

  const manifestPaths = manifestPathsForScope(root, config, scope);
  const workspaceManifestPaths =
    scope.mode === "all" ? manifestPaths : findCargoManifests(root, config);
  const workspacePackageNames = workspacePackageNamesFromManifests(
    root,
    config,
    workspaceManifestPaths,
  );
  for (const manifest of manifestPaths) {
    scanCargoManifest(
      root,
      manifest,
      config,
      violations,
      workspacePackageNames,
    );
  }

  return violations;
}

function runScanner(root, config, scope, options = {}) {
  const violations = [];
  violations.push(...scanWorkspaceFiles(root, config, scope));
  violations.push(...scanCargoMetadata(root, config, scope));
  if (config.failFast) {
    const proofEvidenceCache = new Map();
    for (const filePath of scope.files) {
      violations.push(
        ...scanRustFile(root, filePath, config, { proofEvidenceCache }),
      );
      if (violations.length > 0) break;
    }
    return violations;
  }
  for (const result of rustFileFindings(root, config, scope, options)) {
    violations.push(...result.findings);
  }
  return violations;
}

export { scanWorkspaceFiles, runScanner };
