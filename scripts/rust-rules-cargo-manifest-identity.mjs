import fs from "node:fs";
import path from "node:path";
import {
  contextHas,
  findCargoManifests,
  packageNameFromManifest,
} from "./rust-rules-path-core.mjs";

/** Loads workspace dependency justification metadata from the repository root. */
export function workspaceDependencyJustifications(root) {
  const manifest = path.join(root, "Cargo.toml");
  if (!fs.existsSync(manifest)) return new Map();
  const lines = fs.readFileSync(manifest, "utf8").split(/\r?\n/u);
  const justified = new Map();
  let section = "";
  lines.forEach((line, index) => {
    const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*$/u);
    if (sectionMatch) section = sectionMatch[1];
    if (section === "workspace.dependencies") addJustification(justified, lines, line, index);
  });
  return justified;
}

/** Determines whether a dependency line has an adjacent justification comment. */
export function hasDependencyJustification(lines, index) {
  if (contextHas(lines, index, "DEPENDENCY-JUSTIFICATION:", 4)) return true;
  return precedingComments(lines, index).join(" ").length >= 48;
}

/** Extracts a dependency name from one Cargo manifest line. */
export function dependencyNameFromManifestLine(line) {
  return line.match(/^\s*([A-Za-z0-9_.-]+)\s*=/u)?.[1] ?? null;
}

/** Extracts the version requirement from one Cargo manifest dependency line. */
export function dependencyRequirementFromManifestLine(line) {
  return line.match(/=\s*"([^"]+)"/u)?.[1] ?? line.match(/\bversion\s*=\s*"([^"]+)"/u)?.[1] ?? null;
}

/** Collects workspace package names from the provided or discovered manifests. */
export function workspacePackageNamesFromManifests(root, config, manifests = null) {
  const names = new Set();
  for (const manifest of manifests ?? findCargoManifests(root, config)) {
    const name = packageNameFromManifest(manifest);
    if (name) names.add(name);
  }
  return names;
}

function addJustification(justified, lines, line, index) {
  const dependencyName = dependencyNameFromManifestLine(line);
  if (dependencyName) justified.set(dependencyName, hasDependencyJustification(lines, index));
}

function precedingComments(lines, index) {
  const comments = [];
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const line = lines[cursor].trim();
    if (!line.startsWith("#")) break;
    comments.push(line.slice(1).trim());
  }
  return comments;
}
