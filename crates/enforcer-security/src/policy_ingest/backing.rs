//! The backed-rule catalog: which `RuleId`s this crate actually has a
//! mechanized `Validator` for (h08, POLICY-SPEC-INGESTION honesty seam).
//!
//! This is a deliberately STANDALONE catalog, not a live read of
//! [`crate::rules::registry::build_all`] — `registry.rs` is part of the
//! arc-19 skeleton's `owns:` set (shared seam every h01-h11 feature pack
//! extends), and this workpack's `owns:` set is disjoint by file from it.
//! Wiring `policy_ingest` to import `registry::build_all` directly would
//! make this module's ownership straddle a file it does not own. Instead,
//! [`BackedRuleCatalog::track_h_snapshot`] lists the same Track H rule ids
//! `registry.rs` currently registers, kept in sync by convention (both
//! sides are reviewed together whenever a new Track H validator lands).
//!
//! Whatever this catalog does NOT list is, by construction, unbacked:
//! [`super::map::map_to_profile`] treats catalog absence as "no
//! mechanized validator exists" and flags accordingly. This is the honesty
//! seam the workpack requires: a rule id absent from this list is visibly
//! un-enforced, never silently promoted to "enabled".

use std::collections::BTreeSet;

/// The set of `RuleId` strings this crate currently has a real, registered
/// [`enforcer_validator::validator::Validator`] for. Backed by convention
/// against `crate::rules::registry::build_all`'s current row set (kept in
/// sync at review time, not read live, to keep this module's `owns:`
/// boundary from crossing into the shared `registry.rs` seam file).
#[derive(Debug, Clone)]
pub struct BackedRuleCatalog {
    backed_ids: BTreeSet<String>,
}

impl BackedRuleCatalog {
    /// Build a catalog from an explicit id list — the seam a caller (or a
    /// future live-registry reader, once that refactor is in scope for
    /// someone who owns `registry.rs`) can substitute.
    pub fn from_ids<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            backed_ids: ids.into_iter().map(Into::into).collect(),
        }
    }

    /// The current Track H backed-rule snapshot, mirroring
    /// `crate::rules::registry::build_all`'s rows at the time this module
    /// landed (arc-19 skeleton + h05/h06/h11 rows already registered).
    pub fn track_h_snapshot() -> Self {
        Self::from_ids([
            "H00-1.1",
            "MONEY-CRIT-CLASSIFY.1",
            "MONEY-CRIT-ANNOTATED.1",
            "THREAT-MAP-UNIT-COVERAGE.1",
            "THREAT-MAP-NO-UNMAPPED.1",
            "THREAT-MAP-THREAT-HAS-TEST.1",
            "ECON-INVARIANT-PRESENCE.1",
            "ECON-INVARIANT-SHAPE.1",
            "MCM-SIGNING.1",
            "MCM-TIME.1",
            "MCM-BOUNDARY.1",
            "MCM-KILLSWITCH.1",
            "MCM-ECONOMIC.1",
            "MCM-ROLLBACK.1",
            "CYBER-FRONTMATTER.1",
        ])
    }

    /// Whether `rule_id` has a real mechanized validator backing it.
    pub fn is_backed(&self, rule_id: &str) -> bool {
        self.backed_ids.contains(rule_id)
    }
}
