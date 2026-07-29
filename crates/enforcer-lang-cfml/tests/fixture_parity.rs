//! Fixture/parity proof for every FIXTURE-PROVABLE validator this crate
//! registers (see `enforcer_lang_cfml::all_validators`'s doc comment for
//! why the CFLint adapter is excluded): for each `CF-*`/`CFML-*` rule,
//! `run_fixture_parity` asserts the validator fires on its fail fixture
//! and stays silent on its pass fixture. This is the workpack's required
//! `cargo test -p enforcer-lang-cfml` proof.
//!
//! Most rules use the uniform `fixtures/<RuleId>/fail.cfc` +
//! `.../pass.cfc` shape; a handful need a different filename/path/
//! extension for the check to fire realistically -- those are listed
//! explicitly in [`explicit_fixture_paths`] rather than forced into the
//! uniform pattern (mirrors `enforcer-lang-dart::tests::fixture_parity`'s
//! own `explicit_fixture_paths` convention).

use std::path::Path;

use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_lang_cfml::all_validators;
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

/// Rules whose fixtures do not follow the uniform
/// `fixtures/<RuleId>/{fail,pass}.cfc` shape: `(rule_id, fail_path,
/// pass_path)`, repo-relative to this crate's manifest dir.
fn fixture_pair(
    rule: BuiltInCfmlRule,
    fail_path: &str,
    pass_path: &str,
) -> Result<FixturePair, Box<dyn std::error::Error>> {
    Ok(FixturePair {
        rule_id: rule.id(),
        fail_path: fail_path.parse()?,
        pass_path: pass_path.parse()?,
    })
}

fn explicit_fixture_paths() -> Result<Vec<FixturePair>, Box<dyn std::error::Error>> {
    Ok(vec![
        fixture_pair(
            BuiltInCfmlRule::XssOutput,
            "tests/fixtures/CF-SEC-2.1/fail.cfm",
            "tests/fixtures/CF-SEC-2.1/pass.cfm",
        )?,
        fixture_pair(
            BuiltInCfmlRule::ServiceScopeRead,
            "tests/fixtures/CF-ARCH-3.1/fail/OrderService.cfc",
            "tests/fixtures/CF-ARCH-3.1/pass/OrderService.cfc",
        )?,
        fixture_pair(
            BuiltInCfmlRule::FilenameConvention,
            "tests/fixtures/CF-STYLE-5.1/fail/orderservice.cfc",
            "tests/fixtures/CF-STYLE-5.1/pass/OrderService.cfc",
        )?,
        fixture_pair(
            BuiltInCfmlRule::TestboxBaseSpec,
            "tests/fixtures/CF-TEST-1.1/fail/OrderServiceTest.cfc",
            "tests/fixtures/CF-TEST-1.1/pass/OrderServiceTest.cfc",
        )?,
        fixture_pair(
            BuiltInCfmlRule::CflintrcHardGate,
            "tests/fixtures/CF-TOOL-1.1/fail.cflintrc",
            "tests/fixtures/CF-TOOL-1.1/pass.cflintrc",
        )?,
        fixture_pair(
            BuiltInCfmlRule::PinnedDependency,
            "tests/fixtures/CF-DEP-1.1/fail.box.json",
            "tests/fixtures/CF-DEP-1.1/pass.box.json",
        )?,
        fixture_pair(
            BuiltInCfmlRule::CfformatCiStep,
            "tests/fixtures/CF-TOOL-2.1/fail/.github/workflows/ci.yml",
            "tests/fixtures/CF-TOOL-2.1/pass/.github/workflows/ci.yml",
        )?,
        fixture_pair(
            BuiltInCfmlRule::CoverageFloor,
            "tests/fixtures/CF-CI-2.1/fail/testbox.json",
            "tests/fixtures/CF-CI-2.1/pass/testbox.json",
        )?,
    ])
}

#[test]
fn every_registered_validator_proves_fail_and_pass_fixtures(
) -> Result<(), Box<dyn std::error::Error>> {
    let validators = all_validators()?;
    assert!(
        !validators.is_empty(),
        "expected at least one registered CF-*/CFML-* validator"
    );

    let repo_root = manifest_dir()?;
    let explicit = explicit_fixture_paths()?;
    let mut proven = 0usize;
    for validator in &validators {
        let rule_id = validator.rule_id();
        let (fail_path, pass_path) = match explicit.iter().find(|pair| &pair.rule_id == rule_id) {
            Some(pair) => (&pair.fail_path, &pair.pass_path),
            None => {
                let fail_path: RelPath = format!("tests/fixtures/{rule_id}/fail.cfc").parse()?;
                let pass_path: RelPath = format!("tests/fixtures/{rule_id}/pass.cfc").parse()?;
                run_fixture_parity(validator.as_ref(), &repo_root, &fail_path, &pass_path)
                    .map_err(|error| format!("{rule_id}: {error}"))?;
                proven += 1;
                continue;
            }
        };

        run_fixture_parity(validator.as_ref(), &repo_root, fail_path, pass_path)
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

/// Negative-path proof (mirrors `enforcer-lang-dart`/`enforcer-validator`'s
/// own harness tests): a validator that never fires must be CAUGHT by the
/// harness, not silently accepted.
#[test]
fn harness_catches_a_validator_that_never_fires() -> Result<(), Box<dyn std::error::Error>> {
    use enforcer_domain::findings::Finding;
    use enforcer_domain::ids::RuleId;
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
        rule_id: BuiltInCfmlRule::TypedThrow.id(),
    };
    let repo_root = manifest_dir()?;
    let fail_path: RelPath = "tests/fixtures/CF-ERR-1.1/fail.cfc".parse()?;
    let pass_path: RelPath = "tests/fixtures/CF-ERR-1.1/pass.cfc".parse()?;
    let outcome = run_fixture_parity(&broken, &repo_root, &fail_path, &pass_path);
    assert!(
        outcome.is_err(),
        "the harness must reject a validator that never fires on its fail fixture"
    );
    Ok(())
}
