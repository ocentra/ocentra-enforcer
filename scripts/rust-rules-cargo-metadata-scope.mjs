import { toPosix } from "./rust-rules-path-core.mjs";
import { manifestPathsForScope } from "./rust-rules-cargo-manifest-discovery.mjs";

/** Selects Cargo metadata packages that belong to the requested scan scope. */
export function cargoMetadataScope(root, config, scope, metadata) {
  const scopedManifests = new Set(
    manifestPathsForScope(root, config, scope).map((manifest) =>
      toPosix(manifest),
    ),
  );
  const packageFilter = packageFilterForScope(scope, scopedManifests);
  return {
    packages: (metadata.packages ?? []).filter(packageFilter),
    workspacePackageNames: new Set(
      (metadata.packages ?? []).map((packageInfo) => packageInfo.name),
    ),
    workspaceRoot: toPosix(metadata.workspace_root),
  };
}

function packageFilterForScope(scope, scopedManifests) {
  if (scope.mode === "crate") {
    return (packageInfo) => packageInfo.name === scope.crateName;
  }
  if (scope.mode === "files" || scope.mode === "diff") {
    return (packageInfo) => scopedManifests.has(toPosix(packageInfo.manifest_path));
  }
  return () => true;
}
