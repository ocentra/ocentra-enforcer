//! The structured rule registry: typed rule records and the in-memory
//! [`RuleRegistry`] built from them.
//!
//! Every [`RuleRecord`] carries the full 5-way linkage the plan's doctrine
//! requires: `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor
//! <-> tier`. The registry rejects malformed or duplicate records at LOAD
//! time (fail-closed), so no invalid rule reaches a consumer (arc-05's
//! validator harness, the lang-* crates) as a live value.

use std::collections::BTreeMap;

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Tier;

use crate::{RuleLoadError, RuleResult};

/// A fixture reference: a repo-relative path to a fail or pass fixture file
/// that a rule's validator is proven against.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureRef {
    /// Repo-relative path to the fixture that MUST trip the rule.
    pub fail: String,
    /// Repo-relative path to the fixture that MUST NOT trip the rule.
    pub pass: String,
}

/// Identifies the concrete `Validator` implementation a rule record is
/// paired with, without this crate depending on the validator crates
/// (`enforcer-validator`, `enforcer-lang-*`) that implement it — those
/// crates depend on `enforcer-rules`, not the reverse.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorRef {
    /// The crate that owns the `Validator` implementation, e.g.
    /// `enforcer-lang-rust`.
    pub crate_name: String,
    /// The type/function path within that crate, e.g.
    /// `no_reexports::NoReexportsValidator`.
    pub path: String,
}

/// One structured rule record: the unit the registry loads, validates, and
/// exposes. DTO shape only — this crate does not execute validators, it
/// carries the DATA a validator/harness consumes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleRecord {
    /// Branded rule identifier (e.g. `RR-6.1`).
    pub rule_id: RuleId,
    /// Record content version. Bumped whenever `validator`, `fixtures`, or
    /// `doc_anchor` changes; checked for drift by [`crate::version_drift`].
    pub version: u32,
    /// Short human title.
    pub title: String,
    /// Mechanical-enforcement tier (T1/T2/T3).
    pub tier: Tier,
    /// The paired validator implementation reference.
    pub validator: ValidatorRef,
    /// Fail/pass fixture references proving the validator's behavior.
    pub fixtures: FixtureRef,
    /// Repo-relative path (or URL fragment) to the human-canonical doc
    /// anchor explaining the rule, e.g. `docs/rules/RR-6.md#RR-6.1`.
    pub doc_anchor: String,
    /// Free-form rule-family tags (e.g. `["rust", "reexports"]`) for
    /// grouping/filtering; not part of identity.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Rule-family-specific structured parameters (e.g. the deny-wall's
    /// lint list, or the `blockedProtocolDependencies` map for a
    /// dependency-ban posture record). Opaque to the registry/loader —
    /// each validator interprets its own record's `params` shape. Absent
    /// (`null`) for rules that carry no extra parameters.
    #[serde(default)]
    pub params: serde_json::Value,
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
            validate_record_shape(&record)?;
            if map.contains_key(&record.rule_id) {
                return Err(RuleLoadError::DuplicateRuleId {
                    rule_id: record.rule_id.to_string(),
                });
            }
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
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when no records are loaded.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Reject a record whose linkage fields are structurally empty: an empty
/// validator path, empty fixture path, or empty doc-anchor is treated the
/// same as a missing one (fail-closed rather than silently accepting a
/// half-populated record).
fn validate_record_shape(record: &RuleRecord) -> RuleResult<()> {
    let rule_id = record.rule_id.to_string();
    if record.validator.crate_name.trim().is_empty() || record.validator.path.trim().is_empty() {
        return Err(RuleLoadError::MalformedRecord {
            rule_id,
            reason: "validator crateName/path must not be empty".to_owned(),
        });
    }
    if record.fixtures.fail.trim().is_empty() || record.fixtures.pass.trim().is_empty() {
        return Err(RuleLoadError::MalformedRecord {
            rule_id,
            reason: "fixtures.fail/fixtures.pass must not be empty".to_owned(),
        });
    }
    if record.doc_anchor.trim().is_empty() {
        return Err(RuleLoadError::MalformedRecord {
            rule_id,
            reason: "docAnchor must not be empty".to_owned(),
        });
    }
    if record.title.trim().is_empty() {
        return Err(RuleLoadError::MalformedRecord {
            rule_id,
            reason: "title must not be empty".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
    use enforcer_domain::severity::Tier;

    fn sample(
        rule_id: &str,
    ) -> Result<RuleRecord, enforcer_domain::boundary::decode_error::DecodeError> {
        Ok(RuleRecord {
            rule_id: rule_id.parse()?,
            version: 1,
            title: "Sample rule".to_owned(),
            tier: Tier::T1,
            validator: ValidatorRef {
                crate_name: "enforcer-lang-rust".to_owned(),
                path: "sample::SampleValidator".to_owned(),
            },
            fixtures: FixtureRef {
                fail: "crates/enforcer-lang-rust/fixtures/sample/fail.rs".to_owned(),
                pass: "crates/enforcer-lang-rust/fixtures/sample/pass.rs".to_owned(),
            },
            doc_anchor: "docs/rules/SAMPLE.md#SAMPLE-1".to_owned(),
            tags: vec!["rust".to_owned()],
            params: serde_json::Value::Null,
        })
    }

    #[test]
    fn loads_well_formed_records() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![sample("RR-1.1")?])?;
        assert_eq!(registry.len(), 1);
        assert!(registry.get(&"RR-1.1".parse()?).is_some());
        Ok(())
    }

    #[test]
    fn rejects_duplicate_rule_ids() -> Result<(), Box<dyn std::error::Error>> {
        let a = sample("RR-1.1")?;
        let b = sample("RR-1.1")?;
        let outcome = RuleRegistry::from_records(vec![a, b]);
        assert!(outcome.is_err());
        Ok(())
    }

    #[test]
    fn rejects_malformed_validator_ref() -> Result<(), Box<dyn std::error::Error>> {
        let mut bad = sample("RR-1.2")?;
        bad.validator.path = String::new();
        assert!(RuleRegistry::from_records(vec![bad]).is_err());
        Ok(())
    }

    #[test]
    fn rejects_malformed_fixture_ref() -> Result<(), Box<dyn std::error::Error>> {
        let mut bad = sample("RR-1.3")?;
        bad.fixtures.fail = "   ".to_owned();
        assert!(RuleRegistry::from_records(vec![bad]).is_err());
        Ok(())
    }

    #[test]
    fn rejects_empty_doc_anchor() -> Result<(), Box<dyn std::error::Error>> {
        let mut bad = sample("RR-1.4")?;
        bad.doc_anchor = String::new();
        assert!(RuleRegistry::from_records(vec![bad]).is_err());
        Ok(())
    }

    #[test]
    fn empty_registry_reports_empty() -> Result<(), Box<dyn std::error::Error>> {
        let registry = RuleRegistry::from_records(vec![])?;
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        Ok(())
    }
}
