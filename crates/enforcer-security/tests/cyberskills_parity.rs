//! h11's slice of the d01 `rule-scaffold-parity` oracle sweep — the proof
//! every other Rust rule pack already carries (see the sibling
//! `money_critical_parity.rs`, `fsm_parity.rs`, `size_shape_parity.rs`,
//! ...), which the cyberskills family was previously missing (only a
//! hand-rolled linkage test in `enforcer-rules` existed). It loads the
//! `crates/enforcer-rules/rules/cyberskills.json` catalog (12 records:
//! the frontmatter linter + the 11 fundamental-logic rules), resolves
//! every rule id against its real [`Validator`] — the 11 source-pattern
//! validators from `enforcer-lang-security` plus this crate's own
//! `SkillFrontmatterValidValidator` — and asserts the whole-registry
//! `enforcer_mechanization::parity::ParityOracle` sweep is clean (each
//! rule's fail fixture flags, its pass fixture stays clean, and its 5-way
//! linkage resolves). This runs the ACTUAL oracle the workpack's
//! acceptance criteria names, not just the per-validator fixture unit
//! tests.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::ids::RuleId;
use enforcer_lang_security::rules::cyberskills::registry::build_all as build_lang_rows;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_security::cyberskills::frontmatter_lint::SkillFrontmatterValidValidator;
use enforcer_validator::validator::Validator;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Resolves a cyberskills rule id to its validator, spanning both crates
/// the family lives in (the 11 source-pattern rules in
/// `enforcer-lang-security` + the frontmatter linter here).
struct CyberskillsLookup {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidatorLookup for CyberskillsLookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        self.validators
            .iter()
            .find(|validator| validator.rule_id() == rule_id)
            .map(std::convert::AsRef::as_ref)
    }
}

#[test]
fn cyberskills_rule_scaffold_parity_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    // Catalog lives in enforcer-rules; CARGO_MANIFEST_DIR is
    // `<repo>/crates/enforcer-security`.
    let catalog_path = manifest_dir().join("../enforcer-rules/rules/cyberskills.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(
        registry.len(),
        13,
        "expected the h11 + Wave-1 cyberskills rule records"
    );

    let mut validators: Vec<Box<dyn Validator>> = Vec::new();
    for row in build_lang_rows()? {
        validators.push(row.validator);
    }
    validators.push(Box::new(SkillFrontmatterValidValidator::new()?));
    assert_eq!(
        validators.len(),
        13,
        "expected 12 source-pattern validators + 1 frontmatter linter"
    );

    let lookup = CyberskillsLookup { validators };

    let repo_root = manifest_dir()
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(std::path::Path::to_path_buf)
        .ok_or("could not resolve repo root from CARGO_MANIFEST_DIR")?;

    let oracle = ParityOracle::new(&registry, &repo_root, BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "cyberskills rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}
