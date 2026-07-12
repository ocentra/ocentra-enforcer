//! h04's proof surface: the d01 `rule-scaffold-parity` oracle sweep over
//! `rules/security-test-quality.json`, one fixture-parity check per rule,
//! the family-uniqueness check, and a property test asserting the harness
//! rule-id invariant across a generated corpus (including malformed and
//! invalid inputs).

use std::collections::BTreeSet;
use std::path::PathBuf;

use enforcer_core::error::DecodeError;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_mechanization::parity::{ParityOracle, ValidatorLookup};
use enforcer_rules::loader::load_registry_from_files;
use enforcer_rules::registry::RuleRegistry;
use enforcer_rules::RuleLoadError;
use enforcer_security::rules::security_test_quality::{
    validators, AssertsRejectionValidator, GlobalMutationValidator, MocksForMoneyLogicValidator,
    NoCrashOnlyValidator, PassIfDeletedValidator, ProtectionRemovedHeuristicValidator,
    ReproducibleSeedValidator, SnapshotOnlyValidator, ThreatQualityScoreValidator,
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
}

struct H04Lookup {
    family: Vec<Box<dyn Validator>>,
}

impl ValidatorLookup for H04Lookup {
    fn resolve(&self, rule_id: &RuleId) -> Option<&dyn Validator> {
        self.family
            .iter()
            .map(AsRef::as_ref)
            .find(|validator| validator.rule_id() == rule_id)
    }
}

#[test]
fn h04_rule_scaffold_parity_is_clean() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = manifest_dir.join("rules/security-test-quality.json");
    let registry: RuleRegistry = load_registry_from_files(&[catalog_path.as_path()])?;
    assert_eq!(registry.len(), 9);

    let lookup = H04Lookup {
        family: validators()?,
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
        "h04 rule-scaffold-parity gaps: {findings:#?}"
    );
    Ok(())
}

#[test]
fn nine_validators_registered_with_unique_rule_ids() -> Result<(), ProofFailure> {
    let family = validators()?;
    assert_eq!(family.len(), 9);
    let mut seen = BTreeSet::new();
    for validator in &family {
        // ALLOC-JUSTIFICATION: the uniqueness set owns each rule id
        // rendering so it outlives the borrowed validator iteration.
        assert!(
            seen.insert(validator.rule_id().to_string()),
            "duplicate rule id {} in the h04 family",
            validator.rule_id()
        );
    }
    assert_eq!(seen.len(), 9);
    Ok(())
}

#[test]
fn asserts_rejection_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = AssertsRejectionValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/asserts_success_only/bad/success_only.test.ts",
        "tests/fixtures/security_test_quality/asserts_success_only/good/rejects.test.ts",
    )?;
    Ok(())
}

#[test]
fn mocks_for_money_logic_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = MocksForMoneyLogicValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/mocks_for_money_logic/bad/mock_money.test.ts",
        "tests/fixtures/security_test_quality/mocks_for_money_logic/good/real_money.test.ts",
    )?;
    Ok(())
}

#[test]
fn reproducible_seed_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = ReproducibleSeedValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/reproducible_seed/bad/no_seed.test.ts",
        "tests/fixtures/security_test_quality/reproducible_seed/good/seeded.test.ts",
    )?;
    Ok(())
}

#[test]
fn global_mutation_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = GlobalMutationValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/global_mutation/bad/shared_state.test.ts",
        "tests/fixtures/security_test_quality/global_mutation/good/isolated.test.ts",
    )?;
    Ok(())
}

#[test]
fn pass_if_deleted_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = PassIfDeletedValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/pass_if_deleted/bad/pass_if_deleted.test.ts",
        "tests/fixtures/security_test_quality/pass_if_deleted/good/asserts_real_outcome.test.ts",
    )?;
    Ok(())
}

#[test]
fn no_crash_only_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = NoCrashOnlyValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/no_crash_only/bad/no_crash_only.test.ts",
        "tests/fixtures/security_test_quality/no_crash_only/good/asserts_specific_error.test.ts",
    )?;
    Ok(())
}

#[test]
fn snapshot_only_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = SnapshotOnlyValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/snapshot_only/bad/snapshot_only.test.ts",
        "tests/fixtures/security_test_quality/snapshot_only/good/explicit_assertions.test.ts",
    )?;
    Ok(())
}

