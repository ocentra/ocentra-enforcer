import process from "node:process";

function scopeLabel(scope) {
  if (!scope || scope.mode === "all") return "workspace";
  if (scope.mode === "files") return `${scope.files?.length ?? 0} file(s)`;
  if (scope.mode === "crate") return `crate ${scope.crateName ?? "unknown"}`;
  if (scope.mode === "diff") return `diff ${scope.base ?? "base"}..${scope.head ?? "head"}`;
  return scope.mode;
}

function shouldReportArchitectureProgress(context) {
  if (context.options?.json) return false;
  if (process.env.OCENTRA_ENFORCER_PROGRESS === "0") return false;
  if (process.env.OCENTRA_ENFORCER_PROGRESS === "1") return true;
  return context.rawScope?.mode === "all";
}

function createArchitectureProgressReporter(context, checks) {
  if (!shouldReportArchitectureProgress(context)) return () => {};
  const started = Date.now();
  const scope = scopeLabel(context.rawScope);
  process.stderr.write(
    `[ocentra-enforcer] architecture-policy: ${checks.length} check(s), scope=${scope}\n`,
  );
  return (event, payload = {}) => {
    const elapsed = ((Date.now() - started) / 1000).toFixed(1);
    if (event === "start") {
      process.stderr.write(
        `[ocentra-enforcer] architecture-policy: ${payload.index}/${checks.length} start ${payload.check} elapsed=${elapsed}s\n`,
      );
      return;
    }
    if (event === "done") {
      process.stderr.write(
        `[ocentra-enforcer] architecture-policy: ${payload.index}/${checks.length} done ${payload.check} ok=${payload.ok} violations=${payload.violations} warnings=${payload.warnings} elapsed=${elapsed}s\n`,
      );
    }
  };
}

export function runArchitecturePolicyCheck(context, deps) {
  const checks =
    context.config.architecturePolicyChecks ??
    deps.DEFAULT_ARCHITECTURE_POLICY_CHECKS;
  const routedChecks = checks.filter((check) => check !== "architecture-policy");
  const progress = createArchitectureProgressReporter(context, routedChecks);
  const reports = routedChecks.map((check, index) => {
    progress("start", { check, index: index + 1 });
    const report = deps.runEnforcerCheck(
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
      );
    progress("done", {
      check,
      index: index + 1,
      ok: report.ok,
      violations: report.violations?.length ?? 0,
      warnings: report.warnings?.length ?? 0,
    });
    return report;
  });
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
