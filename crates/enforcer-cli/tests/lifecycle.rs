//! d06 proof row: `cargo test -p enforcer-cli` (`lifecycle-commands`).
//!
//! Proves, at the `enforcer-cli` crate boundary (not spawning the real
//! binary -- `plan|implement|check|fix|review` are not yet wired into
//! `crate::cli::Command`/`main.rs`'s clap dispatch, which is arc-22's
//! surface, not this workpack's `owns:` set; see `src/lifecycle.rs`'s
//! module docs "Integration seam" section):
//!
//! 1. Each of the five phase functions routes to a REAL oracle
//!    computation, never a stubbed prose pass.
//! 2. A failing oracle forces a non-`Success` exit-code class from
//!    [`enforcer_cli::lifecycle::run_check`]/[`enforcer_cli::lifecycle::run_review`]
//!    against real fail/pass fixture trees on disk.
//! 3. `review` blocks when proof rows are missing.
//!
//! `plan`/`implement`/`fix` are proven fail-closed elsewhere (unit tests
//! in `src/lifecycle.rs` and `src/lifecycle/oracle.rs`) since there is no
//! fixture tree that could ever flip them to a pass on this branch (their
//! oracles are unconditional `NotYetWired` until d07/d10 land) -- this
//! integration file focuses on the two phases with real fixture-tree
//! behavior: `check` and `review`.

use std::path::Path;

use enforcer_cli::lifecycle::{run_check, run_review, CheckScope, ExitCodeShim, ReviewRequest};
use enforcer_proof::envelope::{GitState, ProofRun, ProofStatus};
use enforcer_proof::harness::ProofDefinition;

fn write_pass_fixture(root: &Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("lib.rs"), "fn good() -> i32 { 42 }\n")
}

fn write_fail_fixture(root: &Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("lib.rs"),
        "fn bad() { let x: Option<i32> = None; x.unwrap(); }\n",
    )
}

/// Process-global mutex guarding `std::env::set_current_dir`.
/// `cargo test` runs `#[test]` fns on a thread pool by default, and the
/// process cwd is a single global -- without serializing every
/// cwd-mutating test in this file, two tests can interleave their
/// chdir/restore pairs and each corrupt the other's expected root
/// (observed as a "restore cwd" `NotFound` when a sibling test's tempdir
/// vanished mid-flight). `tests/cli_integration.rs` avoids this
/// altogether by spawning a separate OS process per case
/// (`Command::new(binary)`); this in-process suite instead pins one lock
/// around the whole chdir/run/restore sequence.
static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `run_check` with the process's current directory pinned at `root`
/// for the duration of the call, serialized against every other caller of
/// this helper in the same test binary (see [`CWD_LOCK`]).
fn run_check_in(
    root: &Path,
    scope: &CheckScope,
) -> Result<enforcer_cli::lifecycle::PhaseOutcome, Box<dyn std::error::Error>> {
    let _guard = CWD_LOCK
        .lock()
        .map_err(|poisoned| format!("cwd lock poisoned: {poisoned}"))?;
    let original = std::env::current_dir()?;
    std::env::set_current_dir(root)?;
    let outcome = run_check(scope);
    std::env::set_current_dir(original)?;
    Ok(outcome)
}

#[test]
fn check_phase_passes_on_a_clean_fixture_tree() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let outcome = run_check_in(
        temp.path(),
        &CheckScope {
            paths: vec![std::path::PathBuf::from("src/lib.rs")],
        },
    )?;
    assert_eq!(
        outcome.exit_code,
        ExitCodeShim::Success,
        "clean fixture tree must pass the check phase oracle, got {:?}",
        outcome.verdict
    );
    Ok(())
}

#[test]
fn check_phase_reports_violations_class_on_a_fail_fixture_tree(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let outcome = run_check_in(
        temp.path(),
        &CheckScope {
            paths: vec![std::path::PathBuf::from("src/lib.rs")],
        },
    )?;
    assert_eq!(
        outcome.exit_code,
        ExitCodeShim::Violations,
        "a phase reports success while its oracle returns fail -> this must not happen; \
         a fail fixture must force a non-success exit, got {:?}",
        outcome.verdict
    );
    assert!(!outcome.verdict.is_pass());
    Ok(())
}

