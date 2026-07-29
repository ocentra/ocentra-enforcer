/** Consumes a Rust line or block comment segment. */
export function consumeRustCommentText(context, source, index) {
  const ch = source[index];
  const next = source[index + 1];
  context.out += ch;
  if (context.state === "lineComment") {
    if (ch === "\n") context.state = "code";
    return index;
  }
  if (ch === "/" && next === "*") {
    context.blockDepth += 1;
    context.out += next;
    return index + 1;
  }
  if (ch === "*" && next === "/") {
    context.blockDepth -= 1;
    context.out += next;
    if (context.blockDepth === 0) context.state = "code";
    return index + 1;
  }
  return index;
}
