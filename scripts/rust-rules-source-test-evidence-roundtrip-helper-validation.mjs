import { rustSemicolonStatements } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { ENCODE_OPERATION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { equalityAssertionReferences, referencesVariable, rustAssignments } from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";
import { genericDecode, helperDefinitions, helperReturnsNestedRoundTrip } from "./rust-rules-source-test-evidence-roundtrip-helper-definitions.mjs";

/** Returns whether a helper asserts its input round trip. */
export function helperAssertsInputRoundTrip(definition) {
  const assignments = rustAssignments(definition.body);
  const encodedValues = assignments.filter((assignment) =>
    ENCODE_OPERATION.test(assignment.expression)
    && referencesVariable(assignment.expression, definition.parameter));
  for (const encoded of encodedValues) {
    const decodedValues = assignments.filter((assignment) =>
      assignment.index > encoded.index
      && genericDecode(assignment)
      && referencesVariable(assignment.expression, encoded.name));
    if (decodedValues.some((decoded) =>
      rustSemicolonStatements(definition.body.slice(decoded.index)).some((statement) =>
        equalityAssertionReferences(statement, definition.parameter, decoded.name)))) {
      return true;
    }
  }
  return false;
}

/** Returns whether a helper asserts a wire-format round trip. */
export function helperAssertsWireRoundTrip(definition) {
  const assignments = rustAssignments(definition.body);
  const firstDecoded = assignments.filter((assignment) =>
    genericDecode(assignment)
    && referencesVariable(assignment.expression, definition.parameter));
  for (const decoded of firstDecoded) {
    const encodedValues = assignments.filter((assignment) =>
      assignment.index > decoded.index
      && ENCODE_OPERATION.test(assignment.expression)
      && referencesVariable(assignment.expression, decoded.name));
    for (const encoded of encodedValues) {
      const decodedAgain = assignments.filter((assignment) =>
        assignment.index > encoded.index
        && genericDecode(assignment)
        && referencesVariable(assignment.expression, encoded.name));
      if (decodedAgain.some((again) =>
        rustSemicolonStatements(definition.body.slice(again.index)).some((statement) =>
          equalityAssertionReferences(statement, decoded.name, again.name)))) {
        return true;
      }
    }
  }
  return false;
}

/** Describes generic round-trip helpers only after their internal dataflow is proven. */
export function roundTripHelperDescriptors(source, masked = undefined) {
  return helperDefinitions(source, masked).flatMap((definition) => {
    const assertsInternally = helperAssertsInputRoundTrip(definition)
      || helperAssertsWireRoundTrip(definition);
    if (assertsInternally) return [{ name: definition.name, assertsInternally: true }];
    if (helperReturnsNestedRoundTrip(definition)) {
      return [{ name: definition.name, assertsInternally: false }];
    }
    return [];
  });
}
