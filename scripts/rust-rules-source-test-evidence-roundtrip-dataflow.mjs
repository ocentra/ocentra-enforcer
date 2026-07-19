import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { EQUALITY_ASSERTION } from "./rust-rules-source-test-evidence-roundtrip-codecs.mjs";

/** Collects named Rust let assignments for local evidence dataflow. */
export function rustAssignments(body) {
  return [...body.matchAll(/\blet\s+(?:mut\s+)?(?<name>[A-Za-z_][A-Za-z0-9_]*)(?:\s*:\s*(?<type>[^=;]+))?\s*=\s*(?<expression>[^;]+);/gu)]
    .map((match) => ({
      name: match.groups?.name ?? "",
      type: match.groups?.type ?? "",
      expression: match.groups?.expression ?? "",
      index: match.index ?? 0,
    }));
}

/** Returns whether source text references the given variable name. */
export function referencesVariable(source, variableName) {
  return new RegExp(`\\b${escapeRegExp(variableName)}\\b`, "u").test(source);
}

/** Returns whether an equality assertion references both named values. */
export function equalityAssertionReferences(statement, firstName, secondName) {
  if (!EQUALITY_ASSERTION.test(statement)) return false;
  const operand = (name) => `(?:[&*]\\s*)*${escapeRegExp(name)}(?:\\.clone\\(\\))?`;
  const first = operand(firstName);
  const second = operand(secondName);
  return new RegExp(
    `\\b(?:assert_eq|debug_assert_eq|prop_assert_eq)\\s*!\\s*\\(\\s*(?:${first}\\s*,\\s*${second}|${second}\\s*,\\s*${first})\\s*[,)]`,
    "u",
  ).test(statement);
}

/** Returns whether an equality assertion text mentions both names. */
export function equalityAssertionMentions(statement, firstName, secondName) {
  return EQUALITY_ASSERTION.test(statement)
    && referencesVariable(statement, firstName)
    && referencesVariable(statement, secondName);
}

/** Returns whether an assignment binds the requested target value. */
export function isTargetValue(assignment, targetName) {
  const target = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  const directConstructor = new RegExp(
    `^\\s*&?\\s*${escapeRegExp(targetName)}\\s*(?:$|\\{|\\()`,
    "u",
  );
  return target.test(assignment.type)
    || directConstructor.test(assignment.expression);
}

/** Infers an assignment's explicit or top-level transport type. */
export function rustAssignmentValueType(assignment) {
  const annotated = /\b([A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\b/u.exec(assignment.type)?.[1];
  if (annotated) return annotated;
  return /^\s*(?:&\s*)?([A-Z][A-Za-z0-9_]*(?:Dto|DTO|Request|Response|Envelope))\s*(?:\{|::)/u.exec(assignment.expression)?.[1] ?? "";
}

/** Returns whether an assignment decodes the requested Rust type. */
export function decodesType(assignment, typeName) {
  const target = escapeRegExp(typeName);
  return new RegExp(`\\b${target}\\b`, "u").test(assignment.type)
    || new RegExp(`::<\\s*${target}\\s*>`, "u").test(assignment.expression);
}

/** Returns whether a decoded value is projected into the requested target. */
export function projectsDecodedTarget(body, decodedName, targetName) {
  const target = escapeRegExp(targetName);
  const tainted = new Set([decodedName]);
  for (const assignment of rustAssignments(body)) {
    const derivesFromDecoded = [...tainted].some((variableName) =>
      referencesVariable(assignment.expression, variableName));
    if (!derivesFromDecoded) continue;
    if (new RegExp(`\\b${target}\\b`, "u").test(assignment.type)) return true;
    if (assignment.name !== "_") tainted.add(assignment.name);
  }
  return false;
}
