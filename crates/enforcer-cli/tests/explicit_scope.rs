use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let mut dir = exe
        .parent()
        .ok_or("test binary has no parent")?
        .to_path_buf();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let candidate = dir.join(if cfg!(windows) {
        "enforcer.exe"
    } else {
        "enforcer"
    });
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err(format!("enforcer binary not found at {}", candidate.display()).into())
}

fn run_check(
    root: &std::path::Path,
    scope: &str,
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    Ok(Command::new(binary_path()?)
        .current_dir(root)
        .args(["check", scope])
        .status()?)
}

#[test]
fn explicit_file_scope_overrides_ignored_directory_but_all_scope_ignores_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let ignored_file = temp.path().join("ignored/main.rs");
    std::fs::create_dir_all(
        ignored_file
            .parent()
            .ok_or("ignored fixture has no parent directory")?,
    )?;
    std::fs::write(
        &ignored_file,
        "fn bad() { let value: Option<i32> = None; value.unwrap(); }\n",
    )?;
    std::fs::write(
        temp.path().join("ocentra-enforcer.config.json"),
        r#"{"schemaVersion":2,"profileName":"default","ignoreDirs":["ignored"]}"#,
    )?;

    assert_eq!(
        run_check(temp.path(), "ignored/main.rs")?.code(),
        Some(1),
        "explicitly requested files under ignored directories must be scanned"
    );
    let all_status = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["check", "--all"])
        .status()?;
    assert_eq!(
        all_status.code(),
        Some(0),
        "ordinary root discovery must continue honoring ignored directories"
    );
    Ok(())
}
