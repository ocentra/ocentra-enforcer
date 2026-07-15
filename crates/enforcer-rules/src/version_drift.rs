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
//! [`VersionDriftOutcome::RuleIdMismatch`]), not a drift comparison.
//!
//! # Whole-registry manifest drift (d13)
//!
//! [`check_drift`]/[`has_drift`] above are the PAIRWISE oracle arc-04
//! shipped: one baseline record vs one candidate record. d13 adds the
//! WHOLE-REGISTRY layer on top: [`RuleManifest`] is a versioned,
//! `serde`-typed record (parsed at the boundary via
//! [`decode_manifest`], never a bare `String`) pinning a
//! [`enforcer_domain::hashes::Sha256`] content hash per loaded
//! [`crate::registry::RuleRecord`], computed by [`hash_record`] using the
//! `enforcer_core::hash_chain` primitive (never an ad-hoc JSON blob).
//! [`check_registry_drift`] recomputes each record's hash and compares it
//! against the pinned manifest entry, emitting a fail-closed
//! [`enforcer_domain::findings::Finding`] naming the offending rule id with
//! a terse `Fix:` hint whenever:
//!
//! - the hash changed but the manifest `version` for that rule id did not
//!   bump ([`ManifestDrift::HashChangedVersionNotBumped`]), or
//! - the manifest `version` bumped but the hash is unchanged, a hollow bump
//!   ([`ManifestDrift::VersionBumpedHashUnchanged`]).
//!
//! A legitimate change requires both a new `version` AND a new `hash`
//! together in the same manifest edit; neither alone passes.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::ids::RuleId;
use enforcer_domain::rules_types::{
    RuleFailureReason, RuleManifest, RuleManifestEntry, RuleManifestJson,
    RuleManifestSchemaVersion, VersionDriftOutcome,
};
use enforcer_domain::severity::Severity;

use crate::registry::{RuleRecord, RuleRegistry};

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
pub fn check_drift(baseline: &RuleRecord, candidate: &RuleRecord) -> VersionDriftOutcome {
    if baseline.rule_id != candidate.rule_id {
        return VersionDriftOutcome::RuleIdMismatch;
    }
    let content_changed = parity_content_changed(baseline, candidate);
    let version_bumped = candidate.version > baseline.version;

    match (content_changed, version_bumped) {
        (false, false) => VersionDriftOutcome::Unchanged,
        (true, true) => VersionDriftOutcome::ContentChangedVersionBumped,
        (true, false) => VersionDriftOutcome::ContentChangedVersionNotBumped,
        (false, true) => VersionDriftOutcome::VersionBumpedContentUnchanged,
    }
}

/// Convenience: `true` when [`check_drift`] reports a fail-closed
/// violation for this baseline/candidate pair.
pub fn has_drift(baseline: &RuleRecord, candidate: &RuleRecord) -> bool {
    check_drift(baseline, candidate).is_drift()
}

/// Manifest load/parse failure: malformed JSON, or JSON that parsed but
/// failed the typed [`RuleManifest`] shape (e.g. `schemaVersion` of
/// `0`, which this format treats as unset/invalid).
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ManifestError {
    /// The manifest bytes were not valid JSON, or did not decode into the
    /// typed [`RuleManifest`] shape.
    #[error("rule-version-manifest.json parse failed: {0}")]
    Parse(RuleFailureReason),
    /// The JSON decoded but failed a structural invariant (currently:
    /// `schemaVersion` must be `>= 1`).
    #[error("rule-version-manifest.json invalid: {0}")]
    Invalid(RuleFailureReason),
}

impl From<DecodeError> for ManifestError {
    fn from(err: DecodeError) -> Self {
        ManifestError::Invalid(crate::boundary_reason(err))
    }
}

/// Parse-at-boundary: decode validated manifest JSON into a [`RuleManifest`],
/// rejecting malformed JSON and a structurally invalid schema version.
pub fn decode_manifest(raw: &RuleManifestJson) -> Result<RuleManifest, ManifestError> {
    crate::boundary::version_drift::decode_manifest(raw)
}

