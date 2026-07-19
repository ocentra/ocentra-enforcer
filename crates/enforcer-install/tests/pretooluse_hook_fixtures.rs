//! Workpack c04 acceptance proof (`claude-deny-hook-blocks` in
//! TEST_PROOF_EXPECTATIONS.md): exercises
//! [`enforcer_install::hooks::pretooluse::run_enforcer_check`] against the
//! REAL, built `enforcer` binary over the seeded fixtures under
//! `tests/fixtures/pretooluse_hook/**`, then classifies the captured
//! outcome through [`enforcer_install::hooks::pretooluse::classify`] --
//! proving the emitter's decision logic against the actual subprocess
//! contract the emitted Claude hook shells out to, not a simulated one.
//!
//! Cargo's `CARGO_BIN_EXE_<name>` env var is only set for a crate's OWN
//! `[[bin]]` targets, never for a dev-dependency's binary -- `enforcer` is
//! `enforcer-cli`'s bin, and `enforcer-cli` is only a dev-dependency here,
//! so [`enforcer_binary`] instead locates it relative to this test
//! executable's own path in the shared workspace `target/` dir (both
//! crates' outputs land in the same profile dir).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use enforcer_domain::install_types::{HookCheckOutcome, HookDecision};
use enforcer_install::hooks::pretooluse::{classify, run_enforcer_check};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pretooluse_hook")
}

/// Locate the built `enforcer` binary produced by the `enforcer-cli`
/// crate. Cargo's `CARGO_BIN_EXE_<name>` env var is ONLY set for a crate's
/// OWN `[[bin]]` targets, never for a dev-dependency's binary (confirmed
/// empirically: `enforcer-cli` is a dev-dependency of THIS crate, but the
/// var is still `NotPresent` here) -- so this resolves the binary the way
/// `cargo test` itself lays out the shared workspace `target/` dir: the
/// test executable always lives at `<target-profile-dir>/deps/<test>.exe`,
/// and every workspace crate's `[[bin]]` output lands one level up at
/// `<target-profile-dir>/enforcer<EXE_SUFFIX>` (or `.exe` on Windows).
fn enforcer_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let test_exe = std::env::current_exe()?;
    let deps_dir = test_exe
        .parent()
        .ok_or("test executable must have a parent dir")?;
    let profile_dir = deps_dir
        .parent()
        .ok_or("deps dir must have a parent (the profile dir)")?;
    let candidate = profile_dir.join(format!("enforcer{}", std::env::consts::EXE_SUFFIX));
    if !candidate.is_file() {
        return Err(format!(
            "expected the enforcer-cli binary at `{}` (built as a workspace dependency of this \
             crate's dev-dependency on enforcer-cli); run `cargo build -p enforcer-cli` first \
             if this fails",
            candidate.display()
        )
        .into());
    }
    Ok(candidate)
}

/// Copy fixture `name`'s `candidate.rs` into a fresh temp dir and run the
/// real `enforcer check` binary against it with that dir as cwd -- matching
/// exactly how the emitted PreToolUse hook invokes `enforcer` against the
/// repo root Claude is editing.
fn run_fixture(name: &str) -> Result<HookCheckOutcome, Box<dyn std::error::Error>> {
    let src = fixture_root().join(name).join("candidate.rs");
    let temp = tempfile::tempdir()?;
    let dest = temp.path().join("candidate.rs");
    std::fs::copy(&src, &dest)?;
    Ok(run_enforcer_check(
        &enforcer_binary()?,
        temp.path(),
        Path::new("candidate.rs"),
    )?)
}

