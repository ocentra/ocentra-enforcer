use crate::risk_heuristics_context::is_schema_owner_context;
use crate::risk_heuristics_secret::is_secret_like;
use crate::{FileRole, RiskCategory};
use enforcer_domain::scan_types::{LiteralSourceContext, LiteralSourceText};

pub(crate) fn role_specific_category(
    role: FileRole,
    context: &LiteralSourceContext,
    text: &LiteralSourceText,
) -> Option<RiskCategory> {
    if role == FileRole::Test {
        return Some(test_category(text));
    }
    if is_schema_owner_context(role, context.as_str()) {
        return Some(RiskCategory::SchemaOwnerLiteral);
    }
    None
}

fn test_category(text: &LiteralSourceText) -> RiskCategory {
    if is_secret_like(text.as_str()) {
        RiskCategory::SecretLike
    } else {
        RiskCategory::TestFixture
    }
}
