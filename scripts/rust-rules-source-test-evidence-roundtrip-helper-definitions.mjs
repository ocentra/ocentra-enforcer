import { maskRustCode } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { DECODE_OPERATION, ENCODE_OPERATION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { referencesVariable } from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";

/** Parses helper function definitions from source text. */
export function helperDefinitions(source) {
  const masked = maskRustCode(source);
  const definitions = [];
  const declaration = /\bfn\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*<(?<generics>[^>{};]*\bT\b[^>{};]*)>\s*\((?<params>[^)]*)\)(?<tail>[^{;]*)\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const name = match.groups?.name ?? "";
    if (!/round_?trip/iu.test(name)) continue;
    const bounds = `${match.groups?.generics ?? ""} ${match.groups?.tail ?? ""}`;
    if (!/\bSerialize\b/u.test(bounds) || !/\b(?:DeserializeOwned|Deserialize)\b/u.test(bounds)) continue;
    const parameter = /^\s*(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/u.exec(match.groups?.params ?? "")?.[1] ?? "";
    if (!parameter) continue;
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (body) definitions.push({ name, parameter, tail: match.groups?.tail ?? "", body });
  }
  return definitions;
}

/** Returns whether an assignment performs a generic decode operation. */
export function genericDecode(assignment) {
  return DECODE_OPERATION.test(assignment.expression)
    && (/\bT\b/u.test(assignment.type) || /::<\s*T\s*>/u.test(assignment.expression));
}

/** Returns whether a helper definition returns nested round-trip evidence. */
export function helperReturnsNestedRoundTrip(definition) {
  if (!/->\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*Result\s*<\s*T\b/u.test(definition.tail)) return false;
  const decodeIndex = definition.body.search(DECODE_OPERATION);
  if (decodeIndex < 0) return false;
  const remaining = definition.body.slice(decodeIndex);
  const encodeIndex = remaining.search(ENCODE_OPERATION);
  return encodeIndex > 0
    && referencesVariable(remaining.slice(encodeIndex), definition.parameter);
}
