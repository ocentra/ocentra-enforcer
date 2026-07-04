import fs from "node:fs";
import path from "node:path";
import { finding } from "../scripts/check-source-core-helpers.mjs";

export function collectPackageLockPresenceFindings(root, packageJsonPath) {
  return fs.existsSync(path.join(root, "package-lock.json"))
    ? []
    : [finding(root, packageJsonPath, 1, "NPM-1.1", "package-lock.json is missing", null)];
}

export function collectPackageLockInstallScriptFindings(root) {
  const packageLockPath = path.join(root, "package-lock.json");
  if (!fs.existsSync(packageLockPath)) return [];
  const lock = JSON.parse(fs.readFileSync(packageLockPath, "utf8"));
  const findings = [];
  for (const [lockPath, packageEntry] of Object.entries(lock.packages ?? {})) {
    if (packageEntry?.hasInstallScript === true) {
      findings.push(
        finding(root, packageLockPath, 1, "NPM-1.6", `${lockPath || "."} declares an install script`, null),
      );
    }
  }
  return findings;
}
