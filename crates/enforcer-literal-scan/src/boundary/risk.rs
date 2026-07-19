use super::{FileRole, Finding, LiteralCandidate, RiskCategory};
use crate::risk_finding::{make_finding, FindingParts};
use crate::risk_primary::primary_category;
use crate::risk_reason::reason_and_suggestion;
use crate::risk_score::score_literal;
use crate::rule_id_for_category;
use enforcer_domain::scan_types::{LiteralFindingPath, LiteralLanguageId, LiteralRiskScore};
use enforcer_domain::severity::Severity;

#[derive(Clone, Copy)]
pub(crate) struct ClassificationInput<'a> {
    pub(crate) candidate: &'a LiteralCandidate,
    pub(crate) file: &'a LiteralFindingPath,
    pub(crate) language: &'a LiteralLanguageId,
    pub(crate) role: FileRole,
    pub(crate) repeated_files: usize,
    pub(crate) fail_above: Option<LiteralRiskScore>,
}

pub(crate) fn classify_literal(input: ClassificationInput<'_>) -> Finding {
    let ClassificationInput {
        candidate,
        file,
        language,
        role,
        repeated_files,
        fail_above,
    } = input;
    let mut category = primary_category(candidate, role);
    let mut score = score_literal(candidate, role, category, repeated_files);
    if should_upgrade_to_repeated_literal(category, repeated_files, score) {
        category = RiskCategory::RepeatedLiteral;
        score = LiteralRiskScore::try_new(
            score
                .get()
                .saturating_add(repeated_literal_bonus(repeated_files))
                .min(100),
        )
        .unwrap_or(LiteralRiskScore::ZERO);
    }
    let blocking = category == RiskCategory::SecretLike
        || fail_above
            .map(|threshold| score >= threshold)
            .unwrap_or(false);
    let severity = if blocking {
        Severity::Error
    } else if score >= 70 {
        Severity::Warning
    } else {
        Severity::Info
    };
    let (reason, suggestion) = reason_and_suggestion(category, role);
    make_finding(FindingParts {
        rule_id: rule_id_for_category(category),
        severity,
        file,
        candidate,
        language,
        file_role: role,
        category,
        score,
        blocking,
        reason,
        suggestion,
    })
}

fn should_upgrade_to_repeated_literal(
    category: RiskCategory,
    repeated_files: usize,
    score: LiteralRiskScore,
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
