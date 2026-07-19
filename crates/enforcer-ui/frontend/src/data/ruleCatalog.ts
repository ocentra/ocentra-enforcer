import type { Project } from "./enforcerAppData";
import type { UiCount, UiFlag, UiMaybe, UiTextList } from "./enforcerAppData";

export type RuleSeverity = "error" | "warning" | "info";

export type RuleOverride = {
  ruleId: string;
  enabled: UiFlag;
  severity: UiMaybe<RuleSeverity>;
  waiver: UiMaybe<{ owner: string; reason: string }>;
};

export type ProjectRuleCoverage = {
  detectedLanguages: UiTextList;
  catalogLanguages: UiTextList;
  observedWithoutCatalog: UiTextList;
  settingsStatus: string;
  rules: Array<{ ruleId: string; language: string; scope: "universal" | "language-match" | "not-detected"; effectiveSeverity: RuleSeverity; state: string; pathMatchStatus: "matched" | "no-match" | "invalid-pattern" | "unscoped"; matchedPathCount: UiCount }>;
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
  appliesTo: UiTextList;
  triggers: UiTextList;
  validator: string;
  doc: string;
};

type ProjectCatalogRule = CatalogRule & {
  override: UiMaybe<RuleOverride>;
  effectiveSeverity: RuleSeverity;
  coverage: UiMaybe<ProjectRuleCoverage["rules"][number]>;
};

type RuleFamilySummary = {
  id: string;
  language: string;
  family: string;
  count: UiCount;
  blocking: UiCount;
  configurable: UiCount;
};

export function projectRuleLanguages(project: Project, catalog: CatalogRule[]): UiTextList {
  const policyLanguages = new Set(catalog.map((rule) => rule.language));
  return ["common", ...project.detectedLanguages.filter((language) => language !== "common" && policyLanguages.has(language))];
}

export function unsupportedProjectRuleLanguages(project: Project, catalog: CatalogRule[]): UiTextList {
  const policyLanguages = new Set(catalog.map((rule) => rule.language));
  return [...new Set(project.detectedLanguages.filter((language) => language !== "common" && !policyLanguages.has(language)))];
}

export function rulesForProject(project: Project, view: "universal" | "detected" | "all" | "overrides", overrides: RuleOverride[], catalog: CatalogRule[], coverage: UiMaybe<ProjectRuleCoverage>): ProjectCatalogRule[] {
  const coverageByRule = new Map<ProjectRuleCoverage["rules"][number]["ruleId"], ProjectRuleCoverage["rules"][number]>();
  for (const row of coverage?.rules ?? []) coverageByRule.set(row.ruleId, row);
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

export function ruleFamilySummary(rules: CatalogRule[]): RuleFamilySummary[] {
  type RuleFamilyKey = `${string}:${string}`;
  const families = new Map<RuleFamilyKey, { language: string; family: string; count: UiCount; blocking: UiCount; configurable: UiCount }>();
  for (const rule of rules) {
    const key: RuleFamilyKey = `${rule.language}:${rule.family}`;
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

export function ruleSource(rule: CatalogRule): string {
  if (rule.lockLevel === "immutable") return "harness invariant";
  if (rule.language === "common") return "universal policy";
  return `${rule.language} policy`;
}
