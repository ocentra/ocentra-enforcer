//! The arc-22 proof row: spawn the REAL compiled `enforcer` binary (not an
//! in-process call) and run it end-to-end -- a check/scan on fail/pass
//! fixture trees across all three scope modes with the expected exit
//! codes, a usage-error collision case, the "no override flag" assertion,
//! and a `serve` mode responding to `initialize` over stdio. Mirrors
//! `crates/enforcer-mcp/tests/stdio_smoke.rs`'s pattern of spawning the
//! real binary rather than faking the process boundary.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

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

fn write_pass_fixture(root: &std::path::Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("lib.rs"), "fn good() -> i32 { 42 }\n")
}

fn write_fail_fixture(root: &std::path::Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("lib.rs"),
        "fn bad() { let x: Option<i32> = None; x.unwrap(); }\n",
    )
}

fn run_check(
    root: &std::path::Path,
    extra_args: &[&str],
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let mut cmd = Command::new(binary);
    cmd.current_dir(root).arg("check").args(extra_args);
    Ok(cmd.status()?)
}

#[test]
fn pass_fixture_paths_mode_exits_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &["src/lib.rs"])?;
    assert!(status.success(), "clean tree via <paths...> must exit 0");
    Ok(())
}

#[test]
fn fail_fixture_paths_mode_exits_non_zero_with_violations_class(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let status = run_check(temp.path(), &["src/lib.rs"])?;
    assert_eq!(
        status.code(),
        Some(1),
        "a rule violation must exit with the Violations class (1), not a generic non-zero"
    );
    Ok(())
}

#[test]
fn pass_fixture_all_mode_exits_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &["--all"])?;
    assert!(status.success(), "clean tree via --all must exit 0");
    Ok(())
}

#[test]
fn fail_fixture_all_mode_exits_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let status = run_check(temp.path(), &["--all"])?;
    assert_eq!(status.code(), Some(1));
    Ok(())
}

#[test]
fn base_head_mode_parses_and_routes() -> Result<(), Box<dyn std::error::Error>> {
    // `enforcer-scan` does not yet restrict the walked file set by git
    // diff range (see `commands::resolve_files` docs) -- this asserts the
    // mode PARSES AND ROUTES (reaches the engine, produces a Report, exits
    // one of the two documented exit classes) rather than asserting a
    // specific finding count, since true diff-narrowing is a later pack's
    // gap, not this skeleton's.
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &["--base", "HEAD", "--head", "HEAD"])?;
    let code = status.code().ok_or("process terminated by signal")?;
    assert!(
        code == 0 || code == 1,
        "base/head mode must resolve to a normal verdict exit class, got {code}"
    );
    Ok(())
}

#[test]
fn base_head_and_paths_collision_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(
        temp.path(),
        &["--base", "HEAD", "--head", "HEAD", "src/lib.rs"],
    )?;
    assert_eq!(
        status.code(),
        Some(2),
        "a --base/--head + <paths...> collision must be the UsageError class (2)"
    );
    Ok(())
}

#[test]
fn no_scope_at_all_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &[])?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[test]
fn there_is_no_override_flag_on_the_real_binary() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    for bogus in [
        "--force",
        "--no-verify",
        "--skip",
        "--bypass",
        "--ignore-findings",
    ] {
        let status = run_check(temp.path(), &["--all", bogus])?;
        assert_eq!(
            status.code(),
            Some(2),
            "unknown flag `{bogus}` must be a clap usage error, and specifically it must \
             never suppress the fail fixture's violation exit class"
        );
    }
    Ok(())
}

#[test]
fn help_never_advertises_a_bypass_flag() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let output = Command::new(&binary).args(["check", "--help"]).output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for bad_word in ["--force", "--no-verify", "--skip-checks", "--bypass"] {
        assert!(
            !text.contains(bad_word),
            "check --help must never advertise `{bad_word}`"
        );
    }
    Ok(())
}

/// Send one NDJSON request and read exactly one NDJSON reply line -- the
/// same pattern `enforcer-mcp`'s own stdio smoke test uses against its
/// throwaway smoke binary, run here against the PRODUCTION `enforcer
/// serve` mode.
fn round_trip(
    stdin: &mut impl Write,
    stdout: &mut impl BufRead,
    request: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let line = format!("{request}\n");
    stdin.write_all(line.as_bytes())?;
    stdin.flush()?;
    let mut reply_line = String::new();
    stdout.read_line(&mut reply_line)?;
    Ok(serde_json::from_str(reply_line.trim_end())?)
}

#[cfg(feature = "full")]
#[test]
fn serve_mode_responds_to_initialize_over_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let mut child = Command::new(binary)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);

    let reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05" },
        }),
    )?;
    assert_eq!(
        reply["result"]["serverInfo"]["name"],
        serde_json::json!(enforcer_mcp::name::SERVER_NAME),
        "enforcer serve must speak the same MCP server identity as enforcer-mcp::sink"
    );

    drop(stdin);
    let status = child.wait()?;
    assert!(status.success(), "enforcer serve must exit 0 on stdin EOF");
    Ok(())
}

#[test]
fn advise_literals_parses_and_routes() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["advise", "literals"])
        .status()?;
    let code = status.code().ok_or("process terminated by signal")?;
    assert!(
        code == 0 || code == 1,
        "advise literals must reach a normal verdict, got {code}"
    );
    Ok(())
}

#[test]
fn advise_unknown_target_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["advise", "somethingElse"])
        .status()?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[test]
fn verify_mode_bogus_is_a_usage_error_not_a_finding_exit_code(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["verify", "--all", "--mode", "bogus"])
        .status()?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[test]
fn architecture_bare_without_check_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["architecture"])
        .status()?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}
