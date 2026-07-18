import {
  normalizeRel,
  uniqueSorted,
  findCargoManifests,
} from "./rust-rules-path-core.mjs";
import { nearestCargoManifest } from "./rust-rules-workspace-partitioning.mjs";

const manifestPathsCache = new WeakMap();

function manifestPathsForScope(root, config, scope) {
  const cached = manifestPathsCache.get(scope);
  if (cached) return cached;
  let manifestPaths;
  if (scope.mode === "crate" && scope.manifest) return [scope.manifest];
  if (scope.mode === "files" || scope.mode === "diff") {
    manifestPaths = uniqueSorted(
      scope.files
        .map((file) => nearestCargoManifest(root, file))
        .filter(Boolean),
    );
  } else {
    manifestPaths = findCargoManifests(root, config).filter(
      (manifest) => !normalizeRel(root, manifest).includes("/target/"),
    );
  }
  manifestPathsCache.set(scope, manifestPaths);
  return manifestPaths;
}

export { manifestPathsForScope };
