import { addViolation } from "./rust-rules-path-core.mjs";

function directRegistryRequirements(packages) {
  const requirements = new Map();
  for (const packageInfo of packages) {
    for (const dependency of packageInfo.dependencies ?? []) {
      const excluded =
        (dependency.path ?? null) !== null ||
        dependency.kind === "dev" ||
        (dependency.source ?? "").startsWith("git+");
      if (excluded) continue;
      if (!requirements.has(dependency.name)) {
        requirements.set(dependency.name, new Set());
      }
      requirements.get(dependency.name).add(dependency.req);
    }
  }
  return requirements;
}

/** Reports conflicting direct registry requirements across scoped packages. */
export function scanMetadataRequirements(root, packages) {
  const violations = [];
  for (const [dependencyName, requirements] of directRegistryRequirements(
    packages,
  )) {
    if (requirements.size <= 1) continue;
    const detail = [...requirements].join(", ");
    addViolation(
      violations,
      root,
      ".",
      1,
      "RR-9.5",
      `Direct registry dependency ${dependencyName} uses multiple requirements: ${detail}.`,
    );
    addViolation(
      violations,
      root,
      ".",
      1,
      "RR-9.19",
      `Direct registry dependency ${dependencyName} uses duplicate requirements: ${detail}.`,
    );
  }
  return violations;
}
