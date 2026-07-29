import assert from "node:assert/strict";
import test from "node:test";
import { withCachedArchitectureReports } from "../src/cli-architecture-scan-cache.mjs";
test("architecture report cache reuses equivalent scans", () => {
  let scans = 0;
  let generic = 0;
  const cached = withCachedArchitectureReports({ runEnforcerScan: () => ({ id: ++scans }), runGenericScan: (options) => ({ id: ++generic, options }) });
  assert.equal(cached.runEnforcerScan({ root: "x", rawScope: { mode: "files" }, languages: ["rust"] }).id, 1);
  assert.equal(cached.runEnforcerScan({ root: "x", rawScope: { mode: "files" }, languages: ["rust"] }).id, 1);
  const first = cached.runGenericScan({ root: "x", scope: { mode: "files" } });
  assert.strictEqual(first, cached.runGenericScan({ root: "x", scope: { mode: "files" } }));
  assert.equal(generic, 1);
  assert.equal(first.options.sourceOnly, true);
});
