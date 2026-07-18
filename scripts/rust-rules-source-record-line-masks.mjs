import { braceDelta } from "./rust-rules-source-helpers.mjs";

function recordLineMask(lines, declarationPattern) {
  const fieldLines = Array(lines.length).fill(false);
  let pendingRecord = false;
  let recordDepth = null;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (recordDepth !== null) {
      fieldLines[index] = true;
      recordDepth += braceDelta(line);
      if (recordDepth <= 0) recordDepth = null;
      continue;
    }
    if (declarationPattern.test(line)) pendingRecord = true;
    if (!pendingRecord) continue;
    fieldLines[index] = true;
    const depth = braceDelta(line);
    if (depth > 0) recordDepth = depth;
    if (depth > 0 || /;\s*$/u.test(line)) pendingRecord = false;
  }
  return fieldLines;
}

/** Builds a line mask for record field declarations. */
export function recordFieldLineMask(lines) {
  return recordLineMask(lines, /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s+[A-Z][A-Za-z0-9_]*/u);
}

/** Builds a line mask for public record declarations. */
export function publicRecordLineMask(lines) {
  return recordLineMask(lines, /^\s*pub(?:\([^)]*\))?\s+(?:struct|enum)\s+[A-Z][A-Za-z0-9_]*/u);
}
