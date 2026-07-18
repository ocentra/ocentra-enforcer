function pushMask(context, ch) {
  context.out += ch === "\n" ? "\n" : " ";
}

/** Consumes quoted Rust text while preserving scanner state. */
export function consumeRustQuotedText(context, source, index) {
  const ch = source[index];
  pushMask(context, ch);
  if (ch === "\\" && index + 1 < source.length) {
    pushMask(context, source[index + 1]);
    return index + 1;
  }
  const closesString = context.state === "string" && ch === '"';
  const closesChar = context.state === "char" && ch === "'";
  if (closesString || closesChar) context.state = "code";
  return index;
}
