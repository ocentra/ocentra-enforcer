import fs from "node:fs";
import { DEFAULT_ALLOWED_LICENSES } from "./check-metadata.mjs";
import { finding, compactProcessOutput, spawnInRoot } from "../scripts/check-source-core-helpers.mjs";

/** Evaluate npm audit and license policy for the repository package lock. */
export function collectNpmDependencyPolicyFindings(root, packageLockPath, config) {
  const findings = [];
  const audit = spawnInRoot(root, "npm", ["audit", "--audit-level=high", "--json"]);
  if (audit.status !== 0) {
    findings.push(
      finding(root, packageLockPath, 1, "NPM-1.9", "npm audit reported high-or-higher vulnerabilities", compactProcessOutput(audit)),
      finding(root, packageLockPath, 1, "DEP-1.1", "npm audit reported high-or-higher vulnerabilities", compactProcessOutput(audit)),
    );
  }
  const lock = JSON.parse(fs.readFileSync(packageLockPath, "utf8"));
  const allowed = new Set(config.allowedExternalLicenses ?? [...DEFAULT_ALLOWED_LICENSES]);
  for (const [lockPath, packageEntry] of Object.entries(lock.packages ?? {})) {
    if (!lockPath.includes("node_modules")) continue;
    const packageName = lockPath.split("node_modules/").at(-1);
    if (packageName?.startsWith("@ocentra-parent/") || packageName?.startsWith("@ocentra/")) continue;
    const license = packageEntry.license;
    if (typeof license !== "string" || !allowed.has(license)) {
      findings.push(
        finding(root, packageLockPath, 1, "NPM-1.10", `${lockPath}: ${license ?? "MISSING"}`, null),
        finding(root, packageLockPath, 1, "DEP-1.2", `${lockPath}: ${license ?? "MISSING"}`, null),
      );
    }
  }
  return findings;
}
