import { addViolation, lineNumberAtIndex } from "./rust-rules-path-core.mjs";

export function applyBoundaryTransportRules({
  source,
  originalLines,
  root,
  filePath,
  violations,
  isBoundary,
  isConfigurationBoundary,
}) {
  for (const match of source.matchAll(/^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/gmu)) {
    const name = match.groups?.name ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    if (!isBoundary) {
      addViolation(violations, root, filePath, lineNo, "RR-14.20", `DTO struct ${name} is outside a boundary/serde/transport module.`, originalLines[lineNo - 1] ?? null);
    }
    const escapedName = name.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
    const hasConversion = new RegExp(`\\bimpl(?:\\s*<[^>{}]+>)?\\s+(?:core::convert::|std::convert::)?(?:TryFrom|From)\\s*<\\s*&?(?:'[_A-Za-z][_A-Za-z0-9]*\\s+)?${escapedName}\\b|\\bfn\\s+(?:map_to_domain|into_domain|to_domain)\\b[^({;]*\\([^)]*:\\s*&?(?:'[_A-Za-z][_A-Za-z0-9]*\\s+)?${escapedName}\\b|\\bimpl(?:\\s*<[^>{}]+>)?\\s+${escapedName}\\b[\\s\\S]*?\\bfn\\s+(?:into_domain|to_domain)\\b`, "u").test(source);
    const domainName = name.replace(/(?:Dto|DTO|Request|Response|Envelope)$/u, "");
    const hasCounterpart = domainName !== name && new RegExp(`(?:^|\\n)\\s*(?:pub(?:\\([^)]*\\))?\\s+)?(?:struct|enum|type|trait)\\s+${domainName}\\b|(?:^|\\n)\\s*use\\s+[^;{]*(?:::)${domainName}\\s*;|(?:^|\\n)\\s*use\\s+[^;]*\\{[^}]*\\b${domainName}\\b[^}]*\\}\\s*;`, "u").test(source);
    if (hasCounterpart && !hasConversion) {
      addViolation(violations, root, filePath, lineNo, "RR-14.23", `DTO struct ${name} lacks explicit domain conversion.`, originalLines[lineNo - 1] ?? null);
    }
    if (!/\b(?:round[-_ ]?trip|ROUNDTRIP-TEST:)\b/iu.test(source)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.25", `DTO struct ${name} lacks round-trip test evidence.`, originalLines[lineNo - 1] ?? null);
    }
  }
  for (const match of source.matchAll(/^\s*pub\s+struct\s+(?<name>[A-Z][A-Za-z0-9_]*)\b/gmu)) {
    const name = match.groups?.name ?? "";
    const lineNo = lineNumberAtIndex(source, match.index ?? 0);
    if (isBoundary && !isConfigurationBoundary && /\b(?:Serialize|Deserialize)\b/u.test(source.slice(Math.max(0, match.index - 200), match.index)) && !/(?:Dto|DTO|Request|Response|Envelope)$/u.test(name)) {
      addViolation(violations, root, filePath, lineNo, "RR-14.21", `boundary serde struct ${name} lacks DTO/request/response suffix.`, originalLines[lineNo - 1] ?? null);
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
