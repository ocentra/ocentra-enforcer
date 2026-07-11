const FINDING_COLLECTIONS = ["violations", "warnings", "waived", "findings"];

export function decorateRuleDocs(report, { rulesById, ruleDocFor }) {
  const completenessFailures = collectReportCompletenessFailures(report);
  for (const key of FINDING_COLLECTIONS) {
    report[key] = normalizeFindingCollection(report[key], { rulesById, ruleDocFor });
  }
  enforceReportCompleteness(report, completenessFailures);
  report.bySeverity = countActiveFindingsBySeverity(report);
  return report;
}

export function sortFindings(findings) {
  return [...findings].sort(compareFindings);
}

export function compareFindings(a, b) {
  return (
    String(a.file ?? "").localeCompare(String(b.file ?? "")) ||
    Number(a.line ?? 0) - Number(b.line ?? 0) ||
    String(a.ruleId ?? "").localeCompare(String(b.ruleId ?? "")) ||
    String(a.detail ?? "").localeCompare(String(b.detail ?? ""))
  );
}

function normalizeReportFinding(finding, { rulesById, ruleDocFor }) {
  const normalized = {
    ruleId: finding.ruleId ?? "ENF-1.8",
    severity: finding.severity ?? "error",
    title:
      finding.title ??
      rulesById[finding.ruleId]?.title ??
      "Incomplete report finding",
    detail: finding.detail ?? "",
    file: finding.file ?? "",
    line: Number.isInteger(finding.line) ? finding.line : 1,
    snippet:
      finding.snippet ??
      rulesById[finding.ruleId]?.snippet ??
      "Emit complete, deterministic findings.",
    source: finding.source ?? null,
    doc: finding.doc ?? ruleDocFor(finding.ruleId),
  };
  for (const [key, value] of Object.entries(finding)) {
    if (!(key in normalized)) normalized[key] = value;
  }
  return normalized;
}

function normalizeFindingCollection(findings, context) {
  if (!Array.isArray(findings)) return findings;
  const seen = new Set();
  const unique = [];
  for (const finding of findings.map((item) => normalizeReportFinding(item, context))) {
    const fingerprint = findingFingerprint(finding);
    if (seen.has(fingerprint)) continue;
    seen.add(fingerprint);
    unique.push(finding);
  }
  return sortFindings(unique);
}

function findingFingerprint(finding) {
  return [
    finding.ruleId,
    finding.severity,
    finding.title,
    finding.detail,
    finding.file,
    finding.line,
    finding.snippet,
    finding.source,
    finding.doc,
  ].map((value) => value ?? "").join("\u001f");
}

function countActiveFindingsBySeverity(report) {
  return [...(report.violations ?? []), ...(report.warnings ?? [])].reduce(
    (counts, finding) => {
      const severity = String(finding.severity ?? "error");
      counts[severity] = Number(counts[severity] ?? 0) + 1;
      return counts;
    },
    {},
  );
}

function collectReportCompletenessFailures(report) {
  return FINDING_COLLECTIONS.flatMap((key) =>
    (report[key] ?? [])
      .filter((finding) => !isCompleteFinding(finding))
      .map((finding) => ({ key, finding })),
  );
}

function enforceReportCompleteness(report, bad) {
  if (bad.length === 0) return;
  const reportFinding = {
    ruleId: "ENF-1.8",
    severity: "error",
    title: "Enforcer reports must be complete",
    detail: `Report contained ${bad.length} incomplete finding(s).`,
    file: "reports",
    line: 1,
    snippet:
      "Populate ruleId, title, detail, file, line, snippet, and doc for every finding.",
    doc: "rules/common/reporting.md#enf18",
  };
  report.ok = false;
  report.findings = sortFindings([...(report.findings ?? []), reportFinding]);
  report.violations = sortFindings([
    ...(report.violations ?? []),
    reportFinding,
  ]);
}

function isCompleteFinding(finding) {
  return Boolean(
    finding &&
      typeof finding === "object" &&
      typeof finding.ruleId === "string" &&
      finding.ruleId.length > 0 &&
      typeof finding.file === "string" &&
      finding.file.length > 0 &&
      typeof finding.title === "string" &&
      finding.title.length > 0 &&
      typeof finding.detail === "string" &&
      finding.detail.length > 0 &&
      Number.isInteger(finding.line) &&
      finding.line >= 1 &&
      typeof finding.doc === "string" &&
      finding.doc.length > 0,
  );
}
