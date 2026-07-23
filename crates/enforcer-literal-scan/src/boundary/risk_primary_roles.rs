//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.

use enforcer_domain::scan_types::{LiteralSourceContext, LiteralSourceText};

use crate::{FileRole, RiskCategory};

pub(crate) fn role_specific_category(
    role: FileRole,
    context: &LiteralSourceContext,
    text: &LiteralSourceText,
) -> Option<RiskCategory> {
    crate::domain::risk_primary_roles::role_specific_category(role, context, text)
}
