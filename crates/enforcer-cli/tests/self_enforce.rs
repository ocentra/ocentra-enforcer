//! a10 integration proof: the enforcer enforces ITSELF.
//!
//! Three slices, per the a10 workpack's acceptance rows:
//! 1. the real built `enforcer` executable (`CARGO_BIN_EXE_enforcer`),
//!    run on a seeded self-violating fixture, exits with the
//!    `Violations` class (fail-fixture -- the gate bites);
//! 2. the same executable on a clean tree exits zero (pass-fixture);
//! 3. `xtask dogfood --no-toolchain` against the LIVE workspace is green
//!    with a visibly NONZERO dispatched-file count -- the baseline-aware
//!    self-scan: the committed `xtask/dogfood-baseline.json` grandfathers
//!    the pre-existing debt (demoted to warnings by d02's ratchet) and
//!    only a NEW violation fails, so the gate starts green instead of
//!    start-red-then-bypassed.
//!
//! `--no-toolchain` in slice 3 skips xtask's `cargo fmt`/`clippy`/`deny`/
//! `audit` subprocesses: nested `cargo` invocations inside `cargo test`
//! contend for the same target-dir build lock as the (already running)
//! outer cargo and re-gate checks ci.yml/dogfood.yml already run as
//! first-class CI steps. The rust-rule self-scan -- the part a10 uniquely
//! owns -- runs in full.

use std::process::Command;

#[test]
fn enforcer_flags_a_seeded_self_violation() -> Result<(), std::io::Error> {
    let temp = tempfile::tempdir()?;
    std::fs::create_dir_all(temp.path().join("src"))?;
    // Assembled so the flagged token never appears verbatim in this test
    // file's own source (the enforcer scans its own tests too).
    let violating_body = format!(
        "fn bad() {{ let x: Option<i32> = None; x.{}(); }}",
        "unwrap"
    );
    std::fs::write(temp.path().join("src/lib.rs"), &violating_body)?;
    let output = Command::new(env!("CARGO_BIN_EXE_enforcer"))
        .args(["check", "--all"])
        .current_dir(temp.path())
        .output()?;
    let rendered_stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a seeded violation must exit the Violations class (1); stdout: {rendered_stdout}"
    );
    Ok(())
}

#[test]
fn enforcer_passes_a_clean_tree() -> Result<(), std::io::Error> {
    let temp = tempfile::tempdir()?;
    std::fs::create_dir_all(temp.path().join("src"))?;
    std::fs::write(temp.path().join("src/lib.rs"), "fn ok() { let _keep = 7; }")?;
    let output = Command::new(env!("CARGO_BIN_EXE_enforcer"))
        .args(["check", "--all"])
        .current_dir(temp.path())
        .output()?;
    let rendered_stdout = String::from_utf8_lossy(&output.stdout);
    let rendered_stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a clean tree must exit zero; stdout: {rendered_stdout} stderr: {rendered_stderr}"
    );
    Ok(())
}

#[test]
fn xtask_dogfood_is_green_on_the_live_workspace_with_nonzero_ran_count(
) -> Result<(), std::io::Error> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| {
            std::io::Error::other("expected crates/enforcer-cli two levels under the root")
        })?;
    let xtask_name = if cfg!(windows) { "xtask.exe" } else { "xtask" };
    let xtask = std::env::current_exe()?
        .parent()
        .and_then(std::path::Path::parent)
        .map(|target_profile| target_profile.join(xtask_name))
        .filter(|candidate| candidate.is_file());
    let output = if let Some(xtask) = xtask {
        Command::new(xtask)
            .args(["dogfood", "--no-toolchain"])
            .current_dir(workspace_root)
            .output()?
    } else {
        Command::new("cargo")
            .args([
                "run",
                "-p",
                "xtask",
                "--quiet",
                "--",
                "dogfood",
                "--no-toolchain",
            ])
            .current_dir(workspace_root)
            .output()?
    };
    let rendered_stdout = String::from_utf8_lossy(&output.stdout);
    let rendered_stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "xtask dogfood must be green on the live workspace (baseline-gated); stdout: {rendered_stdout} stderr: {rendered_stderr}"
    );

    // a09 coverage: the summary line must report a visibly NONZERO
    // dispatched-file count. A hollow self-scan would already have
    // exited non-zero upstream; this assertion additionally pins the
    // rendered count into the test log.
    let summary = rendered_stdout
        .lines()
        .find(|line| line.starts_with("rust-rule scan:"))
        .ok_or_else(|| std::io::Error::other("xtask dogfood printed no scan summary line"))?;
    assert!(
        !summary.starts_with("rust-rule scan: 0 file(s)"),
        "self-scan must dispatch a nonzero number of files, got: {summary}"
    );
    Ok(())
}
