use enforcer_domain::ids::RuleId;
use enforcer_domain::scan_types::{
    LiteralFindingDisposition, LiteralFindingPath, LiteralLanguageId, LiteralRiskScore,
    LiteralSourceContext,
};
use enforcer_domain::severity::Severity;

use crate::stable_hash::stable_hash_key;
use crate::{FileRole, Finding, LiteralCandidate, LiteralConfidence, RiskCategory};

pub(crate) struct FindingParts<'a> {
    pub(crate) rule_id: RuleId,
    pub(crate) severity: Severity,
    pub(crate) file: &'a LiteralFindingPath,
    pub(crate) candidate: &'a LiteralCandidate,
    pub(crate) language: &'a LiteralLanguageId,
    pub(crate) file_role: FileRole,
    pub(crate) category: RiskCategory,
    pub(crate) score: LiteralRiskScore,
    pub(crate) blocking: bool,
    pub(crate) reason: &'a str,
    pub(crate) suggestion: &'a str,
}

pub(crate) fn make_finding(parts: FindingParts<'_>) -> Finding {
    let FindingParts {
        rule_id,
        severity,
        file,
        candidate,
        language,
        file_role,
        category,
        score,
        blocking,
        reason,
        suggestion,
    } = parts;
    Finding {
        rule_id,
        severity,
        file: file.clone(),
        line: candidate.line,
        column: candidate.column,
        language: language.clone(),
        file_role,
        literal_kind: candidate.kind,
        literal_preview: preview_literal(
            candidate.text.as_str(),
            category == RiskCategory::SecretLike,
        )
        .into(),
        literal_hash: format!("fnv128:{}", stable_hash_key(candidate.text.as_str())).into(),
        category,
        score,
        confidence: confidence(score),
        blocking: LiteralFindingDisposition::from(blocking),
        reason: String::from(reason).into(),
        suggestion: String::from(suggestion).into(),
        context: LiteralSourceContext::from_owned(
            candidate
                .context
                .as_str()
                .trim()
                .chars()
                .take(240)
                .collect(),
        ),
    }
}

fn preview_literal(value: &str, redact: bool) -> String {
    if redact {
        return String::from("[REDACTED]");
    }
    let mut preview = value.chars().take(120).collect::<String>();
    if value.chars().count() > 120 {
        preview.push_str("...");
    }
    preview
}

fn confidence(score: LiteralRiskScore) -> LiteralConfidence {
    if score >= 80 {
        LiteralConfidence::High
    } else if score >= 50 {
        LiteralConfidence::Medium
    } else {
        LiteralConfidence::Low
    }
}
