import path from "node:path";
import process from "node:process";

export function resolveScanLanguages(optionLanguages, config) {
  const languages = optionLanguages ?? config.languages ?? [
    "rust",
    "typescript",
    "python",
    "common",
  ];
  const allowed = new Set(["rust", "typescript", "python", "common", "iac"]);
  const normalized = languages
    .map((language) => String(language).trim())
    .filter(Boolean);
  for (const language of normalized) {
    if (!allowed.has(language))
      throw new Error(`Unknown scan language: ${language}`);
  }
  return normalized.length > 0
    ? normalized
    : ["rust", "typescript", "python", "common"];
}

export function runRustRules(options = {}, deps) {
  const {
    DEFAULT_CONFIG,
    loadConfig,
    normalizeConfig,
    resolveScope,
    runScanner,
    runCargoGates,
    applyPolicyAndWaivers,
    policyPreflightFindings,
    normalizeRel,
    decorateRuleDocs,
    RULES,
    ruleDocFor,
  } = deps;
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ?? loadConfig(root, options.configPath, options.profile)),
  });
  const scope = resolveScope(root, config, options.scope ?? { mode: "all" });
  const command = options.command ?? "scan";
  const scannerViolations = runScanner(root, config, scope);
  const cargoViolations =
    options.scanOnly || command === "scan"
      ? []
      : runCargoGates(root, config, scope);
  const { violations, warnings, waived, findings, bySeverity } =
    applyPolicyAndWaivers(
      [
        ...policyPreflightFindings(root, config, options),
        ...scannerViolations,
        ...cargoViolations,
      ],
      config,
    );
  return decorateRuleDocs(
    {
      ok: violations.length === 0,
      command,
      violations,
      warnings,
      waived,
      findings,
      bySeverity,
      failOn: config.failOn,
      root,
      profileName: config.profileName,
      scanOnly: Boolean(options.scanOnly || command === "scan"),
      scope: {
        ...scope,
        files: scope.files.map((file) => normalizeRel(root, file)),
      },
    },
    { rulesById: RULES, ruleDocFor },
  );
}

export function runEnforcerScan(options = {}, deps) {
  const {
    DEFAULT_CONFIG,
    loadConfig,
    normalizeConfig,
    runGenericScan,
    applyPolicyAndWaivers,
    policyPreflightFindings,
    splitFindings,
    uniqueSorted,
    decorateRuleDocs,
    RULES,
    ruleDocFor,
  } = deps;
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ?? loadConfig(root, options.configPath, options.profile)),
  });
  const activeLanguages = resolveScanLanguages(options.languages, config);
  const rawScope = options.rawScope ?? options.scope ?? { mode: "all" };
  const resolvedScope = deps.resolveScope(root, config, rawScope);
  const genericScope = rawScope.mode === "crate" ? resolvedScope : rawScope;
  const rustReport = activeLanguages.includes("rust")
    ? runRustRules(
        {
          root,
          config,
          scope: resolvedScope,
          command: options.command ?? "scan",
          scanOnly: options.scanOnly,
        },
        deps,
      )
    : {
        ok: true,
        command: options.command ?? "scan",
        violations: [],
        root,
        profileName: config.profileName,
        scanOnly: Boolean(options.scanOnly || options.command === "scan"),
        scope: { ...resolvedScope, files: [] },
      };
  const genericLanguages = activeLanguages.filter(
    (language) => language !== "rust",
  );
  const genericReport =
    genericLanguages.length === 0
      ? { files: [], violations: [] }
      : runGenericScan({
          root,
          scope: genericScope,
          config,
          languages: genericLanguages,
        });
  const genericPolicy = applyPolicyAndWaivers(
    [
      ...(activeLanguages.includes("rust")
        ? []
        : policyPreflightFindings(root, config, options)),
      ...genericReport.violations,
    ],
    config,
  );
  const findings = [
    ...(rustReport.violations ?? []),
    ...(rustReport.warnings ?? []),
    ...genericPolicy.violations,
    ...genericPolicy.warnings,
  ];
  const waived = [...(rustReport.waived ?? []), ...genericPolicy.waived];
  const { violations, warnings, bySeverity } = splitFindings(findings, config);
  const scopeFiles = uniqueSorted([
    ...(rustReport.scope.files ?? []),
    ...genericReport.files,
  ]);
  return decorateRuleDocs(
    {
      ...rustReport,
      ok: violations.length === 0,
      command: options.command ?? rustReport.command,
      violations,
      warnings,
      waived,
      findings: [...findings, ...waived],
      bySeverity,
      failOn: config.failOn,
      languages: activeLanguages,
      scope: { ...rustReport.scope, files: scopeFiles },
    },
    { rulesById: RULES, ruleDocFor },
  );
}
