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

use std::path::{Path, PathBuf};

use enforcer_install::hooks::pretooluse::{classify, run_enforcer_check, HookDecision};

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
fn run_fixture(
    name: &str,
) -> Result<enforcer_install::hooks::pretooluse::CheckOutcome, Box<dyn std::error::Error>> {
    let src = fixture_root().join(name).join("candidate.rs");
    let temp = tempfile::tempdir()?;
    let dest = temp.path().join("candidate.rs");
    std::fs::copy(&src, &dest)?;
    Ok(run_enforcer_check(
        &enforcer_binary()?,
        temp.path(),
        Path::new("candidate.rs"),
    ))
}

/// The seeded T1 violation: a first-party `.unwrap()` fires
/// `T1-RUSTERR.1` (`Severity::Error`) -- the hook MUST deny, and the
/// denial reason must carry the exact `RuleId` plus its `Fix:` hint.
#[test]
fn seeded_violating_edit_denies_with_exact_rule_id_and_fix(
) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = run_fixture("violating")?;
    assert_eq!(outcome.exit_code, Some(1), "stdout was: {}", outcome.stdout);
    let decision = classify(&outcome);
    assert!(decision.is_deny(), "expected Deny, got {decision:?}");
    let reason = decision.reason().ok_or("deny must carry a reason")?;
    assert!(
        reason.contains("T1-RUSTERR.1"),
        "reason must name the exact RuleId, got: {reason}"
    );
    assert!(
        reason.contains("Fix:"),
        "reason must carry the Fix: hint, got: {reason}"
    );
    Ok(())
}

/// A conforming edit (typed `Result` propagation, no banned methods) must
/// allow -- no deny, no warning.
#[test]
fn conforming_edit_allows() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = run_fixture("conforming")?;
    assert_eq!(outcome.exit_code, Some(0), "stdout was: {}", outcome.stdout);
    let decision = classify(&outcome);
    assert_eq!(decision, HookDecision::Allow);
    Ok(())
}

/// A T2-only finding (`LIT-1.1`, `Severity::Warning`, no T1 violation)
/// must allow WITH a warning surfaced -- and must NEVER deny.
#[test]
fn t2_only_finding_allows_with_warning_never_denies() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = run_fixture("t2_only")?;
    let decision = classify(&outcome);
    assert!(
        !decision.is_deny(),
        "a T2-only finding must never deny, got {decision:?}"
    );
    assert!(
        decision.is_allow_with_warning(),
        "expected AllowWithWarning, got {decision:?}"
    );
    let reason = decision.reason().ok_or("warning must carry a reason")?;
    assert!(reason.contains("LIT-1.1"), "got: {reason}");
    Ok(())
}
