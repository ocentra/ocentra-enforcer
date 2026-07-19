import { addViolation } from "./rust-rules-path-core.mjs";
import {
  parserDefinitions,
  parserTargetRequiresFuzzEvidence,
} from "./rust-rules-source-test-evidence-parser-targets.mjs";
import {
  evidenceLineForTarget,
  hasPropertyEvidence,
  hasTargetEvidence,
} from "./rust-rules-source-test-evidence-queries.mjs";

function applyNegativeParserEvidence(context) {
  const { source, masked, originalLines, root, filePath, violations, isBoundary } = context;
  if (isBoundary) return;
  for (const definition of parserDefinitions(masked)) {
    const name = definition.groups?.name ?? "parser";
    const lineNo = evidenceLineForTarget(originalLines, name);
    if (!hasTargetEvidence(root, filePath, source, name, /\b(?:invalid|reject[A-Za-z0-9_]*|malformed|bad(?:\s+|_)input)\b/iu)) {
      addViolation(violations, root, filePath, lineNo, "RR-12.16", `validated constructor/parser ${name} lacks invalid-input test evidence.`, originalLines[lineNo - 1] ?? null);
    }
    if (name.startsWith("parse") && !hasTargetEvidence(root, filePath, source, name, /\b(?:invalid|empty|oversized|malformed)\b/iu)) {
      addViolation(violations, root, filePath, lineNo, "RR-12.17", `parser ${name} lacks invalid/empty/oversized/malformed test evidence.`, originalLines[lineNo - 1] ?? null);
    }
  }
}

function applyPropertyEvidence(context) {
  const { source, masked, originalLines, root, filePath, violations, isTestSource } = context;
  if (isTestSource) return;
  for (const propertyTarget of masked.matchAll(/^\s*pub\s+(?:async\s+)?fn\s+(?<name>(?:normalize|parse)[A-Za-z0-9_]*)\s*\(/gmu)) {
    const name = propertyTarget.groups?.name ?? "parser";
    if (hasPropertyEvidence(root, filePath, source, name)) continue;
    const lineNo = evidenceLineForTarget(originalLines, name);
    addViolation(violations, root, filePath, lineNo, "RR-12.27", `normalizer/parser ${name} lacks property-test evidence.`, originalLines[lineNo - 1] ?? null);
  }
}

function applyFuzzEvidence(context) {
  const { source, masked, originalLines, root, filePath, violations } = context;
  for (const parserTarget of masked.matchAll(/^\s*pub\s+(?:async\s+)?fn\s+(?<name>parse[A-Za-z0-9_]*)\s*\(/gmu)) {
    const name = parserTarget.groups?.name ?? "parser";
    if (!parserTargetRequiresFuzzEvidence(masked, parserTarget)) continue;
    if (hasTargetEvidence(root, filePath, source, name, /(?:\bfuzz(?:_|\b)|\bcargo fuzz\b|\bFUZZ-TARGET:)/iu)) continue;
    const lineNo = evidenceLineForTarget(originalLines, name);
    addViolation(violations, root, filePath, lineNo, "RR-12.28", `binary/network parser ${name} lacks fuzz target evidence.`, originalLines[lineNo - 1] ?? null);
  }
}

/** Applies constructor, parser, property, and fuzz evidence rules. */
export function applyParserEvidenceRules(context) {
  applyNegativeParserEvidence(context);
  applyPropertyEvidence(context);
  applyFuzzEvidence(context);
}
