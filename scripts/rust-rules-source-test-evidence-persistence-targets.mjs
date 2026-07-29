import {
  rustAssignments,
  rustAssignmentValueType,
} from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";
import {
  usesRoundTripDecode,
  usesRoundTripEncode,
} from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { referencesVariable } from "./rust-rules-source-test-evidence-persistence-definitions.mjs";

/** Extracts persisted target names written by a function definition. */
export function writerTargets(definition) {
  if (!/^(?:write|save|persist|store)/u.test(definition.name)) return [];
  if (!/\b(?:std\s*::\s*fs\s*::\s*write|write_all)\s*\(/u.test(definition.body)) return [];
  const assignments = rustAssignments(definition.body);
  return assignments.flatMap((record) => {
    const targetName = rustAssignmentValueType(record);
    if (!targetName) return [];
    const encoded = assignments.some((assignment) =>
      assignment.index > record.index
      && usesRoundTripEncode(assignment.expression)
      && referencesVariable(assignment.expression, record.name));
    return encoded ? [targetName] : [];
  });
}

/** Extracts persisted target names read by a function definition. */
export function readerTargets(definition) {
  if (!/^(?:read|load|restore)/u.test(definition.name)) return [];
  if (!/\b(?:std\s*::\s*fs\s*::\s*read|read_to_string)\s*\(/u.test(definition.body)) return [];
  return rustAssignments(definition.body).flatMap((record) => {
    const targetName = rustAssignmentValueType(record);
    return targetName && usesRoundTripDecode(record.expression) ? [targetName] : [];
  });
}
