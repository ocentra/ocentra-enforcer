import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import {
  executableBody,
  invokesTarget,
  rustTestBodies,
} from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import { hasAssociatedParserAssertion } from "./rust-rules-source-test-evidence-unit-body-assertions.mjs";

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
  const testBodies = evidenceContext?.rustTestBodies
    ?? crateEvidenceSources(root, filePath, source).map(rustTestBodies);
  return testBodies.some((bodies) =>
    bodies.some((testBody) => {
      const body = executableBody(testBody);
      return invokesTarget(body, targetName)
        && evidencePattern.test(testBody)
        && hasAssociatedParserAssertion(body, targetName, requiresRejection);
    }));
}
