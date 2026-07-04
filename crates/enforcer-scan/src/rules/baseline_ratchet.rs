//! d02 baseline-ratchet: a monotonic violation-count baseline. New
//! violations fail closed (a violation not present in the recorded
//! baseline blocks); a violation that used to be in the baseline but is
//! no longer produced ratchets the recorded baseline DOWN — the baseline
//! can only shrink over time, never grow to "grandfather in" a fresh
//! violation.
//!
//! d02 builds three layers on top of arc-15's in-memory
//! [`ratchet`]/[`Baseline`]/[`BaselineKey`] core (which stays exactly as
//! arc-15 landed it — see the skeleton note preserved on those items):
//!
//! - [`BaselineRecord`]: the versioned `serde` wire form of a [`Baseline`],
//!   carrying a [`Sha256`] integrity hash over its own entry payload so a
//!   hand-edited or corrupted baseline file is detected rather than
//!   silently trusted.
//! - [`write_baseline`] / [`load_baseline`]: the persistence boundary
//!   (`enforcer check --baseline write` writes via the former; a
//!   `--baseline` run loads via the latter, verifying the hash before
//!   trusting the record).
//! - [`BaselineRatchetValidator`]: a `Report`-level classifier (the scan
//!   crate's `Validator`-shaped mode, per the workpack's own "Validator/
//!   mode" phrasing — this gate inspects a whole run's [`Violation`]s
//!   against a loaded baseline, not one file's source text, so it does not
//!   implement `enforcer_validator::validator::Validator` itself) that
//!   turns [`ratchet`]'s outcome into `enforcer-domain` findings: baselined
//!   violations demote to warnings, new/grown violations stay errors.

use std::collections::BTreeSet;
use std::path::Path;

use enforcer_core::error::{DecodeError, Error as CoreError, Result as CoreResult};
use enforcer_core::hash_chain::link_digest;
use enforcer_domain::findings::{Finding, Violation};
use enforcer_domain::hashes::Sha256;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;

/// Schema version of [`BaselineRecord`]'s on-disk wire form. Bump on any
/// breaking change to the record shape so an old baseline file fails to
/// load loudly instead of silently misinterpreting.
pub const BASELINE_RECORD_VERSION: u32 = 1;

/// One baseline entry: the (rule, file, line) triple that identifies a
/// specific known violation occurrence. Deliberately does not include the
/// message/detail text — a rule's wording changing should not invalidate
/// the baseline entry for the same occurrence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BaselineKey {
    /// The rule that fired.
    pub rule_id: RuleId,
    /// The file the violation was recorded against.
    pub file: RelPath,
    /// The line the violation was recorded at.
    pub line: u32,
}

impl BaselineKey {
    /// Derive the baseline key for a violation.
    pub fn for_violation(violation: &Violation) -> Self {
        let finding = violation.finding();
        Self {
            rule_id: finding.rule_id.clone(),
            file: finding.file.clone(),
            line: finding.line,
        }
    }
}

/// A recorded baseline: the set of violation occurrences accepted as
/// "already known" as of the last ratchet. Ordered (`BTreeSet`) so two
/// baselines with the same members always compare/serialize identically —
/// this is part of the idempotency contract the parent crate leans on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Baseline {
    known: BTreeSet<BaselineKey>,
}

impl Baseline {
    /// Build a baseline from an explicit set of known keys (e.g. loaded
    /// from a recorded baseline file — the load/persist boundary is a
    /// separate concern this module does not own).
    pub fn from_known(known: impl IntoIterator<Item = BaselineKey>) -> Self {
        Self {
            known: known.into_iter().collect(),
        }
    }

    /// How many occurrences this baseline currently records.
    pub fn len(&self) -> usize {
        self.known.len()
    }

    /// True if this baseline records no occurrences.
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }
}

