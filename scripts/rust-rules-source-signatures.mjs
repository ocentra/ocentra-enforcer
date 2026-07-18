import { lineNumberAt } from "./rust-rules-path-core.mjs";
import {
  blockHeader,
  nearestEnclosingBrace,
} from "./rust-rules-source-signature-helpers.mjs";
export {
  functionName,
  functionParams,
} from "./rust-rules-source-signature-text.mjs";

/** Finds the Rust scope enclosing a source index. */
export function enclosingRustScope(masked, index) {
  const braceIndex = nearestEnclosingBrace(masked, index);
  const header = blockHeader(masked, braceIndex);
  if (/^(?:unsafe\s+)?impl\b/u.test(header)) {
    return {
      kind: /\bfor\b/u.test(header) ? "trait-impl" : "inherent-impl",
      header,
    };
  }
  if (/^(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?trait\b/u.test(header)) {
    return { kind: "trait", header };
  }
  return { kind: "other", header };
}

function findSignatureEnd(masked, startIndex) {
  let end = startIndex;
  let parenDepth = 0;
  let angleDepth = 0;
  let seenParen = false;
  for (; end < masked.length; end += 1) {
    const ch = masked[end];
    if (ch === "(") {
      parenDepth += 1;
      seenParen = true;
      continue;
    }
    if (ch === ")") {
      parenDepth = Math.max(0, parenDepth - 1);
      continue;
    }
    if (ch === "<") {
      angleDepth += 1;
      continue;
    }
    if (ch === ">") {
      angleDepth = Math.max(0, angleDepth - 1);
      continue;
    }
    if (
      seenParen &&
      parenDepth === 0 &&
      angleDepth === 0 &&
      (ch === "{" || ch === ";")
    ) {
      return end + 1;
    }
  }
  return end;
}

/** Collects Rust function signatures from masked source text. */
export function collectFunctionSignatures(masked) {
  const signatures = [];
  const fnRe =
    /\b(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]+"\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\b/gu;
  let match;
  while ((match = fnRe.exec(masked)) !== null) {
    const end = findSignatureEnd(masked, match.index);
    signatures.push({
      text: masked.slice(match.index, end),
      index: match.index,
      line: lineNumberAt(masked, match.index),
      scope: enclosingRustScope(masked, match.index),
    });
    fnRe.lastIndex = Math.max(fnRe.lastIndex, end);
  }
  return signatures;
}
