use enforcer_config::error::ConfigLoadError;
use enforcer_domain::config_types::{
    ConfigEnvironmentValue, ConfigEnvironmentVariable, ConfigErrorReason,
};

#[test]
fn invalid_environment_values_are_not_rendered_in_error_messages(
) -> Result<(), Box<dyn std::error::Error>> {
    let error = ConfigLoadError::InvalidEnvVar {
        var: ConfigEnvironmentVariable::new("ENFORCER_PROFILE".to_owned())?,
        value: ConfigEnvironmentValue::from_owned("secret-override-value".to_owned()),
        reason: ConfigErrorReason::from_owned("unknown profile".to_owned()),
    };

    let rendered = error.to_string();
    assert_eq!(
        rendered,
        "environment variable `ENFORCER_PROFILE` is set to an invalid value: unknown profile"
    );
    Ok(())
}