/// The outcome of ratcheting a baseline against a fresh scan's violations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatchetOutcome {
    /// Violations present in the fresh scan but NOT in the prior baseline
    /// — these fail closed (block), regardless of how many other
    /// violations the baseline already tolerated.
    pub new_violations: Vec<Violation>,
    /// The ratcheted baseline: exactly the occurrences from the prior
    /// baseline that are STILL present in the fresh scan, plus every new
    /// violation (which becomes known going forward). A key present in
    /// the prior baseline but absent from the fresh scan is dropped —
    /// this is the "ratchets down on a fix" half of the contract.
    pub ratcheted_baseline: Baseline,
}

impl RatchetOutcome {
    /// True if the ratchet found no new (unbaselined) violations — the
    /// scan is clean with respect to the baseline, whether or not the
    /// baseline itself shrank.
    pub fn passes(&self) -> bool {
        self.new_violations.is_empty()
    }
}

/// Ratchet `prior` against a fresh scan's `current_violations`.
///
/// Fails closed: any violation in `current_violations` whose
/// [`BaselineKey`] is not in `prior` is reported in
/// [`RatchetOutcome::new_violations`] and [`RatchetOutcome::passes`]
/// returns `false`. Ratchets down: any key in `prior` with no matching
/// violation in `current_violations` is dropped from
/// [`RatchetOutcome::ratcheted_baseline`] — the baseline can only shrink,
/// never grow beyond what the current scan + newly-seen violations
/// justify.
pub fn ratchet(prior: &Baseline, current_violations: &[Violation]) -> RatchetOutcome {
    let mut new_violations = Vec::new();
    let mut still_present: BTreeSet<BaselineKey> = BTreeSet::new();

    for violation in current_violations {
        let key = BaselineKey::for_violation(violation);
        if prior.known.contains(&key) {
            still_present.insert(key);
        } else {
            new_violations.push(violation.clone());
            still_present.insert(key);
        }
    }

    RatchetOutcome {
        new_violations,
        ratcheted_baseline: Baseline {
            known: still_present,
        },
    }
}

/// One entry in the persisted baseline record's wire form. Mirrors
/// [`BaselineKey`] field-for-field, but as an explicit `serde` DTO —
/// [`BaselineKey`] itself stays internal/unserialized so the in-memory
/// core (arc-15's contract) is not coupled to the wire shape.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineEntry {
    /// The rule that fired.
    pub rule_id: RuleId,
    /// The file the violation was recorded against.
    pub file: RelPath,
    /// The line the violation was recorded at.
    pub line: u32,
}

impl From<&BaselineKey> for BaselineEntry {
    fn from(key: &BaselineKey) -> Self {
        Self {
            rule_id: key.rule_id.clone(),
            file: key.file.clone(),
            line: key.line,
        }
    }
}

impl From<BaselineEntry> for BaselineKey {
    fn from(entry: BaselineEntry) -> Self {
        Self {
            rule_id: entry.rule_id,
            file: entry.file,
            line: entry.line,
        }
    }
}

/// The versioned, integrity-hashed wire form of a [`Baseline`]. This is
/// what `enforcer check --baseline write` persists to the baseline file
/// and what a `--baseline` run loads back.
///
/// Entries are stored sorted ([`BaselineEntry`]'s derived `Ord`) so two
/// baselines with the same members always serialize byte-identically —
/// same idempotency contract [`Baseline`] itself upholds via `BTreeSet`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineRecord {
    /// Schema version; see [`BASELINE_RECORD_VERSION`].
    pub version: u32,
    /// The recorded occurrences, sorted for deterministic serialization.
    pub entries: Vec<BaselineEntry>,
    /// Integrity digest over `entries`' canonical JSON payload. Computed
    /// by [`BaselineRecord::compute_hash`]; verified on [`load_baseline`].
    pub integrity: Sha256,
}

impl BaselineRecord {
    /// Build a record from a [`Baseline`], computing its integrity hash.
    pub fn from_baseline(baseline: &Baseline) -> CoreResult<Self> {
        let entries: Vec<BaselineEntry> = baseline.known.iter().map(BaselineEntry::from).collect();
        let integrity = Self::compute_hash(&entries)?;
        Ok(Self {
            version: BASELINE_RECORD_VERSION,
            entries,
            integrity,
        })
    }

