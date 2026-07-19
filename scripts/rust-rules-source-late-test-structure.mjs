import { addViolation, firstLineMatching, lineNumberAtIndex } from "./rust-rules-path-core.mjs";
import { collectTestFunctions } from "./rust-rules-source-test-structure-helpers.mjs";

export function applyTestStructureRules({
  source,
  masked,
  originalLines,
  root,
  filePath,
  violations,
  isTestSource,
}) {
  if (!isTestSource && !/^\s*#\s*\[\s*test\s*\]\s*$/mu.test(masked ?? source)) return;
  const maskedLines = (masked ?? source).split(/\r?\n/u);
  const shouldPanicIndex = maskedLines.findIndex((line) => /#\s*\[\s*should_panic/u.test(line));
  if (shouldPanicIndex >= 0 && !/\bPANIC-CONTRACT:/u.test(source)) {
    const lineNo = shouldPanicIndex + 1;
    addViolation(violations, root, filePath, lineNo, "RR-12.20", "#[should_panic] lacks PANIC-CONTRACT evidence.", originalLines[lineNo - 1] ?? null);
  }
  for (const testFunction of collectTestFunctions(source, masked ?? source)) {
    const body = (masked ?? source).slice(testFunction.bodyStart, testFunction.bodyEnd);
    const originalBody = source.slice(testFunction.bodyStart, testFunction.bodyEnd);
    const lineNo = lineNumberAtIndex(source, testFunction.start);
    if (body.trim() === "") {
      addViolation(violations, root, filePath, lineNo, "RR-12.24", "empty test body found.", originalLines[lineNo - 1] ?? null);
      continue;
    }
    const hasBehavioralAssertion =
      /\b(?:assert[A-Za-z0-9_]*|compare[A-Za-z0-9_]*|run_fixture_parity|run_manifest_fixture_parity|prop_assert[A-Za-z0-9_]*)\s*(?:!\s*)?\(/u.test(body) ||
      /\bmatches!\s*\(/u.test(body);
    if (/\b::(?:new|try_new|parse)\s*\(/u.test(body) && !hasBehavioralAssertion) {
      addViolation(violations, root, filePath, lineNo, "RR-12.25", "construction-only test lacks behavioral assertion.", originalLines[lineNo - 1] ?? null);
    }
    if (/\b(?:toMatchSnapshot|insta::assert(?:_[A-Za-z0-9_]+)?|assert_snapshot)\b/iu.test(body) && /\b(?:\d{4}-\d{2}-\d{2}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}|random|uuid)\b/iu.test(originalBody) && !/\bREDACT|redact/u.test(originalBody)) {
      addViolation(violations, root, filePath, lineNo, "RR-12.26", "snapshot test includes volatile value without redaction.", originalLines[lineNo - 1] ?? null);
    }
  }
}
