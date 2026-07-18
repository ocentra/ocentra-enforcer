import path from "node:path";
import { escapeRegExp, maskRustCode } from "./rust-rules-path-core.mjs";
import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import { nearestCargoRoot } from "./rust-rules-source-test-evidence-paths.mjs";
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

/** Collects property-test bodies from Rust source text. */
export function propertyTestBodies(source) {
  const masked = maskRustCode(source);
  const bodies = macroBodies(masked, /\b(?:proptest|quickcheck)!\s*\{/gu);
  for (const match of masked.matchAll(/^\s*#\[(?:quickcheck|proptest)\]\s*$/gmu)) {
    const functionStart = masked.indexOf("fn ", match.index + match[0].length);
    const openingBrace = functionStart < 0 ? -1 : masked.indexOf("{", functionStart);
    const body = openingBrace < 0 ? "" : balancedBodyAt(masked, openingBrace);
    if (body) bodies.push(body);
  }
  return bodies;
}

/** Collects bodies registered as property-test targets. */
export function registeredPropertyTargetBodies(source) {
  return macroBodies(source, /\bproperty_parser_contracts!\s*\{/gu);
}

/** Checks whether a target has matching property-test evidence. */
export function hasPropertyEvidence(root, filePath, source, targetName, evidenceContext = null) {
  const targetReference = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  const cargoRoot = nearestCargoRoot(root, filePath);
  const relativeTarget = path.relative(cargoRoot ?? root, filePath).replaceAll(path.sep, "/");
  const registeredTarget = new RegExp(`["']${escapeRegExp(`${relativeTarget}::${targetName}`)}["']\\s*=>`, "u");
  const evidenceSources = evidenceContext?.sources ?? crateEvidenceSources(root, filePath, source);
  const propertyBodies = evidenceContext?.propertyBodies ?? evidenceSources.map(propertyTestBodies);
  const registeredBodies = evidenceContext?.registeredBodies ?? evidenceSources.map(registeredPropertyTargetBodies);
  return evidenceSources.some((evidenceSource, index) => {
    if (propertyBodies[index].some((body) => targetReference.test(body))) return true;
    return /\bproptest!\s*\{/u.test(evidenceSource)
      && registeredBodies[index].some((body) => registeredTarget.test(body));
  });
}
