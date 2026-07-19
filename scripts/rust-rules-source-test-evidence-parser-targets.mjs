/** Returns validated constructor and parser definitions in masked Rust source. */
export function parserDefinitions(masked) {
  return [...masked.matchAll(/^\s*(?:pub\s+)?(?:async\s+)?fn\s+(?<name>try_new|parse[A-Za-z0-9_]*)\s*\(/gmu)];
}

function parserBody(masked, startIndex) {
  const tail = masked.slice(startIndex);
  const nextDefinition = /\n\s*(?:pub\s+)?(?:async\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\(/u.exec(tail.slice(1));
  return nextDefinition ? tail.slice(0, nextDefinition.index + 1) : tail;
}

/** Decides whether a parser consumes binary or network-shaped input. */
export function parserTargetRequiresFuzzEvidence(masked, parserTarget) {
  const name = parserTarget.groups?.name ?? "";
  const identifierTokens = name
    .replace(/([a-z0-9])([A-Z])/gu, "$1_$2")
    .toLowerCase()
    .split(/_+/u);
  if (identifierTokens.some((token) => ["binary", "packet", "frame", "network"].includes(token))) {
    return true;
  }
  const body = parserBody(masked, parserTarget.index ?? 0);
  const openingBrace = body.indexOf("{");
  const signature = openingBrace >= 0 ? body.slice(0, openingBrace) : body;
  if (/(?:&\s*)?\[\s*u8\s*\]|\bVec\s*<\s*u8\s*>|\bBytes\b/u.test(signature)) return true;
  return /\b(?:binary|packet|frame|network)\b/iu.test(body);
}
