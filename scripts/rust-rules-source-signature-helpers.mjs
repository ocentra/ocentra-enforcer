function enclosingBlockOpeningBrace(masked, index) {
  let depth = 0;
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    if (masked[cursor] === "}") depth += 1;
    else if (masked[cursor] === "{") {
      if (depth === 0) return cursor;
      depth -= 1;
    }
  }
  return -1;
}

/** Identifies signatures owned by a trait implementation block. */
export function isTraitImplementationSignature(masked, index) {
  const openingBrace = enclosingBlockOpeningBrace(masked, index);
  if (openingBrace < 0) return false;
  const header = masked.slice(Math.max(0, openingBrace - 1200), openingBrace);
  return /\bimpl(?:\s*<[^{}]*>)?\s+[^{};]+\s+for\s+[^{};]+$/u.test(header);
}
