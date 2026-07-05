import path from "node:path";
import process from "node:process";
import { parseArchitectureCheckTokens } from "./cli-architecture.mjs";
import { runArchitecturePolicyCheck } from "./cli-check-routing.mjs";
import {
  runNoNakedDomainStringsCheck,
  runScannerBackedCheck,
} from "./cli-check-scanner-backed.mjs";

export function runEnforcerCheck(options = {}, deps) {
  const {
    DEFAULT_CONFIG,
    loadConfig,
    normalizeConfig,
    decodeCheckToolArguments,
    normalizeCheckName,
    runStandaloneCheck,
    SCANNER_BACKED_CHECKS,
    splitFindings,
    decorateRuleDocs,
    RULES,
    ruleDocFor,
  } = deps;
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ?? loadConfig(root, options.configPath, options.profile)),
  });
  const rawScope = options.rawScope ?? options.scope ?? { mode: "all" };
  const decoded = decodeCheckToolArguments({
    root,
    configPath: options.configPath ?? undefined,
    profile: options.profile ?? undefined,
    check: normalizeCheckName(options.checkName ?? options.check),
    scope: rawScope.mode === "all" ? "workspace" : rawScope.mode,
    files: rawScope.files ?? undefined,
    crateName: rawScope.crateName ?? undefined,
    base: rawScope.base ?? undefined,
    head: rawScope.head ?? undefined,
    checkConfigPath: options.checkConfigPath ?? undefined,
    output: options.output ?? undefined,
    dryRun: options.dryRun ?? undefined,
    staged: options.staged ?? undefined,
    tracked: options.tracked ?? undefined,
    strictEmptyTestTrees: options.strictEmptyTestTrees ?? undefined,
  });
  const checkName = decoded.check;
  const context = {
    options,
    root,
    config,
    rawScope,
    decoded,
    checkName,
  };
  if (checkName === "architecture-policy") {
    return runArchitecturePolicyCheck(context, deps);
  }
  if (checkName === "no-naked-domain-strings") {
    return runNoNakedDomainStringsCheck(context, deps);
  }
  if (SCANNER_BACKED_CHECKS[checkName]) {
    return runScannerBackedCheck(context, deps);
  }
  return decorateRuleDocs(
    runStandaloneCheck({
      checkName,
      root,
      config,
      args: {
        scope: rawScope,
        checkConfigPath: decoded.checkConfigPath,
        output: decoded.output,
        dryRun: decoded.dryRun,
        staged: decoded.staged,
        tracked: decoded.tracked,
        strictEmptyTestTrees: decoded.strictEmptyTestTrees,
        literalRiskMinScore: decoded.literalRiskMinScore,
        literalRiskIncludeLow: decoded.literalRiskIncludeLow,
        literalRiskIncludeIgnored: decoded.literalRiskIncludeIgnored,
        literalRiskIncludeUnknownCode: decoded.literalRiskIncludeUnknownCode,
        literalRiskRespectGitignore: decoded.literalRiskRespectGitignore,
        literalRiskMaxFileBytes: decoded.literalRiskMaxFileBytes,
        literalRiskFailAbove: decoded.literalRiskFailAbove,
        literalRiskHardCategories: decoded.literalRiskHardCategories,
        literalRiskHardRuleIds: decoded.literalRiskHardRuleIds,
      },
    }),
    { rulesById: RULES, ruleDocFor },
  );
}

export function runEnforcerVerify(options = {}, deps) {
  const {
    DEFAULT_CONFIG,
    loadConfig,
    normalizeConfig,
    normalizeVerifyMode,
    splitFindings,
    decorateRuleDocs,
    RULES,
    ruleDocFor,
  } = deps;
  const root = path.resolve(options.root ?? process.cwd());
  const config = normalizeConfig({
    ...DEFAULT_CONFIG,
    ...(options.config ?? loadConfig(root, options.configPath, options.profile)),
  });
  const rawScope = options.rawScope ?? options.scope ?? { mode: "all" };
  const verifyMode = normalizeVerifyMode(options.verifyMode ?? "local");
  const scanReport = deps.runEnforcerScan(
    {
      root,
      config,
      rawScope,
      command: "verify",
      scanOnly: true,
      languages: options.languages,
    },
    deps,
  );
  const checkNames = deps.VERIFY_MODE_CHECKS[verifyMode];
  const checkReports = checkNames.map((checkName) =>
    runEnforcerCheck(
      {
        root,
        config,
        rawScope,
        checkName,
        configPath: options.configPath,
        profile: options.profile,
      },
      deps,
    ),
  );
  const reports = [scanReport, ...checkReports];
  const findings = reports.flatMap((report) => [
    ...(report.violations ?? []),
    ...(report.warnings ?? []),
  ]);
  const waived = reports.flatMap((report) => report.waived ?? []);
  const { violations, warnings, bySeverity } = splitFindings(findings, config);
  return decorateRuleDocs(
    {
      ok: reports.every((report) => report.ok) && violations.length === 0,
      command: "verify",
      verifyMode,
      root,
      profileName: config.profileName ?? "strict",
      violations,
      warnings,
      waived,
      findings: [...findings, ...waived],
      bySeverity,
      scope: scanReport.scope,
      checks: reports.map((report) => ({
        command: report.command,
        check: report.check ?? report.command,
        ok: report.ok,
        violations: (report.violations ?? []).length,
        warnings: (report.warnings ?? []).length,
      })),
    },
    { rulesById: RULES, ruleDocFor },
  );
}

export function runArchitectureCli(tokens, deps) {
  if (tokens[0] !== "check") {
    throw new Error(
      "usage: ocentra-enforcer architecture check --language rust --scope <files|diff|all>",
    );
  }
  const args = parseArchitectureCheckTokens(tokens);
  return {
    json: args.json,
    result: runEnforcerCheck(
      {
        root: args.root,
        configPath: args.configPath,
        profile: args.profile,
        json: args.json,
        rawScope: args.rawScope,
        checkName: "architecture-policy",
      },
      deps,
    ),
  };
}
