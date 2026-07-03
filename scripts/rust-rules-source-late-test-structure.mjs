import { addViolation, firstLineMatching, lineNumberAtIndex } from "./rust-rules-path-core.mjs";

export function applyTestStructureRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isTestSource,
}) {
  if (!isTestSource) return;
  if (/#\s*\[\s*should_panic/u.test(source) && !/\bPANIC-CONTRACT:/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /#\s*\[\s*should_panic/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.20", "#[should_panic] lacks PANIC-CONTRACT evidence.", originalLines[lineNo - 1] ?? null);
  }
  for (const match of source.matchAll(/#\s*\[\s*test\s*\][\s\S]*?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{\s*\}/gu)) {
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    addViolation(violations, root, filePath, lineNo, "RR-12.24", "empty test body found.", originalLines[lineNo - 1] ?? null);
  }
  for (const match of source.matchAll(/#\s*\[\s*test\s*\][\s\S]*?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*\{(?<body>[\s\S]*?)^\s*\}/gmu)) {
    const body = match.groups?.body ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    if (/\b::(?:new|try_new|parse)\s*\(/u.test(body) && !/\bassert(?:_eq|_ne)?!\s*\(|\bmatches!\s*\(/u.test(body)) {
      addViolation(violations, root, filePath, lineNo, "RR-12.25", "construction-only test lacks behavioral assertion.", originalLines[lineNo - 1] ?? null);
    }
    if (/\b(?:toMatchSnapshot|insta::assert|snapshot)\b/iu.test(body) && /\b(?:\d{4}-\d{2}-\d{2}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}|random|uuid)\b/iu.test(body) && !/\bREDACT|redact/u.test(body)) {
      addViolation(violations, root, filePath, lineNo, "RR-12.26", "snapshot test includes volatile value without redaction.", originalLines[lineNo - 1] ?? null);
    }
  }
}
