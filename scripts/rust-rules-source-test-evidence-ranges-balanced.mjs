/** Returns the balanced block body beginning at an opening brace. */
export function balancedBodyAt(source, openingBrace) {
  let depth = 0;
  for (let index = openingBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] !== "}") continue;
    depth -= 1;
    if (depth === 0) return source.slice(openingBrace, index + 1);
  }
  return "";
}

/** Returns a balanced parenthesized expression beginning at an opening parenthesis. */
export function balancedParenthesizedAt(source, openingParenthesis) {
  let depth = 0;
  for (let index = openingParenthesis; index < source.length; index += 1) {
    if (source[index] === "(") depth += 1;
    if (source[index] !== ")") continue;
    depth -= 1;
    if (depth === 0) return source.slice(openingParenthesis, index + 1);
  }
  return "";
}

/** Collects semicolon-terminated Rust statements without discarding nested literals. */
export function rustSemicolonStatements(source) {
  const statements = [];
  let start = 0;
  for (let index = 0; index < source.length; index += 1) {
    if (source[index] !== ";") continue;
    statements.push(source.slice(start, index + 1));
    start = index + 1;
  }
  return statements;
}
