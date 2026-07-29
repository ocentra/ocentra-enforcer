import fs from "node:fs";
import path from "node:path";

/** Finds the nearest Cargo workspace root for a source file. */
export function nearestCargoRoot(root, filePath) {
  const scanRoot = path.resolve(root);
  let current = path.dirname(path.resolve(filePath));
  while (current.toLowerCase().startsWith(scanRoot.toLowerCase())) {
    if (fs.existsSync(path.join(current, "Cargo.toml"))) return current;
    if (current === scanRoot) break;
    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }
  return null;
}

/** Collects Rust evidence files beneath one owned source or test directory. */
export function rustFilesUnder(directory) {
  if (!fs.existsSync(directory)) return [];
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...rustFilesUnder(entryPath));
    else if (entry.isFile() && entry.name.endsWith(".rs")) files.push(entryPath);
  }
  return files;
}
