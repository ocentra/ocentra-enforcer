import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { rustSemicolonStatements } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { ASSERTION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";
import { isTargetValue, referencesVariable, rustAssignments } from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";

function helperCallUsesOriginal(statement, helperName, originalName) {
  const call = new RegExp(`\\b${escapeRegExp(helperName)}\\s*\\(`, "u");
  return call.test(statement) && referencesVariable(statement, originalName);
}

function helperCallConstructsTarget(body, helperName, targetName, factories) {
  const helper = escapeRegExp(helperName);
  const target = escapeRegExp(targetName);
  if (new RegExp(
    `\\b${helper}\\s*\\(\\s*&?\\s*${target}\\s*(?:\\{|\\(|\\))`,
    "u",
  ).test(body)) {
    return true;
  }
  return factories.some((factory) =>
    factory.targetName === targetName
    && new RegExp(
      `\\b${helper}\\s*\\(\\s*&?\\s*${target}\\s*::\\s*${escapeRegExp(factory.method)}\\s*\\(`,
      "u",
    ).test(body));
}

/** Returns whether helper calls establish a round trip for the target value. */
export function hasHelperRoundTrip(body, targetName, helpers, factories) {
  for (const helper of helpers) {
    const typedCall = new RegExp(
      `\\b${escapeRegExp(helper.name)}\\s*::<\\s*${escapeRegExp(targetName)}\\s*>\\s*\\(`,
      "u",
    );
    if (helper.assertsInternally && typedCall.test(body)) return true;
    if (!helper.assertsInternally && rustSemicolonStatements(body).some((statement) =>
      ASSERTION.test(statement) && typedCall.test(statement))) {
      return true;
    }
    if (helper.assertsInternally
      && helperCallConstructsTarget(body, helper.name, targetName, factories)) {
      return true;
    }
  }
  const assignments = rustAssignments(body);
  for (const original of assignments.filter((assignment) => isTargetValue(assignment, targetName, factories))) {
    const remaining = body.slice(original.index);
    for (const statement of rustSemicolonStatements(remaining)) {
      for (const helper of helpers) {
        if (!helperCallUsesOriginal(statement, helper.name, original.name)) continue;
        if (helper.assertsInternally || ASSERTION.test(statement)) return true;
      }
    }
  }
  return false;
}
