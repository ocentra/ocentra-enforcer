//! h02's proof surface: the d01 `rule-scaffold-parity` oracle sweep over
//! `rules/required-test-categories.json`, the per-rule fixture parity
//! checks, the silent-on-malformed/invalid-record contract, and a property
//! test driving the seven-category gate across every category-presence
//! subset.

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_rules::RuleLoadError;
use enforcer_security::rules::required_test_categories::{
    RequiredTestCategoriesMapValidator, RequiredTestCategoriesSevenValidator,
};
use enforcer_validator::error::HarnessError;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

/// Typed failure surface for this proof file — every fallible step maps
/// into one of these variants instead of an erased error object.
#[derive(Debug, thiserror::Error)]
enum ProofFailure {
    #[error("decode failed: {0}")]
    Decode(#[from] DecodeError),
    #[error("harness parity failed: {0}")]
    Harness(#[from] HarnessError),
    #[error("rule catalog load failed: {0}")]
    Rules(#[from] RuleLoadError),
    #[error("fixture io failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not resolve repo root from CARGO_MANIFEST_DIR")]
    RepoRoot,
    #[error("expected a finding but none was produced")]
    MissingFinding,
}

struct H02Lookup {
    seven: RequiredTestCategoriesSevenValidator,
    map: RequiredTestCategoriesMapValidator,
}

impl ValidatorLookup for H02Lookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        if rule_id == self.seven.rule_id() {
            Some(&self.seven)
        } else if rule_id == self.map.rule_id() {
            Some(&self.map)
        } else {
            None
        }
    }
}

#[test]
fn h02_rule_scaffold_parity_is_clean() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = manifest_dir.join("rules/required-test-categories.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(registry.len(), 2);

    let lookup = H02Lookup {
        seven: RequiredTestCategoriesSevenValidator::new()?,
        map: RequiredTestCategoriesMapValidator::new()?,
    };

    let repo_root = manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(std::path::Path::to_path_buf)
        .ok_or(ProofFailure::RepoRoot)?;

    let oracle = ParityOracle::new(&registry, &repo_root, BTreeSet::new());
    let findings = oracle.sweep(&lookup);
    assert!(
        findings.is_empty(),
        "h02 rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}

#[test]
fn req_testcat_seven_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = RequiredTestCategoriesSevenValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/required_test_categories/endpoint_a/bad/missing_replay_concurrency/manifest.json",
        "tests/fixtures/required_test_categories/endpoint_a/good/all_seven/manifest.json",
    )?;

    // The bad fixture's single finding must name every missing category.
    let bad_source = std::fs::read_to_string(manifest_dir.join(
        "tests/fixtures/required_test_categories/endpoint_a/bad/missing_replay_concurrency/manifest.json",
    ))?;
    let record_path: RelPath = "crates/x/required-test-categories.json".parse()?;
    let bad_findings = validator.validate(ValidationInput {
        file: &record_path,
        source: &bad_source,
        scope: ScanScope::Files,
    });
    assert_eq!(bad_findings.len(), 1);
    let Some(first) = bad_findings.first() else {
        return Err(ProofFailure::MissingFinding);
    };
    assert!(
        first.detail.contains("missing: replay, concurrency"),
        "the finding must name exactly the missing categories, got: {}",
        first.detail
    );
    Ok(())
}

#[test]
fn req_testcat_map_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = RequiredTestCategoriesMapValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/required_test_categories/orphan/bad/unit_no_tests/manifest.json",
        "tests/fixtures/required_test_categories/endpoint_a/good/all_seven/manifest.json",
    )?;
    Ok(())
}

#[test]
fn malformed_and_invalid_records_stay_silent() -> Result<(), ProofFailure> {
    let record_path: RelPath = "crates/x/required-test-categories.json".parse()?;
    let seven = RequiredTestCategoriesSevenValidator::new()?;
    let map = RequiredTestCategoriesMapValidator::new()?;
    // Malformed (non-JSON), invalid-shape, and empty documents must all
    // stay silent — an unreadable record is never itself a finding.
    let silent_inputs = [
        "this is not valid json {{{",
        "[1, 2, 3]",
        "",
        "{\"moneyCriticalUnits\": \"not-an-array\"}",
    ];
    for source in silent_inputs {
        let seven_findings = seven.validate(ValidationInput {
            file: &record_path,
            source,
            scope: ScanScope::Files,
        });
        assert!(seven_findings.is_empty(), "expected silence for: {source}");
        let map_findings = map.validate(ValidationInput {
            file: &record_path,
            source,
            scope: ScanScope::Files,
        });
        assert!(map_findings.is_empty(), "expected silence for: {source}");
    }
    Ok(())
}

