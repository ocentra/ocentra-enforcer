import fs from "node:fs";
import path from "node:path";
import { policyForTool } from "../src/policy.mjs";
import { runCargoBuildGates } from "./rust-rules-cargo-gates-build.mjs";
import { runCargoSecurityGates } from "./rust-rules-cargo-gates-security.mjs";

function shouldRunCargoForScope(scope, config) {
  if (scope.mode === "all" || scope.mode === "crate") return true;
  if (scope.mode === "files") return config.cargoOnFileScope;
  if (scope.mode === "diff") return config.cargoOnDiffScope;
  return false;
}

function cargoPackageArgs(scope) {
  return scope.mode === "crate"
    ? ["--package", scope.crateName]
    : ["--workspace"];
}

function cargoFmtArgs(scope) {
  // `cargo fmt --package <crate> --all --check` is not a crate-scoped
  // operation: rustfmt treats `--all` as a workspace-wide traversal and can
  // walk dependency/vendor sources. That made a focused packet gate inspect
  // generated tree-sitter trees (and even hit Windows path-length limits).
  // Keep the package gate package-only; reserve `--all` for the workspace
  // gate where the caller explicitly requested every workspace package.
  return scope.mode === "crate"
    ? ["fmt", "--package", scope.crateName, "--check"]
    : ["fmt", "--all", "--check"];
}

function strongestEnabledSeverity(policies) {
  const enabled = policies
    .filter((policy) => policy.enabled)
    .map((policy) => policy.severity);
  if (enabled.includes("error")) return "error";
  if (enabled.includes("warning")) return "warning";
  return "info";
}

function runCargoGates(root, config, scope) {
  const violations = [];
  if (!fs.existsSync(path.join(root, "Cargo.toml"))) return violations;
  if (!shouldRunCargoForScope(scope, config)) return violations;

  const cargoToolPolicies = [
    policyForTool("cargoFmt", config, { enabled: config.runCargoFmt, severity: "error" }),
    policyForTool("cargoClippy", config, { enabled: config.runCargoClippy, severity: "error" }),
    policyForTool("cargoTest", config, { enabled: config.runCargoTest, severity: "error" }),
    policyForTool("cargoDoc", config, { enabled: config.runCargoDoc, severity: "error" }),
    policyForTool("cargoDeny", config, { enabled: config.requireCargoDeny, severity: "error" }),
    policyForTool("cargoAudit", config, { enabled: config.requireCargoAudit, severity: "error" }),
  ];
  if (!cargoToolPolicies.some((policy) => policy.enabled)) return violations;

  const packageArgs = cargoPackageArgs(scope);
  violations.push(
    ...runCargoBuildGates(
      root,
      config,
      cargoToolPolicies,
      packageArgs,
      cargoFmtArgs(scope),
    ),
  );
  violations.push(...runCargoSecurityGates(root, config, cargoToolPolicies));

  return violations;
}

export {
  shouldRunCargoForScope,
  cargoPackageArgs,
  cargoFmtArgs,
  strongestEnabledSeverity,
  runCargoGates,
};
