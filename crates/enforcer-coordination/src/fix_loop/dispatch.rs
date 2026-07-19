//! The pluggable fix-generator contract dispatched by [`super::run_fix_loop`].
//!
//! A [`FixGenerator`] is handed the current on-disk snapshot root plus the
//! [`Finding`](enforcer_domain::findings::Finding)s the last re-scan
//! produced, and attempts ONE bounded editing move. It never decides
//! whether its own edit was an improvement â€” that judgment belongs solely
//! to the loop's re-scan-and-compare gate (`super::run_fix_loop`), which is
//! what keeps the loop from trusting a fix generator's self-report.

use enforcer_domain::coordination_types::{FixAttemptOutcome, FixGeneratorName, FixWorkspaceRoot};
use enforcer_domain::findings::Finding;

use crate::error::Result;

/// One attempted editing move against the working tree at `root`.
///
/// Implementors mutate files under `root` directly (the loop has already
/// snapshotted the tree before calling this, and will restore it if the
/// attempt does not improve the finding count). Returning `Ok(false)` means
/// "I had nothing to try this iteration" â€” the loop treats a no-op attempt
/// as ineligible to keep (same rule as a non-improving edit) and, more
/// importantly, as a signal to stop iterating rather than spin the cap.
pub trait FixGenerator {
    /// Attempt to address (a subset of) `findings` by editing files under
    /// `root`. Returns `Ok(true)` if any edit was made, `Ok(false)` if the
    /// generator declined to act this iteration (e.g. no findings it knows
    /// how to address), or `Err` on an operational failure (I/O, etc).
    fn attempt_fix(
        &self,
        root: &FixWorkspaceRoot,
        findings: &[Finding],
    ) -> Result<FixAttemptOutcome>;

    /// Human-readable name recorded on the loop's typed events, for
    /// observability (which generator produced/declined a given attempt).
    fn name(&self) -> Result<FixGeneratorName>;
}