/// Compute the pinned content hash for one [`RuleRecord`]: a
/// [`Sha256`]-branded digest (via `enforcer_core::hash_chain::link_digest`)
/// over the record's PARITY ARTIFACTS (`validator`, `fixtures`,
/// `doc_anchor` — the same fields [`parity_content_changed`] compares),
/// serialized deterministically as canonical JSON. Cosmetic fields
/// (`title`, `tags`) are excluded so a title rename does not force a hash
/// (and therefore version) bump, mirroring the pairwise oracle above.
pub fn hash_record(record: &RuleRecord) -> Sha256 {
    crate::boundary::version_drift::hash_record(record)
}

/// Hash arbitrary bytes into a branded [`Sha256`] via
/// `enforcer_core::hash_chain::link_digest`. `link_digest` always returns
/// `sha256:` + 64 lowercase hex chars by construction (`SHA256_PREFIX`
/// followed by a `{:02x}`-formatted loop over a fixed-size SHA-256
/// output — see its definition), which is exactly the shape
/// [`Sha256::try_from`] accepts. The `unreachable!` below matches the
/// same idiom `enforcer_core::hash_chain`'s own tests use for a
/// provably-impossible branch (workspace deny-wall forbids
/// `unwrap`/`expect`/explicit `panic!`, not `unreachable!`).
/// Compute the whole-registry manifest a [`RuleRegistry`]'s current
/// records would pin: one [`RuleManifestEntry`] per loaded record, keyed by
/// [`RuleId`]. `schema_version` is carried through unchanged from a
/// reference manifest (there is exactly one manifest schema today).
pub fn build_manifest(
    registry: &RuleRegistry,
    schema_version: RuleManifestSchemaVersion,
) -> RuleManifest {
    let entries = registry
        .iter()
        .map(|record| {
            (
                record.rule_id.clone(),
                RuleManifestEntry::new(record.version, hash_record(record)),
            )
        })
        .collect();
    RuleManifest::new(schema_version, entries)
}

/// Outcome of comparing one pinned [`RuleManifestEntry`] against the
/// recomputed hash/version for the same [`RuleId`] in the live registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestDrift {
    /// Pinned hash matches the recomputed hash, and the pinned version
    /// matches the record's live version. Clean.
    Unchanged,
    /// Recomputed hash matches, but the manifest's pinned `version` no
    /// longer matches the live record's `version` (should not happen
    /// without a hash change too; treated as drift out of caution).
    HashUnchangedVersionMismatch,
    /// The recomputed hash differs from the pinned hash AND `version`
    /// increased by the expected amount. Clean — a legitimate bump.
    HashChangedVersionBumped,
    /// The recomputed hash differs from the pinned hash but `version` did
    /// NOT increase. FAILS CLOSED: content drifted without a version bump.
    HashChangedVersionNotBumped,
    /// `version` increased but the recomputed hash matches the pinned
    /// hash — a hollow bump. FAILS CLOSED.
    VersionBumpedHashUnchanged,
    /// The rule id is pinned in the manifest but no longer present in the
    /// live registry (removed without a manifest update). FAILS CLOSED.
    MissingFromRegistry,
    /// The rule id is present in the live registry but has no pinned
    /// manifest entry (added without a manifest update). FAILS CLOSED.
    MissingFromManifest,
}

impl ManifestDrift {
    /// True when this outcome represents a fail-closed drift violation.
    /// [`ManifestDrift::Unchanged`] and [`ManifestDrift::HashChangedVersionBumped`]
    /// (a legitimate, matched version+hash bump) are the only two CLEAN
    /// shapes; every other variant fails closed.
    pub fn is_drift(&self) -> bool {
        !matches!(
            self,
            ManifestDrift::Unchanged | ManifestDrift::HashChangedVersionBumped
        )
    }
}

/// One named, fail-closed drift result: which [`RuleId`] drifted and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestDriftEntry {
    /// The rule id this drift result concerns.
    pub rule_id: RuleId,
    /// The classified drift outcome.
    pub outcome: ManifestDrift,
}

