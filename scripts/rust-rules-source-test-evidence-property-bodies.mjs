import { maskRustCode } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

function macroBodies(source, pattern) {
  const bodies = [];
  for (const match of source.matchAll(pattern)) {
    const openingBrace = match.index + match[0].lastIndexOf("{");
    const body = balancedBodyAt(source, openingBrace);
    if (body) bodies.push(body);
  }
  return bodies;
}

export function propertyTestBodies(source, maskedSource = null) {
  const masked = typeof maskedSource === "string" ? maskedSource : maskRustCode(source);
  const bodies = macroBodies(masked, /\b(?:proptest|quickcheck)!\s*\{/gu);
  for (const match of masked.matchAll(/^\s*#\[(?:quickcheck|proptest)\]\s*$/gmu)) {
    const functionStart = masked.indexOf("fn ", match.index + match[0].length);
    const openingBrace = functionStart < 0 ? -1 : masked.indexOf("{", functionStart);
    const body = openingBrace < 0 ? "" : balancedBodyAt(masked, openingBrace);
    if (body) bodies.push(body);
  }
  return bodies;
}

export function registeredPropertyTargetBodies(source) {
  return macroBodies(source, /\bproperty_parser_contracts!\s*\{/gu);
}
