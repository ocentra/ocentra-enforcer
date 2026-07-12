//! f02 CLI-integration proof: spawn the REAL compiled `enforcer` binary and
//! run `enforcer onboard` end-to-end against a temp repo -- mirrors
//! `tests/cli_integration.rs`'s pattern of spawning the real binary rather
//! than faking the process boundary. New file (this workpack's file grant
//! is `crates/enforcer-cli/src/onboard.rs` + its tests/fixtures); the
//! existing `cli_integration.rs` is left untouched.

use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let mut dir = exe
        .parent()
        .ok_or("test binary has no parent directory")?
        .to_path_buf();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let candidate = dir.join(if cfg!(windows) {
        "enforcer.exe"
    } else {
        "enforcer"
    });
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(format!("enforcer binary not found at {}", candidate.display()).into())
}

fn write_fixture(root: &std::path::Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("lib.rs"),
        "fn bad() { let x: Option<i32> = None; x.unwrap(); }\n",
    )
}

#[test]
fn onboard_scaffolds_enforce_dir_via_real_binary() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .arg("onboard")
        .status()?;
    assert!(status.success(), "onboard on a fresh repo must exit 0");
    assert!(temp.path().join(".enforce").join("config").exists());
    assert!(temp.path().join(".enforce").join("baseline.json").exists());
    assert!(temp.path().join(".enforce").join("project.json").exists());
    Ok(())
}

#[test]
fn onboard_accepts_an_explicit_repo_path_argument() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .arg("onboard")
        .arg(temp.path())
        .status()?;
    assert!(status.success());
    assert!(temp.path().join(".enforce").join("baseline.json").exists());
    Ok(())
}

#[test]
fn onboard_is_idempotent_across_two_real_binary_runs() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fixture(temp.path())?;
    let binary = binary_path()?;
    let first = Command::new(&binary)
        .current_dir(temp.path())
        .arg("onboard")
        .status()?;
    assert!(first.success());
    let baseline_path = temp.path().join(".enforce").join("baseline.json");
    let first_bytes = std::fs::read(&baseline_path)?;

    let second = Command::new(&binary)
        .current_dir(temp.path())
        .arg("onboard")
        .status()?;
    assert!(second.success());
    let second_bytes = std::fs::read(&baseline_path)?;
    assert_eq!(
        first_bytes, second_bytes,
        "re-onboarding an unchanged repo must not change the baseline bytes"
    );
    Ok(())
}

#[test]
fn onboard_help_does_not_advertise_a_bypass_flag() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let output = Command::new(&binary).args(["onboard", "--help"]).output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for bad_word in ["--force", "--no-verify", "--skip-checks", "--bypass"] {
        assert!(
            !text.contains(bad_word),
            "onboard --help must never advertise `{bad_word}`"
        );
    }
    Ok(())
}

#[test]
fn onboard_with_no_args_defaults_to_the_current_directory() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    write_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .arg("onboard")
        .status()?;
    assert!(status.success());
    Ok(())
}
