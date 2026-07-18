import fs from "node:fs";
import path from "node:path";
import { collectNpmDependencyPolicyFindings } from "./check-governance-npm-dependency.mjs";
import { collectCargoDependencyPolicyFindings } from "./check-governance-cargo-audit.mjs";

/** Run the configured npm and Cargo dependency-policy checks for a repository. */
export function collectDependencyPolicyFindings(root, config) {
  const findings = [];
  const packageLockPath = path.join(root, "package-lock.json");
  if (fs.existsSync(packageLockPath)) {
    findings.push(...collectNpmDependencyPolicyFindings(root, packageLockPath, config));
  }
  const cargoLockPath = path.join(root, "Cargo.lock");
  if (fs.existsSync(cargoLockPath)) {
    findings.push(...collectCargoDependencyPolicyFindings(root, cargoLockPath));
  }
  return findings;
}
