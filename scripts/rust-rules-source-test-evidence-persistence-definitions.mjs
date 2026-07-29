import { escapeRegExp, maskRustCode } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

/** Returns whether source text references the given variable name. */
export function referencesVariable(source, variableName) {
  return new RegExp(`\\b${escapeRegExp(variableName)}\\b`, "u").test(source);
}

/** Parses named function definitions from source text. */
export function functionDefinitions(source, maskedSource = null) {
  const masked = typeof maskedSource === "string" ? maskedSource : maskRustCode(source);
  const definitions = [];
  const declaration = /\bfn\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>{}]*>)?\s*\((?<params>[^)]*)\)(?<tail>[^{;]*)\{/gu;
  for (const match of masked.matchAll(declaration)) {
    const openingBrace = (match.index ?? 0) + match[0].lastIndexOf("{");
    const body = balancedBodyAt(masked, openingBrace);
    if (body) {
      definitions.push({
        name: match.groups?.name ?? "",
        params: match.groups?.params ?? "",
        tail: match.groups?.tail ?? "",
        body,
      });
    }
  }
  return definitions;
}
