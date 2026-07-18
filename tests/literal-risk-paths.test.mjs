import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { resolveLiteralScannerLayout } from "../src/literal-risk.mjs";

function makePack(relativeManifest) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "literal-risk-layout-"));
  const manifest = path.join(root, relativeManifest);
  fs.mkdirSync(path.dirname(manifest), { recursive: true });
  fs.writeFileSync(manifest, "[package]\nname = \"fixture\"\n", "utf8");
  return root;
}

test("literal-risk resolves the canonical workspace crate and target", () => {
  const root = makePack("crates/enforcer-literal-scan/Cargo.toml");
  const layout = resolveLiteralScannerLayout(root, {});

  assert.equal(layout.root, path.join(root, "crates", "enforcer-literal-scan"));
  assert.equal(layout.manifest, path.join(layout.root, "Cargo.toml"));
  assert.equal(layout.target, path.join(root, "target"));
});

test("literal-risk honors a workspace-relative Cargo target directory", () => {
  const root = makePack("crates/enforcer-literal-scan/Cargo.toml");
  const layout = resolveLiteralScannerLayout(root, {
    CARGO_TARGET_DIR: "target-literal-risk",
  });

  assert.equal(layout.target, path.join(root, "target-literal-risk"));
});

test("literal-risk rejects the removed Tools compatibility layout", () => {
  const root = makePack("Tools/ocentra-literal-scan/Cargo.toml");

  assert.throws(
    () => resolveLiteralScannerLayout(root, {}),
    /crates[\\/]enforcer-literal-scan[\\/]Cargo\.toml/u,
  );
});
