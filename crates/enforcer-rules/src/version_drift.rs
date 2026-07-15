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
//!
//! # Whole-registry manifest drift (d13)
//!
//! [`check_drift`]/[`has_drift`] above are the PAIRWISE oracle arc-04
//! shipped: one baseline record vs one candidate record. d13 adds the
//! WHOLE-REGISTRY layer on top: [`RegistryManifest`] is a versioned,
//! `serde`-typed record (parsed at the boundary via
//! [`RegistryManifest::parse`], never a bare `String`) pinning a
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

use std::collections::BTreeMap;

use enforcer_core::hash_chain;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::Finding;
use enforcer_domain::hashes::Sha256;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;

use crate::registry::{RuleRecord, RuleRegistry};

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

/// One pinned manifest entry: the `version` and content [`Sha256`] a rule
/// record is expected to carry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    /// Pinned `RuleRecord::version` for this rule id.
    pub version: u32,
    /// Pinned content hash for this rule id, computed by [`hash_record`].
    pub hash: Sha256,
}

/// The versioned, `serde`-typed `rule-version-manifest.json` record: a
/// `schemaVersion` plus one [`ManifestEntry`] per pinned [`RuleId`].
/// Parsed only via [`RegistryManifest::parse`] (parse-at-boundary; no bare
/// `String`/`serde_json::Value` escapes this module as a live value).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryManifest {
    /// Manifest wire-schema version (independent of any single rule's
    /// `version`); bumped only if this record's own shape changes.
    pub schema_version: u32,
    /// Pinned `{ version, hash }` per rule id, in the manifest file.
    pub entries: BTreeMap<RuleId, ManifestEntry>,
}

/// Manifest load/parse failure: malformed JSON, or JSON that parsed but
/// failed the typed [`RegistryManifest`] shape (e.g. `schemaVersion` of
/// `0`, which this format treats as unset/invalid).
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ManifestError {
    /// The manifest bytes were not valid JSON, or did not decode into the
    /// typed [`RegistryManifest`] shape.
    #[error("rule-version-manifest.json parse failed: {0}")]
    Parse(String),
    /// The JSON decoded but failed a structural invariant (currently:
    /// `schemaVersion` must be `>= 1`).
    #[error("rule-version-manifest.json invalid: {0}")]
    Invalid(String),
}

impl From<DecodeError> for ManifestError {
    fn from(err: DecodeError) -> Self {
        ManifestError::Invalid(err.to_string())
    }
}

impl RegistryManifest {
    /// Parse-at-boundary: decode `raw` JSON into a [`RegistryManifest`],
    /// rejecting malformed JSON and a structurally invalid schema version.
    /// This is the ONLY sanctioned way to obtain a live `RegistryManifest`
    /// value — there is no public constructor that skips validation.
    pub fn parse(raw: &str) -> Result<Self, ManifestError> {
        let manifest: RegistryManifest =
            serde_json::from_str(raw).map_err(|e| ManifestError::Parse(e.to_string()))?;
        if manifest.schema_version == 0 {
            return Err(ManifestError::Invalid(
                "schemaVersion must be >= 1".to_owned(),
            ));
        }
        Ok(manifest)
    }
}

