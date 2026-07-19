import {
  addViolation,
  escapeRegExp,
  lineNumberAtIndex,
} from "./rust-rules-path-core.mjs";
import { hasRoundTripEvidence } from "./rust-rules-source-roundtrip-evidence.mjs";

function applyTransportStructRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isTestSource,
}) {
  const declarations = source.matchAll(
    /(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/gmu,
  );
  for (const declaration of declarations) {
    const name = declaration.groups?.name ?? "";
    const attrs = declaration.groups?.attrs ?? "";
    const lineNo = lineNumberAtIndex(source, declaration.index ?? 0);
    if (!isBoundary) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.20",
        `DTO struct ${name} is outside a boundary/serde/transport module.`,
        originalLines[lineNo - 1] ?? null,
      );
    }

    const escapedName = escapeRegExp(name);
    const conversionPattern =
      `\\bimpl(?:\\s*<[^>{}]+>)?\\s+(?:core::convert::|std::convert::)?(?:TryFrom|From)\\s*<\\s*&?(?:'[_A-Za-z][_A-Za-z0-9]*\\s+)?${escapedName}\\b` +
      `|\\bfn\\s+(?:map_to_domain|into_domain|to_domain)\\b[^({;]*\\([^)]*:\\s*&?(?:'[_A-Za-z][_A-Za-z0-9]*\\s+)?${escapedName}\\b` +
      `|\\bimpl(?:\\s*<[^>{}]+>)?\\s+${escapedName}\\b[\\s\\S]*?\\bfn\\s+(?:into_domain|to_domain)\\b`;
    const hasConversion = new RegExp(conversionPattern, "u").test(source);
    const domainName = name.replace(
      /(?:Dto|DTO|Request|Response|Envelope)$/u,
      "",
    );
    const escapedDomainName = escapeRegExp(domainName);
    const counterpartPattern =
      `(?:^|\\n)\\s*(?:pub(?:\\([^)]*\\))?\\s+)?(?:struct|enum|type|trait)\\s+${escapedDomainName}\\b` +
      `|(?:^|\\n)\\s*use\\s+[^;{]*(?:::)${escapedDomainName}\\s*;` +
      `|(?:^|\\n)\\s*use\\s+[^;]*\\{[^}]*\\b${escapedDomainName}\\b[^}]*\\}\\s*;`;
    const hasCounterpart =
      domainName !== name && new RegExp(counterpartPattern, "u").test(source);
    if (hasCounterpart && !hasConversion) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.23",
        `DTO struct ${name} lacks explicit domain conversion.`,
        originalLines[lineNo - 1] ?? null,
      );
    }

    const manuallyDeserialized = new RegExp(
      `impl(?:<[^>]+>)?\\s+(?:serde::)?Deserialize(?:<[^>]+>)?\\s+for\\s+${escapedName}\\b`,
      "u",
    ).test(source);
    const acceptsExternalInput =
      /\bDeserialize\b/u.test(attrs) || manuallyDeserialized;
    if (
      !isTestSource &&
      acceptsExternalInput &&
      !hasRoundTripEvidence(root, filePath, source, name)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.25",
        `DTO struct ${name} lacks round-trip test evidence.`,
        originalLines[lineNo - 1] ?? null,
      );
    }
  }
}

function applyPublicStructRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isConfigurationBoundary,
}) {
  const declarations = source.matchAll(
    /^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu,
  );
  for (const declaration of declarations) {
    const name = declaration.groups?.name ?? "";
    const lineNo = lineNumberAtIndex(source, declaration.index ?? 0);
    const serdeAttributes = source.slice(
      Math.max(0, (declaration.index ?? 0) - 200),
      declaration.index,
    );
    const strictAttributes = source.slice(
      Math.max(0, (declaration.index ?? 0) - 260),
      declaration.index,
    );
    const serdeStruct = /\b(?:Serialize|Deserialize)\b/u.test(serdeAttributes);
    const transportName =
      /(?:Dto|DTO|Request|Response|Envelope)$/u.test(name);
    if (
      isBoundary &&
      !isConfigurationBoundary &&
      serdeStruct &&
      !transportName
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.21",
        `boundary serde struct ${name} lacks DTO/request/response suffix.`,
        originalLines[lineNo - 1] ?? null,
      );
    }
    const strictInput = /\b(?:Config|Input|Options|Settings)\b/u.test(name);
    if (
      strictInput &&
      /\bDeserialize\b/u.test(serdeAttributes) &&
      !/deny_unknown_fields/u.test(strictAttributes)
    ) {
      addViolation(
        violations,
        root,
        filePath,
        lineNo,
        "RR-14.26",
        `strict config/input ${name} lacks deny_unknown_fields.`,
        originalLines[lineNo - 1] ?? null,
      );
    }
  }
}

function applyPublicSerdeEnumRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isConfigurationBoundary,
}) {
  const declarations = source.matchAll(
    /(?<attrs>(?:^\s*#\[[^\]]+\]\s*\r?\n)*)^\s*pub\s+enum\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu,
  );
  for (const declaration of declarations) {
    const attrs = declaration.groups?.attrs ?? "";
    const serialized = /\b(?:Serialize|Deserialize)\b/u.test(attrs);
    const tagged = /\bserde\s*\(\s*tag\s*=/u.test(attrs);
    const justified = /SERDE-TAG-JUSTIFICATION:/u.test(attrs);
    if (isConfigurationBoundary || !serialized || tagged || justified) {
      continue;
    }
    const lineNo = lineNumberAtIndex(source, declaration.index ?? 0);
    addViolation(
      violations,
      root,
      filePath,
      lineNo,
      "RR-14.24",
      `public serde enum ${declaration.groups?.name ?? "enum"} lacks tag or justification.`,
      originalLines[lineNo - 1] ?? null,
    );
  }
}

/** Applies transport DTO, public boundary shape, and serde enum rules. */
export function applyBoundaryTransportRules(context) {
  applyTransportStructRules(context);
  applyPublicStructRules(context);
  applyPublicSerdeEnumRules(context);
}
