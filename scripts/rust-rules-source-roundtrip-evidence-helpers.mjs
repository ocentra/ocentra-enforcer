import { escapeRegExp } from "./rust-rules-path-core.mjs";
import {
  roundTripsTargetDataflow,
  targetProjectionValues,
} from "./rust-rules-source-roundtrip-dataflow.mjs";

/** Detects explicit and inferred calls to a crate-local round-trip helper. */
export function callsRoundTripHelper(testBody, target, helperNames) {
  const escapedTarget = escapeRegExp(target);
  const targetValues = targetProjectionValues(testBody, target);
  for (const helperName of helperNames) {
    const escapedHelper = escapeRegExp(helperName);
    const invocation = new RegExp(
      `\\b${escapedHelper}\\s*::\\s*<\\s*${escapedTarget}\\s*>\\s*\\(`,
      "u",
    );
    if (invocation.test(testBody)) return true;
    const directProjectionInvocation = new RegExp(
      `\\b${escapedHelper}\\s*\\(\\s*&?\\s*${escapedTarget}\\b`,
      "u",
    );
    if (directProjectionInvocation.test(testBody)) return true;
    for (const value of targetValues) {
      const escapedValue = escapeRegExp(value);
      const inferredInvocation = new RegExp(
        `\\b${escapedHelper}\\s*\\(\\s*&?\\s*${escapedValue}\\b\\s*\\)`,
        "u",
      );
      if (inferredInvocation.test(testBody)) return true;
    }
  }
  return false;
}

/** Checks one test body for executable round-trip behavior for a target type. */
export function testRoundTripsTarget(testBody, target, helperNames) {
  if (callsRoundTripHelper(testBody, target, helperNames)) return true;
  return roundTripsTargetDataflow(testBody, target);
}
