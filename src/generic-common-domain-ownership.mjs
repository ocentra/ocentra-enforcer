/** Collects declared raw boundary DTO type names from a source file. */
export function declaredRawBoundaryTypeNames(text) {
  const names = new Set();
  const declarationPattern = /\b(?:class|enum|interface|struct)\s+(?<name>Raw[A-Z]\w+|[A-Z]\w+(?:Dto|DTO|Payload|Body|Request))\b/gu;
  for (const match of text.matchAll(declarationPattern)) {
    if (match.groups?.name) names.add(match.groups.name);
  }
  // A type alias is not an additional owned wire shape. Preserve structural
  // aliases (for example `Payload = { ... }`) as raw declarations, while
  // ignoring aliases that merely rename an already-declared DTO.
  const aliasPattern = /\btype\s+(?<name>Raw[A-Z]\w+|[A-Z]\w+(?:Dto|DTO|Payload|Body|Request))\s*=\s*(?<rhs>[^;\n]+)/gu;
  for (const match of text.matchAll(aliasPattern)) {
    const rhs = match.groups?.rhs?.trim() ?? "";
    if (match.groups?.name && !/^(?:Raw[A-Z]\w*|[A-Z]\w+(?:Dto|DTO|Payload|Body|Request))$/u.test(rhs)) {
      names.add(match.groups.name);
    }
  }
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