/// Invoke the exact command emitted into Claude's `PreToolUse` settings and
/// send it the pending `Write` payload on stdin. This is the process-boundary
/// proof that the hook validates proposed content, rather than only checking
/// the old on-disk file.
fn run_emitted_hook(
    name: &str,
) -> Result<(std::process::ExitStatus, serde_json::Value), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(fixture_root().join(name).join("candidate.rs"))?;
    let temp = tempfile::tempdir()?;
    let target = temp.path().join("candidate.rs");
    let mut child = Command::new(enforcer_binary()?)
        .args(["hook", "pretooluse"])
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("hook child stdin unavailable")?;
    let payload = serde_json::json!({
        "cwd": temp.path().display().to_string(),
        "tool_name": "Write",
        "tool_input": {
            "file_path": target.display().to_string(),
            "content": source,
        },
    });
    stdin.write_all(payload.to_string().as_bytes())?;
    drop(stdin);
    let output = child.wait_with_output()?;
    let reply = serde_json::from_slice(&output.stdout)?;
    Ok((output.status, reply))
}

#[test]
fn emitted_hook_denies_violating_proposed_write_before_it_exists(
) -> Result<(), Box<dyn std::error::Error>> {
    let (status, reply) = run_emitted_hook("violating")?;
    assert_eq!(status.code(), Some(1));
    assert_eq!(reply["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(reply["hookSpecificOutput"]["permissionDecision"], "deny");
    let reason = reply["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .ok_or("deny reason missing")?;
    assert!(reason.contains("T1-RUSTERR.1"), "got: {reason}");
    assert!(reason.contains("Fix:"), "got: {reason}");
    Ok(())
}

#[test]
fn emitted_hook_allows_conforming_proposed_write_before_it_exists(
) -> Result<(), Box<dyn std::error::Error>> {
    let (status, reply) = run_emitted_hook("conforming")?;
    assert!(status.success());
    assert_eq!(reply["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(reply["hookSpecificOutput"]["permissionDecision"], "allow");
    Ok(())
}

/// The seeded T1 violation: a first-party `.unwrap()` fires
/// `T1-RUSTERR.1` (`Severity::Error`) -- the hook MUST deny, and the
/// denial reason must carry the exact `RuleId` plus its `Fix:` hint.
#[test]
fn seeded_violating_edit_denies_with_exact_rule_id_and_fix(
) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = run_fixture("violating")?;
    let failure_code = std::num::NonZeroI32::new(1).ok_or("fixture exit code must be non-zero")?;
    assert_eq!(
        outcome.exit_status,
        enforcer_domain::install_types::HookExitStatus::Failure(failure_code),
        "stdout was: {}",
        outcome.stdout.as_str()
    );
    let decision = classify(&outcome)?;
    let HookDecision::Deny { reason } = decision else {
        return Err("expected Deny".into());
    };
    assert!(
        reason.as_str().contains("T1-RUSTERR.1"),
        "reason must name the exact RuleId, got: {}",
        reason.as_str()
    );
    assert!(
        reason.as_str().contains("Fix:"),
        "reason must carry the Fix: hint, got: {}",
        reason.as_str()
    );
    Ok(())
}

/// A conforming edit (typed `Result` propagation, no banned methods) must
/// allow -- no deny, no warning.
#[test]
fn conforming_edit_allows() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = run_fixture("conforming")?;
    assert_eq!(
        outcome.exit_status,
        enforcer_domain::install_types::HookExitStatus::Success,
        "stdout was: {}",
        outcome.stdout.as_str()
    );
    let decision = classify(&outcome)?;
    assert_eq!(decision, HookDecision::Allow);
    Ok(())
}

/// A T2-only finding (`LIT-1.1`, `Severity::Warning`, no T1 violation)
/// must allow WITH a warning surfaced -- and must NEVER deny.
#[test]
fn t2_only_finding_allows_with_warning_never_denies() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = run_fixture("t2_only")?;
    let decision = classify(&outcome)?;
    let HookDecision::AllowWithWarning { reason } = decision else {
        return Err("expected AllowWithWarning".into());
    };
    assert!(
        reason.as_str().contains("LIT-1.1"),
        "got: {}",
        reason.as_str()
    );
    Ok(())
}
