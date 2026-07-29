import path from "node:path";

function workspacePackages(metadata) {
  return metadata.packages.filter((pkg) => metadata.workspace_members.includes(pkg.id));
}

function changedPackageIds(packages, workspaceRoot, changedFiles) {
  return new Set(packages.filter((pkg) => {
    const root = path.dirname(pkg.manifest_path).replaceAll("\\", "/");
    const relativeRoot = path.relative(workspaceRoot, root).replaceAll("\\", "/");
    return changedFiles.some((file) => file === relativeRoot || file.startsWith(`${relativeRoot}/`));
  }).map((pkg) => pkg.id));
}

function reverseDependencies(packages) {
  const reverse = new Map();
  for (const pkg of packages) {
    for (const dependency of pkg.dependencies) {
      const dependencyPackage = packages.find((candidate) => candidate.name === dependency.name && dependency.path);
      if (!dependencyPackage) continue;
      const dependents = reverse.get(dependencyPackage.id) ?? new Set();
      dependents.add(pkg.id);
      reverse.set(dependencyPackage.id, dependents);
    }
  }
  return reverse;
}

export function impactedPackageNames(metadata, changedFiles) {
  const packages = workspacePackages(metadata);
  const byId = new Map(packages.map((pkg) => [pkg.id, pkg]));
  const changed = changedPackageIds(packages, metadata.workspace_root, changedFiles);
  const reverse = reverseDependencies(packages);
  const queue = [...changed];
  while (queue.length > 0) {
    for (const dependent of reverse.get(queue.shift()) ?? []) {
      if (!changed.has(dependent)) {
        changed.add(dependent);
        queue.push(dependent);
      }
    }
  }
  return [...changed].map((id) => byId.get(id)?.name).filter(Boolean).sort();
}
