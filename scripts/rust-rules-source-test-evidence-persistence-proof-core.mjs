import {
  rustAssignments,
  rustAssignmentValueType,
} from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";
import { usesRoundTripDecode } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { rustSemicolonStatements } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { referencesVariable } from "./rust-rules-source-test-evidence-persistence-definitions.mjs";
import {
  argumentVariables,
  callArguments,
  equalityReferences,
} from "./rust-rules-source-test-evidence-persistence-call-dataflow.mjs";

/** Returns whether a test body directly proves the persistence descriptor. */
export function directlyProvesPersistence(body, descriptor) {
  const writerCalls = callArguments(body, descriptor.writer);
  const readerResults = rustAssignments(body).flatMap((assignment) => {
    const calls = callArguments(assignment.expression, descriptor.reader);
    return calls.map((argumentsSource) => ({ assignment, argumentsSource }));
  });
  for (const writerArguments of writerCalls) {
    const writerVariables = argumentVariables(writerArguments);
    for (const readerResult of readerResults) {
      const readerVariables = argumentVariables(readerResult.argumentsSource);
      const sharedLocation = writerVariables.some((name) => readerVariables.includes(name));
      if (!sharedLocation) continue;
      const originalValues = writerVariables.filter((name) => !readerVariables.includes(name));
      const remaining = body.slice(readerResult.assignment.index);
      if (originalValues.some((originalName) =>
        rustSemicolonStatements(remaining).some((statement) =>
          equalityReferences(statement, originalName, readerResult.assignment.name)))) {
        return true;
      }
    }
  }
  return false;
}

/** Returns whether source data is projected into the requested target value. */
export function projectsTarget(body, sourceName, targetName) {
  const tainted = new Set([sourceName]);
  const target = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  for (const assignment of rustAssignments(body)) {
    const derivesFromSource = [...tainted].some((name) =>
      referencesVariable(assignment.expression, name));
    if (!derivesFromSource) continue;
    if (target.test(assignment.type)) return true;
    if (assignment.name !== "_") tainted.add(assignment.name);
  }
  return false;
}
