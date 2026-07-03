export function runArchitecturePolicyCheck(context, deps) {
  const checks =
    context.config.architecturePolicyChecks ??
    deps.DEFAULT_ARCHITECTURE_POLICY_CHECKS;
  const reports = checks
    .filter((check) => check !== "architecture-policy")
    .map((check) =>
      deps.runEnforcerCheck(
        {
          ...context.options,
          root: context.root,
          config: context.config,
          rawScope: context.rawScope,
          checkName: check,
          checkConfigPath: context.decoded.checkConfigPath,
          output: context.decoded.output,
          dryRun: context.decoded.dryRun,
          staged: context.decoded.staged,
          tracked: context.decoded.tracked,
          strictEmptyTestTrees: context.decoded.strictEmptyTestTrees,
        },
        deps,
      ),
    );
  const findings = reports.flatMap((report) => [
    ...(report.violations ?? []),
    ...(report.warnings ?? []),
  ]);
  const waived = reports.flatMap((report) => report.waived ?? []);
  const { violations, warnings, bySeverity } = deps.splitFindings(
    findings,
    context.config,
  );
  return deps.decorateRuleDocs(
    {
      ok: violations.length === 0,
      command: "check",
      check: "architecture-policy",
      root: context.root,
      profileName: context.config.profileName ?? "strict",
      violations,
      warnings,
      waived,
      findings: [...findings, ...waived],
      bySeverity,
      scope: reports.find((report) => report.scope)?.scope ?? {
        mode: context.rawScope.mode === "all" ? "workspace" : context.rawScope.mode,
        files: [],
      },
      checks: reports.map((report) => ({
        check: report.check,
        ok: report.ok,
        violations: report.violations.length,
      })),
      languages: [...new Set(reports.flatMap((report) => report.languages ?? []))],
    },
    { rulesById: deps.RULES, ruleDocFor: deps.ruleDocFor },
  );
}
