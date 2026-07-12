use enforcer_lang_ts::rules::eslint_json::EslintJsonValidator;
use enforcer_validator::harness::run_fixture_parity;
use std::path::PathBuf;

#[test]
fn invalid_eslint_wiring_is_rejected_and_valid_json_wiring_passes(
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = EslintJsonValidator::new()?;
    run_fixture_parity(
        &validator,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        "fixtures/eslint-json/ts-5-2/fail.json",
        "fixtures/eslint-json/ts-5-2/pass.json",
    )?;
    Ok(())
}