    /// Recover the in-memory [`Baseline`] this record describes.
    pub fn to_baseline(&self) -> Baseline {
        Baseline::from_known(self.entries.iter().cloned().map(BaselineKey::from))
    }

    /// Verify `integrity` matches a freshly recomputed hash over `entries`.
    pub fn verify(&self) -> CoreResult<()> {
        let expected = Self::compute_hash(&self.entries)?;
        if expected == self.integrity {
            Ok(())
        } else {
            Err(CoreError::Decode(DecodeError::new(
                "baselineRecord.integrity",
                "recorded hash does not match recomputed entries; baseline file was tampered with or corrupted",
            )))
        }
    }

    /// Digest the canonical JSON payload of `entries` into a branded
    /// [`Sha256`]. Entries are sorted first so key order in the caller's
    /// collection never changes the digest.
    fn compute_hash(entries: &[BaselineEntry]) -> CoreResult<Sha256> {
        let mut sorted = entries.to_vec();
        sorted.sort();
        let payload = serde_json::to_vec(&sorted)?;
        let digest = link_digest(None, &payload);
        digest.parse::<Sha256>().map_err(CoreError::Decode)
    }
}

/// Write `baseline` to `path` as a [`BaselineRecord`] (pretty JSON, one
/// record per file — this is a snapshot, not an append log). This is the
/// `enforcer check --baseline write` persistence step.
pub fn write_baseline(path: &Path, baseline: &Baseline) -> CoreResult<()> {
    let record = BaselineRecord::from_baseline(baseline)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let payload = serde_json::to_vec_pretty(&record)?;
    std::fs::write(path, payload)?;
    Ok(())
}

/// Load a [`Baseline`] from `path`, verifying its integrity hash before
/// trusting the entries. Fails closed: a missing/corrupt/tampered file is
/// an error, never silently treated as an empty baseline (an empty
/// baseline must be written explicitly via [`write_baseline`]).
pub fn load_baseline(path: &Path) -> CoreResult<Baseline> {
    let payload = std::fs::read(path)?;
    let record: BaselineRecord = serde_json::from_slice(&payload)?;
    record.verify()?;
    Ok(record.to_baseline())
}

/// The outcome of running the baseline-ratchet gate over a fresh scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineGateOutcome {
    /// Violations NOT covered by the prior baseline — these stay
    /// blocking errors regardless of what the baseline already tolerates.
    pub errors: Vec<Violation>,
    /// Violations covered by the prior baseline — demoted from blocking
    /// violations to non-blocking warnings (as [`Finding`]s; a demoted
    /// entry is never itself a [`Violation`] since its severity is no
    /// longer `error`).
    pub warnings: Vec<Finding>,
    /// The ratcheted baseline to persist for the next run (see
    /// [`ratchet`]'s contract: shrinks on fixes, grows only to cover the
    /// violations just classified into `errors`).
    pub ratcheted_baseline: Baseline,
}

impl BaselineGateOutcome {
    /// True when the gate found no new/grown violations — i.e. every
    /// violation in the fresh scan was already covered by the baseline.
    pub fn passes(&self) -> bool {
        self.errors.is_empty()
    }
}

/// `enforcer-scan`'s baseline-ratchet gate: the `Report`-level mode a
/// `--baseline` run invokes. Classifies each of `current_violations`
/// against `prior`: in-baseline -> demoted to warning; not-in-baseline
/// (new OR — because [`BaselineKey`] does not carry a count, tracked at
/// the per-occurrence-line granularity the workpack's location
/// normalization calls for — grown past what the baseline recorded) ->
/// stays a blocking error. Delegates the set-diff itself to [`ratchet`]
/// so this gate and the in-memory core can never disagree about what
/// counts as "new".
pub struct BaselineRatchetValidator;

