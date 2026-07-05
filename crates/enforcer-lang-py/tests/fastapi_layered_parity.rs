//! e-pack-python acceptance proof (parity-oracle leg): re-runs the d01
//! `rule-scaffold-parity` oracle (`enforcer_mechanization::parity`) over
//! the FastAPI-layered catalog
//! (`crates/enforcer-rules/rules/fastapi-layered.json`) wired to this
//! crate's [`enforcer_lang_py::rules::fastapi_layered`] validators, proving
//! the full 5-way chain (ruleId <-> doc-anchor <-> validator <->
//! {fail,pass fixtures} <-> registry-record) for every `PYFA-*` rule id,
//! both directions (a seeded gap on each leg fails closed). Named proof
//! row: `python-fastapi-family-parity`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_validator::validator::Validator;

const FASTAPI_LAYERED_JSON: &str = include_str!("../../enforcer-rules/rules/fastapi-layered.json");

/// Repo root: two levels up from this crate's manifest dir
/// (`crates/enforcer-lang-py` -> workspace root), matching the
/// `RuleRecord.fixtures` paths, which are workspace-root-relative.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

/// A lookup mapping each `PYFA-*` `RuleId` to its concrete validator
/// instance.
struct FastapiLayeredLookup {
    by_id: BTreeMap<RuleId, Box<dyn Validator>>,
}

impl FastapiLayeredLookup {
    fn new() -> Result<Self, enforcer_core::error::DecodeError> {
        let mut by_id = BTreeMap::new();
        for validator in enforcer_lang_py::rules::fastapi_layered::validators()? {
            by_id.insert(validator.rule_id().clone(), validator);
        }
        Ok(Self { by_id })
    }
}

impl ValidatorLookup for FastapiLayeredLookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        self.by_id.get(rule_id).map(std::convert::AsRef::as_ref)
    }
}

#[test]
fn every_fastapi_layered_rule_passes_the_d01_five_way_parity_sweep(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(FASTAPI_LAYERED_JSON, "rules/fastapi-layered.json")?;
    let registry = load_registry_from_records(records)?;
    let lookup = FastapiLayeredLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "e-pack-python 5-way parity gaps: {findings:#?}"
    );
    Ok(())
}

#[test]
fn seeded_missing_validator_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(FASTAPI_LAYERED_JSON, "rules/fastapi-layered.json")?;
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
        14,
        "every fastapi-layered rule should gap-out with no validator wired"
    );
    Ok(())
}

#[test]
fn seeded_dangling_doc_anchor_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = parse_catalog(FASTAPI_LAYERED_JSON, "rules/fastapi-layered.json")?;
    records[0].doc_anchor = "docs/plans/enforcer-selfhost-plan/DOES-NOT-EXIST.md#nope".to_owned();
    let registry = load_registry_from_records(records)?;
    let lookup = FastapiLayeredLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(findings
        .iter()
        .any(|f| f.detail.contains("does not resolve")));
    Ok(())
}

#[test]
fn seeded_missing_fail_fixture_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = parse_catalog(FASTAPI_LAYERED_JSON, "rules/fastapi-layered.json")?;
    records[0].fixtures.fail =
        "crates/enforcer-lang-py/tests/fixtures/fastapi_layered/does-not-exist.py".to_owned();
    let registry = load_registry_from_records(records)?;
    let lookup = FastapiLayeredLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}
