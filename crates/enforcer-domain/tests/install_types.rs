use enforcer_domain::install_types::{
    EmptyReleaseVersion, ReleaseVersion, SessionStartHookCommand, SessionStartHookMatcher,
    SessionStartHookReminderBody, TargetPlatform,
};

#[test]
fn release_version_rejects_blank_tags_and_builds_platform_assets() {
    assert_eq!(
        ReleaseVersion::try_new("  ".to_owned()),
        Err(EmptyReleaseVersion)
    );

    match ReleaseVersion::try_new("1.2.3".to_owned()) {
        Ok(version) => assert_eq!(
            TargetPlatform::WindowsX86_64.asset_name(&version),
            "enforcer-v1.2.3-x86_64-pc-windows-msvc.zip"
        ),
        Err(error) => assert_ne!(error, EmptyReleaseVersion),
    }
}

#[test]
fn session_start_hook_brands_preserve_valid_wire_text_and_reject_empty_invariants(
) -> Result<(), enforcer_domain::boundary::decode_error::DecodeError> {
    assert_eq!(
        SessionStartHookMatcher::try_new(String::new())?.as_str(),
        "",
        "the Claude matcher intentionally uses empty text to target every session source"
    );
    assert_eq!(
        SessionStartHookCommand::try_new("enforcer hooks sessionstart".to_owned())?.as_str(),
        "enforcer hooks sessionstart"
    );
    assert_eq!(
        SessionStartHookReminderBody::try_new("enforcer first".to_owned())?.as_str(),
        "enforcer first"
    );
    assert!(SessionStartHookCommand::try_from(" \t".to_owned()).is_err());
    assert!(SessionStartHookReminderBody::try_from("\n".to_owned()).is_err());
    Ok(())
}