/// Compute the pinned content hash for one [`RuleRecord`]: a
/// [`Sha256`]-branded digest (via `enforcer_core::hash_chain::link_digest`)
/// over the record's PARITY ARTIFACTS (`validator`, `fixtures`,
/// `doc_anchor` — the same fields [`parity_content_changed`] compares),
/// serialized deterministically as canonical JSON. Cosmetic fields
/// (`title`, `tags`) are excluded so a title rename does not force a hash
/// (and therefore version) bump, mirroring the pairwise oracle above.
pub fn hash_record(record: &RuleRecord) -> Sha256 {
    // A small canonical struct scoped to exactly the parity-artifact
    // fields, so the hash input is stable regardless of unrelated
    // `RuleRecord` field additions (e.g. a future `params` shape change
    // that is not itself a parity artifact).
    #[derive(serde::Serialize)]
    struct CanonicalParity<'a> {
        rule_id: &'a RuleId,
        validator: &'a crate::registry::ValidatorRef,
        fixtures: &'a crate::registry::FixtureRef,
        doc_anchor: &'a str,
    }
    let canonical = CanonicalParity {
        rule_id: &record.rule_id,
        validator: &record.validator,
        fixtures: &record.fixtures,
        doc_anchor: &record.doc_anchor,
    };
    // `serde_json::to_vec` on a struct of owned/borrowed scalars with a
    // fixed field order (serde_json never reorders object keys) fails
    // only on a type that cannot arise here (a non-string map key); an
    // empty payload is an acceptable, still-deterministic degenerate
    // input rather than a call to `unwrap`/`expect` (workspace deny-wall).
    let payload = serde_json::to_vec(&canonical).unwrap_or_default();
    hash_bytes(&payload)
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
fn hash_bytes(payload: &[u8]) -> Sha256 {
    let digest = hash_chain::link_digest(None, payload);
    match Sha256::try_from(digest) {
        Ok(sha) => sha,
        Err(_) => unreachable!(
            "enforcer_core::hash_chain::link_digest always returns `sha256:` + 64 \
             lowercase hex chars, which Sha256::try_from always accepts"
        ),
    }
}

/// Compute the whole-registry manifest a [`RuleRegistry`]'s current
/// records would pin: one [`ManifestEntry`] per loaded record, keyed by
/// [`RuleId`]. `schema_version` is carried through unchanged from a
/// reference manifest (there is exactly one manifest schema today).
pub fn build_manifest(registry: &RuleRegistry, schema_version: u32) -> RegistryManifest {
    let entries = registry
        .iter()
        .map(|record| {
            (
                record.rule_id.clone(),
                ManifestEntry {
                    version: record.version,
                    hash: hash_record(record),
                },
            )
        })
        .collect();
    RegistryManifest {
        schema_version,
        entries,
    }
}

/// Outcome of comparing one pinned [`ManifestEntry`] against the
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

