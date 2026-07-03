import { normalizeRel } from "./path-utils.mjs";

const DEFAULT_HARD_CATEGORIES = new Set(["secret-like"]);

function uniqueNormalizedList(values) {
  return [...new Set(values.map((value) => String(value ?? "").trim()).filter(Boolean))];
}

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
    const ruleId = String(finding.rule_id ?? finding.ruleId ?? "LIT-1.1").toUpperCase();
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
      fileRole: finding.file_role,
      literalKind: finding.literal_kind,
      literalPreview: finding.literal_preview,
      literalHash: finding.literal_hash,
      blocking,
    };
  };
  const hardFindings = (scan.report?.hardFindings ?? []).map((finding) =>
    mapFinding(finding, "error"),
  );
  const literalRisks = (scan.report?.literalRisks ?? []).map((finding) =>
    mapFinding(finding, "warning"),
  );
  return [...hardFindings, ...literalRisks];
}
