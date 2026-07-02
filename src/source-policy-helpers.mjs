import fs from "node:fs";

const defaultAllowedSensitivePathPatterns = [
  /(^|\/)\.env\.example$/iu,
  /(^|\/)\.env\.sample$/iu,
  /(^|\/)\.env\.template$/iu,
];

const defaultForbiddenSensitivePathPatterns = [
  /(^|\/)\.env(\..+)?$/iu,
  /(^|\/)google-services\.json$/iu,
  /(^|\/)GoogleService-Info\.plist$/u,
  /(^|\/)id_rsa(\.pub)?$/iu,
  /\.(pem|p12|pfx|key)$/iu,
];

function readLines(filePath) {
  return fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
}

function hasNearbyWindowsGuard(lines, lineIndex) {
  const start = Math.max(0, lineIndex - 8);
  const nearby = lines.slice(start, lineIndex + 1).join("\n");
  return /process\.platform\s*={2,3}\s*['"]win32['"]|process\.platform\s*!={1,2}\s*['"]win32['"]/u.test(nearby);
}

function hasNearbyTimerJustification(lines, lineIndex) {
  const start = Math.max(0, lineIndex - 4);
  const end = Math.min(lines.length, lineIndex + 2);
  const nearby = lines.slice(start, end).join("\n");
  return /\b(?:TIMER|HARNESS-TIMER)-JUSTIFICATION:/u.test(nearby);
}

function isForbiddenSensitivePath(
  rel,
  allowedSensitivePathPatterns = defaultAllowedSensitivePathPatterns,
  forbiddenSensitivePathPatterns = defaultForbiddenSensitivePathPatterns,
) {
  const allowedPatterns = Array.isArray(allowedSensitivePathPatterns) ? allowedSensitivePathPatterns : defaultAllowedSensitivePathPatterns;
  const forbiddenPatterns = Array.isArray(forbiddenSensitivePathPatterns) ? forbiddenSensitivePathPatterns : defaultForbiddenSensitivePathPatterns;
  if (allowedPatterns.some((pattern) => pattern.test(rel))) return false;
  return forbiddenPatterns.some((pattern) => pattern.test(rel));
}

function isSourceLikeForTestDoubles(rel) {
  if (!/\.(?:ts|tsx|js|jsx|mjs|cjs|mts|cts|py|rs)$/iu.test(rel)) return false;
  return !(
    /^(?:scripts|mcp|eslint-rules|adapters|schemas)\//u.test(rel) ||
    /^src\/(?:check(?:-|s\b)|codex-install|coordination\/|documentation-hints|generic-|harness|path-utils|policy|proof(?:-|\.mjs$)|routing|rule-|source-policy-(?:helpers|scanners))/.test(rel)
  );
}

function isGeneratedSourcePath(rel) {
  return /(?:^|\/)generated(?:\/|$)/iu.test(rel);
}

export {
  hasNearbyTimerJustification,
  hasNearbyWindowsGuard,
  isForbiddenSensitivePath,
  isGeneratedSourcePath,
  isSourceLikeForTestDoubles,
  readLines,
};
