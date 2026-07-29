import assert from "node:assert/strict";
import test from "node:test";
import {
  isDecoderBoundaryPath,
  isToolingBoundaryPath,
} from "../src/source-policy-paths.mjs";

test("canonical literal-scan integration is a tooling and decoder boundary", () => {
  const integration = "crates/enforcer-literal-scan/integration/ocentra-literal-scan.mjs";
  assert.equal(isToolingBoundaryPath(integration), true);
  assert.equal(isDecoderBoundaryPath(integration), true);
});

test("literal-scan product source is not promoted to a tooling boundary", () => {
  const productSource = "crates/enforcer-literal-scan/src/domain/literal_policy.mjs";
  assert.equal(isToolingBoundaryPath(productSource), false);
  assert.equal(isDecoderBoundaryPath(productSource), false);
});

test("nested package test trees remain tooling boundaries", () => {
  assert.equal(isToolingBoundaryPath("packages/app/tests/browser-proof.mjs"), true);
  assert.equal(isToolingBoundaryPath("packages/app/__tests__/browser-proof.ts"), true);
  assert.equal(isToolingBoundaryPath("packages/app/src/domain/browser.ts"), false);
});
