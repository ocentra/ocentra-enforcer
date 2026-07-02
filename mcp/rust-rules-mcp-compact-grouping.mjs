#!/usr/bin/env node

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

export { compactFinding, countBy, groupFindings, uniqueSorted };
