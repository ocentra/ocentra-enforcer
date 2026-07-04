//! h06's own slice of the d01 `rule-scaffold-parity` oracle sweep: loads
//! `rules/money-critical-mechanics.json`, resolves its six rule ids
//! against this crate's own [`Validator`] implementations, and asserts
//! the whole-registry sweep is clean via
//! `enforcer_mechanization::parity::ParityOracle` — the same oracle
//! h01/h03/h05 prove their own rows through.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_security::rules::boundary::BoundaryValidator;
use enforcer_security::rules::economic::EconomicValidator;
use enforcer_security::rules::killswitch::KillSwitchValidator;
use enforcer_security::rules::rollback::RollbackValidator;
use enforcer_security::rules::signing::SigningValidator;
use enforcer_security::rules::time::TimeValidator;
use enforcer_validator::validator::Validator;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct H06Lookup {
    signing: SigningValidator,
    time: TimeValidator,
    boundary: BoundaryValidator,
    killswitch: KillSwitchValidator,
    economic: EconomicValidator,
    rollback: RollbackValidator,
}

impl ValidatorLookup for H06Lookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        if rule_id == self.signing.rule_id() {
            Some(&self.signing)
        } else if rule_id == self.time.rule_id() {
            Some(&self.time)
        } else if rule_id == self.boundary.rule_id() {
            Some(&self.boundary)
        } else if rule_id == self.killswitch.rule_id() {
            Some(&self.killswitch)
        } else if rule_id == self.economic.rule_id() {
            Some(&self.economic)
        } else if rule_id == self.rollback.rule_id() {
            Some(&self.rollback)
        } else {
            None
        }
    }
}

#[test]
fn h06_rule_scaffold_parity_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    let catalog_path = manifest_dir().join("rules/money-critical-mechanics.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(registry.len(), 6);

    let lookup = H06Lookup {
        signing: SigningValidator::new()?,
        time: TimeValidator::new()?,
        boundary: BoundaryValidator::new()?,
        killswitch: KillSwitchValidator::new()?,
        economic: EconomicValidator::new()?,
        rollback: RollbackValidator::new()?,
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
        "h06 rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}
