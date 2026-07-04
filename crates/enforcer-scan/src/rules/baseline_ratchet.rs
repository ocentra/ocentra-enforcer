//! d02 baseline-ratchet: a monotonic violation-count baseline. New
//! violations fail closed (a violation not present in the recorded
//! baseline blocks); a violation that used to be in the baseline but is
//! no longer produced ratchets the recorded baseline DOWN — the baseline
//! can only shrink over time, never grow to "grandfather in" a fresh
//! violation.
//!
//! **SKELETON BOUNDARY**: arc-15 hosts this file as the crate skeleton's
//! d02 seam per the WORKPACK_INDEX split (`src/rules/baseline_ratchet.rs`
//! is d02-owned content, landed here because f01/f05/d02 are hosted in
//! THIS skeleton per the workpack, not spun out as separate feature
//! packs). This implementation is the full behavioral contract: fail
//! closed on new violations, ratchet down on fixed ones, never ratchet
//! up.

use std::collections::BTreeSet;

use enforcer_domain::findings::Violation;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;

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
