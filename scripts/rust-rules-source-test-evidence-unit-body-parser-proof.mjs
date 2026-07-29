import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import {
  executableBody,
  invokesTarget,
  rustTestBodies,
} from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import { hasAssociatedParserAssertion } from "./rust-rules-source-test-evidence-unit-body-assertions.mjs";

/**
 * Indexes cached Rust test bodies by every callable identifier they invoke.
 * The arrays retain source order, so narrowing the corpus cannot affect which
 * evidence body satisfies a rule.
 */
export function parserTestBodiesByTarget(testBodies) {
  const byTarget = new Map();
  for (const bodies of testBodies) {
    for (const testBody of bodies) {
      const body = executableBody(testBody);
      const targets = new Set();
      for (const match of body.matchAll(/\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:::<[^>{}]+>)?\s*\(/gu)) {
        targets.add(match[1]);
      }
      for (const target of targets) {
        const candidates = byTarget.get(target);
        if (candidates) candidates.push(testBody);
        else byTarget.set(target, [testBody]);
      }
    }
  }
  return byTarget;
}

/** Checks whether a parser target has matching behavioral unit-test evidence. */
export function hasParserTestEvidence(
  root,
  filePath,
  source,
  targetName,
  evidencePattern,
  evidenceContext = null,
  requiresRejection = false,
) {
  const indexedBodies = evidenceContext?.parserTestBodiesByTarget;
  const candidates = indexedBodies
    ? (indexedBodies.get(targetName) ?? [])
    : (evidenceContext?.rustTestBodies
      ?? crateEvidenceSources(root, filePath, source).map(rustTestBodies)).flat();
  return candidates.some((testBody) => {
    const body = executableBody(testBody);
    return invokesTarget(body, targetName)
      && evidencePattern.test(testBody)
      && hasAssociatedParserAssertion(body, targetName, requiresRejection);
  });
}