#[test]
fn threat_quality_score_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = ThreatQualityScoreValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/threat_mapping/bad/unmapped.test.ts",
        "tests/fixtures/security_test_quality/threat_mapping/good/mapped.test.ts",
    )?;
    Ok(())
}

#[test]
fn protection_removed_heuristic_fixture_parity() -> Result<(), ProofFailure> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = ProtectionRemovedHeuristicValidator::new()?;
    run_fixture_parity(
        &validator,
        &manifest_dir,
        "tests/fixtures/security_test_quality/protection_removed/bad/no_mutation_marker.test.ts",
        "tests/fixtures/security_test_quality/protection_removed/good/mutation_marker_present.test.ts",
    )?;
    Ok(())
}

/// Every fixture file in the h04 family, both fail and pass sides — the
/// real-corpus half of the property below.
const FIXTURE_CORPUS: [&str; 18] = [
    "tests/fixtures/security_test_quality/asserts_success_only/bad/success_only.test.ts",
    "tests/fixtures/security_test_quality/asserts_success_only/good/rejects.test.ts",
    "tests/fixtures/security_test_quality/mocks_for_money_logic/bad/mock_money.test.ts",
    "tests/fixtures/security_test_quality/mocks_for_money_logic/good/real_money.test.ts",
    "tests/fixtures/security_test_quality/reproducible_seed/bad/no_seed.test.ts",
    "tests/fixtures/security_test_quality/reproducible_seed/good/seeded.test.ts",
    "tests/fixtures/security_test_quality/global_mutation/bad/shared_state.test.ts",
    "tests/fixtures/security_test_quality/global_mutation/good/isolated.test.ts",
    "tests/fixtures/security_test_quality/pass_if_deleted/bad/pass_if_deleted.test.ts",
    "tests/fixtures/security_test_quality/pass_if_deleted/good/asserts_real_outcome.test.ts",
    "tests/fixtures/security_test_quality/no_crash_only/bad/no_crash_only.test.ts",
    "tests/fixtures/security_test_quality/no_crash_only/good/asserts_specific_error.test.ts",
    "tests/fixtures/security_test_quality/snapshot_only/bad/snapshot_only.test.ts",
    "tests/fixtures/security_test_quality/snapshot_only/good/explicit_assertions.test.ts",
    "tests/fixtures/security_test_quality/threat_mapping/bad/unmapped.test.ts",
    "tests/fixtures/security_test_quality/threat_mapping/good/mapped.test.ts",
    "tests/fixtures/security_test_quality/protection_removed/bad/no_mutation_marker.test.ts",
    "tests/fixtures/security_test_quality/protection_removed/good/mutation_marker_present.test.ts",
];

#[test]
fn sec_test_quality_rule_id_property_over_corpus() -> Result<(), ProofFailure> {
    // Property: for EVERY validator in the family and EVERY corpus input
    // (all 18 real fixtures plus malformed, invalid, and degenerate
    // synthetic documents), each emitted finding carries exactly its own
    // validator's rule id and a 1-based line — the invariant the fixture
    // harness enforces pairwise, proven here across the whole family at
    // once.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut corpus = Vec::new();
    for fixture in FIXTURE_CORPUS {
        corpus.push(std::fs::read_to_string(manifest_dir.join(fixture))?);
    }
    // Synthetic half: malformed JS, invalid fragments, empty input, and
    // marker shards that appear without their required context.
    let synthetic_inputs = [
        "",
        "this is malformed and not a test file at all {{{",
        "test(\"half open",
        "expect(res.ok).toBe(true)",
        "toThrow",
        "fc.assert( seed:",
        "let shared beforeEach(",
        "toMatchSnapshot( toBe(",
    ];
    for synthetic in synthetic_inputs {
        // ALLOC-JUSTIFICATION: the corpus owns every document so fixture
        // and synthetic entries iterate uniformly.
        corpus.push(synthetic.to_owned());
    }

    let target_path: RelPath = "apps/x/payments.test.ts".parse()?;
    for validator in validators()? {
        for document in &corpus {
            let findings = validator.validate(ValidationInput {
                file: &target_path,
                source: document,
                scope: ScanScope::Files,
            });
            for finding in &findings {
                assert_eq!(
                    finding.rule_id.as_str(),
                    validator.rule_id().as_str(),
                    "validator {} emitted a foreign rule id",
                    validator.rule_id()
                );
                assert!(finding.line >= 1, "finding lines are 1-based");
            }
        }
    }
    Ok(())
}
