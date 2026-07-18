import fs from "node:fs";
import path from "node:path";
import { availableParallelism } from "node:os";

const WORKSPACE_WORKER_LIMIT = 8;

/** Finds the nearest Cargo manifest for a source file. */
export function nearestCargoManifest(root, filePath) {
  const rootPath = path.resolve(root);
  let current = path.dirname(path.resolve(filePath));
  while (true) {
    const relative = path.relative(rootPath, current);
    if (relative.startsWith("..") || path.isAbsolute(relative)) return null;
    const manifest = path.join(current, "Cargo.toml");
    if (fs.existsSync(manifest)) return path.resolve(manifest);
    if (current === rootPath) return null;
    const next = path.dirname(current);
    if (next === current) return null;
    current = next;
  }
}

function cargoOwnerKey(root, filePath) {
  return nearestCargoManifest(root, filePath) ?? root;
}

/** Splits Rust workspace files into balanced Cargo-manifest partitions. */
export function balancedWorkspacePartitions(root, files, workerCount) {
  const groups = new Map();
  files.forEach((filePath, index) => {
    const key = cargoOwnerKey(root, filePath);
    const existing = groups.get(key) ?? { bytes: 0, entries: [] };
    let bytes = 0;
    try {
      bytes = fs.statSync(filePath).size;
    } catch {
      // Missing files retain membership with zero scheduling weight; the
      // scanner owns the eventual read diagnostic.
    }
    existing.bytes += bytes;
    existing.entries.push({ index, filePath });
    groups.set(key, existing);
  });
  const partitions = Array.from({ length: workerCount }, () => ({ bytes: 0, entries: [] }));
  const orderedGroups = [...groups.values()].sort((left, right) =>
    right.bytes - left.bytes || left.entries[0].index - right.entries[0].index,
  );
  for (const group of orderedGroups) {
    partitions.sort((left, right) => left.bytes - right.bytes);
    partitions[0].bytes += group.bytes;
    partitions[0].entries.push(...group.entries);
  }
  return partitions.filter((partition) => partition.entries.length > 0);
}

/** Resolves a bounded worker count for a Rust workspace scan. */
export function workspaceWorkerCount(fileCount, requestedCount) {
  const available = Math.max(1, availableParallelism());
  const requested = requestedCount ?? available;
  return Math.max(1, Math.min(requested, available, WORKSPACE_WORKER_LIMIT, fileCount));
}
