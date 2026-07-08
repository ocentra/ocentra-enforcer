function collectFilteredFindings(report, { allowedRuleIds, excludedPathTokens = [], excludedPathPatterns = [] }) {
  const allowed = new Set(allowedRuleIds);
  const matchesExcludedPath = (finding) => {
    const file = String(finding.file ?? "");
    return (
      excludedPathTokens.some((token) => file.includes(token)) ||
      excludedPathPatterns.some((pattern) => pattern.test(file))
    );
  };
  const findings = [...(report.violations ?? []), ...(report.warnings ?? [])].filter(
    (finding) => allowed.has(finding.ruleId) && !matchesExcludedPath(finding),
  );
  const waived = (report.waived ?? []).filter(
    (finding) => allowed.has(finding.ruleId) && !matchesExcludedPath(finding),
  );
  return { findings, waived };
}

function buildFilteredReport({ checkName, report, findings, waived, languages, config }, deps) {
  const { violations, warnings, bySeverity } = deps.splitFindings(findings, config);
  return deps.decorateRuleDocs(
    {
      ok: violations.length === 0,
      command: "check",
      check: checkName,
      root: report.root,
      profileName: report.profileName,
      violations,
      warnings,
      waived,
      findings: [...findings, ...waived],
      bySeverity,
      scope: report.scope,
      languages,
    },
    { rulesById: deps.RULES, ruleDocFor: deps.ruleDocFor },
  );
}

function runFilteredScannerCheck({
  checkName,
  config,
  rawScope,
  root,
  scannerLanguages,
  allowedRuleIds,
  excludedPathTokens,
  excludedPathPatterns,
}, deps) {
  const report = deps.runEnforcerScan(
    {
      root,
      config,
      rawScope,
      command: "check",
      scanOnly: true,
      languages: scannerLanguages,
    },
    deps,
  );
  const { findings, waived } = collectFilteredFindings(report, {
    allowedRuleIds,
    excludedPathTokens,
    excludedPathPatterns,
  });
  return buildFilteredReport(
    {
      checkName,
      report,
      findings,
      waived,
      languages: scannerLanguages,
      config,
    },
    deps,
  );
}

export function runNoNakedDomainStringsCheck(context, deps) {
  return runFilteredScannerCheck(
    {
      checkName: context.checkName,
      config: context.config,
      rawScope: context.rawScope,
      root: context.root,
      scannerLanguages: ["rust", "typescript", "python", "common"],
      allowedRuleIds: ["RR-6.1", "RR-6.5", "RR-18.16", "TS-1.3", "PY-1.3"],
      excludedPathTokens: ["/generated/", "\\generated\\"],
      excludedPathPatterns: [/(?:^|[/\\])generated-[^/\\]+\.(?:ts|tsx|js|jsx|mjs|cjs)$/u],
    },
    deps,
  );
}

export function runScannerBackedCheck(context, deps) {
  const scannerBacked = deps.SCANNER_BACKED_CHECKS[context.checkName];
  return runFilteredScannerCheck(
    {
      checkName: context.checkName,
      config: context.config,
      rawScope: context.rawScope,
      root: context.root,
      scannerLanguages: scannerBacked.languages,
      allowedRuleIds: scannerBacked.ruleIds,
    },
    deps,
  );
}
