import { readFileSync } from "node:fs";

import {
  expectedProductManifests,
  normalizedRelative,
} from "./check-cargo-workspace-members-discovery.mjs";

function sortedUnique(values) {
  return [...new Set(values)].sort();
}

function difference(left, right) {
  const rightSet = new Set(right);
  return left.filter((value) => !rightSet.has(value));
}

function hasWorkspaceLintOptIn(manifestPath) {
  let inLintsTable = false;
  for (const line of readFileSync(manifestPath, "utf8").split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
      inLintsTable = trimmed === "[lints]";
      continue;
    }
    if (inLintsTable && /^workspace\s*=\s*true\s*$/u.test(trimmed)) return true;
  }
  return false;
}

/** Validates that Cargo metadata contains every expected product workspace member. */
export function validateCargoWorkspaceMembers(root, metadata) {
  const packageById = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
  const errors = [];

  function manifestsFor(memberIds, label) {
    return sortedUnique(memberIds.flatMap((id) => {
      const pkg = packageById.get(id);
      if (!pkg) {
        errors.push(`${label} references unknown package id: ${id}`);
        return [];
      }
      return [normalizedRelative(root, pkg.manifest_path)];
    }));
  }

  const expected = expectedProductManifests(root);
  const workspace = manifestsFor(metadata.workspace_members, "workspace_members");
  const defaults = manifestsFor(metadata.workspace_default_members, "workspace_default_members");
  const vendorMembers = workspace.filter((manifest) => manifest.includes("/vendor/"));
  const missing = difference(expected, workspace);
  const unexpected = difference(workspace, expected);
  const missingDefaults = difference(expected, defaults);
  const unexpectedDefaults = difference(defaults, expected);

  if (vendorMembers.length) errors.push(`vendored packages entered workspace: ${vendorMembers.join(", ")}`);
  if (missing.length) errors.push(`product packages missing from workspace: ${missing.join(", ")}`);
  if (unexpected.length) errors.push(`unexpected workspace packages: ${unexpected.join(", ")}`);
  if (missingDefaults.length) errors.push(`product packages missing from defaults: ${missingDefaults.join(", ")}`);
  if (unexpectedDefaults.length) errors.push(`unexpected default packages: ${unexpectedDefaults.join(", ")}`);

  const workspacePackages = metadata.workspace_members
    .map((id) => packageById.get(id))
    .filter(Boolean)
    .sort((left, right) => left.name.localeCompare(right.name));
  const expectedSet = new Set(expected);
  const lintOptIns = workspacePackages
    .filter((pkg) => expectedSet.has(normalizedRelative(root, pkg.manifest_path)))
    .filter((pkg) => hasWorkspaceLintOptIn(pkg.manifest_path))
    .map((pkg) => normalizedRelative(root, pkg.manifest_path));
  const missingLintOptIns = difference(expected, lintOptIns);
  if (missingLintOptIns.length) {
    errors.push(`product packages missing [lints] workspace = true: ${missingLintOptIns.join(", ")}`);
  }

  return {
    ok: errors.length === 0,
    errors,
    expected,
    workspace,
    defaults,
    workspacePackages,
    lintOptIns,
  };
}
