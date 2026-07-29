import assert from "node:assert/strict";
import test from "node:test";
import {
  compactLiteralRiskReport,
  mapLiteralRiskFindings,
} from "../src/literal-risk-findings.mjs";
import { classifyLiteralRiskProvenance } from "../src/literal-risk-provenance.mjs";

function finding(file, category = "secret-like") {
  return {
    rule_id: "LIT-1.1",
    file,
    line: 1,
    category,
    reason: "candidate",
  };
}

test("literal risk provenance is path-specific rather than a blanket exclusion", () => {
  assert.equal(classifyLiteralRiskProvenance("vendor/acme/tool.rs"), "vendored");
  assert.equal(
    classifyLiteralRiskProvenance("crates/memory/vendor/parser/src/parser.c"),
    "vendored",
  );
  assert.equal(
    classifyLiteralRiskProvenance("profiles/ocentra-parent/legacy-scripts/scripts/test/a.mjs"),
    "packaged-profile",
  );
  assert.equal(classifyLiteralRiskProvenance("crates/core/tests/fixtures/key.rs"), "test-fixture");
  assert.equal(
    classifyLiteralRiskProvenance("crates/enforcer-literal-scan/src/lib_tests.rs"),
    "test-fixture",
  );
  assert.equal(classifyLiteralRiskProvenance("proof/ui/report.html"), "proof-artifact");
  assert.equal(
    classifyLiteralRiskProvenance("crates/enforcer-core/src/boundary/redaction.rs"),
    "detector-definition",
  );
  assert.equal(
    classifyLiteralRiskProvenance("crates/enforcer-lang-security/src/boundary/spec.rs"),
    "detector-definition",
  );
  assert.equal(classifyLiteralRiskProvenance("src/harness.mjs"), "detector-definition");
  assert.equal(classifyLiteralRiskProvenance("crates/core/src/key.rs"), "first-party");
});

test("compaction preserves genuine first-party hard findings and groups contextual findings", () => {
  const report = compactLiteralRiskReport({
    hardFindings: [
      finding("crates/core/src/key.rs"),
      finding("vendor/acme/example.rs"),
      finding("tests/fixtures/key.rs"),
    ],
    literalRisks: [finding("src/value.rs", "domain-literal")],
  });
  assert.deepEqual(report.hardFindings.map((item) => item.file), ["crates/core/src/key.rs"]);
  assert.equal(report.groupedFindings.length, 3);
  const mapped = mapLiteralRiskFindings({ report }, ".");
  assert.equal(mapped.filter((item) => item.severity === "error").length, 1);
  assert.equal(mapped.filter((item) => item.severity === "warning").length, 3);
});

test("grouped report size is bounded by provenance groups, not finding count", () => {
  const report = compactLiteralRiskReport({
    hardFindings: [],
    literalRisks: Array.from({ length: 10_000 }, (_, index) =>
      finding(`vendor/acme/example-${index}.rs`, "domain-literal"),
    ),
  });
  assert.equal(report.groupedFindings.length, 1);
  assert.equal(report.groupedFindings[0].count, 10_000);
  assert.equal(Buffer.byteLength(JSON.stringify(report)) < 4_096, true);
});
