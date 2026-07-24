import test from "node:test";
import assert from "node:assert/strict";

import { summarizeCommandOutput } from "../scripts/rust-rules-cargo-command.mjs";

test("cargo diagnostics preserve short output verbatim", () => {
  const output = "compiler error: missing symbol";
  assert.equal(summarizeCommandOutput(output), output);
});

test("cargo diagnostics retain the terminal failure after long compile output", () => {
  const output = `${"compile line\n".repeat(800)}error: linker failed: missing symbol`;
  const summary = summarizeCommandOutput(output);

  assert.equal(summary.length <= 4000, true);
  assert.match(summary, /compile line/u);
  assert.match(summary, /error: linker failed: missing symbol/u);
  assert.match(summary, /tail preserved/u);
});
