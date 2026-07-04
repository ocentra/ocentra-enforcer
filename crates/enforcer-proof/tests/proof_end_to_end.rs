//! End-to-end integration fixtures for `enforcer-proof`, exercising several
//! modules together the way a real caller would: run a proof, capture its
//! artifact, gate a claim against it, and separately prove the
//! hash-chained journal fails closed under tamper.
//!
//! This is the crate's top-level pass/fail fixture pairing named in the
//! workpack's acceptance row: "fresh artifact -> claim GREEN; stale/missing
//! artifact -> claim fails; ... an intact journal verifies on open, and a
//! tampered/reordered record makes verify fail closed."

use enforcer_core::error::Result;
use enforcer_core::redaction::Redactor;
use enforcer_proof::claim::{claim_proof, ClaimArgs};
use enforcer_proof::envelope::{git_state, ArtifactRecord, GitState, ProofStatus};
use enforcer_proof::harness::{run_proof, ProofDefinition, RunProofArgs};
use enforcer_proof::journal::{JournalRecord, ProofJournal, JOURNAL_SCHEMA_VERSION};

fn temp_root(name: &str) -> Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "enforcer-proof-e2e-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn definition() -> ProofDefinition {
    ProofDefinition {
        id: "E2E-PROOF".to_owned(),
        title: "End-to-end demo proof".to_owned(),
        family: "command".to_owned(),
        severity: "error".to_owned(),
        applies_to: vec!["workspace".to_owned()],
        triggers: vec![],
        languages: vec![],
        capabilities: vec!["local".to_owned()],
        collector: "command".to_owned(),
        docs: vec![],
        commands: vec![],
        required_artifacts: vec![],
        required_paths: vec![],
        required_for_pr_ready: true,
        claims_proved: vec!["This proof runs a trivial command.".to_owned()],
        claims_not_proved: vec![],
        ci_support: true,
        device_support: false,
    }
}

