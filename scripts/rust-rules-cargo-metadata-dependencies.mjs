import { addViolation, toPosix } from "./rust-rules-path-core.mjs";

/** Collects dependency findings from Cargo metadata for the selected scope. */
export function cargoMetadataDependencyFindings(root, config, metadataScope) {
  const violations = [];
  const { packages, workspacePackageNames, workspaceRoot } = metadataScope;
  for (const packageInfo of packages) {
    const blocked = new Set(config.blockedProtocolDependencies[packageInfo.name] ?? []);
    for (const dependency of packageInfo.dependencies ?? []) {
      addMetadataDependencyFindings({
        violations, root, config, packageInfo, dependency, blocked,
        workspacePackageNames, workspaceRoot,
      });
    }
  }
  return violations;
}

function addMetadataDependencyFindings(context) {
  addGitDependencyFinding(context);
  addExternalPathFinding(context);
  addWildcardDependencyFinding(context);
  addBlockedDependencyFinding(context);
  addTestOnlyDependencyFinding(context);
  addWorkspaceRegistryFinding(context);
}

function addGitDependencyFinding({ violations, root, config, packageInfo, dependency }) {
  if ((dependency.source ?? "").startsWith("git+") && !config.allowedGitDependenciesSet.has(dependency.name)) {
    addViolation(violations, root, packageInfo.manifest_path, 1, "RR-9.2", "Git dependency found in cargo metadata.");
  }
}

function addExternalPathFinding({ violations, root, packageInfo, dependency, workspaceRoot }) {
  const dependencyPath = dependency.path ?? null;
  if (dependencyPath !== null && !toPosix(dependencyPath).startsWith(workspaceRoot)) {
    addViolation(violations, root, packageInfo.manifest_path, 1, "RR-9.4", "Path dependency points outside the workspace root.");
  }
}

function addWildcardDependencyFinding({ violations, root, packageInfo, dependency }) {
  if (dependency.path === null && dependency.req.trim() === "*") {
    addViolation(violations, root, packageInfo.manifest_path, 1, "RR-9.1", "Wildcard registry dependency version found.");
  }
}

function addBlockedDependencyFinding({ violations, root, packageInfo, dependency, blocked }) {
  if (blocked.has(dependency.name)) {
    addViolation(violations, root, packageInfo.manifest_path, 1, "RR-9.4", `${packageInfo.name} must not depend on ${dependency.name}.`);
  }
}

function addTestOnlyDependencyFinding({ violations, root, config, packageInfo, dependency }) {
  if (config.runtimeCratesSet.has(packageInfo.name) && config.testOnlyCratesSet.has(dependency.name) && dependency.kind !== "dev") {
    addViolation(violations, root, packageInfo.manifest_path, 1, "RR-9.29", "Runtime crate depends on test-only crate outside dev-dependencies.");
  }
}

function addWorkspaceRegistryFinding({ violations, root, packageInfo, dependency, workspacePackageNames }) {
  if (workspacePackageNames.has(dependency.name) && dependency.path === null && dependency.source !== null) {
    addViolation(violations, root, packageInfo.manifest_path, 1, "RR-9.26", `Workspace member ${packageInfo.name} depends on ${dependency.name} by registry version instead of path/workspace linkage.`);
  }
}
