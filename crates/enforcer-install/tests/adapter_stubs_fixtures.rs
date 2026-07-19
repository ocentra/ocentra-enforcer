//! Gemini, Cursor, and Zed native-registration acceptance proofs.
//!
//! Each adapter must plan exactly one user-scope configuration write on a
//! fresh home, apply it, verify the resulting registration, preserve
//! unrelated settings, and resolve through the shared adapter selection
//! path by its typed harness id.

use enforcer_domain::install_types::{CheckStatus, InstallCommand, InstallRequestContext};
use enforcer_install::adapters::cursor::CursorAdapter;
use enforcer_install::adapters::gemini::GeminiAdapter;
use enforcer_install::adapters::zed::ZedAdapter;
use enforcer_install::core::{install, HarnessAdapter};

#[test]
fn every_native_adapter_plans_applies_and_verifies_its_user_config(
) -> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let binary = home.path().join("bin").join("enforcer");
    let gemini = GeminiAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let cursor = CursorAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let zed = ZedAdapter::try_new(home.path().join(".config").join("zed"), binary.clone())?;
    let adapters: Vec<&dyn HarnessAdapter> = vec![&gemini, &cursor, &zed];
    let ctx = InstallRequestContext::try_with_defaults(binary)?;

    for adapter in adapters {
        let plan = adapter.plan(&ctx)?;
        assert_eq!(
            plan.planned_changes.len(),
            1,
            "{} must plan exactly one native user-config write",
            adapter.harness_key()
        );
        let applied = adapter.apply(&plan)?;
        assert_eq!(applied.applied.len(), 1);
        assert!(applied
            .applied
            .iter()
            .all(|change| matches!(change.status, CheckStatus::Passed)));

        let verify = adapter.verify(&ctx)?;
        assert_eq!(verify.checks.len(), 1);
        assert!(verify
            .checks
            .iter()
            .all(|check| matches!(check.status, CheckStatus::Passed)));
        assert!(
            adapter.plan(&ctx)?.planned_changes.is_empty(),
            "{} second plan must be idempotent",
            adapter.harness_key()
        );
    }
    Ok(())
}

#[test]
fn each_native_harness_key_resolves_through_adapter_selection(
) -> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let binary = home.path().join("bin").join("enforcer");
    let gemini = GeminiAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let cursor = CursorAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let zed = ZedAdapter::try_new(home.path().join(".config").join("zed"), binary.clone())?;
    let adapters: Vec<&dyn HarnessAdapter> = vec![&gemini, &cursor, &zed];

    for key in ["gemini", "cursor", "zed"] {
        let request = InstallCommand {
            context: InstallRequestContext::try_with_defaults(binary.clone())?,
            only_harnesses: vec![enforcer_domain::ids::HarnessId::try_from(key.to_owned())?],
        };
        let outcomes = install(&adapters, &request)?;
        assert_eq!(
            outcomes.len(),
            1,
            "selection for `{key}` must resolve exactly one adapter"
        );
        assert_eq!(outcomes[0].0.as_str(), key);
    }
    Ok(())
}

#[test]
fn an_unregistered_harness_key_is_a_typed_error_not_a_silent_skip(
) -> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let binary = home.path().join("bin").join("enforcer");
    let gemini = GeminiAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let cursor = CursorAdapter::try_new(home.path().to_path_buf(), binary.clone())?;
    let zed = ZedAdapter::try_new(home.path().join(".config").join("zed"), binary.clone())?;
    let adapters: Vec<&dyn HarnessAdapter> = vec![&gemini, &cursor, &zed];

    let request = InstallCommand {
        context: InstallRequestContext::try_with_defaults(binary)?,
        only_harnesses: vec![enforcer_domain::ids::HarnessId::try_from(
            "not-a-real-harness".to_owned(),
        )?],
    };
    let result = install(&adapters, &request);
    assert!(matches!(
        result,
        Err(enforcer_install::error::InstallError::UnknownAdapter { ref id, .. })
            if id.as_str() == "not-a-real-harness"
    ));
    Ok(())
}
