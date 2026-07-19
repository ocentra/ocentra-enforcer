import {
  addViolation,
  escapeRegExp,
  firstLineMatching,
  lineNumberAtIndex,
} from "./rust-rules-path-core.mjs";
import { RAW_TYPE_PATTERNS } from "./rust-rules-source-patterns.mjs";

const { RAW_STRING_TYPE_RE } = RAW_TYPE_PATTERNS;

function hasIntentionalDebug(attrs, name, source) {
  return /\bDebug\b/u.test(attrs) ||
    new RegExp(`impl(?:\\s*<[^>{}]*>)?\\s+(?:std::fmt::|fmt::)?Debug\\s+for\\s+${escapeRegExp(name)}(?:\\s*<[^>{}]*>)?\\b`, "u").test(source);
}

function reportMissingDebug({
  source,
  match,
  name,
  originalLines,
  root,
  filePath,
  violations,
}) {
  const lineNo = lineNumberAtIndex(source, match.index ?? 0);
  addViolation(violations, root, filePath, lineNo, "RR-6.50", `public domain value object ${name} lacks intentional Debug implementation.`, originalLines[lineNo - 1] ?? null);
}

export function applyDomainBase64Rules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
}) {
  if (isBoundary || !/\b(?:base64|Base64)\b/u.test(source) || !RAW_STRING_TYPE_RE.test(source)) {
    return;
  }
  const lineNo = firstLineMatching(originalLines, /\b(?:base64|Base64)\b/u);
  addViolation(violations, root, filePath, lineNo, "RR-14.28", "domain source uses raw base64 string shape.", originalLines[lineNo - 1] ?? null);
}

export function applyDomainDebugRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isBenchmark,
}) {
  if (isBoundary || isBenchmark) return;
  for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+(?:struct|enum)\s+(?<name>[A-Z][A-Za-z0-9_]*)(?:\b|[<{(])/gmu)) {
    const name = match.groups?.name ?? "";
    if (/(?:Secret|Token|Key|Credential|Password)/u.test(name)) continue;
    const attrs = match.groups?.attrs ?? "";
    if (hasIntentionalDebug(attrs, name, source)) continue;
    reportMissingDebug({
      source,
      match,
      name,
      originalLines,
      root,
      filePath,
      violations,
    });
  }
}
