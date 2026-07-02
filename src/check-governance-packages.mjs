import { collectDependencyPolicyFindings as collectDependencyPolicyFindingsImpl } from "./check-governance-dependency.mjs";
import { collectPackageDeterminismFindings as collectPackageDeterminismFindingsImpl } from "./check-governance-manifest.mjs";
import { runSbomCheck as runSbomCheckImpl } from "./check-governance-sbom.mjs";

export function collectDependencyPolicyFindings(root, packageJsonPath, manifest) {
  return collectDependencyPolicyFindingsImpl(root, packageJsonPath, manifest);
}

export function collectPackageDeterminismFindings(root, packageJsonPath, manifest) {
  return collectPackageDeterminismFindingsImpl(root, packageJsonPath, manifest);
}

export function runSbomCheck(root, packageJsonPath, manifest) {
  return runSbomCheckImpl(root, packageJsonPath, manifest);
}
