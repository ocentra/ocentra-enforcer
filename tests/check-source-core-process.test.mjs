import assert from "node:assert/strict";
import test from "node:test";
import {
  compactProcessOutput,
  spawnInRoot,
} from "../scripts/check-source-core-helpers.mjs";

test("spawnInRoot captures metadata-sized child output", () => {
  const expectedBytes = 2 * 1024 * 1024;
  const result = spawnInRoot(process.cwd(), process.execPath, [
    "-e",
    `process.stdout.write("x".repeat(${expectedBytes}))`,
  ]);

  assert.equal(result.status, 0, result.error?.message);
  assert.equal(result.stdout.length, expectedBytes);
});

test("compactProcessOutput preserves process errors as readable text", () => {
  const error = new Error("spawnSync cargo ENOBUFS");
  error.code = "ENOBUFS";
  const output = compactProcessOutput({ error, status: null });

  assert.match(output, /status=unknown/u);
  assert.match(output, /ENOBUFS/u);
  assert.doesNotMatch(output, /\[object Object\]/u);
});
