import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { cargoFmtBatches } from "../scripts/check-cargo-workspace-members-format.mjs";
import { validateCargoWorkspaceMembers } from "../scripts/check-cargo-workspace-members-validation.mjs";

function fixture() {
  const root = mkdtempSync(path.join(os.tmpdir(), "cargo-workspace-members-"));
  const manifests = ["crates/enforcer-a/Cargo.toml", "crates/enforcer-b/Cargo.toml", "xtask/Cargo.toml"];
  for (const manifest of manifests) {
    const absolute = path.join(root, manifest);
    mkdirSync(path.dirname(absolute), { recursive: true });
    writeFileSync(
      absolute,
      "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n\n[lints]\nworkspace = true\n",
    );
  }
  const packages = manifests.map((manifest, index) => ({
    id: `product-${index}`,
    name: `product-${index}`,
    manifest_path: path.join(root, manifest),
  }));
  return { root, manifests, packages };
}

test("accepts all and only top-level product workspace packages", (t) => {
  const { root, packages } = fixture();
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const ids = packages.map((pkg) => pkg.id);
  const result = validateCargoWorkspaceMembers(root, {
    packages,
    workspace_members: ids,
    workspace_default_members: ids,
  });
  assert.equal(result.ok, true, result.errors.join("\n"));
});

test("rejects a product package that drops the workspace lint opt-in", (t) => {
  const { root, manifests, packages } = fixture();
  t.after(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(
    path.join(root, manifests[1]),
    "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
  );
  const ids = packages.map((pkg) => pkg.id);
  const result = validateCargoWorkspaceMembers(root, {
    packages,
    workspace_members: ids,
    workspace_default_members: ids,
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /missing \[lints\] workspace = true/u);
  assert.match(result.errors.join("\n"), /crates\/enforcer-b\/Cargo.toml/u);
});

test("rejects a missing product package", (t) => {
  const { root, packages } = fixture();
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const ids = packages.slice(0, 2).map((pkg) => pkg.id);
  const result = validateCargoWorkspaceMembers(root, {
    packages,
    workspace_members: ids,
    workspace_default_members: ids,
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /product packages missing from workspace/);
});

test("rejects a vendored package admitted by Cargo", (t) => {
  const { root, packages } = fixture();
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const vendorManifest = path.join(root, "crates/enforcer-memory/vendor/tree-sitter-fixture/Cargo.toml");
  const vendor = { id: "vendor", name: "vendor", manifest_path: vendorManifest };
  const ids = [...packages.map((pkg) => pkg.id), vendor.id];
  const result = validateCargoWorkspaceMembers(root, {
    packages: [...packages, vendor],
    workspace_members: ids,
    workspace_default_members: ids,
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /vendored packages entered workspace/);
});

test("rejects default members that differ from product workspace members", (t) => {
  const { root, packages } = fixture();
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const ids = packages.map((pkg) => pkg.id);
  const result = validateCargoWorkspaceMembers(root, {
    packages,
    workspace_members: ids,
    workspace_default_members: ids.slice(0, -1),
  });
  assert.equal(result.ok, false);
  assert.match(result.errors.join("\n"), /product packages missing from defaults/);
});

test("builds bounded fmt batches containing every product package once", () => {
  const packages = Array.from({ length: 19 }, (_, index) => ({ name: `product-${index}` }));
  const batches = cargoFmtBatches(packages, 8);
  assert.deepEqual(batches.map((batch) => batch.length), [8, 8, 3]);
  assert.deepEqual(batches.flat(), packages.map((pkg) => pkg.name));
});
