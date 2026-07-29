import { escapeRegExp, maskRustCode } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";

/** Collects unit-test bodies from Rust source text. */
export function rustTestBodies(source, maskedSource = null) {
  const masked = typeof maskedSource === "string" ? maskedSource : maskRustCode(source);
  const bodies = [];
  for (const match of masked.matchAll(/^\s*#\[test\]\s*$/gmu)) {
    const functionStart = masked.indexOf("fn ", match.index + match[0].length);
    if (functionStart < 0) continue;
    const openingBrace = masked.indexOf("{", functionStart);
    if (openingBrace < 0) continue;
    const body = balancedBodyAt(masked, openingBrace);
    if (body) bodies.push(masked.slice(functionStart, openingBrace) + body);
  }
  return bodies;
}

/** Returns executable statements from a Rust test body. */
export function executableBody(testBody) {
  const openingBrace = testBody.indexOf("{");
  return openingBrace < 0 ? "" : testBody.slice(openingBrace);
}

/** Returns whether a test body invokes the named target. */
export function invokesTarget(body, targetName) {
  const invocation = new RegExp(
    `\\b${escapeRegExp(targetName)}\\s*(?:::<[^>{}]+>)?\\s*\\(`,
    "u",
  );
  return invocation.test(body);
}

/** Returns whether an assertion statement references the given variable. */
export function assertionReferences(statement, variableName) {
  const assertion = /\b(?:assert(?:_eq|_ne)?|debug_assert(?:_eq|_ne)?|matches)\s*!\s*\(/u;
  const variable = new RegExp(`\\b${escapeRegExp(variableName)}\\b`, "u");
  return assertion.test(statement) && variable.test(statement);
}
