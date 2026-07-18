import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";

import { scanTypeScriptDomainCoreRules } from "../src/source-policy-typescript-source-domain-domain-core.mjs";

function ruleIds(filePath, source) {
  return new Set(
    scanTypeScriptDomainCoreRules(
      path.parse(filePath).root,
      filePath,
      filePath.replaceAll("\\", "/"),
      source.split(/\r?\n/u),
    ).map((finding) => finding.ruleId),
  );
}

test("TS-6.26 applies to domain APIs but not tooling helpers", () => {
  const toolingIds = ruleIds(
    path.resolve("src/test-doctrine-report-relevance-critical.mjs"),
    "export function optionalReport() { return null; }",
  );
  const domainIds = ruleIds(
    path.resolve("src/domain/value.ts"),
    "export function optionalValue() { return null; }",
  );

  assert.equal(toolingIds.has("TS-6.26"), false);
  assert.equal(domainIds.has("TS-6.26"), true);
});
