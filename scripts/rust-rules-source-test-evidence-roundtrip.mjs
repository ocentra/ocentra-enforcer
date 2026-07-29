import { escapeRegExp } from "./rust-rules-path-core.mjs";
import { hasHelperRoundTrip } from "./rust-rules-source-test-evidence-roundtrip-helper-calls.mjs";
import { hasInlineRoundTrip } from "./rust-rules-source-test-evidence-roundtrip-inline.mjs";

function executableBody(testBody) {
  const brace = testBody.indexOf("{");
  return brace >= 0 ? testBody.slice(brace + 1) : testBody;
}

/** Checks inline or validated-helper round-trip evidence for one exact DTO type. */
export function testBodyHasRoundTrip(
  testBody,
  targetName,
  helpers,
  factories = [],
  decoders = [],
) {
  const body = executableBody(testBody);
  const targetReference = new RegExp(`\\b${escapeRegExp(targetName)}\\b`, "u");
  return targetReference.test(body)
    && (hasInlineRoundTrip(body, targetName, decoders)
      || hasHelperRoundTrip(body, targetName, helpers, factories));
}
