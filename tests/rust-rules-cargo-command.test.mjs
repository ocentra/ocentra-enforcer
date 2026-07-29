import test from "node:test";
import assert from "node:assert/strict";

import { summarizeCommandOutput } from "../scripts/rust-rules-cargo-command.mjs";
import { diagnosticSourceLines } from "../scripts/rust-rules-output-check.mjs";
import {
  cargoBuildBatchArgs,
  compactCargoDiagnostic,
} from "../scripts/check-cargo-workspace-test-process.mjs";

test("bounded Cargo runner builds runnable binaries before cross-package fixtures", () => {
  assert.deepEqual(
    cargoBuildBatchArgs({
      packageName: "enforcer-cli",
      selectorArgs: ["--bin", "enforcer"],
    }),
    [
      "build",
      "--locked",
      "--package",
      "enforcer-cli",
      "--bin",
      "enforcer",
      "--all-features",
    ],
  );
});

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

test("bounded Cargo target diagnostics retain child termination errors", () => {
  const diagnostic = compactCargoDiagnostic({
    stdout: "",
    stderr: "",
    status: null,
    signal: "SIGTERM",
    error: { message: "spawn cargo ENOBUFS" },
  });
  assert.match(diagnostic, /cargo status: unknown; signal: SIGTERM/u);
  assert.match(diagnostic, /spawn cargo ENOBUFS/u);
});

test("bounded Cargo target diagnostics retain stdout failures beside long stderr", () => {
  const diagnostic = compactCargoDiagnostic({
    stdout: "test parser_contract ... FAILED\nchild parser exited with signal 11",
    stderr: `${"compiler warning\n".repeat(800)}terminal compiler warning`,
    status: 101,
    signal: null,
  });
  assert.match(diagnostic, /cargo status: 101/u);
  assert.match(diagnostic, /stdout:\ntest parser_contract \.\.\. FAILED/u);
  assert.match(diagnostic, /child parser exited with signal 11/u);
  assert.match(diagnostic, /stderr truncated; tail preserved/u);
  assert.match(diagnostic, /terminal compiler warning/u);
});
