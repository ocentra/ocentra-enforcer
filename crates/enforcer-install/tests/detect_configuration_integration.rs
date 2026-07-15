//! External proof for the harness-configuration detection boundary.

use enforcer_install::detect::{detect_harnesses, HarnessId, MapEnv, RealFs, Support};
use std::fs;

#[test]
fn public_detection_boundary_normalizes_configuration_and_rejects_invalid_ids(
) -> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let agents_dir = home.path().join(".codex").join("agents");
    fs::create_dir_all(&agents_dir)?;
    fs::write(
        agents_dir.join("openai.yaml"),
        "allow_implicit_invocation: true\n",
    )?;

    let env = MapEnv::new().with("HOME", home.path().display().to_string());
    let records = detect_harnesses(&env, &RealFs)?;
    let codex = records
        .iter()
        .find(|record| record.id.as_str() == "codex")
        .ok_or("codex configuration record is missing")?;

    assert!(codex.present);
    let capabilities = codex
        .capabilities
        .as_ref()
        .ok_or("present codex must include a capability configuration")?;
    assert_eq!(capabilities.implicit_invocation.value, Support::Yes);

    let serialized: serde_json::Value = serde_json::to_value(codex)?;
    assert_eq!(
        serialized["capabilities"]["implicitInvocation"]["value"],
        serde_json::Value::String("yes".to_owned())
    );

    let invalid = serde_json::from_str::<HarnessId>("\"Not Valid\"")
        .map(|id| id.to_string())
        .map_err(|error| error.to_string());
    assert_eq!(
        invalid,
        Err(
            "decode/validation failed at `harnessId`: expected lowercase kebab-case (e.g. `claude`, `codex`, `kilocode`)"
                .to_owned()
        )
    );
    Ok(())
}
