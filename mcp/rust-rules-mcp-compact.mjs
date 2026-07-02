#!/usr/bin/env node

function parseJson(text) {
  if (!text || !text.trim().startsWith("{")) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function compactFinding(finding) {
  return {
    ruleId: finding.ruleId,
    severity: finding.severity ?? "error",
    file: finding.file,
    line: finding.line,
    detail: finding.detail,
    doc: finding.doc,
  };
}

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

function countBy(values, key) {
  const result = {};
  for (const value of values) {
    const group = value?.[key] ?? "unknown";
    result[group] = (result[group] ?? 0) + 1;
  }
  return result;
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function sliceKey(file) {
  const parts = String(file ?? "").split("/");
  if (["apps", "packages", "crates", "tools"].includes(parts[0]) && parts[1]) {
    return `${parts[0]}/${parts[1]}`;
  }
  return parts[0] || ".";
}

function groupFindings(findings, mode) {
  const groups = new Map();
  for (const finding of findings) {
    const key = mode === "slice" ? sliceKey(finding.file) : finding.file;
    const group = groups.get(key) ?? {
      key,
      count: 0,
      bySeverity: {},
      ruleIds: new Set(),
      docs: new Set(),
      first: null,
    };
    group.count += 1;
    const severity = finding.severity ?? "error";
    group.bySeverity[severity] = (group.bySeverity[severity] ?? 0) + 1;
    group.ruleIds.add(finding.ruleId);
    if (finding.doc) group.docs.add(finding.doc);
    group.first ??= compactFinding(finding);
    groups.set(key, group);
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      ruleIds: [...group.ruleIds].sort(),
      docs: [...group.docs].sort(),
    }))
    .sort((left, right) => right.count - left.count || left.key.localeCompare(right.key));
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
    Number.isFinite(args.diagnosticLimit)
      ? Math.trunc(args.diagnosticLimit)
      : 20,
  );
  const diagnostics = args.summaryOnly
    ? []
    : findings.slice(0, limit).map(compactFinding);
  const compact = {
    ok: report.ok,
    command: report.command,
    check: report.check,
    root: report.root,
    profileName: report.profileName,
    languages: report.languages,
    bySeverity: report.bySeverity ?? countBy(findings, "severity"),
    counts: {
      findings: findings.length,
      violations: report.violations?.length ?? 0,
      warnings: report.warnings?.length ?? 0,
      returned: diagnostics.length,
      truncated: findings.length > diagnostics.length,
    },
    ruleIds: uniqueSorted(findings.map((finding) => finding.ruleId)),
    docs: uniqueSorted(findings.map((finding) => finding.doc).filter(Boolean)),
    diagnostics,
  };
  if (args.groupBy) compact.groups = groupFindings(findings, args.groupBy);
  if (args.includeScope !== false) compact.scope = compactScope(report.scope);
  return compact;
}

export {
  compactFinding,
  compactScope,
  countBy,
  groupFindings,
  maybeCompactReport,
  parseJson,
  uniqueSorted,
};
