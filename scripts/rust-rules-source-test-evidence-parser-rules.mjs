import { addViolation, escapeRegExp, firstLineMatching } from "./rust-rules-path-core.mjs";
import { hasParserTestEvidence } from "./rust-rules-source-test-evidence-unit-body-parser-proof.mjs";
import { isInsideRanges, testOnlyRanges } from "./rust-rules-source-test-evidence-ranges.mjs";

function parserRequiresErrorOutcome(masked, declarationIndex) {
  const openingBrace = masked.indexOf("{", declarationIndex);
  if (openingBrace < 0) return true;
  const signature = masked.slice(declarationIndex, openingBrace);
  return /->[^{};]*\b(?:Result|Option)\s*</u.test(signature);
}

/** Applies parser-test evidence rules to a Rust source scan context. */
export function applyParserEvidenceRules(context) {
  const { source, masked, originalLines, root, filePath, violations, isBoundary, isTestSource, evidenceContext } = context;
  if (isBoundary) return;
  const declarations = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+((?:try_new|parse[A-Za-z0-9_]*))\s*\(/gmu;
  const invalidEvidence = /(?:^|[^A-Za-z0-9])(?:invalid|reject[A-Za-z0-9_]*|malformed|bad_input|unparseable)(?:$|[^A-Za-z0-9])/iu;
  const edgeEvidence = /(?:^|[^A-Za-z0-9])(?:invalid|empty|oversized|malformed|unparseable)(?:$|[^A-Za-z0-9])/iu;
  const excludedRanges = testOnlyRanges(masked);
  for (const match of masked.matchAll(declarations)) {
    if (isTestSource || isInsideRanges(match.index, excludedRanges)) continue;
    const targetName = match[1];
    const targetLine = new RegExp(`^\\s*(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+${escapeRegExp(targetName)}\\s*\\(`, "u");
    const lineNo = firstLineMatching(originalLines, targetLine);
    if (!hasParserTestEvidence(
      root,
      filePath,
      source,
      targetName,
      invalidEvidence,
      evidenceContext,
      parserRequiresErrorOutcome(masked, match.index ?? 0),
    )) {
      addViolation(violations, root, filePath, lineNo, "RR-12.16", `validated constructor/parser ${targetName} lacks invalid-input test evidence.`, originalLines[lineNo - 1] ?? null);
    }
    if (targetName.startsWith("parse") && !hasParserTestEvidence(root, filePath, source, targetName, edgeEvidence, evidenceContext)) {
      addViolation(violations, root, filePath, lineNo, "RR-12.17", `parser ${targetName} lacks invalid/empty/oversized/malformed test evidence.`, originalLines[lineNo - 1] ?? null);
    }
  }
}
