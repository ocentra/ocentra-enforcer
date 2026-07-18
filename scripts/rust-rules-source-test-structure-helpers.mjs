function balancedEndAt(source, openingBrace) {
  let depth = 0;
  for (let index = openingBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
  }
  return source.length;
}

/** Collects Rust test-function declarations from masked source text. */
export function collectTestFunctions(masked) {
  const functions = [];
  for (const attribute of masked.matchAll(/^\s*#\s*\[\s*test\s*\]\s*$/gmu)) {
    const functionStart = masked.indexOf("fn ", attribute.index + attribute[0].length);
    if (functionStart < 0) continue;
    const nextTest = masked.indexOf("#[test]", attribute.index + attribute[0].length);
    if (nextTest >= 0 && nextTest < functionStart) continue;
    const openingBrace = masked.indexOf("{", functionStart);
    if (openingBrace < 0) continue;
    const end = balancedEndAt(masked, openingBrace);
    functions.push({
      start: attribute.index,
      bodyStart: openingBrace + 1,
      bodyEnd: Math.max(openingBrace + 1, end - 1),
    });
  }
  return functions;
}
