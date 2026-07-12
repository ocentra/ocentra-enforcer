use enforcer_security::activation::{
    activation_path, load_project_activation, write_project_activation, SecurityProfileActivation,
    MONEY_CRITICAL_PROFILE,
};

#[test]
fn project_activation_round_trips_through_the_typed_record() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "enforcer-security-activation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir_all(&root)?;
    let activation = SecurityProfileActivation {
        schema_version: 1,
        profile_name: MONEY_CRITICAL_PROFILE.to_owned(),
        source_spec: "docs/security-policy.md".to_owned(),
        owner: "platform-security".to_owned(),
        reason: "the service processes customer funds".to_owned(),
    };

    write_project_activation(&root, &activation)?;
    assert!(activation_path(&root).is_file());
    assert_eq!(load_project_activation(&root)?, Some(activation));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn activation_rejects_an_unknown_profile() {
    let root = std::env::temp_dir().join("enforcer-security-activation-invalid");
    let activation = SecurityProfileActivation {
        schema_version: 1,
        profile_name: "unknown".to_owned(),
        source_spec: "policy".to_owned(),
        owner: "security".to_owned(),
        reason: "test".to_owned(),
    };

    let error = match write_project_activation(&root, &activation) {
        Ok(()) => String::new(),
        Err(error) => error,
    };
    assert_eq!(error, "unsupported security profile: unknown");
}
