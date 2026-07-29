import { braceDelta } from "./rust-rules-source-helpers.mjs";

/** Builds a line mask for inline Rust test blocks. */
export function inlineTestLineMask(lines) {
  const testLines = Array(lines.length).fill(false);
  let pendingTestFunction = false;
  let pendingTestModule = false;
  let activeTestDepth = null;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (activeTestDepth !== null) {
      testLines[index] = true;
      activeTestDepth += braceDelta(line);
      if (activeTestDepth <= 0) activeTestDepth = null;
      continue;
    }
    if (/^\s*#\[cfg\(test\)\]/u.test(line)) {
      testLines[index] = true;
      pendingTestModule = true;
      continue;
    }
    if (/^\s*#\[(?:test|tokio::test|async_std::test)\b/u.test(line)) {
      testLines[index] = true;
      pendingTestFunction = true;
      continue;
    }
    if (pendingTestModule) {
      testLines[index] = true;
      const depth = /\bmod\s+[A-Za-z_][A-Za-z0-9_]*\b/u.test(line) ? braceDelta(line) : 0;
      if (depth > 0) activeTestDepth = depth;
      pendingTestModule = depth <= 0;
      continue;
    }
    if (pendingTestFunction) {
      testLines[index] = true;
      const depth = /\bfn\s+[A-Za-z_][A-Za-z0-9_]*\b/u.test(line) ? braceDelta(line) : 0;
      if (depth > 0) activeTestDepth = depth;
      pendingTestFunction = depth <= 0;
    }
  }
  return testLines;
}
