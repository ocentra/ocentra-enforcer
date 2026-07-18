import { normalizeRel } from "./path-utils.mjs";
import { classifyLiteralRiskProvenance } from "./literal-risk-provenance.mjs";

const DEFAULT_HARD_CATEGORIES = new Set(["secret-like"]);

function findingRuleId(finding) {
  return String(finding.rule_id ?? finding.ruleId ?? "LIT-1.1").toUpperCase();
}

function groupNonBlockingFindings(findings) {
  const groups = new Map();
  for (const finding of findings) {
    const provenance = classifyLiteralRiskProvenance(finding.file);
    const category = String(finding.category ?? "literal-risk");
    const ruleId = findingRuleId(finding);
    const key = `${provenance}|${category}|${ruleId}`;
    const group = groups.get(key) ?? {
      provenance,
      category,
      ruleId,
      count: 0,
      representative: finding,
    };
    group.count += 1;
    groups.set(key, group);
  }
  return [...groups.values()].sort((left, right) =>
    left.provenance.localeCompare(right.provenance) ||
    left.category.localeCompare(right.category) ||
    left.ruleId.localeCompare(right.ruleId),
  );
}

/** Reduces a literal-risk report to its compact public representation. */
export function compactLiteralRiskReport(report = {}) {
  const hardFindings = Array.isArray(report.hardFindings) ? report.hardFindings : [];
  const literalRisks = Array.isArray(report.literalRisks) ? report.literalRisks : [];
  const firstPartyHard = hardFindings.filter(
    (finding) => classifyLiteralRiskProvenance(finding.file) === "first-party",
  );
  const contextual = [
    ...hardFindings.filter(
      (finding) => classifyLiteralRiskProvenance(finding.file) !== "first-party",
    ),
    ...literalRisks,
  ];
  return {
    ...report,
    hardFindings: firstPartyHard,
    literalRisks: [],
    groupedFindings: groupNonBlockingFindings(contextual),
    rawCounts: {
      hardFindings: hardFindings.length,
      literalRisks: literalRisks.length,
      firstPartyHardFindings: firstPartyHard.length,
    },
  };
}

function uniqueNormalizedList(values) {
  return [...new Set(values.map((value) => String(value ?? "").trim()).filter(Boolean))];
}

/** Maps literal-scanner findings into enforcer finding objects. */
export function mapLiteralRiskFindings(scan, root, options = {}) {
  const hardCategories = new Set(
    uniqueNormalizedList([
      ...(Array.isArray(options.hardCategories) ? options.hardCategories : []),
      ...DEFAULT_HARD_CATEGORIES,
    ]),
  );
  const hardRuleIds = new Set(
    uniqueNormalizedList(Array.isArray(options.hardRuleIds) ? options.hardRuleIds : []),
  );
  const mapFinding = (finding, fallbackSeverity) => {
    const ruleId = findingRuleId(finding);
    const category = String(finding.category ?? "").trim();
    const blocking =
      finding.blocking === true ||
      hardCategories.has(category) ||
      hardRuleIds.has(ruleId);
    return {
      ruleId,
      severity: blocking ? "error" : fallbackSeverity,
      file: normalizeRel(root, finding.file ?? ""),
      line: Number.isInteger(finding.line) ? finding.line : Number(finding.line ?? 1) || 1,
      detail: String(finding.reason ?? finding.detail ?? "").trim(),
      snippet: String(finding.suggestion ?? finding.literal_preview ?? finding.snippet ?? "").trim(),
      source: finding.context ?? null,
      category,
      score: finding.score,
      confidence: finding.confidence,
      fileRole: finding.file_role ?? finding.fileRole,
      literalKind: finding.literal_kind ?? finding.literalKind,
      literalPreview: finding.literal_preview ?? finding.literalPreview,
      literalHash: finding.literal_hash ?? finding.literalHash,
      blocking,
    };
  };
  const hardFindings = (scan.report?.hardFindings ?? []).map((finding) =>
    mapFinding(finding, "error"),
  );
  const literalRisks = (scan.report?.literalRisks ?? []).map((finding) =>
    mapFinding(finding, "warning"),
  );
  const groupedFindings = (scan.report?.groupedFindings ?? []).map((group) => {
    const representative = group.representative ?? {};
    return {
      ...mapFinding(representative, "warning"),
      ruleId: group.ruleId ?? findingRuleId(representative),
      severity: "warning",
      blocking: false,
      detail: `Grouped ${group.count} ${group.provenance} ${group.category} findings.`,
      provenance: group.provenance,
      groupedCount: group.count,
    };
  });
  return [...hardFindings, ...literalRisks, ...groupedFindings];
}
