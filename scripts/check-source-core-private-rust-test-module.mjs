import fs from "node:fs";
import path from "node:path";

function isAllowedPrivateRustTestModule(root, file, lines, config) {
  if (!file.endsWith(".rs")) return false;
  const ownerFile = normalizeRelativePath(root, file);
  const entry = (config.privateRustTestModuleAllowlist ?? []).find(
    (candidate) => candidate.ownerFile === ownerFile,
  );
  if (!isValidPrivateRustTestModuleEntry(entry, ownerFile)) return false;
  if (!fs.existsSync(path.join(root, entry.moduleFile))) return false;
  return hasExactPrivateTestModuleDeclaration(lines, entry);
}

function hasExactPrivateTestModuleDeclaration(lines, entry) {
  const cfgIndex = lines.findIndex((line) => /^\s*#\[cfg\(test\)\]\s*$/u.test(line));
  if (cfgIndex < 0 || hasAnotherCfgTestAttribute(lines, cfgIndex)) return false;
  return lines[cfgIndex].trim() === "#[cfg(test)]"
    && lines[cfgIndex + 1].trim() === `#[path=\"${path.basename(entry.moduleFile)}\"]`
    && lines[cfgIndex + 2].trim() === `mod ${entry.moduleName};`;
}

function hasAnotherCfgTestAttribute(lines, allowedIndex) {
  return lines.some(
    (line, index) => index !== allowedIndex && /^\s*#\[cfg\(test\)\]/u.test(line),
  );
}

function isValidPrivateRustTestModuleEntry(entry, ownerFile) {
  if (!entry || !isPlainRelativeRustPath(entry.ownerFile)) return false;
  if (!isPlainRelativeRustPath(entry.moduleFile) || entry.ownerFile !== ownerFile) return false;
  if (!/^[A-Za-z_][A-Za-z0-9_]*_private_tests$/u.test(entry.moduleName)) return false;
  return path.basename(entry.moduleFile) === `${entry.moduleName}.rs`
    && path.dirname(entry.ownerFile) === path.dirname(entry.moduleFile);
}

function isPlainRelativeRustPath(value) {
  return typeof value === "string"
    && value.length > 0
    && !/[\\*?\[\]{}]/u.test(value)
    && !value.startsWith("/")
    && !value.includes("../")
    && value.endsWith(".rs");
}

function normalizeRelativePath(root, target) {
  return path.relative(root, target).replaceAll("\\", "/");
}

export { isAllowedPrivateRustTestModule, isValidPrivateRustTestModuleEntry };
