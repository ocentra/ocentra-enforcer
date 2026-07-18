/** Finds the nearest enclosing opening brace before an index. */
export function nearestEnclosingBrace(masked, index) {
  let nestedDepth = 0;
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const ch = masked[cursor];
    if (ch === "}") {
      nestedDepth += 1;
      continue;
    }
    if (ch !== "{") continue;
    if (nestedDepth === 0) return cursor;
    nestedDepth -= 1;
  }
  return -1;
}

/** Returns the declaration header associated with an opening brace. */
export function blockHeader(masked, braceIndex) {
  if (braceIndex < 0) return "";
  const prefix = masked.slice(0, braceIndex);
  const candidates = [
    ...prefix.matchAll(
      /(?:^|\n)[\t ]*(?:(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait|(?:unsafe\s+)?impl)\b/gu,
    ),
  ];
  const start = candidates.at(-1)?.index;
  if (start === undefined) return "";
  const candidate = prefix.slice(start).trim();
  if (/[{}]/u.test(candidate)) return "";
  return candidate;
}
