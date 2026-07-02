import fs from "node:fs";
import { normalizeRel } from "./path-utils.mjs";
import { scanAdditionalTypeScriptFile } from "./source-policy-scanners.mjs";
import { addViolation, isTestPath, jsStyleCommentText } from "./generic-scanner-shared.mjs";

export function scanTypeScriptFile(root, filePath) {
  const rel = normalizeRel(root, filePath);
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/u);
  const violations = [];
  lines.forEach((line, idx) => {
    const lineNo = idx + 1;
    const comment = jsStyleCommentText(line);
    if (
      /^\s*export\s+(?:\*\s+from|\*\s+as\s+[A-Za-z_$][\w$]*\s+from|(?:type\s+)?\{[^}]*\}\s+from)/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-1.1",
        "Barrel-style re-export found.",
        line,
      );
    }
    if (
      /(?:\b(?:eslint-disable|biome-ignore|oxlint-disable|prettier-ignore)\b|@ts-(?:ignore|expect-error|nocheck)\b)/u.test(
        comment,
      )
    ) {
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
    if (
      isTestPath(rel) &&
      /\b(?:describe|it|test)\s*\.\s*(?:skip|only|todo)\s*\(/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-3.1",
        "Skipped or focused test found.",
        line,
      );
    }
    if (
      isTestPath(rel) &&
      /\btest\s*\.\s*(?:fixme|skip|only)\s*\(/u.test(line)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TS-3.1",
        "Playwright skipped or focused test found.",
        line,
      );
    }
    if (
      isTestPath(rel) &&
      /\bexpect\s*\(\s*(?:true|false|null|undefined)\s*\)\s*\.\s*(?:toBe|toEqual)\s*\(/u.test(
        line,
      )
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "TEST-1.2",
        "literal truth assertion is too weak.",
        line,
      );
    }
  });
  violations.push(...scanAdditionalTypeScriptFile(root, filePath));
  return violations;
}