#[test]
fn check_phase_all_scope_also_reports_violations_on_fail_fixture(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let outcome = run_check_in(temp.path(), &CheckScope { paths: vec![] })?;
    assert_eq!(outcome.exit_code, ExitCodeShim::Violations);
    Ok(())
}

#[test]
fn review_without_proof_rows_is_blocked() {
    let outcome = run_review(&ReviewRequest {
        proof_ids: vec!["d06-lifecycle".to_owned()],
        current_git: GitState::default(),
        latest_run: &|_| None,
        definition: &|_| None,
        artifact_exists: &|_| true,
        required_path_exists: &|_| true,
    });
    assert_ne!(
        outcome.exit_code,
        ExitCodeShim::Success,
        "review w/o proof rows -> non-zero, got {:?}",
        outcome.verdict
    );
}

#[test]
fn review_with_a_passed_clean_proof_row_passes() {
    let run = ProofRun {
        schema_version: 1,
        proof_id: "d06-lifecycle".to_owned(),
        run_id: "run-1".to_owned(),
        title: "d06 lifecycle proof".to_owned(),
        capability: "local".to_owned(),
        git: GitState {
            commit: Some("deadbeef".to_owned()),
            branch: Some("lane/d06".to_owned()),
            dirty: Some(false),
        },
        status: ProofStatus::Passed,
        exit_code: Some(0),
        started_at: "2026-07-04T00:00:00Z".to_owned(),
        ended_at: "2026-07-04T00:00:01Z".to_owned(),
        command: vec![],
        diagnostic_count: 0,
        pinned: false,
        artifacts: vec![],
        claims_proved: vec![],
        claims_not_proved: vec![],
    };
    let definition = ProofDefinition {
        id: "d06-lifecycle".to_owned(),
        title: "d06 lifecycle proof".to_owned(),
        family: "command".to_owned(),
        severity: "error".to_owned(),
        applies_to: vec![],
        triggers: vec![],
        languages: vec![],
        capabilities: vec!["local".to_owned()],
        collector: "command".to_owned(),
        docs: vec![],
        commands: vec![],
        required_artifacts: vec![],
        required_paths: vec![],
        required_for_pr_ready: true,
        claims_proved: vec![],
        claims_not_proved: vec![],
        ci_support: true,
        device_support: false,
    };
    let outcome = run_review(&ReviewRequest {
        proof_ids: vec!["d06-lifecycle".to_owned()],
        current_git: GitState {
            commit: Some("deadbeef".to_owned()),
            branch: Some("lane/d06".to_owned()),
            dirty: Some(false),
        },
        latest_run: &move |_| Some(run.clone()),
        definition: &move |_| Some(definition.clone()),
        artifact_exists: &|_| true,
        required_path_exists: &|_| true,
    });
    assert_eq!(
        outcome.exit_code,
        ExitCodeShim::Success,
        "a green proof row + matching commit must pass, got {:?}",
        outcome.verdict
    );
}

#[test]
fn plan_implement_fix_phases_never_report_success_with_no_landed_oracle() {
    // [G] "no phase can report success unless its oracle returns a pass
    // Finding set; there is no prose-only pass path" -- proven for the
    // three phases whose owning workpacks (arc-20/none/d07) have not
    // landed a Rust oracle on this branch yet.
    assert_ne!(
        enforcer_cli::lifecycle::run_plan().exit_code,
        ExitCodeShim::Success
    );
    assert_ne!(
        enforcer_cli::lifecycle::run_implement().exit_code,
        ExitCodeShim::Success
    );
    assert_ne!(
        enforcer_cli::lifecycle::run_fix().exit_code,
        ExitCodeShim::Success
    );
}
