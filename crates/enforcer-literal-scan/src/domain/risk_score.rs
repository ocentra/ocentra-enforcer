use crate::risk_heuristics_context::{is_logging_context, is_magic_string_comparison};
use crate::{FileRole, LiteralCandidate, RiskCategory};
use enforcer_domain::scan_types::{LiteralRiskScore, LiteralScanCount};

#[derive(Clone, Copy)]
struct ScoreBias(i16);

pub(crate) fn score_literal(
    candidate: &LiteralCandidate,
    role: FileRole,
    category: RiskCategory,
    repeated_files: LiteralScanCount,
) -> LiteralRiskScore {
    let mut score = 5 + role_adjustment(role).0 + category_adjustment(category).0;
    if is_magic_string_comparison(candidate.context.as_str()) {
        score += 35;
    }
    if is_logging_context(candidate.context.as_str()) {
        score -= 20;
    }
    if repeated_files.get() >= 3 {
        score += 20;
    } else if repeated_files.get() == 2 {
        score += 10;
    }
    let bounded = u8::try_from(score.clamp(0, 100)).unwrap_or(0);
    LiteralRiskScore::try_new(bounded).unwrap_or(LiteralRiskScore::ZERO)
}

const fn role_adjustment(role: FileRole) -> ScoreBias {
    match role {
        FileRole::Domain => ScoreBias(20),
        FileRole::Boundary => ScoreBias(5),
        FileRole::Config => ScoreBias(-5),
        FileRole::Test => ScoreBias(-30),
        FileRole::Generated => ScoreBias(-50),
        FileRole::Tooling | FileRole::Script => ScoreBias(-10),
        FileRole::Docs | FileRole::CommonText => ScoreBias(-50),
        FileRole::Unknown => ScoreBias(0),
    }
}

const fn category_adjustment(category: RiskCategory) -> ScoreBias {
    match category {
        RiskCategory::SecretLike => ScoreBias(100),
        RiskCategory::EventOrCommandName | RiskCategory::RawJsonBlob => ScoreBias(45),
        RiskCategory::RouteOrUrl => ScoreBias(40),
        RiskCategory::ProtocolHeaderOrMedia | RiskCategory::StateOrStatus => ScoreBias(30),
        RiskCategory::IdOrKeyName => ScoreBias(35),
        RiskCategory::SqlFragment => ScoreBias(60),
        RiskCategory::ShellFragment => ScoreBias(70),
        RiskCategory::MagicStringComparison => ScoreBias(45),
        RiskCategory::RepeatedLiteral => ScoreBias(20),
        RiskCategory::HumanMessage => ScoreBias(5),
        RiskCategory::TestFixture => ScoreBias(-20),
        RiskCategory::ImportSpecifier => ScoreBias(-60),
        RiskCategory::SchemaOwnerLiteral => ScoreBias(-30),
        RiskCategory::UnknownLiteral => ScoreBias(0),
    }
}
