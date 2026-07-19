use crate::risk_heuristics_context::{is_logging_context, is_magic_string_comparison};
use crate::{FileRole, LiteralCandidate, RiskCategory};
use enforcer_domain::scan_types::LiteralRiskScore;

pub(crate) fn score_literal(
    candidate: &LiteralCandidate,
    role: FileRole,
    category: RiskCategory,
    repeated_files: usize,
) -> LiteralRiskScore {
    let mut score = 5 + role_adjustment(role) + category_adjustment(category);
    if is_magic_string_comparison(candidate.context.as_str()) {
        score += 35;
    }
    if is_logging_context(candidate.context.as_str()) {
        score -= 20;
    }
    if repeated_files >= 3 {
        score += 20;
    } else if repeated_files == 2 {
        score += 10;
    }
    let bounded = u8::try_from(score.clamp(0, 100)).unwrap_or(0);
    LiteralRiskScore::try_new(bounded).unwrap_or(LiteralRiskScore::ZERO)
}

const fn role_adjustment(role: FileRole) -> i16 {
    match role {
        FileRole::Domain => 20,
        FileRole::Boundary => 5,
        FileRole::Config => -5,
        FileRole::Test => -30,
        FileRole::Generated => -50,
        FileRole::Tooling | FileRole::Script => -10,
        FileRole::Docs | FileRole::CommonText => -50,
        FileRole::Unknown => 0,
    }
}

const fn category_adjustment(category: RiskCategory) -> i16 {
    match category {
        RiskCategory::SecretLike => 100,
        RiskCategory::EventOrCommandName | RiskCategory::RawJsonBlob => 45,
        RiskCategory::RouteOrUrl => 40,
        RiskCategory::ProtocolHeaderOrMedia | RiskCategory::StateOrStatus => 30,
        RiskCategory::IdOrKeyName => 35,
        RiskCategory::SqlFragment => 60,
        RiskCategory::ShellFragment => 70,
        RiskCategory::MagicStringComparison => 45,
        RiskCategory::RepeatedLiteral => 20,
        RiskCategory::HumanMessage => 5,
        RiskCategory::TestFixture => -20,
        RiskCategory::ImportSpecifier => -60,
        RiskCategory::SchemaOwnerLiteral => -30,
        RiskCategory::UnknownLiteral => 0,
    }
}
