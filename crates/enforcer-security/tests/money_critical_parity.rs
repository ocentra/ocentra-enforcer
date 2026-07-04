//! h01's own slice of the d01 `rule-scaffold-parity` oracle sweep: loads
//! `rules/money-critical.json`, resolves its two rule ids against this
//! crate's own [`Validator`] implementations, and asserts the whole-registry
//! sweep is clean — the actual `enforcer_mechanization::parity::ParityOracle`
//! this workpack's acceptance criteria names, not just the ad-hoc
//! fail/pass fixture check each validator's own unit tests already run.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_security::rules::money_critical::{
    MoneyCriticalAnnotatedValidator, MoneyCriticalClassifyValidator,
};
use enforcer_validator::validator::Validator;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct H01Lookup {
    classify: MoneyCriticalClassifyValidator,
    annotated: MoneyCriticalAnnotatedValidator,
}

impl ValidatorLookup for H01Lookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        if rule_id == self.classify.rule_id() {
            Some(&self.classify)
        } else if rule_id == self.annotated.rule_id() {
            Some(&self.annotated)
        } else {
            None
        }
    }
}

#[test]
fn h01_rule_scaffold_parity_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    let catalog_path = manifest_dir().join("rules/money-critical.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(registry.len(), 2);

    let lookup = H01Lookup {
        classify: MoneyCriticalClassifyValidator::new()?,
        annotated: MoneyCriticalAnnotatedValidator::new()?,
    };

    let repo_root = manifest_dir()
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(std::path::Path::to_path_buf)
        .ok_or("could not resolve repo root from CARGO_MANIFEST_DIR")?;

    let oracle = ParityOracle::new(&registry, &repo_root, BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "h01 rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}
