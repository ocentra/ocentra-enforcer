import { addViolation, escapeRegExp, firstLineMatching } from "./rust-rules-path-core.mjs";
import { rustCommentText } from "./rust-rules-rust-comment-text.mjs";
import { applyParserEvidenceRules } from "./rust-rules-source-test-evidence-parser-rules.mjs";
import { crateEvidenceCacheRecord } from "./rust-rules-source-test-evidence-cache.mjs";
import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import { nearestCargoRoot } from "./rust-rules-source-test-evidence-paths.mjs";
import {
  hasPropertyEvidence,
  propertyTestBodies,
  registeredPropertyTargetBodies,
} from "./rust-rules-source-test-evidence-properties.mjs";
import { hasConversionRejectionEvidence } from "./rust-rules-source-test-evidence-unit-body-conversion-evidence.mjs";
import { rustTestBodies } from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import {
  roundTripDecoderDescriptors,
  roundTripFactoryDescriptors,
  roundTripValueProducerDescriptors,
} from "./rust-rules-source-test-evidence-roundtrip-associated-descriptors.mjs";
import { roundTripHelperDescriptors } from "./rust-rules-source-test-evidence-roundtrip-helper-validation.mjs";
import { roundTripPersistenceDescriptors } from "./rust-rules-source-test-evidence-persistence-descriptors.mjs";

function createEvidenceContext(root, filePath, source, crateEvidenceCache) {
  const cargoRoot = nearestCargoRoot(root, filePath);
  const record = crateEvidenceCacheRecord(crateEvidenceCache, cargoRoot);
  if (record?.context) return record.context;
  const sources = crateEvidenceSources(root, filePath, source, crateEvidenceCache);
  const context = {
    sources,
    rustTestBodies: sources.map(rustTestBodies),
    roundTripHelpers: sources.flatMap(roundTripHelperDescriptors),
    roundTripFactories: sources.flatMap(roundTripFactoryDescriptors),
    roundTripDecoders: sources.flatMap(roundTripDecoderDescriptors),
    roundTripValueProducers: sources.flatMap(roundTripValueProducerDescriptors),
    roundTripPersistence: sources.flatMap(roundTripPersistenceDescriptors),
    propertyBodies: sources.map(propertyTestBodies),
    registeredBodies: sources.map(registeredPropertyTargetBodies),
  };
  if (record) record.context = context;
  return context;
}

function applyConversionAndRegressionEvidence(context) {
  const { source, originalLines, root, filePath, violations, evidenceContext } = context;
  const conversions = /\bTryFrom\s*<\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*(?<dto>[A-Z][A-Za-z0-9_]*(?:Dto|Request|Response|Envelope))\s*>\s+for\s+(?<domain>[A-Z][A-Za-z0-9_]*)/gu;
  for (const match of source.matchAll(conversions)) {
    const dtoName = match.groups?.dto ?? "";
    const domainName = match.groups?.domain ?? "";
    if (hasConversionRejectionEvidence(
      root,
      filePath,
      source,
      dtoName,
      domainName,
      evidenceContext,
    )) continue;
    const lineNo = firstLineMatching(originalLines, new RegExp(`\\bTryFrom\\s*<[^>]*${escapeRegExp(dtoName)}[^>]*>`, "u"));
    addViolation(violations, root, filePath, lineNo, "RR-12.18", `DTO conversion from ${dtoName} to ${domainName} lacks negative test evidence.`, originalLines[lineNo - 1] ?? null);
  }
  const comments = rustCommentText(source);
  if (/\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u.test(comments) && !/\bREGRESSION-TEST:/u.test(comments)) {
    const lineNo = firstLineMatching(comments.split(/\r?\n/u), /\b(?:BUGFIX|FIXES|bugfix|fixes)\b/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.19", "bugfix marker lacks REGRESSION-TEST evidence.", originalLines[lineNo - 1] ?? null);
  }
}

function applyPropertyEvidence(context) {
  const { source, masked, originalLines, root, filePath, violations, isTestSource, evidenceContext } = context;
  if (isTestSource) return;
  for (const match of masked.matchAll(/^\s*pub\s+(?:async\s+)?fn\s+((?:normalize|parse)[A-Za-z0-9_]*)\s*\(/gmu)) {
    const targetName = match[1];
    if (hasPropertyEvidence(root, filePath, source, targetName, evidenceContext)) continue;
    const lineNo = firstLineMatching(originalLines, new RegExp(`^\\s*pub\\s+(?:async\\s+)?fn\\s+${escapeRegExp(targetName)}\\s*\\(`, "u"));
    addViolation(violations, root, filePath, lineNo, "RR-12.27", `normalizer/parser ${targetName} lacks property-test evidence.`, originalLines[lineNo - 1] ?? null);
  }
}

function applyLifecycleEvidence(context) {
  const { source, masked, originalLines, root, filePath, violations } = context;
  if (/\b(?:binary|packet|frame|network)\b/iu.test(masked) && /^\s*pub\s+fn\s+parse[A-Za-z0-9_]*\s*\(/mu.test(masked) && !/\b(?:fuzz|cargo fuzz|FUZZ-TARGET:)/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /^\s*pub\s+fn\s+parse[A-Za-z0-9_]*\s*\(/u);
    addViolation(violations, root, filePath, lineNo, "RR-12.28", "binary/network parser lacks fuzz target evidence.", originalLines[lineNo - 1] ?? null);
  }
  const hasAsyncLoop = /\basync\s+fn\b|\.await\b/u.test(masked) && /^\s*loop\s*\{/mu.test(masked);
  if ((/\b(?:tokio::spawn|select!|unbounded_channel)\b/u.test(masked) || hasAsyncLoop)
      && !/\b(?:shutdown|cancellation|CANCELLATION-TEST:|SHUTDOWN-TEST:)\b/iu.test(source)) {
    const lineNo = firstLineMatching(originalLines, /\b(?:tokio::spawn|select!|unbounded_channel)\b|^\s*loop\s*\{/mu);
    addViolation(violations, root, filePath, lineNo, "RR-12.29", "concurrency code lacks cancellation/shutdown test evidence.", originalLines[lineNo - 1] ?? null);
  }
}

/** Applies proof-evidence rules to a Rust source scan context. */
export function applyProofEvidenceRules(context) {
  const evidenceContext = context.cacheProofEvidence === false
    ? null
    : createEvidenceContext(
        context.root,
        context.filePath,
        context.source,
        context.proofEvidenceCache,
      );
  const enrichedContext = { ...context, evidenceContext };
  applyParserEvidenceRules(enrichedContext);
  applyConversionAndRegressionEvidence(enrichedContext);
  applyPropertyEvidence(enrichedContext);
  applyLifecycleEvidence(enrichedContext);
}
