import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import {
  roundTripFactoryDescriptors,
  roundTripValueProducerDescriptors,
} from "./rust-rules-source-test-evidence-roundtrip-associated-descriptors.mjs";
import {
  executableBody,
  rustTestBodies,
} from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import { hasAssociatedConversionRejection } from "./rust-rules-source-test-evidence-unit-body-conversion-proof.mjs";

/** Checks that a fallible DTO conversion has an associated rejection assertion. */
export function hasConversionRejectionEvidence(
  root,
  filePath,
  source,
  dtoName,
  domainName,
  evidenceContext = null,
) {
  const evidencePattern = /(?:^|[^A-Za-z0-9])(?:invalid|reject[A-Za-z0-9_]*|malformed|bad_input|unparseable)(?:$|[^A-Za-z0-9])/iu;
  const sources = evidenceContext?.sources ?? crateEvidenceSources(root, filePath, source);
  const testBodies = evidenceContext?.rustTestBodies ?? sources.map(rustTestBodies);
  const factories = evidenceContext?.roundTripFactories
    ?? sources.flatMap(roundTripFactoryDescriptors);
  const producers = evidenceContext?.roundTripValueProducers
    ?? sources.flatMap(roundTripValueProducerDescriptors);
  return testBodies.some((bodies) =>
    bodies.some((testBody) => {
      const body = executableBody(testBody);
      return evidencePattern.test(testBody)
        && hasAssociatedConversionRejection(
          body,
          dtoName,
          domainName,
          factories,
          producers,
        );
    }));
}
