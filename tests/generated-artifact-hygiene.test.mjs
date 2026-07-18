import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function isIgnored(relativePath) {
  const result = spawnSync(
    "git",
    ["check-ignore", "--no-index", "--quiet", "--", relativePath],
    { cwd: ROOT, encoding: "utf8", shell: false },
  );
  assert.equal(result.error, undefined, result.stderr);
  assert.equal([0, 1].includes(result.status), true, result.stderr);
  return result.status === 0;
}

test("generated Cargo and packet directories stay out of integration", () => {
  for (const relativePath of [
    "target-proof/debug/output.bin",
    "crates/example/target-proof/debug/output.bin",
    "target-proof-report.json",
    "crates/example/target-proof-report.json",
    ".tmp-proof/report.json",
    "crates/example/.tmp-proof/report.json",
  ]) {
    assert.equal(isIgnored(relativePath), true, relativePath);
  }
});

test("legitimate target and temporary-like names remain visible", () => {
  for (const relativePath of [
    "targeting/src/lib.rs",
    "crates/example/targeting/src/lib.rs",
    "target-proof.rs",
    "crates/example/target-proof.rs",
    ".tmpkeeper/report.json",
  ]) {
    assert.equal(isIgnored(relativePath), false, relativePath);
  }
});
