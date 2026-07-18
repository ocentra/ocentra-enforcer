/** Collects declared raw boundary DTO type names from a source file. */
export function declaredRawBoundaryTypeNames(text) {
  const names = new Set();
  const pattern = /\b(?:class|enum|interface|struct|type)\s+(?<name>Raw[A-Z]\w+|[A-Z]\w+(?:Dto|DTO|Payload|Body|Request))\b/gu;
  for (const match of text.matchAll(pattern)) if (match.groups?.name) names.add(match.groups.name);
  return names;
}

/** Determines whether Rust DTO declarations form a cohesive owned family. */
export function hasCohesiveRustDtoFamily(rel, text, rawTypeNames) {
  if (!rel.endsWith(".rs") || rawTypeNames.size === 0
      || !/#\[derive\([^\]]*(?:Serialize|Deserialize)/u.test(text)) return false;
  const boundaryPurpose = /(?:^|\n)\s*\/\/[!/]?[\s\S]{0,800}\b(?:wire|serialized|persistence|transport|provider response|bundle)\b/iu.test(text);
  if (!boundaryPurpose) return false;
  const everyTypeIsSerdeOwned = [...rawTypeNames].every((typeName) => new RegExp(
    `#\\[derive\\([^\\]]*(?:Serialize|Deserialize)[^\\]]*\\)\\][\\s\\S]{0,240}\\b(?:pub(?:\\([^)]*\\))?\\s+)?(?:struct|enum)\\s+${typeName.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")}\\b`,
    "u",
  ).test(text));
  if (!everyTypeIsSerdeOwned) return false;
  const everyTypeConvertsFallibly = [...rawTypeNames].every((typeName) => new RegExp(
    `\\bimpl(?:\\s*<[^>{}]+>)?\\s+(?:core::convert::|std::convert::)?TryFrom\\s*<\\s*${typeName.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&")}\\s*>`,
    "u",
  ).test(text));
  return everyTypeConvertsFallibly || /(?:^|\n)\s*\/\/\s*ROUNDTRIP-TEST:\s*\S+/u.test(text);
}

/** Joins a source file's lines for decision-code policy checks. */
export function decisionCodeText(lines) {
  return lines.filter((line) => !/^\s*(?:\/\/|#(?!\[)|\/\*|\*)/u.test(line)).join("\n");
}
