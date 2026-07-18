import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { balancedParenthesizedAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

/** Returns occurrences of calls to the requested callee in a test body. */
export function callOccurrences(body, callee) {
  const calls = [];
  for (const match of body.matchAll(callee)) {
    const openingParenthesis = body.indexOf("(", match.index ?? 0);
    const expression = balancedParenthesizedAt(body, openingParenthesis);
    if (!expression) continue;
    calls.push({
      start: match.index ?? 0,
      end: openingParenthesis + expression.length,
      arguments: expression.slice(1, -1),
    });
  }
  return calls;
}

/** Returns target function calls found in a test body. */
export function targetCalls(body, targetName) {
  return callOccurrences(
    body,
    new RegExp(`\\b${escapeRegExp(targetName)}\\s*(?:::<[^>{}]+>)?\\s*\\(`, "gu"),
  );
}

/** Returns the statement at a source index within a test body. */
export function statementAt(body, index) {
  const start = body.lastIndexOf(";", index - 1) + 1;
  const terminator = body.indexOf(";", index);
  return body.slice(start, terminator < 0 ? body.length : terminator + 1);
}
