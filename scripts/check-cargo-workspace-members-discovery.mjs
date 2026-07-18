import { existsSync, readdirSync } from "node:fs";
import path from "node:path";

/** Returns a workspace-relative manifest path in normalized slash form. */
export function normalizedRelative(root, manifestPath) {
  return path.relative(root, manifestPath).split(path.sep).join("/");
}

/** Discovers product Cargo manifests expected below the workspace root. */
export function expectedProductManifests(root) {
  const manifests = [];
  const cratesRoot = path.join(root, "crates");
  if (existsSync(cratesRoot)) {
    for (const entry of readdirSync(cratesRoot, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const manifest = path.join(cratesRoot, entry.name, "Cargo.toml");
      if (existsSync(manifest)) manifests.push(normalizedRelative(root, manifest));
    }
  }

  const xtaskManifest = path.join(root, "xtask", "Cargo.toml");
  if (existsSync(xtaskManifest)) manifests.push(normalizedRelative(root, xtaskManifest));
  return manifests.sort();
}
