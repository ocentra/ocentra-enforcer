import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { decodeRuleRegistry } from "../schemas/effect/enforcer-schemas.mjs";
import { RULE_REGISTRY_PATH, uniqueSorted } from "./rust-rules-mcp-helpers.mjs";
import { buildRouteSpec } from "./rust-rules-mcp-route-shared.mjs";

export function routeRules(args) {
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadRuleRegistry();
  const profileName = resolveProfileName(root, args);
  const explicitRuleId = args.ruleId?.toUpperCase() ?? null;
  const routeSpec = explicitRuleId ? null : buildRouteSpec(args);
  const rules = explicitRuleId
    ? registry.rules.filter((rule) => rule.id === explicitRuleId)
    : collectRoutedRules(registry.rules, routeSpec);
  const docs = uniqueSorted(rules.map((rule) => rule.doc));

  return {
    ok: true,
    productName: registry.productName,
    profileName,
    index: "rules/INDEX.md",
    scope: describeRouteScope(args),
    docs,
    rules: rules.map((rule) => ({
      id: rule.id,
      family: rule.family,
      severity: rule.severity,
      doc: rule.doc,
      validator: rule.validator,
    })),
  };
}

function loadRuleRegistry() {
  return decodeRuleRegistry(
    JSON.parse(fs.readFileSync(RULE_REGISTRY_PATH, "utf8")),
  );
}

function resolveProfileName(root, args) {
  if (args.configPath) {
    const configPath = path.isAbsolute(args.configPath)
      ? args.configPath
      : path.join(root, args.configPath);
    if (!fs.existsSync(configPath)) {
      return "custom";
    }
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8"));
    return parsed.profileName ?? "custom";
  }
  return args.profile ?? "strict";
}

function collectRoutedRules(rules, routeSpec) {
  const primaryRules = rules.filter((rule) => {
    return (
      routeSpec.languageFamilies.has(rule.family) &&
      routeSpec.languages.has(rule.language)
    );
  });
  const commonRules = rules.filter((rule) => {
    return (
      rule.language === "common" &&
      routeSpec.commonFamilies.has(rule.family)
    );
  });
  return primaryRules.concat(commonRules);
}

function describeRouteScope(args) {
  if (args.ruleId) {
    return { mode: "rule", ruleId: args.ruleId.toUpperCase() };
  }
  if (args.scope === "crate") {
    return { mode: "crate", crateName: args.crateName ?? null };
  }
  if (args.scope === "diff") {
    return {
      mode: "diff",
      base: args.base ?? null,
      head: args.head ?? null,
      files: args.files ?? [],
    };
  }
  if (args.scope === "workspace") {
    return { mode: "workspace" };
  }
  return { mode: "files", files: args.files ?? [] };
}
