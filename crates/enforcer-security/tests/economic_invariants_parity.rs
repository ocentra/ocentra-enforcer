//! h05's own slice of the d01 `rule-scaffold-parity` oracle sweep: loads
//! `rules/economic-invariants.json`, resolves its two rule ids against
//! this crate's own [`Validator`] implementations, and asserts the
//! whole-registry sweep is clean via
//! `enforcer_mechanization::parity::ParityOracle` — the same oracle h01/h03
//! prove their own rows through.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_security::rules::economic_invariants::{
    EconomicInvariantPresenceValidator, EconomicInvariantShapeValidator,
};
use enforcer_validator::validator::Validator;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct H05Lookup {
    presence: EconomicInvariantPresenceValidator,
    shape: EconomicInvariantShapeValidator,
}

impl ValidatorLookup for H05Lookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        if rule_id == self.presence.rule_id() {
            Some(&self.presence)
        } else if rule_id == self.shape.rule_id() {
            Some(&self.shape)
        } else {
            None
        }
    }
}

#[test]
fn h05_rule_scaffold_parity_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    let catalog_path = manifest_dir().join("rules/economic-invariants.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(registry.len(), 2);

    let lookup = H05Lookup {
        presence: EconomicInvariantPresenceValidator::new()?,
        shape: EconomicInvariantShapeValidator::new()?,
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
        "h05 rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}
