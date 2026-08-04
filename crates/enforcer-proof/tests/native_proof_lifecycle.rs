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
    let duplicate = if cfg!(windows) {
        vec!["cmd".to_owned(), "/C".to_owned(), "exit 0".to_owned()]
    } else {
        vec!["true".to_owned()]
    };
    assert!(matches!(
        lifecycle.run(&args(root.clone(), "native-run", duplicate)?, None),
        Err(enforcer_core::error::Error::InvalidConfig(message)) if message == "duplicate proof run id"
    ));
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
