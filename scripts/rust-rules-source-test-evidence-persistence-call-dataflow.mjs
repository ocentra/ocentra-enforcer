import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { balancedParenthesizedAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

/** Returns argument source text for calls to a named function. */
export function callArguments(source, functionName) {
  const calls = [];
  const pattern = new RegExp(`\\b${escapeRegExp(functionName)}\\s*\\(`, "gu");
  for (const match of source.matchAll(pattern)) {
    const openingParenthesis = source.indexOf("(", match.index ?? 0);
    const expression = balancedParenthesizedAt(source, openingParenthesis);
    if (expression) calls.push(expression.slice(1, -1));
  }
  return calls;
}

/** Extracts variable names referenced by a call argument list. */
export function argumentVariables(argumentsSource) {
  const ignored = new Set(["as", "mut", "None", "Some", "true", "false"]);
  return [...argumentsSource.matchAll(/\b[A-Za-z_][A-Za-z0-9_]*\b/gu)]
    .map((match) => match[0])
    .filter((name) => !ignored.has(name));
}

/** Returns whether an equality assertion references both variable names. */
export function equalityReferences(statement, firstName, secondName) {
  const operand = (name) => `(?:[&*]\\s*)*${escapeRegExp(name)}(?:\\.clone\\(\\))?`;
  const first = operand(firstName);
  const second = operand(secondName);
  return new RegExp(
    `\\b(?:assert_eq|debug_assert_eq|prop_assert_eq)\\s*!\\s*\\(\\s*(?:${first}\\s*,\\s*${second}|${second}\\s*,\\s*${first})\\s*[,)]`,
    "u",
  ).test(statement);
}