/// Compare a pinned [`RegistryManifest`] against a live [`RuleRegistry`],
/// returning one [`ManifestDriftEntry`] per rule id that appears in either
/// side (manifest union registry), fail-closed on any shape that is not
/// exactly `Unchanged`.
pub fn check_registry_drift(
    manifest: &RegistryManifest,
    registry: &RuleRegistry,
) -> Vec<ManifestDriftEntry> {
    let mut rule_ids: std::collections::BTreeSet<&RuleId> = manifest.entries.keys().collect();
    rule_ids.extend(registry.iter().map(|record| &record.rule_id));

    rule_ids
        .into_iter()
        .map(|rule_id| {
            let pinned = manifest.entries.get(rule_id);
            let live = registry.get(rule_id);
            let outcome = match (pinned, live) {
                (None, None) => unreachable!("rule_id came from one of the two sets"),
                (Some(_), None) => ManifestDrift::MissingFromRegistry,
                (None, Some(_)) => ManifestDrift::MissingFromManifest,
                (Some(entry), Some(record)) => {
                    let recomputed = hash_record(record);
                    let hash_changed = recomputed != entry.hash;
                    // Three-way version comparison, not boolean: a version
                    // DECREASE with an unchanged hash is distinguished from
                    // an exact match, so a hand-edited manifest rollback
                    // (same content, lower pinned version) is still named
                    // as drift rather than silently folding into
                    // `Unchanged`. A decrease with a changed hash still
                    // fails closed the same way a non-bump does.
                    match (hash_changed, record.version.cmp(&entry.version)) {
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
    use super::{check_drift, has_drift, DriftOutcome};
    use crate::registry::{FixtureRef, RuleRecord, ValidatorRef};
    use enforcer_domain::severity::Tier;

    fn base() -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
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

#[cfg(test)]
mod manifest_tests {
    use super::{
        build_manifest, check_registry_drift, hash_record, ManifestDrift, ManifestEntry,
        RegistryManifest,
    };
    use crate::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
    use enforcer_domain::severity::Tier;

    fn sample(
        rule_id: &str,
        version: u32,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version,
            title: "Sample rule".to_owned(),
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
    fn hash_record_is_deterministic_and_ignores_cosmetic_fields(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let a = sample("RR-9.1", 1)?;
        let mut b = a.clone();
        b.title = "Renamed title".to_owned();
        b.tags = vec!["extra".to_owned()];
        assert_eq!(hash_record(&a), hash_record(&b));
        Ok(())
    }

    #[test]
    fn hash_record_changes_when_a_parity_artifact_changes() -> Result<(), Box<dyn std::error::Error>>
    {
        let a = sample("RR-9.1", 1)?;
        let mut b = a.clone();
        b.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".to_owned();
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
        let outcome = super::RegistryManifest::parse("{not json");
        assert!(outcome.is_err());
    }

    #[test]
    fn manifest_parse_rejects_zero_schema_version() -> Result<(), Box<dyn std::error::Error>> {
        let record = sample("RR-9.1", 1)?;
        let hash = hash_record(&record);
        let raw = format!(
            r#"{{"schemaVersion":0,"entries":{{"RR-9.1":{{"version":1,"hash":"{hash}"}}}}}}"#,
        );
        assert!(super::RegistryManifest::parse(&raw).is_err());
        Ok(())
    }

    #[test]
    fn manifest_round_trips_through_serde() -> Result<(), Box<dyn std::error::Error>> {
        let record = sample("RR-9.1", 1)?;
        let hash = hash_record(&record);
        let raw = format!(
            r#"{{"schemaVersion":1,"entries":{{"RR-9.1":{{"version":1,"hash":"{hash}"}}}}}}"#,
        );
        let manifest = RegistryManifest::parse(&raw)?;
        assert_eq!(manifest.schema_version, 1);
        let entry = manifest
            .entries
            .get(&"RR-9.1".parse()?)
            .ok_or("expected RR-9.1 entry")?;
        assert_eq!(entry.version, 1);
        assert_eq!(entry.hash, hash);
        Ok(())
    }

    #[test]
    fn build_manifest_pins_every_loaded_record() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![sample("RR-9.1", 1)?, sample("RR-9.2", 3)?])
            .map_err(|e| e.to_string())?;
        let manifest = build_manifest(&registry, 1);
        assert_eq!(manifest.entries.len(), 2);
        let entry = manifest
            .entries
            .get(&"RR-9.2".parse()?)
            .ok_or("expected RR-9.2 entry")?;
        assert_eq!(entry.version, 3);
        Ok(())
    }

    #[test]
    fn unchanged_registry_matches_its_own_generated_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;
        let manifest = build_manifest(&registry, 1);
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
        let manifest = build_manifest(&baseline_registry, 1);

        // Candidate registry: doc_anchor changed, version left at 1 — the
        // seeded drift the acceptance block requires this test to prove.
        let mut drifted = sample("RR-9.1", 1)?;
        drifted.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".to_owned();
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
        let manifest = build_manifest(&baseline_registry, 1);

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
        let manifest = build_manifest(&baseline_registry, 1);

        // Legitimate change: content changed AND version bumped together.
        let mut bumped = sample("RR-9.1", 2)?;
        bumped.doc_anchor = "docs/rules/SAMPLE.md#SAMPLE-2".to_owned();
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
        let manifest = build_manifest(&baseline_registry, 1);
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
        let manifest = build_manifest(&empty_registry, 1);

        let candidate_registry =
            RuleRegistry::from_records(vec![sample("RR-9.1", 1)?]).map_err(|e| e.to_string())?;

        let drift = check_registry_drift(&manifest, &candidate_registry);
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].outcome, ManifestDrift::MissingFromManifest);
        assert!(drift[0].outcome.is_drift());
        Ok(())
    }

    #[test]
    fn manifest_entry_serde_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let entry = ManifestEntry {
            version: 3,
            hash: hash_record(&sample("RR-9.1", 3)?),
        };
        let wire = serde_json::to_string(&entry)?;
        let back: ManifestEntry = serde_json::from_str(&wire)?;
        assert_eq!(back, entry);
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
        let manifest = build_manifest(&higher_version_registry, 1);

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
