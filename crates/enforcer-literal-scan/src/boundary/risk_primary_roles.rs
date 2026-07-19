use crate::risk_heuristics_context::is_schema_owner_context;
use crate::risk_heuristics_secret::is_secret_like;
use crate::{FileRole, RiskCategory};

pub(crate) fn role_specific_category(
    role: FileRole,
    context: &str,
    text: &str,
) -> Option<RiskCategory> {
    if role == FileRole::Test {
        return Some(test_category(text));
    }
    if is_schema_owner_context(role, context) {
        return Some(RiskCategory::SchemaOwnerLiteral);
    }
    None
}

fn test_category(text: &str) -> RiskCategory {
    if is_secret_like(text) {
        RiskCategory::SecretLike
    } else {
        RiskCategory::TestFixture
    }
}