/// PASS: a run whose artifact still exists on disk, gated with a matching
/// commit and no dirty-worktree conflict, claims GREEN with zero
/// violations.
#[test]
fn fresh_artifact_claims_green() -> Result<()> {
    let root = temp_root("fresh")?;
    let definition = definition();
    let command = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    let args = RunProofArgs {
        proof_id: definition.id.clone(),
        root: root.clone(),
        run_id: "run-fresh".to_owned(),
        command,
        claims_proved: definition.claims_proved.clone(),
        ..Default::default()
    };
    let outcome = run_proof(&args, Some(&definition))?;
    assert!(outcome.ok);
    assert_eq!(outcome.proof_run.status, ProofStatus::Passed);

    // Write the artifact the claim will check for.
    let artifact_path = root.join("artifact.txt");
    std::fs::write(&artifact_path, "evidence")?;
    let mut run = outcome.proof_run;
    run.artifacts.push(ArtifactRecord {
        name: "artifact.txt".to_owned(),
        path: "artifact.txt".to_owned(),
        sha256: enforcer_core::hash_chain::link_digest(None, b"evidence")
            .parse()
            .map_err(enforcer_core::error::Error::Decode)?,
        byte_length: 8,
    });

    let current_git = GitState {
        commit: run.git.commit.clone(),
        branch: run.git.branch.clone(),
        dirty: Some(false),
    };
    let claim_args = ClaimArgs {
        claim_id: "claim-fresh".to_owned(),
        pr_ready: true,
        allow_dirty: false,
        proof_ids: vec![definition.id.clone()],
        current_git,
        latest_run: &|_| Some(run.clone()),
        definition: &|_| Some(definition.clone()),
        artifact_exists: &|path| root.join(path).exists(),
        required_path_exists: &|_| true,
    };
    let claim = claim_proof(&claim_args);
    assert!(
        claim.ok(),
        "fresh artifact + matching commit must claim GREEN: {:?}",
        claim.violations
    );

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// FAIL: the SAME run, but its artifact has since been deleted from disk —
/// the claim must fail with `missing-artifact`, never silently pass.
#[test]
fn stale_missing_artifact_claim_fails() -> Result<()> {
    let root = temp_root("stale")?;
    let definition = definition();
    let command = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    let args = RunProofArgs {
        proof_id: definition.id.clone(),
        root: root.clone(),
        run_id: "run-stale".to_owned(),
        command,
        ..Default::default()
    };
    let outcome = run_proof(&args, Some(&definition))?;
    let mut run = outcome.proof_run;
    // Record an artifact that is never actually written to disk (simulates
    // a deleted/missing artifact at claim time).
    run.artifacts.push(ArtifactRecord {
        name: "gone.txt".to_owned(),
        path: "gone.txt".to_owned(),
        sha256: enforcer_core::hash_chain::link_digest(None, b"evidence")
            .parse()
            .map_err(enforcer_core::error::Error::Decode)?,
        byte_length: 8,
    });

    let current_git = GitState {
        commit: run.git.commit.clone(),
        branch: run.git.branch.clone(),
        dirty: Some(false),
    };
    let claim_args = ClaimArgs {
        claim_id: "claim-stale".to_owned(),
        pr_ready: true,
        allow_dirty: false,
        proof_ids: vec![definition.id.clone()],
        current_git,
        latest_run: &|_| Some(run.clone()),
        definition: &|_| Some(definition.clone()),
        artifact_exists: &|path| root.join(path).exists(),
        required_path_exists: &|_| true,
    };
    let claim = claim_proof(&claim_args);
    assert!(
        !claim.ok(),
        "a claim over a missing artifact must fail closed"
    );

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// FAIL: a claim for a proof id with no run at all must fail closed with
/// `missing-proof-run`, never silently pass as if it were fresh.
#[test]
fn missing_run_claim_fails() {
    let claim_args = ClaimArgs {
        claim_id: "claim-missing".to_owned(),
        pr_ready: false,
        allow_dirty: false,
        proof_ids: vec!["NEVER-RUN".to_owned()],
        current_git: GitState::default(),
        latest_run: &|_| None,
        definition: &|_| None,
        artifact_exists: &|_| true,
        required_path_exists: &|_| true,
    };
    let claim = claim_proof(&claim_args);
    assert!(!claim.ok());
}

/// PASS: an intact hash-chained journal verifies on open AND on replay.
#[test]
fn intact_journal_verifies_on_open_and_replay() -> Result<()> {
    let root = temp_root("journal-intact")?;
    let redactor = Redactor::with_defaults()?;
    let journal_path = root.join("journal.ndjson");
    {
        let mut journal = ProofJournal::open(&journal_path)?;
        journal.append(
            &redactor,
            JournalRecord {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: "proof-started".to_owned(),
                run_id: "run-journal".to_owned(),
                proof_id: "E2E-PROOF".to_owned(),
                timestamp: "2026-07-04T00:00:00Z".to_owned(),
                payload: serde_json::json!({}),
            },
        )?;
        journal.append(
            &redactor,
            JournalRecord {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: "proof-finished".to_owned(),
                run_id: "run-journal".to_owned(),
                proof_id: "E2E-PROOF".to_owned(),
                timestamp: "2026-07-04T00:00:01Z".to_owned(),
                payload: serde_json::json!({ "status": "passed" }),
            },
        )?;
    }
    let journal = ProofJournal::open(&journal_path)?;
    assert_eq!(journal.verify_on_replay()?, 2);
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// FAIL: a tampered/reordered journal must fail closed on BOTH open and
/// replay — the core tamper-evidence contract this crate adds over the
/// legacy `.mjs` proof system.
#[test]
fn tampered_journal_fails_closed_on_open_and_replay() -> Result<()> {
    let root = temp_root("journal-tampered")?;
    let redactor = Redactor::with_defaults()?;
    let journal_path = root.join("journal.ndjson");
    let journal = {
        let mut journal = ProofJournal::open(&journal_path)?;
        journal.append(
            &redactor,
            JournalRecord {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: "proof-started".to_owned(),
                run_id: "run-tamper".to_owned(),
                proof_id: "E2E-PROOF".to_owned(),
                timestamp: "2026-07-04T00:00:00Z".to_owned(),
                payload: serde_json::json!({}),
            },
        )?;
        journal.append(
            &redactor,
            JournalRecord {
                schema_version: JOURNAL_SCHEMA_VERSION,
                event_type: "proof-finished".to_owned(),
                run_id: "run-tamper".to_owned(),
                proof_id: "E2E-PROOF".to_owned(),
                timestamp: "2026-07-04T00:00:01Z".to_owned(),
                payload: serde_json::json!({ "status": "passed" }),
            },
        )?;
        journal
    };

    // Tamper: reorder the two lines at rest.
    let content = std::fs::read_to_string(&journal_path)?;
    let mut lines: Vec<&str> = content.lines().collect();
    lines.reverse();
    std::fs::write(&journal_path, format!("{}\n", lines.join("\n")))?;

    assert!(
        ProofJournal::open(&journal_path).is_err(),
        "reordered journal must fail to open"
    );
    assert!(
        journal.verify_on_replay().is_err(),
        "reordered journal must fail replay"
    );

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// Sanity check that this crate's `git_state` helper is reachable from an
/// integration test (used implicitly by `run_proof`; exercised here
/// directly against this crate's own worktree root so the test suite does
/// not depend on an external fixture repo).
#[test]
fn git_state_is_queryable_without_erroring() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let state = git_state(&cwd);
    // No assertion on values (CI/sandbox git availability varies); the
    // contract under test is that this never panics/errors regardless of
    // whether `.git` or `git` itself is present.
    let _ = state;
    Ok(())
}
