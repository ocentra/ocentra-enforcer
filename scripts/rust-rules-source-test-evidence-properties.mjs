import path from "node:path";
import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import { nearestCargoRoot } from "./rust-rules-source-test-evidence-paths.mjs";
import {
  propertyTestBodies,
  registeredPropertyTargetBodies,
} from "./rust-rules-source-test-evidence-property-bodies.mjs";

/** Checks whether a target has matching property-test evidence. */
export function hasPropertyEvidence(root, filePath, source, targetName, evidenceContext = null) {
  const indexedPropertyTargets = evidenceContext?.propertyTargetNames;
  const indexedRegisteredTargets = evidenceContext?.registeredPropertyTargets;
  const targetReference = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  const cargoRoot = nearestCargoRoot(root, filePath);
  const relativeTarget = path.relative(cargoRoot ?? root, filePath).replaceAll(path.sep, "/");
  const registeredTarget = new RegExp(`["']${escapeRegExp(`${relativeTarget}::${targetName}`)}["']\\s*=>`, "u");
  const evidenceSources = evidenceContext?.sources ?? crateEvidenceSources(root, filePath, source);
  const propertyBodies = evidenceContext?.propertyBodies ?? evidenceSources.map(propertyTestBodies);
  const registeredBodies = evidenceContext?.registeredBodies ?? evidenceSources.map(registeredPropertyTargetBodies);
  if (indexedPropertyTargets || indexedRegisteredTargets) {
    return indexedPropertyTargets?.has(targetName) === true
      || indexedRegisteredTargets?.has(`${relativeTarget}::${targetName}`) === true;
  }
  return evidenceSources.some((evidenceSource, index) => {
    if (propertyBodies[index].some((body) => targetReference.test(body))) return true;
    return /\bproptest!\s*\{/u.test(evidenceSource)
      && registeredBodies[index].some((body) => registeredTarget.test(body));
  });
}
