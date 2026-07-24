import test from "node:test";
import assert from "node:assert/strict";

import { summarizeCommandOutput } from "../scripts/rust-rules-cargo-command.mjs";
import { diagnosticSourceLines } from "../scripts/rust-rules-output-check.mjs";

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

test("human diagnostics retain both the beginning and terminal failure lines", () => {
  const source = `${"compile line\n".repeat(40)}error: parser test failed`;
  const lines = diagnosticSourceLines(source);

  assert.equal(lines.length, 13);
  assert.match(lines[0], /compile line/u);
  assert.match(lines[6], /diagnostic source truncated; tail preserved/u);
  assert.match(lines.at(-1), /error: parser test failed/u);
});
