//! d16 acceptance proof (parity-oracle leg): re-runs the d01
//! `rule-scaffold-parity` oracle (`enforcer_mechanization::parity`) over
//! the FSM catalog (`crates/enforcer-rules/rules/fsm.json`) wired to this
//! crate's own [`enforcer_lang_common::rules::fsm`] validators, proving the
//! full 5-way chain (ruleId <-> doc-anchor <-> validator <-> {fail,pass
//! fixtures} <-> registry-record) for every FSM rule id, both directions
//! (a seeded gap on each leg must fail closed).

use std::collections::BTreeMap;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_validator::validator::Validator;

const FSM_JSON: &str = include_str!("../../enforcer-rules/rules/fsm.json");

/// Repo root: three levels up from this crate's manifest dir
/// (`crates/enforcer-lang-common` -> workspace root), matching the
/// `RuleRecord.fixtures` paths, which are workspace-root-relative.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

/// A lookup mapping each FSM `RuleId` to its concrete validator instance.
struct FsmLookup {
    by_id: BTreeMap<RuleId, Box<dyn Validator>>,
}

impl FsmLookup {
    fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        let mut by_id = BTreeMap::new();
        for validator in enforcer_lang_common::rules::fsm::validators()? {
            by_id.insert(validator.rule_id().clone(), validator);
        }
        Ok(Self { by_id })
    }
}

impl ValidatorLookup for FsmLookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        self.by_id.get(rule_id).map(std::convert::AsRef::as_ref)
    }
}

#[test]
fn every_fsm_rule_passes_the_d01_five_way_parity_sweep() -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(FSM_JSON, "rules/fsm.json")?;
    let registry = load_registry_from_records(records)?;
    let lookup = FsmLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(findings.is_empty(), "FSM 5-way parity gaps: {findings:#?}");
    Ok(())
}

#[test]
fn seeded_missing_validator_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(FSM_JSON, "rules/fsm.json")?;
    let registry = load_registry_from_records(records)?;

    struct EmptyLookup;
    impl ValidatorLookup for EmptyLookup {
        fn resolve(&self, _rule_id: &RuleId) -> Option<&dyn Validator> {
            None
        }
    }

    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&EmptyLookup);
    assert_eq!(
        findings.len(),
        10,
        "every FSM rule should gap-out with no validator wired"
    );
    Ok(())
}

#[test]
fn seeded_dangling_doc_anchor_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = parse_catalog(FSM_JSON, "rules/fsm.json")?;
    records[0].doc_anchor = "docs/plans/enforcer-selfhost-plan/DOES-NOT-EXIST.md#nope".to_owned();
    let registry = load_registry_from_records(records)?;
    let lookup = FsmLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(findings
        .iter()
        .any(|f| f.detail.contains("does not resolve")));
    Ok(())
}

#[test]
fn seeded_missing_fail_fixture_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = parse_catalog(FSM_JSON, "rules/fsm.json")?;
    records[0].fixtures.fail =
        "crates/enforcer-lang-common/tests/fixtures/fsm/does-not-exist.py".to_owned();
    let registry = load_registry_from_records(records)?;
    let lookup = FsmLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}
