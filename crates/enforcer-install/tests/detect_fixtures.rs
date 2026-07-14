//! Integration proof over the CHECKED-IN `tests/fixtures/detect/**`
//! fixtures (workpack c02 acceptance row: "Fixtures:
//! `tests/fixtures/detect/caps-codex/**`, `caps-claude-bare/**`"), as
//! distinct from the tempdir-generated fixtures inside
//! `src/detect.rs`'s own unit tests. Both prove the same behavior; this
//! file additionally proves the on-disk fixture layout itself stays
//! correct (a reviewer can inspect the exact bytes a fixture carries).

use enforcer_install::detect::{detect_harnesses, Cap, EnvSource, MapEnv, RealFs, Support};
use std::path::{Path, PathBuf};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/detect")
        .join(name)
}

fn env_for(home: &Path) -> impl EnvSource {
    MapEnv::new().with("HOME", home.display().to_string())
}

#[test]
fn empty_home_fixture_detects_no_harness_present() -> Result<(), Box<dyn std::error::Error>> {
    let home = fixture_root("empty-home");
    let env = env_for(&home);
    let fs = RealFs;

    let records = detect_harnesses(&env, &fs)?;
    for record in &records {
        assert!(!record.present, "expected {} absent", record.id);
        assert!(record.capabilities.is_none());
    }
    Ok(())
}

#[test]
fn blank_home_override_fails_closed_without_falling_back_to_default(
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    std::fs::create_dir_all(temporary.path().join(".codex"))?;
    let env = MapEnv::new()
        .with("HOME", temporary.path().display().to_string())
        .with("CODEX_HOME", "   ");
    let records = detect_harnesses(&env, &RealFs)?;
    let codex = records
        .iter()
        .find(|record| record.id.as_str() == "codex")
        .ok_or("expected codex record")?;

    assert!(!codex.present);
    assert!(codex.home_path.is_none());
    assert_eq!(
        codex.evidence[0].observation,
        "override is blank; refusing to resolve an ambiguous harness home"
    );
    Ok(())
}

#[test]
fn caps_codex_fixture_declares_implicit_invocation_yes() -> Result<(), Box<dyn std::error::Error>> {
    let home = fixture_root("caps-codex");
    let env = env_for(&home);
    let fs = RealFs;

    let records = detect_harnesses(&env, &fs)?;
    let codex = records
        .iter()
        .find(|r| r.id.as_str() == "codex")
        .ok_or("expected a codex record")?;
    assert!(codex.present);
    let caps = codex
        .capabilities
        .as_ref()
        .ok_or("present harness must carry a capability manifest")?;

    assert_eq!(caps.implicit_invocation.value, Support::Yes);
    assert_eq!(caps.cross_session_messaging.value, Support::Yes);
    // Never guessed Yes for fields with no on-disk detector.
    assert_eq!(caps.max_concurrent_agents.value, Cap::Unknown);
    assert_eq!(caps.sub_agent_nesting_depth.value, Cap::Unknown);
    assert_eq!(caps.background_tasks.value, Support::Unknown);
    assert_eq!(caps.scheduled_tasks.value, Support::Unknown);
    Ok(())
}

#[test]
fn caps_claude_bare_fixture_declares_every_capability_unknown(
) -> Result<(), Box<dyn std::error::Error>> {
    let home = fixture_root("caps-claude-bare");
    let env = env_for(&home);
    let fs = RealFs;

    let records = detect_harnesses(&env, &fs)?;
    let claude = records
        .iter()
        .find(|r| r.id.as_str() == "claude")
        .ok_or("expected a claude record")?;
    assert!(claude.present);
    let caps = claude
        .capabilities
        .as_ref()
        .ok_or("present harness must carry a capability manifest")?;

    assert_eq!(caps.max_concurrent_agents.value, Cap::Unknown);
    assert_eq!(caps.sub_agent_nesting_depth.value, Cap::Unknown);
    assert_eq!(caps.background_tasks.value, Support::Unknown);
    assert_eq!(caps.scheduled_tasks.value, Support::Unknown);
    assert_eq!(caps.cross_session_messaging.value, Support::Unknown);
    assert_eq!(caps.implicit_invocation.value, Support::Unknown);
    Ok(())
}
