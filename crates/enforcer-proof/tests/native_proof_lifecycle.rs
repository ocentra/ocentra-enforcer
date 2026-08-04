//! Native lifecycle persistence regressions: durable runs, reopen, and
//! fail-closed duplicate/path handling.

use enforcer_core::error::Result;
use enforcer_domain::proof_types::{ProofId, ProofRunId};
use enforcer_proof::boundary::lifecycle::NativeProofLifecycle;
use enforcer_proof::boundary::proof_query::{ProofInventoryQuery, ProofStatusQuery};
use enforcer_proof::harness::RunProofArgs;

fn args(root: std::path::PathBuf, run: &str, command: Vec<String>) -> Result<RunProofArgs> {
    Ok(RunProofArgs {
        proof_id: ProofId::try_from("native.lifecycle".to_owned())?,
        root,
        run_id: ProofRunId::try_from(run.to_owned())?,
        command,
        capability: None,
        claims_proved: vec![],
        claims_not_proved: vec![],
        pin: false,
    })
}

#[test]
fn success_manual_reopen_and_duplicate_runs_are_durable_and_fail_closed() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().canonicalize()?;
    let lifecycle = NativeProofLifecycle::open(&root)?;
    let command = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    assert!(
        lifecycle
            .run(&args(root.clone(), "native-run", command)?, None)?
            .ok
    );
    assert_eq!(lifecycle.snapshot()?.runs.len(), 1);
    assert!(
        NativeProofLifecycle::open(&root)?
            .snapshot()?
            .journal
            .record_count
            >= 2
    );
    let duplicate_effect = root.join("duplicate-command-ran.txt");
    let duplicate = if cfg!(windows) {
        vec![
            "cmd".to_owned(),
            "/C".to_owned(),
            format!("echo repeated>\"{}\"", duplicate_effect.display()),
        ]
    } else {
        vec![
            "sh".to_owned(),
            "-c".to_owned(),
            format!("printf repeated > '{}'", duplicate_effect.display()),
        ]
    };
    assert!(matches!(
        lifecycle.run(&args(root.clone(), "native-run", duplicate)?, None),
        Err(enforcer_core::error::Error::InvalidConfig(message)) if message == "duplicate proof run id"
    ));
    assert!(!duplicate_effect.exists());
    let manual = lifecycle.run(&args(root, "native-manual", vec![])?, None)?;
    assert!(!manual.ok);
    Ok(())
}

#[test]
fn malformed_and_escaping_artifact_inputs_fail_closed() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let lifecycle = NativeProofLifecycle::open(fixture.path())?;
    let run: ProofRunId = "missing-run".parse()?;
    let escaped = enforcer_domain::paths::RelPath::try_from("../outside".to_owned())
        .err()
        .ok_or_else(|| {
            enforcer_core::error::Error::InvalidConfig(
                "escaping relative path must be rejected".to_owned(),
            )
        })?;
    assert_eq!(escaped.path, "relPath");
    assert_eq!(
        escaped.reason,
        "invalid relative path: `..` segment escapes the repository root"
    );
    let safe: enforcer_domain::paths::RelPath = "evidence.txt".parse()?;
    assert!(matches!(
        lifecycle.read_declared_artifact(&run, &safe),
        Err(enforcer_core::error::Error::Io(_))
    ));
    Ok(())
}

#[test]
fn lifecycle_rejects_a_redirected_proof_state_root_before_writing() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join(".enforce"))?;
    let link = fixture.path().join(".enforce/proofs");
    make_directory_symlink(outside.path(), &link)?;

    assert!(matches!(
        NativeProofLifecycle::open(fixture.path()),
        Err(enforcer_core::error::Error::InvalidConfig(message))
            if message == "proof state path must not be a symlink or reparse point"
    ));
    assert!(std::fs::read_dir(outside.path())?.next().is_none());
    Ok(())
}

#[cfg(windows)]
fn make_directory_symlink(source: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(source, link)
}

