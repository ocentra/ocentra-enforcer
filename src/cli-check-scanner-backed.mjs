import { collectFiles, normalizeRel } from "./path-utils.mjs";

function matchesExcludedPath(file, excludedPathTokens = [], excludedPathPatterns = []) {
  const value = String(file ?? "");
  return (
    excludedPathTokens.some((token) => value.includes(token)) ||
    excludedPathPatterns.some((pattern) => pattern.test(value))
  );
}

function collectFilteredFindings(report, { allowedRuleIds, excludedPathTokens = [], excludedPathPatterns = [] }) {
  const allowed = new Set(allowedRuleIds);
  const findings = [...(report.violations ?? []), ...(report.warnings ?? [])].filter(
    (finding) => allowed.has(finding.ruleId) && !matchesExcludedPath(finding.file, excludedPathTokens, excludedPathPatterns),
  );
  const waived = (report.waived ?? []).filter(
    (finding) => allowed.has(finding.ruleId) && !matchesExcludedPath(finding.file, excludedPathTokens, excludedPathPatterns),
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
  const effectiveScope =
    rawScope.mode === "all" && (excludedPathTokens?.length || excludedPathPatterns?.length)
      ? {
          mode: "files",
          files: collectFiles(
            root,
            [],
            config,
            (_file, rel) => !matchesExcludedPath(rel, excludedPathTokens, excludedPathPatterns),
            false,
          ).map((file) => normalizeRel(root, file)),
        }
      : rawScope;
  const report = deps.runEnforcerScan(
    {
      root,
      config,
      rawScope: effectiveScope,
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

/** Runs the scanner-backed naked-domain-string check. */
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

/** Dispatches a scanner-backed CLI check using the supplied dependencies. */
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
      excludedPathTokens: scannerBacked.excludedPathTokens,
      excludedPathPatterns: scannerBacked.excludedPathPatterns,
    },
    deps,
  );
}
