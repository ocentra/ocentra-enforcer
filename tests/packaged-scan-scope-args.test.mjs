import assert from "node:assert/strict";
import test from "node:test";

import { parseArgs } from "../scripts/rust-rules-scan-core-args.mjs";

test("diff scope parses base and head into an executable scan request", () => {
  const args = parseArgs([
    "node",
    "ocentra-enforcer.mjs",
    "scan",
    "--root",
    "C:/fixture",
    "--base",
    "HEAD~1",
    "--head",
    "HEAD",
  ]);

  assert.deepEqual(args.scope, {
    mode: "diff",
    base: "HEAD~1",
    head: "HEAD",
  });
});

test("file scope remains higher priority than an accidental diff request", () => {
  const args = parseArgs([
    "node",
    "ocentra-enforcer.mjs",
    "scan",
    "--base",
    "HEAD~1",
    "--head",
    "HEAD",
    "--files",
    "src/lib.rs",
  ]);

  assert.deepEqual(args.scope, {
    mode: "files",
    files: ["src/lib.rs"],
  });
});
