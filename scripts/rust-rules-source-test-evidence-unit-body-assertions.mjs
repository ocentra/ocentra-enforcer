import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { balancedBodyAt } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { rustSemicolonStatements } from "./rust-rules-source-test-evidence-ranges-balanced.mjs";
import { assertionReferences } from "./rust-rules-source-test-evidence-unit-body-collection.mjs";
import { statementAt, targetCalls } from "./rust-rules-source-test-evidence-unit-body-call-inspection.mjs";

/** Returns whether a call has direct rejection evidence in its test body. */
export function callHasDirectRejection(body, call) {
  const statement = statementAt(body, call.start);
  const localEnd = call.end - (body.lastIndexOf(";", call.start - 1) + 1);
  const tail = statement.slice(localEnd);
  if (/^\s*\.(?:err|expect_err|is_err|unwrap_err)\s*\(/u.test(tail)) return true;
  const prefix = statement.slice(0, localEnd);
  return (/\bmatches\s*!\s*\([^;]*$/u.test(prefix) && /^\s*,\s*Err\b/u.test(tail))
    || (/\bassert_eq\s*!\s*\([^;]*$/u.test(prefix) && /^\s*,\s*Err\b/u.test(tail));
}

/** Returns whether match syntax rejects the given variable in a test body. */
export function matchRejectionEvidence(body, variableName) {
  const pattern = new RegExp(`\\bmatch\\s+${escapeRegExp(variableName)}\\s*\\{`, "gu");
  for (const match of body.matchAll(pattern)) {
    const openingBrace = body.indexOf("{", match.index ?? 0);
    const matchBody = balancedBodyAt(body, openingBrace);
    const errorArmObservesFailure = /\bErr(?:(?!=>)[\s\S]){0,300}?=>(?:(?!\b(?:Ok(?:\s*\([^)]*\))?|other|unexpected|_)\s*=>)[\s\S]){0,800}?\b(?:assert(?:_eq|_ne)?|debug_assert(?:_eq|_ne)?)\s*!\s*\(/u.test(matchBody);
    const nonErrorArmRejects = /\b(?!Err\b)(?:Ok(?:\s*\([^)]*\))?|other|unexpected|_)\s*=>[\s\S]{0,300}?\b(?:panic|unreachable)\s*!\s*\(/u.test(matchBody);
    if (/\bErr\s*\(/u.test(matchBody)
      && (errorArmObservesFailure || nonErrorArmRejects)) {
      return true;
    }
  }
  return false;
}

/** Returns whether a statement proves rejection of the given variable. */
export function variableRejectionEvidence(statement, variableName) {
  const variable = escapeRegExp(variableName);
  return new RegExp(
    `(?:\\b${variable}\\s*\\.(?:err|expect_err|is_err|unwrap_err)\\s*\\(|\\bmatches\\s*!\\s*\\(\\s*${variable}\\s*,\\s*Err\\b|\\bassert_eq\\s*!\\s*\\(\\s*${variable}\\s*,\\s*Err\\b)`,
    "u",
  ).test(statement);
}

/** Returns whether parser assertions cover the target and required rejection case. */
export function hasAssociatedParserAssertion(body, targetName, requiresRejection) {
  const statements = rustSemicolonStatements(body);
  for (const call of targetCalls(body, targetName)) {
    const statement = statementAt(body, call.start);
    if (requiresRejection && callHasDirectRejection(body, call)) return true;
    if (!requiresRejection && /\b(?:assert(?:_eq|_ne)?|debug_assert(?:_eq|_ne)?|matches)\s*!\s*\(/u.test(statement)) return true;
    if (!requiresRejection && /\.(?:expect_err|unwrap_err)\s*\(/u.test(statement)) return true;
  }
  const target = escapeRegExp(targetName);
  const assignment = new RegExp(
    `\\blet\\s+(?:mut\\s+)?([A-Za-z_][A-Za-z0-9_]*)[^;=]*=([^;]*\\b${target}\\s*(?:::<[^>{}]+>)?\\s*\\([^;]*);`,
    "gu",
  );
  for (const match of body.matchAll(assignment)) {
    const variableName = match[1];
    const remaining = body.slice((match.index ?? 0) + match[0].length);
    for (const statement of rustSemicolonStatements(remaining)) {
      if (!assertionReferences(statement, variableName)) continue;
      if (!requiresRejection || variableRejectionEvidence(statement, variableName)) return true;
    }
    if (matchRejectionEvidence(remaining, variableName)) return true;
  }
  return false;
}
