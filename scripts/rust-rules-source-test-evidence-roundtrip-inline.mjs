import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { balancedParenthesizedAt, rustSemicolonStatements } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { DECODE_OPERATION, ENCODE_OPERATION, EQUALITY_ASSERTION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import {
  decodesType,
  equalityAssertionMentions,
  isTargetValue,
  projectsDecodedTarget,
  referencesVariable,
  rustAssignments,
  rustAssignmentValueType,
} from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";

function inlineDecodeUsesTarget(statement, encodedName, decodedTypeName, decoders) {
  if (!EQUALITY_ASSERTION.test(statement) || !referencesVariable(statement, encodedName)) {
    return false;
  }
  const target = escapeRegExp(decodedTypeName);
  if (DECODE_OPERATION.test(statement)
    && new RegExp(`::<\\s*${target}\\s*>`, "u").test(statement)) {
    return true;
  }
  return decoders.some((decoder) =>
    decoder.targetName === decodedTypeName
    && new RegExp(`\\b${escapeRegExp(decoder.name)}\\s*\\(`, "u").test(statement));
}

function nestedCodecDecodeUsesOriginal(expression, originalName) {
  const decodeCalls = new RegExp(DECODE_OPERATION.source, "gu");
  for (const match of expression.matchAll(decodeCalls)) {
    const openingParenthesis = expression.indexOf("(", match.index ?? 0);
    const decodeArguments = balancedParenthesizedAt(expression, openingParenthesis);
    if (!decodeArguments) continue;
    if (ENCODE_OPERATION.test(decodeArguments)
      && referencesVariable(decodeArguments, originalName)) {
      return true;
    }
  }
  return false;
}

/** Returns whether inline test statements establish a target round trip. */
export function hasInlineRoundTrip(body, targetName, decoders) {
  const assignments = rustAssignments(body);
  for (const original of assignments) {
    const nestedDecodedValues = assignments.filter((assignment) =>
      assignment.index > original.index
      && nestedCodecDecodeUsesOriginal(assignment.expression, original.name));
    for (const decoded of nestedDecodedValues) {
      const remaining = body.slice(decoded.index);
      const comparesValues = rustSemicolonStatements(remaining).some((statement) =>
        isTargetValue(original, targetName)
          ? equalityAssertionMentions(statement, original.name, decoded.name)
          : equalityAssertionMentions(statement, original.name, decoded.name));
      if (comparesValues && (
        decodesType(decoded, targetName)
        || projectsDecodedTarget(remaining, decoded.name, targetName)
      )) {
        return true;
      }
    }
    const encodedValues = assignments.filter((assignment) =>
      assignment.index > original.index
      && ENCODE_OPERATION.test(assignment.expression)
      && referencesVariable(assignment.expression, original.name));
    for (const encoded of encodedValues) {
      const encodedType = rustAssignmentValueType(original);
      if (encodedType
        && isTargetValue(original, targetName)
        && rustSemicolonStatements(body.slice(encoded.index)).some((statement) =>
          referencesVariable(statement, original.name)
          && inlineDecodeUsesTarget(statement, encoded.name, encodedType, decoders))) {
        return true;
      }
      const decodedValues = assignments.filter((assignment) =>
        assignment.index > encoded.index
        && DECODE_OPERATION.test(assignment.expression)
        && referencesVariable(assignment.expression, encoded.name));
      for (const decoded of decodedValues) {
        const remaining = body.slice(decoded.index);
        const comparesValues = rustSemicolonStatements(remaining).some((statement) =>
          equalityAssertionMentions(statement, original.name, decoded.name));
        if (comparesValues && (
          decodesType(decoded, targetName)
          || projectsDecodedTarget(remaining, decoded.name, targetName)
        )) {
          return true;
        }
      }
    }
  }
  return false;
}
