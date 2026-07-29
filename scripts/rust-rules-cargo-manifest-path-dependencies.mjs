import fs from "node:fs";
import path from "node:path";
import { packageNameFromManifest, toPosix } from "./rust-rules-path-core.mjs";

/** Returns whether a manifest path dependency satisfies the configured policy. */
export function isAllowedPathDependency(context) {
  if (isTreeSitterDependency(context.dependencyName)) return isFirstPartyVendoredParserDependency(context);
  return isWorkspaceMemberPathDependency(context);
}

function isTreeSitterDependency(dependencyName) {
  return /^tree-sitter-[a-z0-9-]+$/u.test(dependencyName ?? "");
}

function isWorkspaceMemberPathDependency({ currentSection, dependencyName, workspacePackageNames }) {
  return /^(?:workspace\.)?(?:dependencies|dev-dependencies|build-dependencies|target\..+\.dependencies)(?:\.|$)/u.test(currentSection) && dependencyName !== null && workspacePackageNames.has(dependencyName);
}

function isFirstPartyVendoredParserDependency({ root, manifest, dependencyName, line }) {
  const dependencyPath = resolvedVendoredDependencyPath(root, manifest, dependencyName, line);
  if (!dependencyPath) return false;
  const manifestPath = path.join(dependencyPath, "Cargo.toml");
  const parserPath = path.join(dependencyPath, "src", "parser.c");
  if (!fs.existsSync(manifestPath) || !fs.existsSync(parserPath)) return false;
  const cargoText = fs.readFileSync(manifestPath, "utf8");
  return parserManifestIsFirstParty(dependencyPath, manifestPath, line, cargoText, dependencyName);
}

function resolvedVendoredDependencyPath(root, manifest, dependencyName, line) {
  if (!isTreeSitterDependency(dependencyName)) return null;
  const pathMatch = line.match(/\bpath\s*=\s*"([^"]+)"/u);
  if (!pathMatch) return null;
  const dependencyPath = path.resolve(path.dirname(manifest), pathMatch[1]);
  const relative = toPosix(path.relative(path.resolve(root), dependencyPath));
  const valid = !relative.startsWith("../") && !path.isAbsolute(relative) && /(?:^|\/)vendor\/tree-sitter-[a-z0-9-]+$/u.test(relative) && path.basename(dependencyPath) === dependencyName;
  return valid ? dependencyPath : null;
}

function parserManifestIsFirstParty(dependencyPath, manifestPath, line, cargoText, dependencyName) {
  const librarySource = parserLibrarySource(dependencyPath, cargoText);
  const alias = line.match(/\bpackage\s*=\s*"([^"]+)"/u)?.[1] ?? dependencyName;
  return librarySource !== null && packageNameFromManifest(manifestPath) === alias && /(?:^|\n)\s*publish\s*=\s*false\b/u.test(cargoText) && !manifestHasNestedSourceDependency(cargoText);
}

function parserLibrarySource(dependencyPath, cargoText) {
  const relative = cargoText.match(/(?:^|\n)\s*\[lib\][\s\S]*?(?:^|\n)\s*path\s*=\s*"([^"]+)"/mu)?.[1] ?? "src/lib.rs";
  const librarySource = path.resolve(dependencyPath, relative);
  return librarySource.startsWith(`${dependencyPath}${path.sep}`) && fs.existsSync(librarySource) ? librarySource : null;
}

function manifestHasNestedSourceDependency(cargoText) {
  let section = "";
  for (const line of cargoText.split(/\r?\n/u)) {
    const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*$/u);
    if (sectionMatch) section = sectionMatch[1];
    const dependencySection = /^(?:dependencies|dev-dependencies|build-dependencies|target\..+\.dependencies)(?:\.|$)/u.test(section);
    if (dependencySection && /^\s*[\w.-]+\s*=.*\b(?:path|git)\s*=/u.test(line)) return true;
  }
  return false;
}
