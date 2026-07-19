//! Fixture/parity proof for every validator this crate registers: for
//! each `DART-*` rule, `run_fixture_parity` asserts the validator fires
//! on its fail fixture and stays silent on its pass fixture. This is the
//! workpack's required `cargo test -p enforcer-lang-dart` proof.
//!
//! Most rules use the uniform `fixtures/<RuleId>/fail.dart` +
//! `.../pass.dart` shape; a handful need a different filename/extension
//! (`DART-ARCH-1.1`'s layer-boundary fixtures must live under a `data/`
//! subdirectory for the path check to fire; the toolchain-manifest rules
//! (`DART-TOOL-*`/`DART-DEP-1.1`) scan YAML-shaped manifest/CI text, not
//! Dart source; `DART-NAME-1.1` fires on the FILENAME itself, so its
//! fixtures are `OrderCard.dart` / `order_card.dart`; `DART-GEN-1.1`
//! fires on a generated-file EXTENSION shape (`.g.dart`)) — those are
//! listed explicitly in [`explicit_fixture_paths`] rather than forced
//! into the uniform pattern.

use std::path::Path;

use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_lang_dart::all_validators;
use enforcer_validator::harness::run_fixture_parity;

fn manifest_dir() -> Result<RepoRoot, Box<dyn std::error::Error>> {
    Ok(RepoRoot::try_from(Path::new(env!("CARGO_MANIFEST_DIR")))?)
}

#[derive(Debug)]
struct FixturePair {
    rule_id: RuleId,
    fail_path: RelPath,
    pass_path: RelPath,
}

fn fixture_pair(
    rule: BuiltInDartRule,
    fail_path: &str,
    pass_path: &str,
) -> Result<FixturePair, Box<dyn std::error::Error>> {
    Ok(FixturePair {
        rule_id: rule.id(),
        fail_path: fail_path.parse()?,
        pass_path: pass_path.parse()?,
    })
}

/// Rules whose fixtures do not follow the uniform
/// `fixtures/<RuleId>/{fail,pass}.dart` shape: `(rule_id, fail_path,
/// pass_path)`, repo-relative to this crate's manifest dir.
fn explicit_fixture_paths() -> Result<Vec<FixturePair>, Box<dyn std::error::Error>> {
    Ok(vec![
        fixture_pair(
            BuiltInDartRule::LayerBoundary,
            "tests/fixtures/DART-ARCH-1.1/data/order_repo.dart",
            "tests/fixtures/DART-ARCH-1.1/data/order_repo_clean.dart",
        )?,
        fixture_pair(
            BuiltInDartRule::StrictAnalysisOptions,
            "tests/fixtures/DART-TOOL-1.1/fail.yaml",
            "tests/fixtures/DART-TOOL-1.1/pass.yaml",
        )?,
        fixture_pair(
            BuiltInDartRule::CiRunsAnalyze,
            "tests/fixtures/DART-TOOL-1.2/fail.yaml",
            "tests/fixtures/DART-TOOL-1.2/pass.yaml",
        )?,
        fixture_pair(
            BuiltInDartRule::CiRunsFormatCheck,
            "tests/fixtures/DART-TOOL-1.3/fail.yaml",
            "tests/fixtures/DART-TOOL-1.3/pass.yaml",
        )?,
        fixture_pair(
            BuiltInDartRule::UnpinnedDependency,
            "tests/fixtures/DART-DEP-1.1/fail.yaml",
            "tests/fixtures/DART-DEP-1.1/pass.yaml",
        )?,
        fixture_pair(
            BuiltInDartRule::HandEditedGeneratedFile,
            "tests/fixtures/DART-GEN-1.1/order.g.dart",
            "tests/fixtures/DART-GEN-1.1/order_pass.g.dart",
        )?,
        fixture_pair(
            BuiltInDartRule::SnakeCaseFilename,
            "tests/fixtures/DART-NAME-1.1/OrderCard.dart",
            "tests/fixtures/DART-NAME-1.1/order_card.dart",
        )?,
    ])
}

#[test]
fn every_registered_validator_proves_fail_and_pass_fixtures(
) -> Result<(), Box<dyn std::error::Error>> {
    let validators = all_validators()?;
    assert!(
        !validators.is_empty(),
        "expected at least one registered DART-* validator"
    );

    let root = manifest_dir()?;
    let explicit = explicit_fixture_paths()?;
    let mut proven = 0usize;
    for validator in &validators {
        let rule_id = validator.rule_id();
        let Some(pair) = explicit.iter().find(|pair| &pair.rule_id == rule_id) else {
            let fail_path: RelPath = format!("tests/fixtures/{rule_id}/fail.dart").parse()?;
            let pass_path: RelPath = format!("tests/fixtures/{rule_id}/pass.dart").parse()?;
            run_fixture_parity(validator.as_ref(), &root, &fail_path, &pass_path)
                .map_err(|error| format!("{rule_id}: {error}"))?;
            proven += 1;
            continue;
        };

        run_fixture_parity(validator.as_ref(), &root, &pair.fail_path, &pair.pass_path)
            .map_err(|error| format!("{rule_id}: {error}"))?;
        proven += 1;
    }

    assert_eq!(
        proven,
        validators.len(),
        "every rule's fixture pair must be proven"
    );
    Ok(())
}

/// Negative-path proof (mirrors `enforcer-lang-py`/`enforcer-validator`'s
/// own harness tests): a validator that never fires must be CAUGHT by
/// the harness, not silently accepted.
#[test]
fn harness_catches_a_validator_that_never_fires() -> Result<(), Box<dyn std::error::Error>> {
    use enforcer_domain::findings::Finding;
    use enforcer_validator::validator::{ValidationInput, Validator};

    struct NeverFires {
        rule_id: RuleId,
    }

    impl Validator for NeverFires {
        fn rule_id(&self) -> &RuleId {
            &self.rule_id
        }

        fn validate(&self, _input: ValidationInput<'_>) -> Vec<Finding> {
            Vec::new()
        }
    }

    let broken = NeverFires {
        rule_id: BuiltInDartRule::UncheckedBangOrCast.id(),
    };
    let root = manifest_dir()?;
    let fail_path: RelPath = "tests/fixtures/DART-BANG-1.1/fail.dart".parse()?;
    let pass_path: RelPath = "tests/fixtures/DART-BANG-1.1/pass.dart".parse()?;
    let outcome = run_fixture_parity(&broken, &root, &fail_path, &pass_path);
    assert!(
        outcome.is_err(),
        "the harness must reject a validator that never fires on its fail fixture"
    );
    Ok(())
}
