#!/usr/bin/env node

import {
  compactFinding,
  countBy,
  groupFindings,
  uniqueSorted,
} from "./rust-rules-mcp-compact-grouping.mjs";

function compactScope(scope) {
  if (!scope) return undefined;
  return {
    mode: scope.mode,
    fileCount: Array.isArray(scope.files) ? scope.files.length : undefined,
    sampleFiles: Array.isArray(scope.files) ? scope.files.slice(0, 20) : undefined,
    crateName: scope.crateName,
    base: scope.base,
    head: scope.head,
  };
}

function compactCounts(report, findings, diagnostics) {
  return {
    findings: findings.length,
    violations: report.violations?.length ?? 0,
    warnings: report.warnings?.length ?? 0,
    returned: diagnostics.length,
    truncated: findings.length > diagnostics.length,
  };
}

function compactLists(findings, field) {
  return uniqueSorted(findings.map((finding) => finding[field]).filter(Boolean));
}

function compactBaseReport(report, findings, diagnostics) {
  return {
    ok: report.ok,
    command: report.command,
    check: report.check,
    root: report.root,
    profileName: report.profileName,
    languages: report.languages,
    bySeverity: report.bySeverity ?? countBy(findings, "severity"),
    counts: compactCounts(report, findings, diagnostics),
    ruleIds: compactLists(findings, "ruleId"),
    docs: compactLists(findings, "doc"),
    diagnostics,
  };
}

function maybeCompactReport(report, args) {
  const wantsCompact =
    args.summaryOnly === true ||
    args.diagnosticLimit !== undefined ||
    args.groupBy !== undefined ||
    args.includeScope === false;
  if (!wantsCompact) return report;

  const findings = [...(report.violations ?? []), ...(report.warnings ?? [])];
  const limit = Math.max(
    0,
    Number.isFinite(args.diagnosticLimit) ? Math.trunc(args.diagnosticLimit) : 20,
  );
  const diagnostics = args.summaryOnly ? [] : findings.slice(0, limit).map(compactFinding);
  const compact = compactBaseReport(report, findings, diagnostics);
  if (args.groupBy) compact.groups = groupFindings(findings, args.groupBy);
  if (args.includeScope !== false) compact.scope = compactScope(report.scope);
  return compact;
}

export { compactScope, maybeCompactReport };
