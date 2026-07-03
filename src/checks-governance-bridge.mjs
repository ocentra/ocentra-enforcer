import { resolveScope } from "../scripts/rust-rules-path-core.mjs";
import {
  collectCiIntegrityFindings,
  collectDependencyPolicyFindings,
  collectMutationRiskFindings,
  collectPackageDeterminismFindings,
  collectRepoGovernanceFindings,
  runSbomCheck,
} from "./check-governance.mjs";
import {
  collectConfigLockdownFindings,
  collectWaiverPolicyFindings,
} from "./check-policy.mjs";

export function collectGovernanceStandaloneFindings({
  checkName,
  root,
  config,
  args,
  scope,
}) {
  switch (checkName) {
    case "dependency-policy":
      return {
        findings: collectDependencyPolicyFindings(root, config),
      };
    case "sbom":
      return {
        findings: runSbomCheck(root, args),
      };
    case "config-lockdown":
      return {
        findings: collectConfigLockdownFindings(root, config),
      };
    case "waiver-policy":
      return {
        findings: collectWaiverPolicyFindings(root, config),
      };
    case "ci-integrity":
      return {
        findings: collectCiIntegrityFindings(root),
      };
    case "repo-governance":
      return {
        findings: collectRepoGovernanceFindings(root),
      };
    case "package-determinism":
      return {
        findings: collectPackageDeterminismFindings(root),
      };
    case "mutation-risk":
      return {
        findings: collectMutationRiskFindings(root, scope),
      };
    case "policy-integrity":
      return {
        findings: [
          ...collectConfigLockdownFindings(root, config),
          ...collectWaiverPolicyFindings(root, config),
        ],
      };
    default:
      throw new Error(`Unsupported governance check: ${checkName}`);
  }
}
