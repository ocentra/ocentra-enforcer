use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_lang_ts::rules::eslint_json::EslintJsonValidator;
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

#[test]
fn invalid_eslint_wiring_is_rejected_and_valid_json_wiring_passes(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = EslintJsonValidator::new()?;
    let root: RepoRoot = env!("CARGO_MANIFEST_DIR").parse()?;
    let fail: RelPath = "fixtures/eslint-json/ts-5-2/fail.json".parse()?;
    let pass: RelPath = "fixtures/eslint-json/ts-5-2/pass.json".parse()?;

    let invalid_findings = validator.validate(ValidationInput {
        file: &fail,
        source: ValidationSource::from_text(r#"{"scripts":{"lint":"eslint ."}}"#),
        scope: ScanScope::Files,
    });
    assert_eq!(invalid_findings.len(), 1);
    assert_eq!(invalid_findings[0].rule_id, *validator.rule_id());

    let valid_findings = validator.validate(ValidationInput {
        file: &pass,
        source: ValidationSource::from_text(
            r#"{"devDependencies":{"typescript-eslint":"latest"},"scripts":{"lint":"eslint . --format json"}}"#,
        ),
        scope: ScanScope::Files,
    });
    assert!(valid_findings.is_empty());

    run_fixture_parity(&validator, &root, &fail, &pass)?;
    Ok(())
}
