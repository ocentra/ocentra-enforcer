import { consumeRustCodeText } from "./rust-rules-rust-comment-text-code.mjs";
import { consumeRustCommentText } from "./rust-rules-rust-comment-text-comment.mjs";
import { consumeRustQuotedText } from "./rust-rules-rust-comment-text-quoted.mjs";
import { consumeRustRawText } from "./rust-rules-rust-comment-text-raw.mjs";

/** Produces a Rust source mask that retains only comment text. */
export function rustCommentText(source) {
  const context = { out: "", state: "code", blockDepth: 0, rawHashes: "" };
  for (let index = 0; index < source.length; index += 1) {
    if (context.state === "code") index = consumeRustCodeText(context, source, index);
    else if (context.state === "lineComment" || context.state === "blockComment") {
      index = consumeRustCommentText(context, source, index);
    } else if (context.state === "string" || context.state === "char") {
      index = consumeRustQuotedText(context, source, index);
    } else index = consumeRustRawText(context, source, index);
  }
  return context.out;
}
