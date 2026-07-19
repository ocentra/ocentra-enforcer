import { addViolation, toPosix } from "./rust-rules-path-core.mjs";

function addDependencyViolation(violations, root, packageInfo, ruleId, detail) {
  addViolation(
    violations,
    root,
    packageInfo.manifest_path,
    1,
    ruleId,
    detail,
  );
}

/** Applies metadata-backed policy to scoped package dependencies. */
export function scanMetadataDependencies({
  root,
  config,
  packages,
  workspaceRoot,
  workspacePackageNames,
}) {
  const violations = [];
  for (const packageInfo of packages) {
    const blocked = new Set(
      config.blockedProtocolDependencies[packageInfo.name] ?? [],
    );
    for (const dependency of packageInfo.dependencies ?? []) {
      if (
        (dependency.source ?? "").startsWith("git+") &&
        !config.allowedGitDependenciesSet.has(dependency.name)
      ) {
        addDependencyViolation(
          violations,
          root,
          packageInfo,
          "RR-9.2",
          "Git dependency found in cargo metadata.",
        );
      }
      const dependencyPath = dependency.path ?? null;
      if (
        dependencyPath !== null &&
        !toPosix(dependencyPath).startsWith(workspaceRoot)
      ) {
        addDependencyViolation(
          violations,
          root,
          packageInfo,
          "RR-9.4",
          "Path dependency points outside the workspace root.",
        );
      }
      if (dependencyPath === null && dependency.req.trim() === "*") {
        addDependencyViolation(
          violations,
          root,
          packageInfo,
          "RR-9.1",
          "Wildcard registry dependency version found.",
        );
      }
      if (blocked.has(dependency.name)) {
        addDependencyViolation(
          violations,
          root,
          packageInfo,
          "RR-9.4",
          `${packageInfo.name} must not depend on ${dependency.name}.`,
        );
      }
      if (
        config.runtimeCratesSet.has(packageInfo.name) &&
        config.testOnlyCratesSet.has(dependency.name) &&
        dependency.kind !== "dev"
      ) {
        addDependencyViolation(
          violations,
          root,
          packageInfo,
          "RR-9.29",
          "Runtime crate depends on test-only crate outside dev-dependencies.",
        );
      }
      if (
        workspacePackageNames.has(dependency.name) &&
        dependency.path === null &&
        dependency.source !== null
      ) {
        addDependencyViolation(
          violations,
          root,
          packageInfo,
          "RR-9.26",
          `Workspace member ${packageInfo.name} depends on ${dependency.name} by registry version instead of path/workspace linkage.`,
        );
      }
    }
  }
  return violations;
}