#[cfg(not(windows))]
fn make_directory_symlink(source: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[test]
fn legacy_import_copies_evidence_into_lifecycle_owned_storage() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().canonicalize()?;
    let source = root.join("legacy/report.json");
    std::fs::create_dir_all(source.parent().ok_or_else(|| {
        enforcer_core::error::Error::InvalidConfig("legacy fixture has no parent".to_owned())
    })?)?;
    let expected = br#"{"status":"passed","claimsProved":["legacy survives"]}"#;
    std::fs::write(&source, expected)?;

    let lifecycle = NativeProofLifecycle::open(&root)?;
    let proof_id: ProofId = "native.legacy".parse()?;
    let run_id: ProofRunId = "legacy-custody".parse()?;
    let run = lifecycle.import_legacy(&proof_id, &run_id, &["legacy"])?;
    assert_eq!(run.artifacts.len(), 1);
    let owned = root.join(run.artifacts[0].path.as_str());
    assert_eq!(std::fs::read(&owned)?, expected);

    std::fs::remove_file(source)?;
    assert_eq!(std::fs::read(&owned)?, expected);
    let reopened = NativeProofLifecycle::open(&root)?;
    let status = reopened.status(&ProofStatusQuery {
        proof_id: Some(proof_id),
        status: None,
        limit: 1,
    })?;
    assert_eq!(status.runs[0].artifacts, run.artifacts);
    Ok(())
}

#[test]
fn reset_removes_runs_but_preserves_completed_audit_records() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().canonicalize()?;
    let lifecycle = NativeProofLifecycle::open(&root)?;
    let command = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    lifecycle.run(&args(root.clone(), "reset-me", command)?, None)?;
    lifecycle.reset()?;

    let reopened = NativeProofLifecycle::open(&root)?;
    assert!(reopened.snapshot()?.runs.is_empty());
    let journal = std::fs::read_to_string(root.join(".enforce/proofs/journal.ndjson"))?;
    let events = journal
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(
        events
            .iter()
            .rev()
            .take(2)
            .filter_map(|event| event["record"]["eventType"].as_str())
            .collect::<Vec<_>>(),
        ["proof-reset-finished", "proof-reset-started"]
    );
    Ok(())
}

#[test]
fn persisted_status_is_filtered_sorted_and_bounded_without_a_project_snapshot() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().canonicalize()?;
    let lifecycle = NativeProofLifecycle::open(&root)?;
    let command = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    lifecycle.run(&args(root.clone(), "query-a", command)?, None)?;
    let command = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    lifecycle.run(&args(root, "query-b", command)?, None)?;
    let response = lifecycle.status(&ProofStatusQuery {
        proof_id: Some(ProofId::try_from("native.lifecycle".to_owned())?),
        status: None,
        limit: 1,
    })?;
    assert_eq!(response.runs.len(), 1);
    assert_eq!(response.runs[0].proof_id.as_str(), "native.lifecycle");
    Ok(())
}

#[test]
fn inventory_is_safe_and_optional_rows_are_bounded() -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let scripts = fixture.path().join("scripts/test");
    std::fs::create_dir_all(&scripts)?;
    std::fs::write(scripts.join("one-proof.mjs"), "spawn('x');")?;
    std::fs::write(scripts.join("two.mjs"), "writeFile('proof.md', 'x');")?;
    let lifecycle = NativeProofLifecycle::open(fixture.path())?;
    let hidden = lifecycle.inventory(&ProofInventoryQuery {
        include_scripts: false,
        limit: 1,
    })?;
    assert_eq!(hidden.totals.scripts, 2);
    assert!(hidden.scripts.is_empty());
    let bounded = lifecycle.inventory(&ProofInventoryQuery {
        include_scripts: true,
        limit: 1,
    })?;
    assert_eq!(bounded.scripts.len(), 1);
    assert_eq!(bounded.omitted_script_count, 1);
    Ok(())
}
