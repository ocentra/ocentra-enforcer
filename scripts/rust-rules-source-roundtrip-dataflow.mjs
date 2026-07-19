import { escapeRegExp } from "./rust-rules-path-core.mjs";

function decodedRoundTripOutputs(testBody, target) {
  const escapedTarget = escapeRegExp(target);
  const variables = new Set();
  const typed = new RegExp(
    `\\blet\\s+(?:mut\\s+)?(?<name>[A-Za-z_][A-Za-z0-9_]*)\\s*:\\s*${escapedTarget}\\b[^=]*=\\s*(?:serde_json|toml|serde_yaml)::from_[A-Za-z0-9_]+`,
    "gu",
  );
  const turbofish = new RegExp(
    `\\blet\\s+(?:mut\\s+)?(?<name>[A-Za-z_][A-Za-z0-9_]*)\\s*=\\s*(?:serde_json|toml|serde_yaml)::from_[A-Za-z0-9_]+\\s*::<\\s*${escapedTarget}\\s*>`,
    "gu",
  );
  for (const match of testBody.matchAll(typed)) {
    variables.add(match.groups?.name ?? "");
  }
  for (const match of testBody.matchAll(turbofish)) {
    variables.add(match.groups?.name ?? "");
  }
  variables.delete("");
  return variables;
}

function inputWasEncoded(testBody, input) {
  const escapedInput = escapeRegExp(input);
  return new RegExp(
    `\\b(?:serde_json|toml|serde_yaml)::to_[A-Za-z0-9_]+\\s*\\(\\s*&?\\s*${escapedInput}\\b`,
    "u",
  ).test(testBody);
}

function assertedRoundTripInput(testBody, output) {
  const escapedOutput = escapeRegExp(output);
  const forward = new RegExp(
    `\\bassert_eq!\\s*\\(\\s*&?\\s*${escapedOutput}\\s*,\\s*&?\\s*(?<input>[A-Za-z_][A-Za-z0-9_]*)\\s*(?:,[^)]*)?\\)`,
    "u",
  ).exec(testBody);
  const reverse = new RegExp(
    `\\bassert_eq!\\s*\\(\\s*&?\\s*(?<input>[A-Za-z_][A-Za-z0-9_]*)\\s*,\\s*&?\\s*${escapedOutput}\\s*(?:,[^)]*)?\\)`,
    "u",
  ).exec(testBody);
  return forward?.groups?.input ?? reverse?.groups?.input ?? "";
}

/** Proves serialize, deserialize, and equality data flow for one target type. */
export function roundTripsTargetDataflow(testBody, target) {
  for (const output of decodedRoundTripOutputs(testBody, target)) {
    const input = assertedRoundTripInput(testBody, output);
    if (input && inputWasEncoded(testBody, input)) return true;
  }

  const escapedTarget = escapeRegExp(target);
  const decodedExpression =
    `(?:serde_json|toml|serde_yaml)::from_[A-Za-z0-9_]+\\s*::\\s*<\\s*${escapedTarget}\\s*>\\s*\\([^)]*\\)\\s*\\??`;
  const forward = new RegExp(
    `\\bassert_eq!\\s*\\(\\s*${decodedExpression}\\s*,\\s*&?\\s*(?<input>[A-Za-z_][A-Za-z0-9_]*)\\s*(?:,[^)]*)?\\)`,
    "u",
  ).exec(testBody);
  const reverse = new RegExp(
    `\\bassert_eq!\\s*\\(\\s*&?\\s*(?<input>[A-Za-z_][A-Za-z0-9_]*)\\s*,\\s*${decodedExpression}\\s*(?:,[^)]*)?\\)`,
    "u",
  ).exec(testBody);
  const input = forward?.groups?.input ?? reverse?.groups?.input ?? "";
  return Boolean(input && inputWasEncoded(testBody, input));
}
