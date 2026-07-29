use enforcer_domain::config_types::{ConfigProfileName, PolicyOwner, PolicyReason};
use enforcer_security::activation::{
    activation_path, load_project_activation, write_project_activation, SecurityProfileActivation,
    SecurityProfileActivationDto, MONEY_CRITICAL_PROFILE,
};

#[test]
fn project_activation_round_trips_through_the_typed_record(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "enforcer-security-activation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir_all(&root)?;
    let activation = SecurityProfileActivation {
        schema_version: std::num::NonZeroU32::new(1).ok_or("non-zero schema version")?,
        profile_name: ConfigProfileName::try_new(MONEY_CRITICAL_PROFILE.to_owned())?,
        source_spec: "docs/security-policy.md".parse()?,
        owner: PolicyOwner::try_new("platform-security".to_owned())?,
        reason: PolicyReason::try_new("the service processes customer funds".to_owned())?,
    };

    write_project_activation(&root, &activation)?;
    assert!(activation_path(&root).is_file());
    assert_eq!(load_project_activation(&root)?, Some(activation));
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn security_profile_activation_dto_round_trip_json() -> Result<(), Box<dyn std::error::Error>> {
    let dto = SecurityProfileActivationDto {
        schema_version: 1,
        profile_name: MONEY_CRITICAL_PROFILE.to_owned(),
        source_spec: "docs/security-policy.md".to_owned(),
        owner: "platform-security".to_owned(),
        reason: "the service processes customer funds".to_owned(),
    };
    let encoded = serde_json::to_vec(&dto)?;
    let decoded: SecurityProfileActivationDto = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, dto);
    Ok(())
}

#[test]
fn activation_rejects_an_unknown_profile() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join("enforcer-security-activation-invalid");
    let activation = SecurityProfileActivation {
        schema_version: std::num::NonZeroU32::new(1).ok_or("non-zero schema version")?,
        profile_name: ConfigProfileName::try_new("unknown".to_owned())?,
        source_spec: "policy".parse()?,
        owner: PolicyOwner::try_new("security".to_owned())?,
        reason: PolicyReason::try_new("test".to_owned())?,
    };

    let error = match write_project_activation(&root, &activation) {
        Ok(()) => String::new(),
        Err(error) => error,
    };
    assert_eq!(error, "unsupported security profile: unknown");
    Ok(())
}