impl BaselineRatchetValidator {
    /// Run the gate. `current_violations` is the fresh scan's full
    /// violation list (already computed by the rest of `enforcer-scan`'s
    /// pipeline); `prior` is the baseline loaded via [`load_baseline`] (or
    /// [`Baseline::default`] for a first, unbaselined run — which fails
    /// closed on every current violation, as it must).
    pub fn gate(prior: &Baseline, current_violations: &[Violation]) -> BaselineGateOutcome {
        let outcome = ratchet(prior, current_violations);
        let new_keys: BTreeSet<BaselineKey> = outcome
            .new_violations
            .iter()
            .map(BaselineKey::for_violation)
            .collect();

        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        for violation in current_violations {
            let key = BaselineKey::for_violation(violation);
            if new_keys.contains(&key) {
                errors.push(violation.clone());
            } else {
                let mut demoted = violation.finding().clone();
                demoted.severity = Severity::Warning;
                warnings.push(demoted);
            }
        }

        BaselineGateOutcome {
            errors,
            warnings,
            ratcheted_baseline: outcome.ratcheted_baseline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ratchet, Baseline, BaselineKey};
    use enforcer_domain::findings::{Finding, Violation};
    use enforcer_domain::severity::Severity;

    fn violation(
        rule_id: &str,
        file: &str,
        line: u32,
    ) -> Result<Violation, Box<dyn std::error::Error>> {
        let finding = Finding {
            rule_id: rule_id.parse()?,
            severity: Severity::Error,
            title: "test".to_owned(),
            detail: "test detail".to_owned(),
            file: file.parse()?,
            line,
            snippet: None,
        };
        Ok(Violation::try_from(finding)?)
    }

    fn key(
        rule_id: &str,
        file: &str,
        line: u32,
    ) -> Result<BaselineKey, Box<dyn std::error::Error>> {
        Ok(BaselineKey {
            rule_id: rule_id.parse()?,
            file: file.parse()?,
            line,
        })
    }

    #[test]
    fn fails_closed_on_a_brand_new_violation() -> Result<(), Box<dyn std::error::Error>> {
        let prior = Baseline::default();
        let current = vec![violation("RR-6.1", "src/lib.rs", 10)?];
        let outcome = ratchet(&prior, &current);
        assert!(!outcome.passes(), "a new violation must fail closed");
        assert_eq!(outcome.new_violations.len(), 1);
        assert_eq!(outcome.ratcheted_baseline.len(), 1);
        Ok(())
    }

    #[test]
    fn known_violation_does_not_re_fail() -> Result<(), Box<dyn std::error::Error>> {
        let prior = Baseline::from_known([key("RR-6.1", "src/lib.rs", 10)?]);
        let current = vec![violation("RR-6.1", "src/lib.rs", 10)?];
        let outcome = ratchet(&prior, &current);
        assert!(
            outcome.passes(),
            "a known violation must not re-trip the gate"
        );
        assert!(outcome.new_violations.is_empty());
        Ok(())
    }

    #[test]
    fn ratchets_down_when_a_known_violation_is_fixed() -> Result<(), Box<dyn std::error::Error>> {
        let prior = Baseline::from_known([key("RR-6.1", "src/lib.rs", 10)?]);
        let current: Vec<Violation> = Vec::new();
        let outcome = ratchet(&prior, &current);
        assert!(outcome.passes());
        assert!(
            outcome.ratcheted_baseline.is_empty(),
            "a fixed violation must be dropped from the ratcheted baseline, not carried forward"
        );
        Ok(())
    }

    #[test]
    fn baseline_never_grows_beyond_current_plus_prior_still_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let baseline_key = key("RR-6.1", "src/lib.rs", 10)?;
        let prior = Baseline::from_known([baseline_key.clone()]);
        let current = vec![
            violation("RR-6.1", "src/lib.rs", 10)?,
            violation("RR-6.2", "src/other.rs", 5)?,
        ];
        let outcome = ratchet(&prior, &current);
        assert!(!outcome.passes(), "RR-6.2 is new and must fail closed");
        assert_eq!(outcome.ratcheted_baseline.len(), 2);
        assert!(outcome.ratcheted_baseline.known.contains(&baseline_key));
        Ok(())
    }
}
