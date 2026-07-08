/*
 * Shared filesystem primitives for the test-doctrine scanner. Split out from
 * the orchestrator to avoid a circular import with test-doctrine-ci.mjs.
 */
import fs from "node:fs";
import path from "node:path";

const IGNORE_DIRS = new Set([
  ".git", "node_modules", ".venv", "venv", "env", "__pycache__",
  ".pytest_cache", ".mypy_cache", ".ruff_cache", ".hypothesis",
  "dist", "build", "target", ".next", "coverage", ".enforce", ".turbo",
  "out", ".cache", "vendor", ".stryker-tmp",
]);

const MAX_FILES = 40000;

function walk(root) {
  const files = [];
  const stack = [root];
  while (stack.length && files.length < MAX_FILES) {
    const dir = stack.pop();
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (IGNORE_DIRS.has(entry.name)) continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) stack.push(full);
      else files.push(full);
    }
  }
  return files;
}

function readTextSafe(filePath) {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch {
    return "";
  }
}

function relFiles(files, root) {
  return files.map((f) => path.relative(root, f).split(path.sep).join("/"));
}

export { walk, readTextSafe, relFiles };
