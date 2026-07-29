use crate::risk_heuristics_context::{is_import_like_context, is_magic_string_comparison};
use crate::risk_heuristics_secret::is_secret_like;
use crate::risk_primary_patterns::pattern_category;
use crate::risk_primary_roles::role_specific_category;
use crate::{FileRole, LiteralCandidate, LiteralKind, RiskCategory};

pub(crate) fn primary_category(candidate: &LiteralCandidate, role: FileRole) -> RiskCategory {
    let text = candidate.text.as_str().trim();
    let context = candidate.context.as_str();

    if is_secret_like(text) {
        return RiskCategory::SecretLike;
    }
    if candidate.kind == LiteralKind::ImportSpecifier || is_import_like_context(context, text) {
        return RiskCategory::ImportSpecifier;
    }
    if let Some(category) = role_specific_category(role, &candidate.context, &candidate.text) {
        return category;
    }
    let category = pattern_category(text);
    if is_magic_string_comparison(context) && matches!(category, Some(RiskCategory::StateOrStatus))
    {
        return RiskCategory::MagicStringComparison;
    }
    if let Some(category) = category {
        return category;
    }
    RiskCategory::UnknownLiteral
}
