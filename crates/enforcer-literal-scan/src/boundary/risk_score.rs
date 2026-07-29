//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.

use enforcer_domain::scan_types::{LiteralRiskScore, LiteralScanCount};

use crate::{FileRole, LiteralCandidate, RiskCategory};

pub(crate) fn score_literal(
    candidate: &LiteralCandidate,
    role: FileRole,
    category: RiskCategory,
    repeated_files: LiteralScanCount,
) -> LiteralRiskScore {
    crate::domain::risk_score::score_literal(candidate, role, category, repeated_files)
}
