import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { decodeRuleRegistry } from "../schemas/effect/enforcer-schemas.mjs";

const DEFAULT_PACK_ROOT = path.resolve(
  path.join(path.dirname(fileURLToPath(import.meta.url)), ".."),
);

const registryCache = new Map();

export function loadRuleRegistry(packRoot = DEFAULT_PACK_ROOT) {
  const root = path.resolve(packRoot);
  const cached = registryCache.get(root);
  if (cached) return cached;
  const registryPath = path.join(root, "rules", "rules.json");
  const registry = fs.existsSync(registryPath)
    ? decodeRuleRegistry(JSON.parse(fs.readFileSync(registryPath, "utf8")))
    : { rules: [] };
  registryCache.set(root, registry);
  return registry;
}

export function registryRules(packRoot = DEFAULT_PACK_ROOT) {
  return loadRuleRegistry(packRoot).rules ?? [];
}

export function registryRuleMap(packRoot = DEFAULT_PACK_ROOT) {
  return new Map(registryRules(packRoot).map((rule) => [rule.id, rule]));
}

export function registryRule(ruleId, packRoot = DEFAULT_PACK_ROOT) {
  return registryRuleMap(packRoot).get(String(ruleId ?? "").toUpperCase()) ?? null;
}

export function enrichFindingMetadata(
  finding,
  packRoot = DEFAULT_PACK_ROOT,
  fallback = {},
) {
  const ruleId = String(finding.ruleId ?? "").toUpperCase();
  const rule = registryRule(ruleId, packRoot) ?? fallback[ruleId] ?? {};
  return {
    ...finding,
    ruleId,
    severity: finding.severity ?? rule.severity ?? "error",
    title: finding.title ?? rule.title ?? "Unknown rule",
    snippet: finding.snippet ?? rule.snippet ?? "",
    doc: finding.doc ?? rule.doc ?? undefined,
  };
}

export function enrichFindingsMetadata(
  findings,
  packRoot = DEFAULT_PACK_ROOT,
  fallback = {},
) {
  return findings.map((finding) =>
    enrichFindingMetadata(finding, packRoot, fallback),
  );
}
