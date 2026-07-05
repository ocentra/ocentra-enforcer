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

use std::path::PathBuf;

use enforcer_lang_cfml::all_validators;
use enforcer_validator::harness::run_fixture_parity;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Rules whose fixtures do not follow the uniform
/// `fixtures/<RuleId>/{fail,pass}.cfc` shape: `(rule_id, fail_path,
/// pass_path)`, repo-relative to this crate's manifest dir.
fn explicit_fixture_paths() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "CF-SEC-2.1",
            "tests/fixtures/CF-SEC-2.1/fail.cfm",
            "tests/fixtures/CF-SEC-2.1/pass.cfm",
        ),
        (
            "CF-ARCH-3.1",
            "tests/fixtures/CF-ARCH-3.1/fail/OrderService.cfc",
            "tests/fixtures/CF-ARCH-3.1/pass/OrderService.cfc",
        ),
        (
            "CF-STYLE-5.1",
            "tests/fixtures/CF-STYLE-5.1/fail/orderservice.cfc",
            "tests/fixtures/CF-STYLE-5.1/pass/OrderService.cfc",
        ),
        (
            "CF-TEST-1.1",
            "tests/fixtures/CF-TEST-1.1/fail/OrderServiceTest.cfc",
            "tests/fixtures/CF-TEST-1.1/pass/OrderServiceTest.cfc",
        ),
        (
            "CF-TOOL-1.1",
            "tests/fixtures/CF-TOOL-1.1/fail.cflintrc",
            "tests/fixtures/CF-TOOL-1.1/pass.cflintrc",
        ),
        (
            "CF-DEP-1.1",
            "tests/fixtures/CF-DEP-1.1/fail.box.json",
            "tests/fixtures/CF-DEP-1.1/pass.box.json",
        ),
        (
            "CF-TOOL-2.1",
            "tests/fixtures/CF-TOOL-2.1/fail/.github/workflows/ci.yml",
            "tests/fixtures/CF-TOOL-2.1/pass/.github/workflows/ci.yml",
        ),
        (
            "CF-CI-2.1",
            "tests/fixtures/CF-CI-2.1/fail/testbox.json",
            "tests/fixtures/CF-CI-2.1/pass/testbox.json",
        ),
    ]
}

#[test]
fn every_registered_validator_proves_fail_and_pass_fixtures(
) -> Result<(), Box<dyn std::error::Error>> {
    let validators = all_validators()?;
    assert!(
        !validators.is_empty(),
        "expected at least one registered CF-*/CFML-* validator"
    );

    let explicit = explicit_fixture_paths();
    let mut proven = 0usize;
    for validator in &validators {
        let rule_id = validator.rule_id().to_string();
        let (fail_path, pass_path) = match explicit.iter().find(|(id, _, _)| *id == rule_id) {
            Some((_, fail, pass)) => (fail.to_string(), pass.to_string()),
            None => (
                format!("tests/fixtures/{rule_id}/fail.cfc"),
                format!("tests/fixtures/{rule_id}/pass.cfc"),
            ),
        };

        run_fixture_parity(validator.as_ref(), &manifest_dir(), &fail_path, &pass_path)
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
        rule_id: "CF-ERR-1.1".parse()?,
    };
    let outcome = run_fixture_parity(
        &broken,
        &manifest_dir(),
        "tests/fixtures/CF-ERR-1.1/fail.cfc",
        "tests/fixtures/CF-ERR-1.1/pass.cfc",
    );
    assert!(
        outcome.is_err(),
        "the harness must reject a validator that never fires on its fail fixture"
    );
    Ok(())
}
