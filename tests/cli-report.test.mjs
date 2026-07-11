import assert from "node:assert/strict";
import test from "node:test";

import { decorateRuleDocs } from "../src/cli-report.mjs";

const context = {
  rulesById: {},
  ruleDocFor: (ruleId) => `rules/test.md#${ruleId}`,
};

function finding(detail = "Console logging found") {
  return {
    ruleId: "TS-6.24",
    severity: "error",
    title: "console logging is forbidden in source",
    detail,
    file: "src/client.ts",
    line: 2,
    snippet: "console.log(value);",
    source: "console.log(value);",
    doc: "rules/typescript/source.md#covered-rules",
  };
}

test("report decoration removes exact duplicate findings and recalculates totals", () => {
  const duplicate = finding();
  const report = decorateRuleDocs({
    ok: false,
    violations: [duplicate, { ...duplicate }],
    warnings: [],
    waived: [],
    findings: [duplicate, { ...duplicate }],
    bySeverity: { error: 2 },
  }, context);

  assert.equal(report.violations.length, 1);
  assert.equal(report.findings.length, 1);
  assert.deepEqual(report.bySeverity, { error: 1 });
});

test("report decoration keeps distinct findings at the same source location", () => {
  const report = decorateRuleDocs({
    ok: false,
    violations: [finding(), finding("Different matcher evidence")],
    warnings: [],
    waived: [],
    findings: [finding(), finding("Different matcher evidence")],
  }, context);

  assert.equal(report.violations.length, 2);
  assert.equal(report.findings.length, 2);
  assert.deepEqual(report.bySeverity, { error: 2 });
});
