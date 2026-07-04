//! Fixture/parity proof for every validator this crate registers: for
//! each of the 61 PY-* rules, `run_fixture_parity` asserts the validator
//! fires on `fixtures/<RuleId>/fail.*` and stays silent on
//! `fixtures/<RuleId>/pass.*`. This is the workpack's required
//! `cargo test -p enforcer-lang-py` proof -- fail/pass fixture coverage
//! per rule, run under the arc-05 `enforcer-validator` harness.

use std::path::PathBuf;

use enforcer_lang_py::all_validators;
use enforcer_validator::harness::run_fixture_parity;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Fixture file extension for one rule's fail/pass pair. Most rules scan
/// `.py` source; the toolchain-diagnostics rules scan JSON diagnostics
/// blobs, the manifest-shape rules scan `pyproject.toml`-shaped `.toml`
/// text or `requirements.txt`-shaped `.txt` text.
fn fixture_extension(rule_id: &str) -> &'static str {
    match rule_id {
        "PY-3.1" | "PY-3.2" | "PY-5.5" | "PY-5.6" => "json",
        "PY-5.1" | "PY-5.2" | "PY-5.3" | "PY-5.4" | "PY-5.7" => "toml",
        "PY-5.8" | "PY-5.9" | "PY-5.10" => "txt",
        _ => "py",
    }
}

#[test]
fn every_registered_validator_proves_fail_and_pass_fixtures(
) -> Result<(), Box<dyn std::error::Error>> {
    let validators = all_validators()?;
    assert_eq!(
        validators.len(),
        61,
        "expected exactly 61 registered PY-* validators"
    );

    let mut proven = 0usize;
    for validator in &validators {
        let rule_id = validator.rule_id().to_string();
        let ext = fixture_extension(&rule_id);
        let fail_path = format!("fixtures/{rule_id}/fail.{ext}");
        let pass_path = format!("fixtures/{rule_id}/pass.{ext}");

        run_fixture_parity(validator.as_ref(), &manifest_dir(), &fail_path, &pass_path)
            .map_err(|error| format!("{rule_id}: {error}"))?;
        proven += 1;
    }

    assert_eq!(proven, 61, "every rule's fixture pair must be proven");
    Ok(())
}

/// Negative-path proof (mirrors `enforcer-validator`'s own harness tests):
/// a validator that never fires must be CAUGHT by the harness, not silently
/// accepted. Reuses one of this crate's real rule ids and fixture files so
/// the assertion is meaningful rather than synthetic.
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
        rule_id: "PY-1.1".parse()?,
    };
    let outcome = run_fixture_parity(
        &broken,
        &manifest_dir(),
        "fixtures/PY-1.1/fail.py",
        "fixtures/PY-1.1/pass.py",
    );
    assert!(
        outcome.is_err(),
        "the harness must reject a validator that never fires on its fail fixture"
    );
    Ok(())
}
