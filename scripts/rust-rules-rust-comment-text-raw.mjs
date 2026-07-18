function pushMask(context, ch) {
  context.out += ch === "\n" ? "\n" : " ";
}

/** Consumes a raw Rust string literal while preserving scanner state. */
export function consumeRustRawText(context, source, index) {
  const ch = source[index];
  pushMask(context, ch);
  const suffix = source.slice(index + 1, index + 1 + context.rawHashes.length);
  if (ch !== '"' || suffix !== context.rawHashes) return index;
  for (let offset = 0; offset < context.rawHashes.length; offset += 1) {
    pushMask(context, source[index + 1 + offset]);
  }
  context.state = "code";
  return index + context.rawHashes.length;
}
