import fs from "node:fs";
import path from "node:path";
import { finding } from "../scripts/check-source-core-helpers.mjs";
import { repoAbsolute } from "./path-utils.mjs";
import { parsePackageManifest } from "./check-governance-manifest-shared.mjs";
import { collectPackageLockPresenceFindings } from "./check-governance-manifest-lock.mjs";
import { collectPackageLockInstallScriptFindings } from "./check-governance-manifest-lock.mjs";
import { collectPackageManifestPolicyFindings } from "./check-governance-manifest-policy.mjs";
import { collectPackageDependencyVersionFindings } from "./check-governance-manifest-deps.mjs";

export function collectPackageDeterminismFindings(root) {
  const packageJsonPath = path.join(root, "package.json");
  if (!fs.existsSync(packageJsonPath)) return [];
  const parsed = parsePackageManifest(packageJsonPath);
  if (!parsed.ok) return parsed.findings;
  return [
    ...collectPackageLockPresenceFindings(root, packageJsonPath),
    ...collectPackageLockInstallScriptFindings(root),
    ...collectPackageManifestPolicyFindings(root, packageJsonPath, parsed.manifest),
    ...collectPackageDependencyVersionFindings(root, packageJsonPath, parsed.manifest),
  ];
}
