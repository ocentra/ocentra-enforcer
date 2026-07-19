//! d10 acceptance proof (parity-oracle leg): re-runs the d01
//! `rule-scaffold-parity` oracle (`enforcer_mechanization::parity`) over
//! the resilience catalog (`crates/enforcer-rules/rules/resilience.json`)
//! wired to this crate's own [`enforcer_lang_common::rules::resilience`]
//! validators, proving the full 5-way chain (ruleId <-> doc-anchor <->
//! validator <-> {fail,pass fixtures} <-> registry-record) for every
//! resilience rule id, both directions (a seeded gap on each leg must fail
//! closed).

use std::collections::BTreeMap;
use std::path::Path;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::paths::RepoRoot;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_records;
use enforcer_validator::validator::Validator;

mod support;

const RESILIENCE_JSON: &str = include_str!("../../enforcer-rules/rules/resilience.json");

/// Repo root: three levels up from this crate's manifest dir
/// (`crates/enforcer-lang-common` -> workspace root), matching the
/// `RuleRecord.fixtures` paths, which are workspace-root-relative.
fn repo_root() -> Result<RepoRoot, DecodeError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .ok_or_else(|| {
            DecodeError::new("repoRoot", "crate manifest directory has no workspace root")
        })?;
    RepoRoot::try_from(root)
}

/// A lookup mapping each resilience `RuleId` to its concrete validator
/// instance.
struct ResilienceLookup {
    by_id: BTreeMap<RuleId, Box<dyn Validator>>,
}

impl ResilienceLookup {
    fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        let mut by_id = BTreeMap::new();
        for validator in enforcer_lang_common::rules::resilience::validators()? {
            by_id.insert(validator.rule_id().clone(), validator);
        }
        Ok(Self { by_id })
    }
}

impl ValidatorLookup for ResilienceLookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        self.by_id.get(rule_id).map(std::convert::AsRef::as_ref)
    }
}

#[test]
fn every_resilience_rule_passes_the_d01_five_way_parity_sweep(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = support::parse_catalog(RESILIENCE_JSON, "rules/resilience.json")?;
    let registry = load_registry_from_records(records)?;
    let lookup = ResilienceLookup::new()?;
    let oracle = ParityOracle::new(&registry, repo_root()?, std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "resilience 5-way parity gaps: {findings:#?}"
    );
    Ok(())
}

#[test]
fn seeded_missing_validator_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let records = support::parse_catalog(RESILIENCE_JSON, "rules/resilience.json")?;
    let registry = load_registry_from_records(records)?;

    struct EmptyLookup;
    impl ValidatorLookup for EmptyLookup {
        fn resolve(&self, _rule_id: &RuleId) -> Option<&dyn Validator> {
            None
        }
    }

    let oracle = ParityOracle::new(&registry, repo_root()?, std::collections::BTreeSet::new());
    let findings = oracle.sweep(&EmptyLookup);
    assert_eq!(
        findings.len(),
        3,
        "every resilience rule should gap-out with no validator wired"
    );
    Ok(())
}

#[test]
fn seeded_dangling_doc_anchor_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = support::parse_catalog(RESILIENCE_JSON, "rules/resilience.json")?;
    records[0].doc_anchor = "docs/plans/enforcer-selfhost-plan/DOES-NOT-EXIST.md#nope".parse()?;
    let registry = load_registry_from_records(records)?;
    let lookup = ResilienceLookup::new()?;
    let oracle = ParityOracle::new(&registry, repo_root()?, std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(findings
        .iter()
        .any(|f| f.detail.as_str().contains("does not resolve")));
    Ok(())
}

#[test]
fn seeded_missing_fail_fixture_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = support::parse_catalog(RESILIENCE_JSON, "rules/resilience.json")?;
    records[0].fixtures.fail = support::rel_path(
        "crates/enforcer-lang-common/tests/fixtures/resilience/does-not-exist.rs",
    )?;
    let registry = load_registry_from_records(records)?;
    let lookup = ResilienceLookup::new()?;
    let oracle = ParityOracle::new(&registry, repo_root()?, std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}
