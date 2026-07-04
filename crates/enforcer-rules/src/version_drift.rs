//! d13 — rule-version-drift detection.
//!
//! A [`crate::registry::RuleRecord`] carries an explicit `version`. Its
//! `validator`, `fixtures`, and `doc_anchor` fields are the record's
//! PARITY ARTIFACTS: whenever any of them changes, `version` MUST bump in
//! the same edit. This module compares a BASELINE record (the previously
//! committed/known-good record) against a CANDIDATE record (the one being
//! loaded now) and fails closed on either drift shape:
//!
//! - **content changed, version did not bump** — a parity artifact
//!   (validator/fixtures/doc-anchor) differs but `version` is unchanged.
//! - **version bumped, content did not change** — `version` increased but
//!   none of the parity artifacts actually changed (a hollow bump, which
//!   would mask a REAL future drift by making the version number
//!   untrustworthy as a change signal).
//!
//! Renaming a rule id counts as removal+addition (see
//! [`DriftOutcome::RuleIdMismatch`]), not a drift comparison.

use crate::registry::RuleRecord;

/// Outcome of comparing a baseline record against a candidate record for
/// the same [`enforcer_domain::ids::RuleId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftOutcome {
    /// No parity-artifact content changed, and `version` did not change
    /// either. Clean.
    Unchanged,
    /// Parity-artifact content changed AND `version` bumped by exactly the
    /// expected amount (strictly increased). Clean.
    ContentChangedVersionBumped,
    /// Parity-artifact content changed but `version` did NOT increase.
    /// FAILS CLOSED.
    ContentChangedVersionNotBumped,
    /// `version` increased but no parity-artifact content changed (a
    /// hollow bump). FAILS CLOSED.
    VersionBumpedContentUnchanged,
    /// The two records do not share a `ruleId`; not comparable as a
    /// baseline/candidate pair.
    RuleIdMismatch,
}

impl DriftOutcome {
    /// True when this outcome represents a fail-closed drift violation.
    pub fn is_drift(&self) -> bool {
        matches!(
            self,
            DriftOutcome::ContentChangedVersionNotBumped
                | DriftOutcome::VersionBumpedContentUnchanged
                | DriftOutcome::RuleIdMismatch
        )
    }
}

/// Whether the parity-artifact fields differ between two records of the
/// SAME rule id. `title` and `tags` are cosmetic and intentionally
/// excluded — only `validator`, `fixtures`, and `doc_anchor` are parity
/// artifacts per doctrine.
fn parity_content_changed(baseline: &RuleRecord, candidate: &RuleRecord) -> bool {
    baseline.validator != candidate.validator
        || baseline.fixtures != candidate.fixtures
        || baseline.doc_anchor != candidate.doc_anchor
}

/// Compare a baseline record against a candidate record for the same rule
/// id and classify the outcome. Fail-closed: any ambiguous or invalid
/// shape (mismatched ids, version decreasing) surfaces as a drift variant
/// rather than being silently accepted.
pub fn check_drift(baseline: &RuleRecord, candidate: &RuleRecord) -> DriftOutcome {
    if baseline.rule_id != candidate.rule_id {
        return DriftOutcome::RuleIdMismatch;
    }
    let content_changed = parity_content_changed(baseline, candidate);
    let version_bumped = candidate.version > baseline.version;

    match (content_changed, version_bumped) {
        (false, false) => DriftOutcome::Unchanged,
        (true, true) => DriftOutcome::ContentChangedVersionBumped,
        (true, false) => DriftOutcome::ContentChangedVersionNotBumped,
        (false, true) => DriftOutcome::VersionBumpedContentUnchanged,
    }
}

/// Convenience: `true` when [`check_drift`] reports a fail-closed
/// violation for this baseline/candidate pair.
pub fn has_drift(baseline: &RuleRecord, candidate: &RuleRecord) -> bool {
    check_drift(baseline, candidate).is_drift()
}

#[cfg(test)]
mod tests {
    use super::{check_drift, has_drift, DriftOutcome};
    use crate::registry::{FixtureRef, RuleRecord, ValidatorRef};
    use enforcer_domain::severity::Tier;

    fn base() -> Result<RuleRecord, enforcer_core::error::DecodeError> {
        Ok(RuleRecord {
            rule_id: "RR-3.1".parse()?,
            version: 1,
            title: "Base rule".to_owned(),
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".to_owned(),
                path: "sample::SampleValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "fixtures/fail.rs".to_owned(),
                pass: "fixtures/pass.rs".to_owned(),
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".to_owned(),
            tags: vec![],
            params: serde_json::Value::Null,
        })
    }

    #[test]
    fn identical_records_are_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let baseline = base()?;
        let candidate = baseline.clone();
        assert_eq!(check_drift(&baseline, &candidate), DriftOutcome::Unchanged);
        assert!(!has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn content_change_with_matching_version_bump_is_clean() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".to_owned();
        candidate.version = 2;
        assert_eq!(
            check_drift(&baseline, &candidate),
            DriftOutcome::ContentChangedVersionBumped
        );
        assert!(!has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn content_change_without_version_bump_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.fixtures.fail = "fixtures/fail-v2.rs".to_owned();
        // version left at 1 — the seeded drift.
        assert_eq!(
            check_drift(&baseline, &candidate),
            DriftOutcome::ContentChangedVersionNotBumped
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn validator_change_without_version_bump_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.validator.path = "sample::OtherValidator".to_owned();
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn version_bump_without_content_change_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.version = 2;
        // No validator/fixtures/doc_anchor change — a hollow bump.
        assert_eq!(
            check_drift(&baseline, &candidate),
            DriftOutcome::VersionBumpedContentUnchanged
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn cosmetic_only_changes_do_not_count_as_content_drift(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.title = "Renamed title".to_owned();
        candidate.tags = vec!["extra".to_owned()];
        // version left at 1 — fine, because title/tags are not parity
        // artifacts.
        assert_eq!(check_drift(&baseline, &candidate), DriftOutcome::Unchanged);
        assert!(!has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn version_decrease_with_content_change_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut baseline = base()?;
        baseline.version = 5;
        let mut candidate = baseline.clone();
        candidate.version = 4;
        candidate.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-3".to_owned();
        assert_eq!(
            check_drift(&baseline, &candidate),
            DriftOutcome::ContentChangedVersionNotBumped
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn mismatched_rule_ids_are_not_comparable() -> Result<(), Box<dyn std::error::Error>> {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.rule_id = "RR-3.2".parse()?;
        assert_eq!(
            check_drift(&baseline, &candidate),
            DriftOutcome::RuleIdMismatch
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }
}
