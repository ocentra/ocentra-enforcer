//! h03's own slice of the d01 `rule-scaffold-parity` oracle sweep: loads
//! `rules/threat-test-mapping.json`, resolves its three rule ids against
//! this crate's own [`Validator`] implementations, and asserts the
//! whole-registry sweep is clean via
//! `enforcer_mechanization::parity::ParityOracle` — the same oracle h01
//! proves its own rows through.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_security::rules::threat_test_mapping::{
    ThreatMapNoUnmappedValidator, ThreatMapThreatHasTestValidator, ThreatMapUnitCoverageValidator,
};
use enforcer_validator::validator::Validator;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct H03Lookup {
    unit_coverage: ThreatMapUnitCoverageValidator,
    no_unmapped: ThreatMapNoUnmappedValidator,
    threat_has_test: ThreatMapThreatHasTestValidator,
}

impl ValidatorLookup for H03Lookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        if rule_id == self.unit_coverage.rule_id() {
            Some(&self.unit_coverage)
        } else if rule_id == self.no_unmapped.rule_id() {
            Some(&self.no_unmapped)
        } else if rule_id == self.threat_has_test.rule_id() {
            Some(&self.threat_has_test)
        } else {
            None
        }
    }
}

#[test]
fn h03_rule_scaffold_parity_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    let catalog_path = manifest_dir().join("rules/threat-test-mapping.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(
        registry.count(),
        enforcer_domain::rules_types::RuleRecordCount::from_records(0..3)
    );

    let lookup = H03Lookup {
        unit_coverage: ThreatMapUnitCoverageValidator::new()?,
        no_unmapped: ThreatMapNoUnmappedValidator::new()?,
        threat_has_test: ThreatMapThreatHasTestValidator::new()?,
    };

    let repo_root = manifest_dir()
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(std::path::Path::to_path_buf)
        .ok_or("could not resolve repo root from CARGO_MANIFEST_DIR")?;

    let oracle = ParityOracle::new(
        &registry,
        enforcer_domain::paths::RepoRoot::try_from(repo_root.as_path())?,
        BTreeSet::new(),
    );
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "h03 rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}
