import fs from "node:fs";
import path from "node:path";
import { finding } from "../scripts/check-source-core-helpers.mjs";
import { lineForJsonKey, packageExportTargets, isBoundedNodeEngine } from "./check-governance-manifest-shared.mjs";

export function collectPackageManifestPolicyFindings(root, packageJsonPath, manifest) {
  const findings = [];
  if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, "files"), "NPM-1.13", "package.json must declare an explicit files allowlist for publishing", null),
    );
  }
  for (const [name, target] of Object.entries(manifest.bin ?? {})) {
    const targetPath = path.join(root, String(target));
    if (!fs.existsSync(targetPath)) {
      findings.push(
        finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, name), "NPM-1.14", `bin ${name} points at missing path ${target}`, null),
      );
    }
  }
  for (const target of packageExportTargets(manifest.exports)) {
    const targetPath = path.join(root, target);
    const exists = target.includes("*")
      ? fs.existsSync(path.join(root, target.split("*")[0]))
      : fs.existsSync(targetPath);
    if (!exists) {
      findings.push(
        finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, "exports"), "NPM-1.15", `exports target ${target} does not exist`, null),
      );
    }
  }
  if (!/^npm@\d+\.\d+\.\d+$/u.test(String(manifest.packageManager ?? ""))) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, "packageManager"), "NPM-1.4", "packageManager must pin an exact npm version, for example npm@11.7.0", null),
    );
  }
  if (!isBoundedNodeEngine(manifest.engines?.node)) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, "engines"), "NPM-1.5", `engines.node must be bounded; found ${manifest.engines?.node ?? "MISSING"}`, null),
    );
  }
  return findings;
}
