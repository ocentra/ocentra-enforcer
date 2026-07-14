use enforcer_config::error::ConfigLoadError;

#[test]
fn invalid_environment_values_are_not_rendered_in_error_messages() {
    let error = ConfigLoadError::InvalidEnvVar {
        var: "ENFORCER_PROFILE",
        value: "secret-override-value".to_owned(),
        reason: "unknown profile".to_owned(),
    };

    let rendered = error.to_string();
    assert!(rendered.contains("ENFORCER_PROFILE"));
    assert!(rendered.contains("unknown profile"));
    assert!(!rendered.contains("secret-override-value"));
}
