import { addViolation } from "./rust-rules-path-core.mjs";

/** Collects registry-source policy findings from Cargo metadata packages. */
export function cargoMetadataRegistryFindings(root, packages) {
  const violations = [];
  for (const [dependencyName, requirements] of directRegistryRequirements(packages)) {
    if (requirements.size > 1) {
      const detail = `Direct registry dependency ${dependencyName} uses multiple requirements: ${[...requirements].join(", ")}.`;
      addViolation(violations, root, ".", 1, "RR-9.5", detail);
      addViolation(violations, root, ".", 1, "RR-9.19", detail.replace("multiple", "duplicate"));
    }
  }
  return violations;
}

function directRegistryRequirements(packages) {
  const requirements = new Map();
  for (const packageInfo of packages) {
    for (const dependency of packageInfo.dependencies ?? []) {
      if (isDirectRegistryDependency(dependency)) addRequirement(requirements, dependency);
    }
  }
  return requirements;
}

function isDirectRegistryDependency(dependency) {
  return dependency.path === null && dependency.kind !== "dev" && !(dependency.source ?? "").startsWith("git+");
}

function addRequirement(requirements, dependency) {
  if (!requirements.has(dependency.name)) requirements.set(dependency.name, new Set());
  requirements.get(dependency.name).add(dependency.req);
}
