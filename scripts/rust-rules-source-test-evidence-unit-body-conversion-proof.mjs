import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { rustAssignments } from "./rust-rules-source-test-evidence-roundtrip-dataflow.mjs";
import { rustSemicolonStatements } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { expressionProducesDto } from "./rust-rules-source-test-evidence-unit-body-dto-producers.mjs";
import { assertionReferences } from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import {
  callOccurrences,
} from "./rust-rules-source-test-evidence-unit-body-call-inspection.mjs";
import {
  callHasDirectRejection,
  matchRejectionEvidence,
  variableRejectionEvidence,
} from "./rust-rules-source-test-evidence-unit-body-assertions.mjs";

/** Returns whether conversion tests include rejection evidence for the target. */
export function hasAssociatedConversionRejection(
  body,
  dtoName,
  domainName,
  factories,
  producers,
) {
  const dto = new RegExp(`\\b${escapeRegExp(dtoName)}\\b`, "u");
  const dtoAssignments = rustAssignments(body).filter((assignment) =>
    dto.test(assignment.type)
    || new RegExp(`\\b${escapeRegExp(dtoName)}\\b\\s*\\{`, "u").test(assignment.expression)
    || expressionProducesDto(assignment.expression, dtoName, factories, producers));
  const dtoVariables = dtoAssignments.map((assignment) => assignment.name);
  const domainCalls = (source) => callOccurrences(
    source,
    new RegExp(`\\b${escapeRegExp(domainName)}\\s*::\\s*try_from\\s*\\(`, "gu"),
  );
  const consumesTargetDto = (call) => dto.test(call.arguments)
    || dtoVariables.some((variableName) =>
      new RegExp(`\\b${escapeRegExp(variableName)}\\b`, "u").test(call.arguments));
  for (const call of domainCalls(body).filter(consumesTargetDto)) {
    if (callHasDirectRejection(body, call)) return true;
  }
  for (const assignment of rustAssignments(body)) {
    if (!domainCalls(assignment.expression).some(consumesTargetDto)) continue;
    const remaining = body.slice(assignment.index + assignment.expression.length);
    if (rustSemicolonStatements(remaining).some((statement) =>
      assertionReferences(statement, assignment.name)
      && variableRejectionEvidence(statement, assignment.name))) return true;
    if (matchRejectionEvidence(remaining, assignment.name)) return true;
  }
  return false;
}
