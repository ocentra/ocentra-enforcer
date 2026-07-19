use enforcer_domain::boundary::decode_error::DecodeError;

use enforcer_domain::config_types::{CrateName, HarnessConfig, HarnessRunLimit};

#[test]
fn crate_name_rejects_empty_or_non_cargo_spelling() -> Result<(), DecodeError> {
    let empty = match "".parse::<CrateName>() {
        Err(error) => error,
        Ok(_) => return Err(DecodeError::new("crateName", "empty name was accepted")),
    };
    assert_eq!(empty.path, "crateName");
    let spaced = match "enforcer rules".parse::<CrateName>() {
        Err(error) => error,
        Ok(_) => return Err(DecodeError::new("crateName", "spaced name was accepted")),
    };
    assert_eq!(spaced.path, "crateName");
    let valid = "enforcer-rules"
        .parse::<CrateName>()
        .map(|value| value.as_str().to_owned());
    assert_eq!(valid, Ok("enforcer-rules".to_owned()));
    Ok(())
}

#[test]
fn harness_retention_distinguishes_unlimited_from_explicit_zero() {
    let unlimited = HarnessConfig {
        max_runs: None,
        ..HarnessConfig::default()
    };
    let zero = HarnessConfig {
        max_runs: Some(HarnessRunLimit::from_value(0)),
        ..HarnessConfig::default()
    };
    assert_eq!(unlimited.max_runs, None);
    assert_eq!(zero.max_runs.map(HarnessRunLimit::get), Some(0));
}