/// Compare a pinned [`RuleManifest`] against a live [`RuleRegistry`],
/// returning one [`ManifestDriftEntry`] per rule id that appears in either
/// side (manifest union registry), fail-closed on any shape that is not
/// exactly `Unchanged`.
pub fn check_registry_drift(
    manifest: &RuleManifest,
    registry: &RuleRegistry,
) -> Vec<ManifestDriftEntry> {
    let mut rule_ids: std::collections::BTreeSet<&RuleId> = manifest.rule_ids().collect();
    rule_ids.extend(registry.iter().map(|record| &record.rule_id));

    rule_ids
        .into_iter()
        .map(|rule_id| {
            let pinned = manifest.entry(rule_id);
            let live = registry.get(rule_id);
            let outcome = match (pinned, live) {
                (None, None) => unreachable!("rule_id came from one of the two sets"),
                (Some(_), None) => ManifestDrift::MissingFromRegistry,
                (None, Some(_)) => ManifestDrift::MissingFromManifest,
                (Some(entry), Some(record)) => {
                    let recomputed = hash_record(record);
                    let hash_changed = recomputed != *entry.hash();
                    // Three-way version comparison, not boolean: a version
                    // DECREASE with an unchanged hash is distinguished from
                    // an exact match, so a hand-edited manifest rollback
                    // (same content, lower pinned version) is still named
                    // as drift rather than silently folding into
                    // `Unchanged`. A decrease with a changed hash still
                    // fails closed the same way a non-bump does.
                    match (hash_changed, record.version.cmp(&entry.version())) {
                        (false, std::cmp::Ordering::Equal) => ManifestDrift::Unchanged,
                        (false, std::cmp::Ordering::Greater) => {
                            ManifestDrift::VersionBumpedHashUnchanged
                        }
                        (false, std::cmp::Ordering::Less) => {
                            ManifestDrift::HashUnchangedVersionMismatch
                        }
                        (true, std::cmp::Ordering::Greater) => {
                            ManifestDrift::HashChangedVersionBumped
                        }
                        (true, std::cmp::Ordering::Equal | std::cmp::Ordering::Less) => {
                            ManifestDrift::HashChangedVersionNotBumped
                        }
                    }
                }
            };
            ManifestDriftEntry {
                rule_id: rule_id.clone(),
                outcome,
            }
        })
        .collect()
}

/// Repo-relative path the drift [`Finding`]s below point at: the manifest
/// is the file an author fixes (either by re-pinning the hash+version, or
/// by reverting the unintended registry content change).
const MANIFEST_FILE: &str = "crates/enforcer-rules/rule-version-manifest.json";

/// Render every fail-closed [`ManifestDriftEntry`] (per [`ManifestDrift::is_drift`])
/// as a [`Finding`] naming the offending rule id with a terse `Fix:` hint.
/// Entries that are [`ManifestDrift::Unchanged`] produce no finding.
pub fn drift_findings(entries: &[ManifestDriftEntry]) -> Vec<Finding> {
    entries
        .iter()
        .filter(|entry| entry.outcome.is_drift())
        .filter_map(|entry| {
            let file: enforcer_domain::paths::RelPath = MANIFEST_FILE.parse().ok()?;
            let (detail, fix) = drift_message(entry);
            Some(Finding {
                rule_id: entry.rule_id.clone(),
                severity: Severity::Error,
                title: "Rule registry drifted from its pinned version manifest".to_owned(),
                detail: format!("{detail} Fix: {fix}"),
                file,
                line: 1,
                snippet: None,
            })
        })
        .collect()
}

