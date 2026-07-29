//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.

use crate::{FileRole, RiskCategory};

pub(crate) fn reason_and_suggestion(
    category: RiskCategory,
    role: FileRole,
) -> crate::domain::risk_reason::ReasonSuggestion {
    crate::domain::risk_reason::reason_and_suggestion(category, role)
}
