import fs from "node:fs";
import path from "node:path";
import { finding } from "../scripts/check-source-core-helpers.mjs";
import { dependencySections, isDeterministicDependencyVersion, isSuspiciousDependencyName, lineForJsonKey } from "./check-governance-manifest-shared.mjs";

export function collectPackageDependencyVersionFindings(root, packageJsonPath, manifest) {
  const findings = [];
  for (const [sectionName, dependencies] of dependencySections(manifest)) {
    findings.push(...collectDependencyEntriesFindings(root, packageJsonPath, sectionName, dependencies));
  }
  return findings;
}

function collectDependencyEntriesFindings(root, packageJsonPath, sectionName, dependencies) {
  const findings = [];
  for (const [dependencyName, version] of Object.entries(dependencies)) {
    findings.push(...collectDependencyVersionFinding(root, packageJsonPath, sectionName, dependencyName, version));
  }
  return findings;
}

function collectDependencyVersionFinding(root, packageJsonPath, sectionName, dependencyName, version) {
  const findings = [];
  const versionText = String(version ?? "").trim();
  if (isGitDependencyVersion(versionText)) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, dependencyName), "NPM-1.7", `${sectionName}.${dependencyName} uses git dependency ${versionText}`, null),
    );
    return findings;
  }
  if (isFileDependencyVersion(versionText)) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, dependencyName), "NPM-1.8", `${sectionName}.${dependencyName} uses file/path dependency ${versionText}`, null),
    );
    return findings;
  }
  if (isSuspiciousDependencyName(dependencyName)) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, dependencyName), "NPM-1.11", `${sectionName}.${dependencyName} has suspicious package name`, null),
    );
  }
  if (!isDeterministicDependencyVersion(version)) {
    findings.push(
      finding(root, packageJsonPath, lineForJsonKey(packageJsonPath, dependencyName), "NPM-1.3", `${sectionName}.${dependencyName} uses non-deterministic version ${version}`, null),
    );
  }
  return findings;
}

function isGitDependencyVersion(versionText) {
  return /^(?:git\+|github:|git:|https?:\/\/.*\.git)/iu.test(versionText);
}

function isFileDependencyVersion(versionText) {
  return /^(?:file:|link:|workspace:)|^\.\.?(?:\/|\\)/iu.test(versionText);
}
