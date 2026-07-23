//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.

use crate::{FileRole, LiteralCandidate, RiskCategory};

pub(crate) fn primary_category(candidate: &LiteralCandidate, role: FileRole) -> RiskCategory {
    crate::domain::risk_primary::primary_category(candidate, role)
}
