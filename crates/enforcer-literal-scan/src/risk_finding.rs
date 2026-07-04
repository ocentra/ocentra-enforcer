use crate::stable_hash_hex;
use crate::{FileRole, Finding, LiteralCandidate, RiskCategory};

// Inherited from the standalone Tools/ocentra-literal-scan tool (arc-13
// fold-in preserves its scoring behavior as-is; a param-struct refactor is
// out of scope for this workpack -- see arc-13 workpack "no regression").
#[allow(clippy::too_many_arguments)]
pub(crate) fn make_finding(
    rule_id: &str,
    severity: &str,
    file: &str,
    candidate: &LiteralCandidate,
    language: &str,
    file_role: FileRole,
    category: RiskCategory,
    score: u8,
    blocking: bool,
    reason: &str,
    suggestion: &str,
) -> Finding {
    Finding {
        rule_id: rule_id.to_string(),
        severity: severity.to_string(),
        file: file.to_string(),
        line: candidate.line,
        column: candidate.column,
        language: language.to_string(),
        file_role,
        literal_kind: candidate.kind,
        literal_preview: preview_literal(&candidate.text, category == RiskCategory::SecretLike),
        literal_hash: format!("fnv128:{}", stable_hash_hex(&candidate.text)),
        category,
        score,
        confidence: confidence(score).to_string(),
        blocking,
        reason: reason.to_string(),
        suggestion: suggestion.to_string(),
        context: candidate.context.trim().chars().take(240).collect(),
    }
}

fn preview_literal(value: &str, redact: bool) -> String {
    if redact {
        return "[REDACTED]".to_string();
    }
    let mut preview = value.chars().take(120).collect::<String>();
    if value.chars().count() > 120 {
        preview.push_str("...");
    }
    preview
}

fn confidence(score: u8) -> &'static str {
    if score >= 80 {
        "high"
    } else if score >= 50 {
        "medium"
    } else {
        "low"
    }
}
