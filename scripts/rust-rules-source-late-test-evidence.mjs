import { addViolation, firstLineMatching } from "./rust-rules-path-core.mjs";

export function applyProofEvidenceRules({
  source,
  masked,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isTestSource,
}) {
  if (!isBoundary && /\b(?:try_new|parse)\s*\(/u.test(masked) && !/\b(?:invalid|reject|malformed|bad input)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:try_new|parse)\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.16", "validated constructor/parser lacks invalid-input test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (!isBoundary && /\bparse[A-Za-z0-9_]*\s*\(/u.test(masked) && !/\b(?:invalid|empty|oversized|malformed)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\bparse[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.17", "parser lacks invalid/empty/oversized/malformed test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:TryFrom|From)\s*<[^>]*(?:Dto|Request|Response|Envelope)[^>]*>/u.test(source) && !/(?:^|[^A-Za-z0-9])(?:negative|invalid|reject(?:s|ed|ion)?)(?:$|[^A-Za-z0-9])/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:TryFrom|From)\s*</u);
    addViolation(violations, root, filePath, lineNo, "RR-12.18", "DTO conversion lacks negative test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u.test(source) && !/\bREGRESSION-TEST:/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.19", "bugfix marker lacks REGRESSION-TEST evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (!isTestSource && /^\s*pub\s+fn\s+(?:normalize|parse)[A-Za-z0-9_]*\s*\(/mu.test(masked) && !/\b(?:proptest|quickcheck|PROPERTY-TEST:)/u.test(source)) {
    const lineNo = firstLineMatching(originalLines, /^\s*pub\s+fn\s+(?:normalize|parse)[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.27", "normalizer/parser lacks property-test evidence.", originalLines[lineNo - 1] ?? null);
  }
  if (/\b(?:binary|packet|frame|network)\b/iu.test(source) && /^\s*pub\s+fn\s+parse[A-Za-z0-9_]*\s*\(/mu.test(masked) && !/\b(?:fuzz|cargo fuzz|FUZZ-TARGET:)/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /^\s*pub\s+fn\s+parse[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.28", "binary/network parser lacks fuzz target evidence.", originalLines[lineNo - 1] ?? null);
  }
  // A finite async operation or a bounded request/reply channel has no owned
  // background lifecycle to cancel. Require evidence only for constructs that
  // can outlive their immediate caller and therefore need shutdown coverage.
  const hasAsyncLoop =
    /\basync\s+fn\b|\.await\b/u.test(masked) &&
    /^\s*loop\s*\{/mu.test(masked);
  if ((/\b(?:tokio::spawn|select!|unbounded_channel)\b/u.test(masked) || hasAsyncLoop) && !/\b(?:shutdown|cancellation|CANCELLATION-TEST:|SHUTDOWN-TEST:)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:tokio::spawn|select!|unbounded_channel)\b|^\s*loop\s*\{/mu);
    addViolation(violations, root, filePath, lineNo, "RR-12.29", "concurrency code lacks cancellation/shutdown test evidence.", originalLines[lineNo - 1] ?? null);
  }
}