/// Build the `(detail, fix-hint)` pair for one drifted entry, naming the
/// rule id and the concrete corrective action. Never called with
/// [`ManifestDrift::Unchanged`] or [`ManifestDrift::HashChangedVersionBumped`]
/// — [`drift_findings`] filters both clean variants out via
/// [`ManifestDrift::is_drift`] before reaching this function.
fn drift_message(entry: &ManifestDriftEntry) -> (String, String) {
    let rule_id = &entry.rule_id;
    match &entry.outcome {
        ManifestDrift::Unchanged => (
            format!("`{rule_id}` is unchanged (unreachable: not a drift outcome)."),
            "no action needed.".to_owned(),
        ),
        ManifestDrift::HashChangedVersionBumped => (
            format!(
                "`{rule_id}` changed and its manifest entry is up to date (unreachable: not a drift outcome)."
            ),
            "no action needed.".to_owned(),
        ),
        ManifestDrift::HashUnchangedVersionMismatch => (
            format!(
                "`{rule_id}`'s content hash matches but its pinned `version` in rule-version-manifest.json does not match the live RuleRecord.version."
            ),
            format!(
                "re-pin `{rule_id}` in rule-version-manifest.json with the version that matches its current record."
            ),
        ),
        ManifestDrift::HashChangedVersionNotBumped => (
            format!(
                "`{rule_id}`'s validator/fixtures/doc-anchor changed but its RuleRecord.version was not bumped, so rule-version-manifest.json's pinned hash no longer matches."
            ),
            format!(
                "bump `{rule_id}`'s `version` in its rule catalog record AND re-run the manifest generator to pin the new hash."
            ),
        ),
        ManifestDrift::VersionBumpedHashUnchanged => (
            format!(
                "`{rule_id}`'s RuleRecord.version was bumped but its validator/fixtures/doc-anchor content did not change (hollow bump)."
            ),
            format!(
                "revert the unexplained `version` bump on `{rule_id}`, or make the intended content change before bumping."
            ),
        ),
        ManifestDrift::MissingFromRegistry => (
            format!(
                "`{rule_id}` is pinned in rule-version-manifest.json but no longer loads from any rule catalog."
            ),
            format!("remove `{rule_id}` from rule-version-manifest.json, or restore its catalog record."),
        ),
        ManifestDrift::MissingFromManifest => (
            format!(
                "`{rule_id}` loads from a rule catalog but has no pinned entry in rule-version-manifest.json."
            ),
            format!("add a `{rule_id}` entry to rule-version-manifest.json pinning its current version+hash."),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{check_drift, has_drift};
    use crate::registry::{FixtureRef, RuleRecord, ValidatorRef};
    use enforcer_domain::rules_types::VersionDriftOutcome;
    use enforcer_domain::{
        rules_types::{RuleParameters, RuleVersion},
        severity::Tier,
    };

    fn base() -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: "RR-3.1".parse()?,
            version: RuleVersion::new(1)?,
            title: "Base rule".parse()?,
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".parse()?,
                path: "sample::SampleValidator".parse()?,
            },
            fixtures: FixtureRef {
                fail: "fixtures/fail.rs".parse()?,
                pass: "fixtures/pass.rs".parse()?,
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".parse()?,
            tags: vec![],
            params: RuleParameters::default(),
        })
    }

    #[test]
    fn identical_records_are_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let baseline = base()?;
        let candidate = baseline.clone();
        assert_eq!(
            check_drift(&baseline, &candidate),
            VersionDriftOutcome::Unchanged
        );
        assert!(!has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn content_change_with_matching_version_bump_is_clean() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".parse()?;
        candidate.version = RuleVersion::new(2)?;
        assert_eq!(
            check_drift(&baseline, &candidate),
            VersionDriftOutcome::ContentChangedVersionBumped
        );
        assert!(!has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn content_change_without_version_bump_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.fixtures.fail = "fixtures/fail-v2.rs".parse()?;
        // version left at 1 — the seeded drift.
        assert_eq!(
            check_drift(&baseline, &candidate),
            VersionDriftOutcome::ContentChangedVersionNotBumped
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn validator_change_without_version_bump_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.validator.path = "sample::OtherValidator".parse()?;
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn version_bump_without_content_change_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.version = RuleVersion::new(2)?;
        // No validator/fixtures/doc_anchor change — a hollow bump.
        assert_eq!(
            check_drift(&baseline, &candidate),
            VersionDriftOutcome::VersionBumpedContentUnchanged
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn cosmetic_only_changes_do_not_count_as_content_drift(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline = base()?;
        let mut candidate = baseline.clone();
        candidate.title = "Renamed title".parse()?;
        candidate.tags = vec!["extra".parse()?];
        // version left at 1 — fine, because title/tags are not parity
        // artifacts.
        assert_eq!(
            check_drift(&baseline, &candidate),
            VersionDriftOutcome::Unchanged
        );
        assert!(!has_drift(&baseline, &candidate));
        Ok(())
    }

    #[test]
    fn version_decrease_with_content_change_fails_closed() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut baseline = base()?;
        baseline.version = RuleVersion::new(5)?;
        let mut candidate = baseline.clone();
        candidate.version = RuleVersion::new(4)?;
        candidate.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-3".parse()?;
        assert_eq!(
            check_drift(&baseline, &candidate),
            VersionDriftOutcome::ContentChangedVersionNotBumped
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
            VersionDriftOutcome::RuleIdMismatch
        );
        assert!(has_drift(&baseline, &candidate));
        Ok(())
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::{
        build_manifest, check_registry_drift, decode_manifest, hash_record, ManifestDrift,
    };
    use crate::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
    use enforcer_domain::{
        rules_types::{RuleManifestSchemaVersion, RuleParameters, RuleVersion},
        severity::Tier,
    };

    fn sample(
        rule_id: &str,
        version: u32,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: RuleVersion::new(version)?,
            title: "Sample rule".parse()?,
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".parse()?,
                path: "sample::SampleValidator".parse()?,
            },
            fixtures: FixtureRef {
                fail: "fixtures/fail.rs".parse()?,
                pass: "fixtures/pass.rs".parse()?,
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".parse()?,
            tags: vec![],
            params: RuleParameters::default(),
        })
    }

    #[test]
    fn hash_record_is_deterministic_and_ignores_cosmetic_fields(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let a = sample("RR-9.1", 1)?;
        let mut b = a.clone();
        b.title = "Renamed title".parse()?;
        b.tags = vec!["extra".parse()?];
        assert_eq!(hash_record(&a), hash_record(&b));
        Ok(())
    }

    #[test]
    fn hash_record_changes_when_a_parity_artifact_changes() -> Result<(), Box<dyn std::error::Error>>
    {
        let a = sample("RR-9.1", 1)?;
        let mut b = a.clone();
        b.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".parse()?;
        assert_ne!(hash_record(&a), hash_record(&b));
        Ok(())
    }

    #[test]
    fn hash_record_produces_a_valid_sha256_brand() -> Result<(), Box<dyn std::error::Error>> {
        let record = sample("RR-9.1", 1)?;
        let digest = hash_record(&record);
        assert_eq!(digest.hex().len(), 64);
        assert!(digest.as_str().starts_with("sha256:"));
        Ok(())
    }

    #[test]
    fn manifest_parse_rejects_malformed_json() {
        let outcome =
            enforcer_domain::rules_types::RuleManifestJson::try_from("{not json".to_owned())
                .map_err(super::ManifestError::from)
                .and_then(|raw| decode_manifest(&raw));
        assert!(outcome.is_err());
    }

    #[test]
    fn manifest_parse_rejects_zero_schema_version() -> Result<(), Box<dyn std::error::Error>> {
        let record = sample("RR-9.1", 1)?;
        let hash = hash_record(&record);
        let raw = format!(
            r#"{{"schemaVersion":0,"entries":{{"RR-9.1":{{"version":1,"hash":"{hash}"}}}}}}"#,
        );
        let raw = enforcer_domain::rules_types::RuleManifestJson::try_from(raw)?;
        assert!(decode_manifest(&raw).is_err());
        Ok(())
    }

    #[test]
    fn manifest_round_trips_through_serde() -> Result<(), Box<dyn std::error::Error>> {
        let record = sample("RR-9.1", 1)?;
        let hash = hash_record(&record);
        let raw = format!(
            r#"{{"schemaVersion":1,"entries":{{"RR-9.1":{{"version":1,"hash":"{hash}"}}}}}}"#,
        );
        let raw = enforcer_domain::rules_types::RuleManifestJson::try_from(raw)?;
        let manifest = decode_manifest(&raw)?;
        assert_eq!(manifest.schema_version().value(), 1);
        let entry = manifest
            .entry(&"RR-9.1".parse()?)
            .ok_or("expected RR-9.1 entry")?;
        assert_eq!(entry.version().value(), 1);
        assert_eq!(entry.hash(), &hash);
        Ok(())
    }

    #[test]
    fn build_manifest_pins_every_loaded_record() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![sample("RR-9.1", 1)?, sample("RR-9.2", 3)?])
            .map_err(|e| e.to_string())?;
        let manifest = build_manifest(&registry, RuleManifestSchemaVersion::new(1)?);
        assert_eq!(manifest.len(), 2);
        let entry = manifest
            .entry(&"RR-9.2".parse()?)
            .ok_or("expected RR-9.2 entry")?;
        assert_eq!(entry.version().value(), 3);
        Ok(())
    }

    #[test]
    fn unchanged_registry_matches_its_own_generated_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&registry, RuleManifestSchemaVersion::new(1)?);
        let drift = check_registry_drift(&manifest, &registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::Unchanged);
        assert!(!drift[0].outcome.is_drift());
        Ok(())
    }

    #[test]
    fn content_change_without_version_bump_fails_closed_against_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&baseline_registry, RuleManifestSchemaVersion::new(1)?);

        // Candidate registry: doc_anchor changed, version left at 1 — the
        // seeded drift the acceptance block requires this test to prove.
        let mut drifted = sample("RR-9.1", 1)?;
        drifted.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".parse()?;
        let candidate_registry =
            RuleRegistry::from_records(vec![drifted]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &candidate_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::HashChangedVersionNotBumped);
        assert!(drift[0].outcome.is_drift());

        let findings = super::drift_findings(&drift);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "RR-9.1".parse()?);
        assert!(findings[0].detail.contains("Fix:"));
        assert!(findings[0].detail.contains("RR-9.1"));
        Ok(())
    }

    #[test]
    fn version_bump_without_content_change_fails_closed_against_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&baseline_registry, RuleManifestSchemaVersion::new(1)?);

        // Candidate registry: version bumped, no parity-artifact content
        // change at all — a hollow bump, the acceptance block's other
        // required seeded-fail scenario.
        let bumped = sample("RR-9.1", 2)?;
        let candidate_registry =
            RuleRegistry::from_records(vec![bumped]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &candidate_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::VersionBumpedHashUnchanged);
        assert!(drift[0].outcome.is_drift());

        let findings = super::drift_findings(&drift);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("hollow bump") || findings[0].detail.contains("Fix:"));
        Ok(())
    }

    #[test]
    fn matched_version_and_hash_bump_together_passes() -> Result<(), Box<dyn std::error::Error>> {
        let baseline_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&baseline_registry, RuleManifestSchemaVersion::new(1)?);

        // Legitimate change: content changed AND version bumped together.
        let mut bumped = sample("RR-9.1", 2)?;
        bumped.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".parse()?;
        let candidate_registry =
            RuleRegistry::from_records(vec![bumped]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &candidate_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::HashChangedVersionBumped);
        assert!(!drift[0].outcome.is_drift());
        assert!(super::drift_findings(&drift).is_empty());
        Ok(())
    }

    #[test]
    fn rule_id_removed_from_registry_without_manifest_update_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&baseline_registry, RuleManifestSchemaVersion::new(1)?);
        let empty_registry = RuleRegistry::from_records(vec![]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &empty_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::MissingFromRegistry);
        assert!(drift[0].outcome.is_drift());
        Ok(())
    }

    #[test]
    fn rule_id_added_to_registry_without_manifest_update_fails_closed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let empty_registry = RuleRegistry::from_records(vec![]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&empty_registry, RuleManifestSchemaVersion::new(1)?);

        let candidate_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &candidate_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::MissingFromManifest);
        assert!(drift[0].outcome.is_drift());
        Ok(())
    }

    #[test]
    fn manifest_entry_preserves_branded_values() -> Result<(), Box<dyn std::error::Error>> {
        let entry = enforcer_domain::rules_types::RuleManifestEntry::new(
            RuleVersion::new(3)?,
            hash_record(&sample("RR-9.1", 3)?),
        );
        assert_eq!(entry.version().value(), 3);
        Ok(())
    }

    #[test]
    fn version_decrease_with_unchanged_content_fails_closed_against_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Pin at version 5 (no content change relative to the baseline
        // sample), then present a candidate registry with the SAME
        // content but a hand-edited-looking lower version — the manifest
        // entry itself pins version 5 while the live record still reads
        // 5 too in `sample`, so seed the mismatch on the manifest side by
        // building it from a higher-version baseline than the live record.
        let higher_version_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 5)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&higher_version_registry, RuleManifestSchemaVersion::new(1)?);

        let lower_version_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 3)?]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &lower_version_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(
            drift[0].outcome,
            ManifestDrift::HashUnchangedVersionMismatch
        );
        assert!(drift[0].outcome.is_drift());

        let findings = super::drift_findings(&drift);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("Fix:"));
        Ok(())
    }
}
