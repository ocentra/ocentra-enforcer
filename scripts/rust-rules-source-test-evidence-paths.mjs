import fs from "node:fs";
import path from "node:path";

/** Finds the nearest Cargo package root that owns a Rust source file. */
export function crateRootForEvidence(root, filePath) {
  let current = path.dirname(filePath);
  while (current.toLowerCase().startsWith(path.resolve(root).toLowerCase())) {
    if (fs.existsSync(path.join(current, "Cargo.toml"))) return current;
    const parent = path.dirname(current);
    if (parent === current) return null;
    current = parent;
  }
  return null;
}

/** Collects Rust files beneath one crate-owned source or evidence directory. */
export function collectRustEvidenceTree(directory, files) {
  if (!fs.existsSync(directory)) return;
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) collectRustEvidenceTree(entryPath, files);
    else if (entry.isFile() && entry.name.endsWith(".rs")) {
      files.push({ path: entryPath, source: fs.readFileSync(entryPath, "utf8") });
    }
  }
}
