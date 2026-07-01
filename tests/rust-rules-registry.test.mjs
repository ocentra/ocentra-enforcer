import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { decodeRuleRegistry } from "../schemas/effect/enforcer-schemas.mjs";
import { CHECK_RULES } from "../src/checks.mjs";
import { GENERIC_RULES } from "../src/generic-scanners.mjs";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const REGISTRY_PATH = path.join(PACK_ROOT, "rules", "rules.json");
const SCRIPT_PATH = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");

function loadRegistry() {
  return decodeRuleRegistry(JSON.parse(fs.readFileSync(REGISTRY_PATH, "utf8")));
}

function scannerRuleIds() {
  const source = fs.readFileSync(SCRIPT_PATH, "utf8");
  return [
    ...new Set(
      [...source.matchAll(/['"]RR-[0-9]+\.[0-9]+['"]/gu)].map(
        (match) => match[0].slice(1, -1),
      ),
    ),
  ].sort();
}

test("registry has the expected schema shape", () => {
  const registry = loadRegistry();
  assert.equal(registry.schemaVersion, 2);
  assert.equal(registry.productName, "ocentra-enforcer");
  assert.deepEqual(registry.languages, [
    "rust",
    "typescript",
    "python",
    "common",
  ]);
  assert.ok(Array.isArray(registry.rules));

  for (const rule of registry.rules) {
    assert.match(
      rule.id,
      /^(RR|TS|PY|SEC|GEN|DOC|DOCENF|HAR|TEST|PORT|SRC|CONTRACT|DEP|NPM|CI|REPO|SBOM|AI|ENF|CFG|WAIVER|ARCH|BOUND|MCP|PROOF|SCAN)-[0-9]+\.[0-9]+$/u,
    );
    assert.ok(
      ["rust", "typescript", "python", "common"].includes(rule.language),
    );
    assert.ok(
      [
        "source",
        "domain",
        "imports-modules",
        "toolchain-cargo",
        "dependencies",
        "async-runtime",
        "tests",
        "toolchain",
        "security",
        "generated-artifacts",
        "documentation",
        "harness",
        "mcp",
        "portability",
        "proof",
        "registry",
        "source-shape",
        "contracts",
        "sbom",
        "agent-rules",
        "ci",
        "repo",
        "package",
        "scanner",
      ].includes(rule.family),
    );
    assert.ok(["error", "warning", "info"].includes(rule.severity));
    assert.ok(Array.isArray(rule.appliesTo) && rule.appliesTo.length > 0);
    assert.ok(Array.isArray(rule.triggers) && rule.triggers.length > 0);
    assert.equal(typeof rule.validator, "string");
    assert.equal(typeof rule.doc, "string");
    assert.equal(typeof rule.title, "string");
    assert.equal(typeof rule.snippet, "string");
    assert.ok(
      [
        "immutable",
        "waiver-required",
        "profile-overridable",
        "advisory",
      ].includes(rule.lockLevel),
    );
    assert.equal(typeof rule.canDisable, "boolean");
    assert.equal(typeof rule.canDowngrade, "boolean");
    assert.equal(typeof rule.requiresFailFixture, "boolean");
    assert.equal(typeof rule.requiresPassFixture, "boolean");
  }
});

test("registry exactly covers enforced scanner rules", () => {
  const registryIds = loadRegistry()
    .rules.map((rule) => rule.id)
    .sort();
  const duplicateIds = registryIds.filter(
    (id, index) => registryIds.indexOf(id) !== index,
  );
  assert.deepEqual(duplicateIds, []);
  assert.deepEqual(
    registryIds,
    [
      ...scannerRuleIds(),
      ...Object.keys(GENERIC_RULES),
      ...Object.keys(CHECK_RULES),
    ].sort(),
  );
});

test("registry docs exist and stay inside implemented language docs", () => {
  for (const rule of loadRegistry().rules) {
    assert.match(rule.doc, /^rules\/(?:rust|typescript|python|common)\//u);
    const [docPath, anchor] = rule.doc.split("#");
    assert.equal(
      fs.existsSync(path.join(PACK_ROOT, docPath)),
      true,
      `${rule.id} doc is missing: ${rule.doc}`,
    );
    assert.equal(Boolean(anchor), true, `${rule.id} doc anchor is missing`);
    const docText = fs.readFileSync(path.join(PACK_ROOT, docPath), "utf8");
    assert.match(
      docText,
      /^#{1,6}\s+Covered Rules$/mu,
      `${rule.id} doc anchor is missing: ${rule.doc}`,
    );
  }
});
