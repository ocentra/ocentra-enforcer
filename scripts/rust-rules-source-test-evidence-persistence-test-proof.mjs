import {
  rustAssignments,
  rustAssignmentValueType,
} from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";
import { usesRoundTripDecode } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { directlyProvesPersistence, projectsTarget } from "./rust-rules-source-test-evidence-persistence-proof-core.mjs";

/** Checks a real write/read equality cycle, including typed nested DTO projections. */
export function testBodyHasPersistenceRoundTrip(testBody, targetName, descriptors) {
  const openingBrace = testBody.indexOf("{");
  const body = openingBrace < 0 ? "" : testBody.slice(openingBrace);
  for (const descriptor of descriptors) {
    if (!directlyProvesPersistence(body, descriptor)) continue;
    if (descriptor.targetName === targetName) return true;
    for (const decoded of rustAssignments(body)) {
      if (rustAssignmentValueType(decoded) !== descriptor.targetName) continue;
      if (!usesRoundTripDecode(decoded.expression)) continue;
      if (projectsTarget(body.slice(decoded.index), decoded.name, targetName)) return true;
    }
  }
  return false;
}
