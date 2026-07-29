use enforcer_proof::legacy_import::{classify_scripts, collect_legacy_artifacts};

#[test]
fn legacy_import_rejects_script_roots_outside_the_repository(
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("outside-proof.mjs"), "// proof")?;

    let outcome = classify_scripts(repository.path(), outside.path());
    assert!(matches!(
        outcome,
        Err(enforcer_core::error::Error::Io(ref error))
            if error.kind() == std::io::ErrorKind::InvalidInput
                && error.to_string().contains("outside repository root")
    ));
    Ok(())
}

#[test]
fn legacy_import_rejects_artifact_roots_outside_the_repository(
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("outside.json"), r#"{"ok": true}"#)?;
    let outside_root = outside.path().to_string_lossy().into_owned();

    let outcome = collect_legacy_artifacts(repository.path(), &[outside_root.as_str()]);
    assert!(matches!(
        outcome,
        Err(enforcer_core::error::Error::Io(ref error))
            if error.kind() == std::io::ErrorKind::InvalidInput
                && error.to_string().contains("outside repository root")
    ));
    Ok(())
}

#[test]
fn legacy_import_deduplicates_overlapping_artifact_roots() -> Result<(), Box<dyn std::error::Error>>
{
    let repository = tempfile::tempdir()?;
    let artifact_root = repository.path().join("legacy");
    std::fs::create_dir_all(&artifact_root)?;
    std::fs::write(artifact_root.join("result.json"), r#"{"status":"passed"}"#)?;

    let bundle = collect_legacy_artifacts(repository.path(), &["legacy", "legacy"])?;
    assert_eq!(bundle.artifacts.len(), 1);
    assert_eq!(bundle.artifacts[0].path.as_str(), "legacy/result.json");
    Ok(())
}