#[test]
fn fully_covered_unit_stays_clean_across_both_validators() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(manifest_dir.join(
            "tests/fixtures/required_test_categories/endpoint_a/good/all_seven/manifest.json",
        ))?;
    let record_path: RelPath = "crates/x/required-test-categories.json".parse()?;
    for validator in [
        Box::new(RequiredTestCategoriesSevenValidator::new()?) as Box<dyn Validator>,
        Box::new(RequiredTestCategoriesMapValidator::new()?) as Box<dyn Validator>,
    ] {
        let findings = validator.validate(ValidationInput {
            file: &record_path,
            source: &source,
            scope: ScanScope::Files,
        });
        assert!(
            findings.is_empty(),
            "rule {} must stay clean on the fully-covered fixture: {findings:#?}",
            validator.rule_id()
        );
    }
    Ok(())
}

#[test]
fn orphan_unit_is_silent_under_the_seven_category_validator() -> Result<(), ProofFailure> {
    // The orphan fixture has no `units` entry at all for `orphan_unit`, so
    // REQ-TESTCAT-SEVEN.1 (which only inspects present units) must stay
    // silent — this is exclusively REQ-TESTCAT-MAP.1's fail case.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(
        manifest_dir
            .join("tests/fixtures/required_test_categories/orphan/bad/unit_no_tests/manifest.json"),
    )?;
    let record_path: RelPath = "crates/x/required-test-categories.json".parse()?;
    let validator = RequiredTestCategoriesSevenValidator::new()?;
    let findings = validator.validate(ValidationInput {
        file: &record_path,
        source: &source,
        scope: ScanScope::Files,
    });
    assert!(findings.is_empty());
    Ok(())
}

/// The seven wire category keys, bit-indexed for the subset property below.
const CATEGORY_KEYS: [&str; 7] = [
    "negative",
    "replay",
    "concurrency",
    "rollback",
    "economic_exhaustion",
    "time_based",
    "signing",
];

#[test]
fn req_testcat_seven_property_over_all_category_subsets() -> Result<(), ProofFailure> {
    // Property (exhaustive over the whole input space): for every one of
    // the 128 category-presence subsets, the seven-category gate fires
    // exactly when at least one category is empty, and the map gate fires
    // exactly when every category is empty. Every emitted finding carries
    // its own validator's rule id (the harness invariant).
    //
    // Renders a one-unit record whose category `i` carries a test id
    // exactly when bit `i` of `mask` is set.
    let record_for_subset = |mask: u8| {
        let mut category_fields = Vec::new();
        for (bit, key) in CATEGORY_KEYS.iter().enumerate() {
            let ids = if mask & (1u8 << bit) != 0 {
                "\"tagged_test\""
            } else {
                ""
            };
            category_fields.push(format!("\"{key}\": [{ids}]"));
        }
        format!(
            "{{\"moneyCriticalUnits\": [\"endpoint_a\"], \"units\": [{{\"unit\": \"endpoint_a\", \
             \"tests\": {{{}}}}}]}}",
            category_fields.join(", ")
        )
    };
    let record_path: RelPath = "crates/x/required-test-categories.json".parse()?;
    let seven = RequiredTestCategoriesSevenValidator::new()?;
    let map = RequiredTestCategoriesMapValidator::new()?;
    let full_mask = 0b111_1111u8;

    for mask in 0u8..128 {
        let source = record_for_subset(mask);
        let seven_findings = seven.validate(ValidationInput {
            file: &record_path,
            source: &source,
            scope: ScanScope::Files,
        });
        let expected_seven = usize::from(mask != full_mask);
        assert_eq!(
            seven_findings.len(),
            expected_seven,
            "seven-category gate mismatch for mask {mask:#09b}: {seven_findings:#?}"
        );
        for finding in &seven_findings {
            assert_eq!(finding.rule_id.as_str(), "REQ-TESTCAT-SEVEN.1");
        }

        let map_findings = map.validate(ValidationInput {
            file: &record_path,
            source: &source,
            scope: ScanScope::Files,
        });
        let expected_map = usize::from(mask == 0);
        assert_eq!(
            map_findings.len(),
            expected_map,
            "map gate mismatch for mask {mask:#09b}: {map_findings:#?}"
        );
        for finding in &map_findings {
            assert_eq!(finding.rule_id.as_str(), "REQ-TESTCAT-MAP.1");
        }
    }
    Ok(())
}
