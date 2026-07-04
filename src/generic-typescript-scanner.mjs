import fs from "node:fs";
import { normalizeRel } from "./path-utils.mjs";
import { scanAdditionalTypeScriptFile } from "./source-policy-scanners.mjs";
import { maskJavaScriptLines } from "./source-policy-text.mjs";
import { addViolation, isTestPath, jsStyleCommentText } from "./generic-scanner-shared.mjs";

function addBarrelViolations(violations, root, filePath, lineNo, line) {
  addViolation(
    violations,
    root,
    filePath,
    lineNo,
    "TS-1.1",
    "Barrel-style re-export found.",
    line,
  );
  addViolation(
    violations,
    root,
    filePath,
    lineNo,
    "TS-6.14",
    "Index barrel re-export found.",
    line,
  );
}

function addSuppressionViolation(violations, root, filePath, lineNo, line) {
  addViolation(
    violations,
    root,
    filePath,
    lineNo,
    "TS-2.1",
    "TypeScript/JavaScript validation suppression found.",
    line,
  );
}

function addTestViolations(
  violations,
  root,
  filePath,
  rel,
  lineNo,
  originalLine,
  maskedLine,
) {
  if (!isTestPath(rel)) return;
  if (/\b(?:describe|it|test)\s*\.\s*(?:skip|only|todo)\s*\(/u.test(maskedLine)) {
    addViolation(
      violations,
      root,
      filePath,
      lineNo,
      "TS-3.1",
      "Skipped or focused test found.",
      originalLine,
    );
  }
  if (/\btest\s*\.\s*(?:fixme|skip|only)\s*\(/u.test(maskedLine)) {
    addViolation(
      violations,
      root,
      filePath,
      lineNo,
      "TS-3.1",
      "Playwright skipped or focused test found.",
      originalLine,
    );
  }
  if (
    /\bexpect\s*\(\s*(?:true|false|null|undefined)\s*\)\s*\.\s*(?:toBe|toEqual)\s*\(/u.test(
      maskedLine,
    )
  ) {
    addViolation(
      violations,
      root,
      filePath,
      lineNo,
      "TEST-1.2",
      "literal truth assertion is too weak.",
      originalLine,
    );
  }
}

export function scanTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const maskedLines = maskJavaScriptLines(lines);
  const violations = [];
  maskedLines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const originalLine = lines[idx] ?? line;
    const comment = jsStyleCommentText(originalLine);
    if (
      /^\s*export\s+(?:\*\s+from|\*\s+as\s+[A-Za-z_$][\w$]*\s+from|(?:type\s+)?\{[^}]*\}\s+from)/u.test(
        line,
      )
    ) {
      addBarrelViolations(violations, root, filePath, lineNo, originalLine);
    }
    if (
      /(?:\b(?:eslint-disable|biome-ignore|oxlint-disable|prettier-ignore)\b|@ts-(?:ignore|expect-error|nocheck)\b)/u.test(
        comment,
      )
    ) {
      addSuppressionViolation(violations, root, filePath, lineNo, originalLine);
    }
    addTestViolations(violations, root, filePath, rel, lineNo, originalLine, line);
  });
  violations.push(...scanAdditionalTypeScriptFile(root, filePath));
  return violations;
}
