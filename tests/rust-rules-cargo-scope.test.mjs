import test from "node:test";
import assert from "node:assert/strict";

import { cargoFmtArgs } from "../scripts/rust-rules-cargo-gates.mjs";
import { cargoFmtCommand } from "../scripts/rust-rules-cargo-gates-build.mjs";

test("crate cargo fmt gate stays inside the selected package", () => {
  assert.deepEqual(cargoFmtArgs({ mode: "crate", crateName: "enforcer-security" }), [
    "fmt",
    "--package",
    "enforcer-security",
    "--check",
  ]);
});

test("workspace cargo fmt gate is explicit about all packages", () => {
  assert.deepEqual(cargoFmtArgs({ mode: "workspace" }), ["fmt", "--all", "--check"]);
});

test("workspace cargo fmt gate uses bounded package batches when the helper exists", () => {
  const command = cargoFmtCommand(process.cwd(), ["fmt", "--all", "--check"]);
  assert.equal(command.command, process.execPath);
  assert.deepEqual(command.args.slice(-1), ["--fmt-check"]);
});

test("cargo fmt falls back to the direct invocation outside this repository", () => {
  const command = cargoFmtCommand("C:\\missing-enforcer-project", ["fmt", "--all", "--check"]);
  assert.equal(command.command, "cargo");
  assert.deepEqual(command.args, ["fmt", "--all", "--check"]);
});
