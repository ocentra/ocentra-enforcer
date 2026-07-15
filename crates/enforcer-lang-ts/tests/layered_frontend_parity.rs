//! d12 acceptance proof (parity-oracle leg): re-runs the d01
//! `rule-scaffold-parity` oracle (`enforcer_mechanization::parity`) over the
//! layered/frontend catalog
//! (`crates/enforcer-rules/rules/layered-frontend.json`) wired to this
//! crate's [`enforcer_lang_ts::rules::layered_frontend`] validators, proving
//! the full 5-way chain (ruleId <-> doc-anchor <-> validator <->
//! {fail,pass fixtures} <-> registry-record) for every `LFE-*` rule id,
//! both directions (a seeded gap on each leg fails closed). Named proof
//! row: `layered-frontend-family-parity`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_lang_ts::rules::layered_frontend::{
    FeatureBoundariesValidator, NoRepoInRouterValidator, StrEnumOnlyValidator,
    SymbolLevelDiValidator,
};
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_validator::validator::{ValidationInput, Validator};

const LAYERED_FRONTEND_JSON: &str =
    include_str!("../../enforcer-rules/rules/layered-frontend.json");

/// Repo root: two levels up from this crate's manifest dir
/// (`crates/enforcer-lang-ts` -> workspace root), matching the
/// `RuleRecord.fixtures` paths, which are workspace-root-relative.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

/// A lookup mapping each `LFE-*` `RuleId` to its concrete validator
/// instance.
struct LayeredFrontendLookup {
    by_id: BTreeMap<RuleId, Box<dyn Validator>>,
}

impl LayeredFrontendLookup {
    fn new() -> Result<Self, enforcer_domain::boundary::decode_error::DecodeError> {
        let mut by_id = BTreeMap::new();
        for validator in enforcer_lang_ts::rules::layered_frontend::validators()? {
            by_id.insert(validator.rule_id().clone(), validator);
        }
        Ok(Self { by_id })
    }
}

impl ValidatorLookup for LayeredFrontendLookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        self.by_id.get(rule_id).map(std::convert::AsRef::as_ref)
    }
}

#[test]
fn every_layered_frontend_rule_passes_the_d01_five_way_parity_sweep(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(LAYERED_FRONTEND_JSON, "rules/layered-frontend.json")?;
    let registry = load_registry_from_records(records)?;
    let lookup = LayeredFrontendLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "layered-frontend 5-way parity gaps: {findings:#?}"
    );
    Ok(())
}

#[test]
fn seeded_missing_validator_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(LAYERED_FRONTEND_JSON, "rules/layered-frontend.json")?;
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
        5,
        "every layered-frontend rule should gap-out with no validator wired"
    );
    Ok(())
}

#[test]
fn seeded_dangling_doc_anchor_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = parse_catalog(LAYERED_FRONTEND_JSON, "rules/layered-frontend.json")?;
    records[0].doc_anchor = "docs/plans/enforcer-selfhost-plan/DOES-NOT-EXIST.md#nope".to_owned();
    let registry = load_registry_from_records(records)?;
    let lookup = LayeredFrontendLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(findings
        .iter()
        .any(|f| f.detail.contains("does not resolve")));
    Ok(())
}

#[test]
fn seeded_missing_fail_fixture_fails_the_sweep_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = parse_catalog(LAYERED_FRONTEND_JSON, "rules/layered-frontend.json")?;
    records[0].fixtures.fail =
        "crates/enforcer-lang-ts/tests/fixtures/layered_frontend/does-not-exist.ts".to_owned();
    let registry = load_registry_from_records(records)?;
    let lookup = LayeredFrontendLookup::new()?;
    let oracle = ParityOracle::new(&registry, &repo_root(), std::collections::BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert_eq!(findings.len(), 1);
    Ok(())
}

#[test]
fn layered_frontend_text_parsers_handle_complete_and_truncated_syntax(
) -> Result<(), Box<dyn std::error::Error>> {
    let router = NoRepoInRouterValidator::new()?;
    let router_path: RelPath = "routers/users.ts".parse()?;
    assert_eq!(
        router
            .validate(ValidationInput {
                file: &router_path,
                source: "import { UserRepository } from 'data';",
                scope: ScanScope::Files,
            })
            .len(),
        1
    );
    assert!(router
        .validate(ValidationInput {
            file: &router_path,
            source: "import { UserRepository } from",
            scope: ScanScope::Files,
        })
        .is_empty());

    let feature = FeatureBoundariesValidator::new()?;
    let feature_path: RelPath = "features/checkout/ui.ts".parse()?;
    assert_eq!(
        feature
            .validate(ValidationInput {
                file: &feature_path,
                source: "import x from '@/features/payments/internal/api';",
                scope: ScanScope::Files,
            })
            .len(),
        1
    );
    assert!(feature
        .validate(ValidationInput {
            file: &feature_path,
            source: "import x from '@/features/';",
            scope: ScanScope::Files,
        })
        .is_empty());

    let enum_validator = StrEnumOnlyValidator::new()?;
    let enum_path: RelPath = "features/checkout/status.ts".parse()?;
    assert!(enum_validator
        .validate(ValidationInput {
            file: &enum_path,
            source: "enum Status {\n    Ready = 'ready'\n}",
            scope: ScanScope::Files,
        })
        .is_empty());

    let di = SymbolLevelDiValidator::new()?;
    assert!(di
        .validate(ValidationInput {
            file: &enum_path,
            source: "@inject(",
            scope: ScanScope::Files,
        })
        .is_empty());
    Ok(())
}
