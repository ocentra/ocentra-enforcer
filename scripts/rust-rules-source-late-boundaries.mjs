import { addViolation, lineNumberAtIndex } from "./rust-rules-path-core.mjs";
import {
  hasDomainConversionFor,
  hasSeparateDomainCounterpart,
} from "./rust-rules-source-late-boundary-helpers.mjs";
import { hasRoundTripTestEvidence } from "./rust-rules-source-test-evidence-unit-body-roundtrip-evidence.mjs";

/** Applies transport-boundary rules to a Rust source scan context. */
export function applyBoundaryTransportRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isConfigurationBoundary,
  evidenceContext,
}) {
  for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/gmu)) {
    const name = match.groups?.name ?? "";
    const attrs = match.groups?.attrs ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    const explicitDtoName = /(?:Dto|DTO)$/u.test(name);
    const hasSerdeContract = /\b(?:Serialize|Deserialize)\b/u.test(attrs);
    if (!explicitDtoName && !isBoundary && !hasSerdeContract) {
      continue;
    }
    if (!isBoundary) {
      addViolation(violations, root, filePath, lineNo, "RR-14.20", `DTO struct ${name} is outside a boundary/serde/transport module.`, originalLines[lineNo - 1] ?? null);
    }
    if (hasSeparateDomainCounterpart(source, name) && !hasDomainConversionFor(source, name)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.23", `DTO struct ${name} lacks explicit domain conversion.`, originalLines[lineNo - 1] ?? null);
    }
    const manuallyDeserialized = new RegExp(
      `impl(?:<[^>]+>)?\\s+(?:serde::)?Deserialize(?:<[^>]+>)?\\s+for\\s+${name}\\b`,
      "u",
    ).test(source);
    const acceptsExternalInput = /\bDeserialize\b/u.test(attrs) || manuallyDeserialized;
    if (
      acceptsExternalInput
      && !hasRoundTripTestEvidence(
        root,
        filePath,
        source,
        name,
        evidenceContext,
      )
    ) {
      addViolation(violations, root, filePath, lineNo, "RR-14.25", `DTO struct ${name} lacks round-trip test evidence.`, originalLines[lineNo - 1] ?? null);
    }
  }
  for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu)) {
    const name = match.groups?.name ?? "";
    const attrs = match.groups?.attrs ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    if (isBoundary && !isConfigurationBoundary && /\b(?:Serialize|Deserialize)\b/u.test(attrs) && !/(?:Dto|DTO|Wire|Request|Response|Envelope)$/u.test(name)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.21", `boundary serde struct ${name} lacks DTO/wire/request/response suffix.`, originalLines[lineNo - 1] ?? null);
    }
    if (/\b(?:Config|Input|Options|Settings)\b/u.test(name) && /\bDeserialize\b/u.test(source.slice(Math.max(0, match.index - 200), match.index)) && !/deny_unknown_fields/u.test(source.slice(Math.max(0, match.index - 260), match.index))) {
      addViolation(violations, root, filePath, lineNo, "RR-14.26", `strict config/input ${name} lacks deny_unknown_fields.`, originalLines[lineNo - 1] ?? null);
    }
  }
  for (const match of source.matchAll(/(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+enum\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu)) {
    const attrs = match.groups?.attrs ?? "";
    if (!isConfigurationBoundary && /\b(?:Serialize|Deserialize)\b/u.test(attrs) && !/\bserde\s*\(\s*tag\s*=/u.test(attrs) && !/SERDE-TAG-JUSTIFICATION:/u.test(attrs)) {
      const lineNo = lineNumberAtIndex(source, match.index ?? 0);
      addViolation(violations, root, filePath, lineNo, "RR-14.24", `public serde enum ${match.groups?.name ?? "enum"} lacks tag or justification.`, originalLines[lineNo - 1] ?? null);
    }
  }
}
