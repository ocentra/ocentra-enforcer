import { readdirSync } from "node:fs";
import { join } from "node:path";

export function discoverTargets(repoRoot) {
  const testsRoot = join(repoRoot, "crates", "enforcer-memory", "tests");
  return readdirSync(testsRoot, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".rs"))
    .map((entry) => entry.name.slice(0, -".rs".length))
    .sort((left, right) => left.localeCompare(right));
}

export function classifyTarget(target) {
  if (target.startsWith("unit_languages_")) return "unit-languages";
  if (target.startsWith("unit_")) return "unit-core";
  if (target.startsWith("model_") || target === "local_runtime") return "model-runtime";
  if (target.startsWith("x06_") || target === "feature_parity_harness") return "x06-proof";
  if (target.startsWith("parity_") || target === "mcp_cli_live") return "parity-live";
  return "integration";
}

export function selectTargets(targets, args) {
  const filtered = args.only ? targets.filter((target) => target.startsWith(args.only)) : targets;
  if (!args.shard) return filtered;
  return filtered.filter((_, zeroIndex) => zeroIndex % args.shard.total === args.shard.index - 1);
}

export function countByCategory(targets) {
  const byCategory = {};
  for (const target of targets) {
    const category = classifyTarget(target);
    byCategory[category] = (byCategory[category] ?? 0) + 1;
  }
  return byCategory;
}
