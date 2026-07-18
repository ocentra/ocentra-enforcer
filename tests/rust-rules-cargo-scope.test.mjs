import test from "node:test";
import assert from "node:assert/strict";

import { cargoFmtArgs } from "../scripts/rust-rules-cargo-gates.mjs";

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
