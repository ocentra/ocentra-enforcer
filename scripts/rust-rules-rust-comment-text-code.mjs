function pushMask(context, ch) {
  context.out += ch === "\n" ? "\n" : " ";
}

/** Consumes a Rust code-text segment while scanning comment text. */
export function consumeRustCodeText(context, source, index) {
  const ch = source[index];
  const next = source[index + 1];
  const rawMatch = source.slice(index).match(/^(?:b|c|br)?r(#+)?"/u);
  if (rawMatch) {
    context.state = "rawString";
    context.rawHashes = rawMatch[1] ?? "";
    for (let offset = 0; offset < rawMatch[0].length; offset += 1) {
      pushMask(context, source[index + offset]);
    }
    return index + rawMatch[0].length - 1;
  }
  if (ch === "/" && next === "/") {
    context.state = "lineComment";
    context.out += ch;
  } else if (ch === "/" && next === "*") {
    context.state = "blockComment";
    context.blockDepth = 1;
    context.out += ch;
  } else if (ch === "b" && next === '"') {
    context.state = "string";
    pushMask(context, ch);
    pushMask(context, next);
    return index + 1;
  } else if (ch === '"') {
    context.state = "string";
    pushMask(context, ch);
  } else if (ch === "'" && !/^'[A-Za-z_][A-Za-z0-9_]*\b/u.test(source.slice(index))) {
    context.state = "char";
    pushMask(context, ch);
  } else pushMask(context, ch);
  return index;
}
