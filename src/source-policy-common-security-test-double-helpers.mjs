/** Determines whether a path contains source rule definitions. */
export function isRuleDefinitionSourcePath(rel) {
  return /^crates\/enforcer-lang-common\/src\/(?:boundary\/source_analysis|rules\/(?:deferred_work|test_quality))\.rs$/u.test(rel);
}

/** Removes quoted Rust string content before lexical policy analysis. */
export function rustCodeOutsideStringLiterals(line) {
  if (!line.includes('"') && !line.includes("'")) {
    const lineComment = line.indexOf("//");
    const blockComment = line.indexOf("/*");
    const commentIndexes = [lineComment, blockComment].filter((index) => index >= 0);
    return commentIndexes.length === 0 ? line : line.slice(0, Math.min(...commentIndexes));
  }
  let result = "";
  let quote = null;
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (quote != null) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
      result += " ";
      continue;
    }
    if (character === "'") {
      const lifetime = line.slice(index).match(/^'[A-Za-z_][A-Za-z0-9_]*/u)?.[0] ?? "";
      if (lifetime && line[index + lifetime.length] !== "'") {
        result += lifetime;
        index += lifetime.length - 1;
        continue;
      }
    }
    if (character === '"' || character === "'") {
      quote = character;
      result += " ";
    } else if (
      character === "/" &&
      (line[index + 1] === "/" || line[index + 1] === "*")
    ) {
      result += " ".repeat(line.length - index);
      break;
    } else {
      result += character;
    }
  }
  return result;
}
