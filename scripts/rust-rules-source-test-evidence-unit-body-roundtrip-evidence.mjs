import { crateEvidenceSources } from "./rust-rules-source-test-evidence-files.mjs";
import {
  roundTripDecoderDescriptors,
  roundTripFactoryDescriptors,
  roundTripValueProducerDescriptors,
} from "./rust-rules-source-test-evidence-roundtrip-associated-descriptors.mjs";
import { roundTripHelperDescriptors } from "./rust-rules-source-test-evidence-roundtrip-helper-validation.mjs";
import { testBodyHasRoundTrip } from "./rust-rules-source-test-evidence-roundtrip.mjs";
import { roundTripPersistenceDescriptors } from "./rust-rules-source-test-evidence-persistence-descriptors.mjs";
import { testBodyHasPersistenceRoundTrip } from "./rust-rules-source-test-evidence-persistence-test-proof.mjs";
import { rustTestBodies } from "./rust-rules-source-test-evidence-unit-body-collection.mjs";

/** Checks that a DTO test performs encode, decode, and behavioral comparison. */
export function hasRoundTripTestEvidence(root, filePath, source, targetName, evidenceContext = null) {
  const sources = evidenceContext?.sources ?? crateEvidenceSources(root, filePath, source);
  const testBodies = evidenceContext?.rustTestBodies ?? sources.map(rustTestBodies);
  const helpers = evidenceContext?.roundTripHelpers
    ?? sources.flatMap(roundTripHelperDescriptors);
  const factories = evidenceContext?.roundTripFactories
    ?? sources.flatMap(roundTripFactoryDescriptors);
  const decoders = evidenceContext?.roundTripDecoders
    ?? sources.flatMap(roundTripDecoderDescriptors);
  const persistence = evidenceContext?.roundTripPersistence
    ?? sources.flatMap(roundTripPersistenceDescriptors);
  return testBodies.some((bodies) =>
    bodies.some((testBody) =>
      testBodyHasRoundTrip(testBody, targetName, helpers, factories, decoders)
      || testBodyHasPersistenceRoundTrip(testBody, targetName, persistence)));
}
