import { addViolation, firstLineMatching } from "./rust-rules-path-core.mjs";
import { applyParserEvidenceRules } from "./rust-rules-source-test-evidence-parser-rules.mjs";
import { hasExternalNegativeConversionEvidence } from "./rust-rules-source-test-evidence-queries.mjs";

function applyConversionEvidence({ source, originalLines, root, filePath, violations }) {
  const conversions = source.matchAll(
    /\b(?:TryFrom|From)\s*<\s*(?<name>[A-Z][A-Za-z0-9_]*(?:Dto|Request|Response|Envelope))\b[^>]*>/gu,
  );
  for (const conversion of conversions) {
    const dtoName = conversion.groups?.name ?? "DTO";
    const localEvidence = /\b(?:negative|invalid|reject)\b/iu.test(source);
    if (localEvidence) continue;
    if (hasExternalNegativeConversionEvidence(root, filePath, source, dtoName)) continue;
    const lineNo = firstLineMatching(originalLines, /\b(?:TryFrom|From)\s*</u);
    addViolation(
      violations,
      root,
      filePath,
      lineNo,
      "RR-12.18",
      `DTO conversion for ${dtoName} lacks negative test evidence.`,
      originalLines[lineNo - 1] ?? null,
    );
  }
}

function applyBugfixEvidence({ source, originalLines, root, filePath, violations }) {
  if (!/\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u.test(source)) return;
  if (/\bREGRESSION-TEST:/u.test(source)) return;
  const lineNo = firstLineMatching(
    originalLines,
    /\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u,
  );
  addViolation(
    violations,
    root,
    filePath,
    lineNo,
    "RR-12.19",
    "bugfix marker lacks REGRESSION-TEST evidence.",
    originalLines[lineNo - 1] ?? null,
  );
}

function applyConcurrencyEvidence({ source, masked, originalLines, root, filePath, violations }) {
  const hasAsyncLoop =
    /\basync\s+fn\b|\.await\b/u.test(masked) && /^\s*loop\s*\{/mu.test(masked);
  const hasConcurrency =
    /\b(?:tokio::spawn|unbounded_channel)\b|\b(?:tokio::)?select!/u.test(masked) ||
    hasAsyncLoop;
  if (!hasConcurrency) return;
  if (/\b(?:shutdown|cancellation|CANCELLATION-TEST:|SHUTDOWN-TEST:)\b/iu.test(source)) {
    return;
  }
  const lineNo = firstLineMatching(
    originalLines,
    /\b(?:tokio::spawn|unbounded_channel)\b|\b(?:tokio::)?select!|^\s*loop\s*\{/mu,
  );
  addViolation(
    violations,
    root,
    filePath,
    lineNo,
    "RR-12.29",
    "concurrency code lacks cancellation/shutdown test evidence.",
    originalLines[lineNo - 1] ?? null,
  );
}

/** Applies executable proof-evidence rules to one Rust source file. */
export function applyProofEvidenceRules(context) {
  applyParserEvidenceRules(context);
  applyConversionEvidence(context);
  applyBugfixEvidence(context);
  applyConcurrencyEvidence(context);
}
