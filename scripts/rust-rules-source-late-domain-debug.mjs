import {
  addViolation,
  escapeRegExp,
  lineNumberAtIndex,
} from "./rust-rules-path-core.mjs";
import { RAW_TYPE_PATTERNS } from "./rust-rules-source-patterns.mjs";

const { RAW_STRING_TYPE_RE } = RAW_TYPE_PATTERNS;

function hasIntentionalDebug(attrs, name, source) {
  return /\bDebug\b/u.test(attrs) ||
    new RegExp(`impl(?:<[^>]+>)?\\s+(?:std::fmt::|fmt::)?Debug\\s+for\\s+${escapeRegExp(name)}(?:<[^>]+>)?\\b`, "u").test(source);
}

function isOperationalTypeName(name) {
  return /(?:Validator|RegistryRow)$/u.test(name);
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

/** Applies base64 domain-value rules to a Rust source scan context. */
export function applyDomainBase64Rules({
  source: _source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
}) {
  if (isBoundary) return;
  const rawBase64Line = originalLines.findIndex((line) => {
    const trimmed = line.trimStart();
    return !trimmed.startsWith("//") && /\b[A-Za-z0-9_]*base64[A-Za-z0-9_]*\b/iu.test(line) && RAW_STRING_TYPE_RE.test(line);
  });
  if (rawBase64Line < 0) {
    return;
  }
  const lineNo = rawBase64Line + 1;
  addViolation(violations, root, filePath, lineNo, "RR-14.28", "domain source uses raw base64 string shape.", originalLines[lineNo - 1] ?? null);
}

/** Applies domain debug-formatting rules to a Rust source scan context. */
export function applyDomainDebugRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
}) {
  if (isBoundary) return;
  for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+(?:struct|enum)\s+(?<name>[A-Z][A-Za-z0-9_]*)(?:\b|[<{(])/gmu)) {
    const name = match.groups?.name ?? "";
    if (/(?:Secret|Token|Key|Credential|Password)/u.test(name)) continue;
    if (isOperationalTypeName(name)) continue;
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
