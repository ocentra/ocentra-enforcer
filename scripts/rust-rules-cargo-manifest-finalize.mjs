import fs from "node:fs";
import path from "node:path";
import { advisoryPolicyValue, isAllowedBuildScript } from "./rust-rules-cargo-manifest-policy.mjs";
import { addViolation } from "./rust-rules-path-core.mjs";

/** Applies the final Cargo-manifest policy checks to collected scan findings. */
export function applyCargoManifestFinalChecks({
  root,
  manifest,
  config,
  violations,
  dependencyNamesBySection,
  dependencyRequirementsByName,
}) {
  const prodDeps = new Set([
    ...(dependencyNamesBySection.get("dependencies") ?? []),
    ...[...dependencyNamesBySection.entries()]
      .filter(([section]) => /^target\..+\.dependencies/u.test(section))
      .flatMap(([, names]) => [...names]),
  ]);
  const devDeps = dependencyNamesBySection.get("dev-dependencies") ?? new Set();
  for (const name of devDeps) {
    if (prodDeps.has(name)) {
      addViolation(violations, root, manifest, 1, "RR-9.27", `${name} appears in both production and dev dependencies.`);
    }
  }
  for (const [dependencyName, requirements] of dependencyRequirementsByName) {
    if (requirements.size > 1) {
      addViolation(violations, root, manifest, 1, "RR-9.19", `Direct dependency ${dependencyName} uses multiple requirements: ${[...requirements].join(", ")}.`);
    }
  }
  const denyPath = path.join(root, "deny.toml");
  if (fs.existsSync(denyPath)) {
    const denyText = fs.readFileSync(denyPath, "utf8");
    if (advisoryPolicyValue(denyText, "yanked") !== "deny") {
      addViolation(violations, root, denyPath, 1, "RR-9.23", "deny.toml must deny yanked crate versions.");
    }
    const unmaintainedPolicy = advisoryPolicyValue(denyText, "unmaintained");
    if (unmaintainedPolicy !== "all" && unmaintainedPolicy !== "deny") {
      addViolation(violations, root, denyPath, 1, "RR-9.24", "deny.toml must deny unmaintained crates when advisory data is available.");
    }
  }
  const buildRs = path.join(path.dirname(manifest), "build.rs");
  if (fs.existsSync(buildRs) && !isAllowedBuildScript(root, buildRs, config)) {
    addViolation(violations, root, buildRs, 1, "RR-7.5", "build.rs is forbidden by default because it can hide non-deterministic build behavior.");
  }
}
