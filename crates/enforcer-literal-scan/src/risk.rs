use super::{FileRole, Finding, LiteralCandidate, RiskCategory};
use crate::risk_finding::make_finding;
use crate::risk_primary::primary_category;
use crate::risk_reason::reason_and_suggestion;
use crate::risk_score::score_literal;

// Inherited from the standalone Tools/ocentra-literal-scan tool (arc-13
// fold-in preserves its scoring behavior as-is; a param-struct refactor is
// out of scope for this workpack -- see arc-13 workpack "no regression").
#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_literal(
    candidate: &LiteralCandidate,
    file: &str,
    language: &str,
    role: FileRole,
    repeated_files: usize,
    fail_above: Option<u8>,
) -> Finding {
    let mut category = primary_category(candidate, role);
    let mut score = score_literal(candidate, role, category, repeated_files);
    if should_upgrade_to_repeated_literal(category, repeated_files, score) {
        category = RiskCategory::RepeatedLiteral;
        score = score
            .saturating_add(repeated_literal_bonus(repeated_files))
            .min(100);
    }
    let blocking = category == RiskCategory::SecretLike
        || fail_above
            .map(|threshold| score >= threshold)
            .unwrap_or(false);
    let severity = if blocking {
        "error"
    } else if score >= 70 {
        "warning"
    } else {
        "info"
    };
    let (reason, suggestion) = reason_and_suggestion(category, role);
    make_finding(
        category.rule_id(),
        severity,
        file,
        candidate,
        language,
        role,
        category,
        score,
        blocking,
        reason,
        suggestion,
    )
}

fn should_upgrade_to_repeated_literal(
    category: RiskCategory,
    repeated_files: usize,
    score: u8,
) -> bool {
    repeated_files >= 2
        && score < 70
        && !matches!(
            category,
            RiskCategory::SecretLike | RiskCategory::ImportSpecifier | RiskCategory::TestFixture
        )
}

fn repeated_literal_bonus(repeated_files: usize) -> u8 {
    if repeated_files >= 3 {
        20
    } else {
        10
    }
}
