//! The structured rule registry: typed rule records and the in-memory
//! [`RuleRegistry`] built from them.
//!
//! Every [`RuleRecord`] carries the full 5-way linkage the plan's doctrine
//! requires: `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor
//! <-> tier`. The registry rejects malformed or duplicate records at LOAD
//! time (fail-closed), so no invalid rule reaches a consumer (arc-05's
//! validator harness, the lang-* crates) as a live value.

use std::collections::BTreeMap;

use enforcer_domain::config_types::CrateName;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::{
    RuleDocAnchor, RuleParameters, RuleRecordCount, RuleRegistryState, RuleTag, RuleTitle,
    RuleVersion, ValidatorPath,
};
use enforcer_domain::severity::Tier;

use crate::{RuleLoadError, RuleResult};

/// A fixture reference: a repo-relative path to a fail or pass fixture file
/// that a rule's validator is proven against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureRef {
    /// Repo-relative path to the fixture that MUST trip the rule.
    pub fail: RelPath,
    /// Repo-relative path to the fixture that MUST NOT trip the rule.
    pub pass: RelPath,
}

/// Identifies the concrete `Validator` implementation a rule record is
/// paired with, without this crate depending on the validator crates
/// (`enforcer-validator`, `enforcer-lang-*`) that implement it — those
/// crates depend on `enforcer-rules`, not the reverse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorRef {
    /// The crate that owns the `Validator` implementation, e.g.
    /// `enforcer-lang-rust`.
    pub crate_name: CrateName,
    /// The type/function path within that crate, e.g.
    /// `no_reexports::NoReexportsValidator`.
    pub path: ValidatorPath,
}

/// One structured rule record: the unit the registry loads, validates, and
/// exposes. DTO shape only — this crate does not execute validators, it
/// carries the DATA a validator/harness consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleRecord {
    /// Branded rule identifier (e.g. `RR-6.1`).
    pub rule_id: RuleId,
    /// Record content version. Bumped whenever `validator`, `fixtures`, or
    /// `doc_anchor` changes; checked for drift by [`crate::version_drift`].
    pub version: RuleVersion,
    /// Short human title.
    pub title: RuleTitle,
    /// Mechanical-enforcement tier (T1/T2/T3).
    pub tier: Tier,
    /// The paired validator implementation reference.
    pub validator: ValidatorRef,
    /// Fail/pass fixture references proving the validator's behavior.
    pub fixtures: FixtureRef,
    /// Repo-relative path (or URL fragment) to the human-canonical doc
    /// anchor explaining the rule, e.g. `docs/rules/RR-6.md#RR-6.1`.
    pub doc_anchor: RuleDocAnchor,
    /// Free-form rule-family tags (e.g. `["rust", "reexports"]`) for
    /// grouping/filtering; not part of identity.
    pub tags: Vec<RuleTag>,
    /// Rule-family-specific structured parameters (e.g. the deny-wall's
    /// lint list, or the `blockedProtocolDependencies` map for a
    /// dependency-ban posture record). Opaque to the registry/loader —
    /// each validator interprets its own record's `params` shape. Absent
    /// (`null`) for rules that carry no extra parameters.
    pub params: RuleParameters,
}

/// The loaded, validated set of rule records, keyed by [`RuleId`].
#[derive(Debug, Clone, Default)]
pub struct RuleRegistry {
    records: BTreeMap<RuleId, RuleRecord>,
}

impl RuleRegistry {
    /// Build a registry from already-parsed records, rejecting duplicates
    /// and structurally-empty fields (fail-closed: a malformed catalog
    /// never becomes a live registry).
    pub fn from_records(records: Vec<RuleRecord>) -> RuleResult<Self> {
        let mut map = BTreeMap::new();
        for record in records {
            if map.contains_key(&record.rule_id) {
                return Err(RuleLoadError::DuplicateRuleId {
                    rule_id: record.rule_id,
                });
            }
            // CLONE-JUSTIFICATION: the map key and retained record each own the same canonical id.
            map.insert(record.rule_id.clone(), record);
        }
        Ok(Self { records: map })
    }

    /// Look up one record by id.
    pub fn get(&self, rule_id: &RuleId) -> Option<&RuleRecord> {
        self.records.get(rule_id)
    }

    /// Iterate every loaded record, in `RuleId` order.
    pub fn iter(&self) -> impl Iterator<Item = &RuleRecord> {
        self.records.values()
    }

    /// Number of loaded records.
    pub fn count(&self) -> RuleRecordCount {
        RuleRecordCount::from_records(self.records.values())
    }

    /// Whether records are loaded.
    pub fn state(&self) -> RuleRegistryState {
        if self.records.is_empty() {
            RuleRegistryState::Empty
        } else {
            RuleRegistryState::Populated
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
    use enforcer_domain::{
        config_types::CrateName,
        ids::RuleId,
        rules_types::{RuleParameters, RuleRegistryState, RuleVersion},
        severity::Tier,
    };

    fn sample(
        rule_id: RuleId,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id,
            version: RuleVersion::try_new(std::num::NonZeroU32::MIN),
            title: "Sample rule".parse()?,
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".parse()?,
                path: "sample::SampleValidator".parse()?,
            },
            fixtures: FixtureRef {
                fail: "crates/enforcer-lang-rust/fixtures/sample/fail.rs".parse()?,
                pass: "crates/enforcer-lang-rust/fixtures/sample/pass.rs".parse()?,
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".parse()?,
            tags: vec!["rust".parse()?],
            params: RuleParameters::default(),
        })
    }

    #[test]
    fn loads_well_formed_records() -> Result<(), Box<dyn std::error::Error>> {
        let rule_id: RuleId = "RR-1.1".parse()?;
        let registry = RuleRegistry::from_records(vec![sample(rule_id.clone())?])?;
        assert_eq!(registry.iter().count(), 1);
        assert_eq!(
            registry.get(&rule_id).map(|record| &record.rule_id),
            Some(&rule_id)
        );
        Ok(())
    }

    #[test]
    fn rejects_duplicate_rule_ids() -> Result<(), Box<dyn std::error::Error>> {
        let a = sample("RR-1.1".parse()?)?;
        let b = sample("RR-1.1".parse()?)?;
        let outcome = RuleRegistry::from_records(vec![a, b]);
        assert!(matches!(
            outcome,
            Err(crate::RuleLoadError::DuplicateRuleId { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_malformed_validator_crate_name_at_domain_boundary() {
        let outcome = "".parse::<CrateName>();
        assert!(matches!(
            outcome,
            Err(enforcer_domain::boundary::decode_error::DecodeError {
                path,
                ..
            }) if path == "crateName"
        ));
    }

    #[test]
    fn empty_registry_reports_empty() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        assert_eq!(registry.state(), RuleRegistryState::Empty);
        assert_eq!(registry.iter().count(), 0);
        Ok(())
    }
}
