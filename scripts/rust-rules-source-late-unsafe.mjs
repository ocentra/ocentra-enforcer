import { addViolation, escapeRegExp, lineNumberAtIndex } from "./rust-rules-path-core.mjs";

export function applyUnsafeEvidenceRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
}) {
  const unsafeLine = originalLines.findIndex((line) => /\bunsafe\b/u.test(line));
  if (unsafeLine >= 0 && !/\bMIRI-PROOF:/u.test(source)) {
    addViolation(violations, root, filePath, unsafeLine + 1, "RR-3.30", "unsafe source lacks MIRI-PROOF evidence.", originalLines[unsafeLine]);
    addViolation(violations, root, filePath, unsafeLine + 1, "RR-12.30", "unsafe module lacks MIRI-PROOF evidence.", originalLines[unsafeLine]);
  }
  if (unsafeLine >= 0 && !/\bGEIGER-PROOF:/u.test(source)) {
    addViolation(violations, root, filePath, unsafeLine + 1, "RR-3.31", "unsafe source lacks GEIGER-PROOF evidence.", originalLines[unsafeLine]);
  }
}

export function applyNewtypeConstructorRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
}) {
  for (const match of source.matchAll(/pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\s*\(\s*(?:pub\s+)?(?<inner>String|&\s*str|str|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|bool)[^)]*\)\s*;/gu)) {
    const typeName = match.groups?.name ?? "";
    if (!new RegExp(`impl\\s+${escapeRegExp(typeName)}[\\s\\S]*?\\b(?:try_new|parse)\\s*\\(`, "u").test(source)) {
      const lineNo = lineNumberAtIndex(source, match.index ?? 0);
      addViolation(violations, root, filePath, lineNo, "RR-6.44", `newtype ${typeName} lacks try_new or parse constructor.`, originalLines[lineNo - 1] ?? null);
    }
  }
}
