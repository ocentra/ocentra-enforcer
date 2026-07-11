import type { Project } from "./enforcerAppData";

export type RuleSeverity = "error" | "warning" | "info";

export type RuleOverride = {
  ruleId: string;
  enabled: boolean;
  severity?: RuleSeverity;
  waiver?: { owner: string; reason: string };
};

export type ProjectRuleCoverage = {
  detectedLanguages: string[];
  catalogLanguages: string[];
  observedWithoutCatalog: string[];
  settingsStatus: string;
  rules: Array<{ ruleId: string; language: string; scope: "universal" | "language-match" | "not-detected"; effectiveSeverity: RuleSeverity; state: string; pathMatchStatus: "matched" | "no-match" | "invalid-pattern" | "unscoped"; matchedPathCount: number }>;
};

export type CatalogRule = {
  id: string;
  language: string;
  family: string;
  severity: RuleSeverity;
  title: string;
  snippet: string;
  lockLevel: "immutable" | "configurable" | string;
  canDisable: boolean;
  canDowngrade: boolean;
  waivable: boolean;
  requiresFailFixture: boolean;
  requiresPassFixture: boolean;
  appliesTo: string[];
  triggers: string[];
  validator: string;
  doc: string;
};

export function projectRuleLanguages(project: Project, catalog: CatalogRule[]): string[] {
  const policyLanguages = new Set(catalog.map((rule) => rule.language));
  return ["common", ...project.detectedLanguages.filter((language) => language !== "common" && policyLanguages.has(language))];
}

export function unsupportedProjectRuleLanguages(project: Project, catalog: CatalogRule[]): string[] {
  const policyLanguages = new Set(catalog.map((rule) => rule.language));
  return [...new Set(project.detectedLanguages.filter((language) => language !== "common" && !policyLanguages.has(language)))];
}

export function rulesForProject(project: Project, view: "universal" | "detected" | "all" | "overrides", overrides: RuleOverride[], catalog: CatalogRule[], coverage?: ProjectRuleCoverage) {
  const coverageByRule = new Map(coverage?.rules.map((row) => [row.ruleId, row]));
  const projectLanguages = coverage?.catalogLanguages.filter((language) => language === "common" || coverage.detectedLanguages.includes(language)) ?? projectRuleLanguages(project, catalog);
  const rows = catalog.filter((rule) => {
    if (view === "all") return true;
    if (view === "universal") return rule.language === "common";
    if (view === "overrides") return overrides.some((override) => override.ruleId === rule.id);
    if (coverage) {
      const scope = coverageByRule.get(rule.id)?.scope;
      return scope === "universal" || scope === "language-match";
    }
    return projectLanguages.includes(rule.language);
  });

  return rows.map((rule) => ({
    ...rule,
    override: overrides.find((item) => item.ruleId === rule.id),
    effectiveSeverity: coverageByRule.get(rule.id)?.effectiveSeverity ?? overrides.find((item) => item.ruleId === rule.id)?.severity ?? rule.severity,
    coverage: coverageByRule.get(rule.id),
  }));
}

export function ruleFamilySummary(rules: CatalogRule[]) {
  const families = new Map<string, { language: string; family: string; count: number; blocking: number; configurable: number }>();
  for (const rule of rules) {
    const key = `${rule.language}:${rule.family}`;
    const current = families.get(key) ?? { language: rule.language, family: rule.family, count: 0, blocking: 0, configurable: 0 };
    current.count += 1;
    if (rule.severity === "error") current.blocking += 1;
    if (rule.canDisable || rule.canDowngrade) current.configurable += 1;
    families.set(key, current);
  }
  return [...families.entries()]
    .map(([id, family]) => ({ id, ...family }))
    .sort((left, right) => right.count - left.count || left.family.localeCompare(right.family));
}

export function ruleSource(rule: CatalogRule) {
  if (rule.lockLevel === "immutable") return "harness invariant";
  if (rule.language === "common") return "universal policy";
  return `${rule.language} policy`;
}
