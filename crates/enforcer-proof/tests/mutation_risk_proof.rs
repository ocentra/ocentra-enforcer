//! MJS-compatible mutation-risk proof acceptance and fail-closed negatives.

use std::path::Path;
use std::process::Command;

use enforcer_proof::boundary::mutation_risk::{
    validate, MutationRiskProofRejection, MutationRiskProofValidation, MUTATION_RISK_PROOF_ID,
};

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new("git").args(args).current_dir(root).output()?)
}

fn git_fixture() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join("scripts"))?;
    std::fs::write(
        fixture.path().join("scripts/ci-local.mjs"),
        "console.log('ci-local');\n",
    )?;
    assert!(git(fixture.path(), &["init", "--quiet"])?.status.success());
    assert!(git(fixture.path(), &["add", "."])?.status.success());
    assert!(git(
        fixture.path(),
        &[
            "-c",
            "user.name=Enforcer Test",
            "-c",
            "user.email=enforcer-test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "proof fixture",
        ],
    )?
    .status
    .success());
    let output = git(fixture.path(), &["rev-parse", "HEAD"])?;
    assert!(output.status.success());
    let commit = String::from_utf8(output.stdout)?.trim().to_owned();
    Ok((fixture, commit))
}

fn valid_run(commit: &str) -> serde_json::Value {
    serde_json::json!({
        "runId": "mutation-risk-run",
        "proofId": MUTATION_RISK_PROOF_ID,
        "status": "passed",
        "git": {"commit": commit},
        "command": ["node.exe", "scripts/ci-local.mjs"]
    })
}

fn write_manifest_and_run(
    root: &Path,
    manifest: serde_json::Value,
    run: Option<&[u8]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = root.join(".enforce/proofs/db/proof-manifest.json");
    std::fs::create_dir_all(
        manifest_path
            .parent()
            .ok_or("manifest path has no parent")?,
    )?;
    std::fs::write(manifest_path, serde_json::to_vec(&manifest)?)?;
    if let Some(run) = run {
        let run_path = root.join(".enforce/proofs/runs/mutation-risk-run/proof-run.json");
        std::fs::create_dir_all(run_path.parent().ok_or("run path has no parent")?)?;
        std::fs::write(run_path, run)?;
    }
    Ok(())
}

fn rejection(result: MutationRiskProofValidation) -> Option<MutationRiskProofRejection> {
    match result {
        MutationRiskProofValidation::Rejected { reason } => Some(reason),
        MutationRiskProofValidation::Accepted { .. } => None,
    }
}

#[test]
fn exact_current_commit_canonical_command_is_accepted_without_writes(
) -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, commit) = git_fixture()?;
    let run = serde_json::to_vec(&valid_run(&commit))?;
    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({
            "schemaVersion": 1,
            "runs": [{"runId": "mutation-risk-run"}]
        }),
        Some(&run),
    )?;
    let manifest_before = std::fs::read(
        fixture
            .path()
            .join(".enforce/proofs/db/proof-manifest.json"),
    )?;
    let run_before = std::fs::read(
        fixture
            .path()
            .join(".enforce/proofs/runs/mutation-risk-run/proof-run.json"),
    )?;

    let result = validate(fixture.path(), Some(&commit));
    assert!(result.is_accepted());
    assert_eq!(
        std::fs::read(
            fixture
                .path()
                .join(".enforce/proofs/db/proof-manifest.json",)
        )?,
        manifest_before
    );
    assert_eq!(
        std::fs::read(
            fixture
                .path()
                .join(".enforce/proofs/runs/mutation-risk-run/proof-run.json",)
        )?,
        run_before
    );
    Ok(())
}

#[test]
fn missing_and_malformed_manifest_states_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, _) = git_fixture()?;
    assert!(matches!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::ManifestMissing(_))
    ));

    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 2, "runs": []}),
        None,
    )?;
    assert_eq!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::ManifestSchema)
    );

    std::fs::write(
        fixture
            .path()
            .join(".enforce/proofs/db/proof-manifest.json"),
        b"not-json",
    )?;
    assert!(matches!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::ManifestMalformed(_))
    ));
    Ok(())
}

#[test]
fn missing_malformed_duplicate_and_escaping_runs_fail_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, commit) = git_fixture()?;
    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 1, "runs": [{"runId": "mutation-risk-run"}]}),
        None,
    )?;
    assert_eq!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::RunMissing(
            "mutation-risk-run".to_owned()
        ))
    );

    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 1, "runs": [{"runId": "mutation-risk-run"}]}),
        Some(b"not-json"),
    )?;
    assert!(matches!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::RunMalformed(_))
    ));

    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 1, "runs": [
            {"runId": "mutation-risk-run"}, {"runId": "mutation-risk-run"}
        ]}),
        Some(&serde_json::to_vec(&valid_run(&commit))?),
    )?;
    assert_eq!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::DuplicateRunId(
            "mutation-risk-run".to_owned()
        ))
    );

    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 1, "runs": [{"runId": "../escape"}]}),
        None,
    )?;
    assert_eq!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::RunIdMalformed(
            "../escape".to_owned()
        ))
    );
    Ok(())
}

#[test]
fn accepted_first_run_does_not_hide_a_later_duplicate() -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, commit) = git_fixture()?;
    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 1, "runs": [
            {"runId": "mutation-risk-run"}, {"runId": "mutation-risk-run"}
        ]}),
        Some(&serde_json::to_vec(&valid_run(&commit))?),
    )?;
    assert_eq!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::DuplicateRunId(
            "mutation-risk-run".to_owned()
        ))
    );
    Ok(())
}

#[test]
fn accepted_first_run_does_not_hide_a_later_malformed_run() -> Result<(), Box<dyn std::error::Error>>
{
    let (fixture, commit) = git_fixture()?;
    let manifest_path = fixture
        .path()
        .join(".enforce/proofs/db/proof-manifest.json");
    std::fs::create_dir_all(manifest_path.parent().ok_or("manifest parent")?)?;
    std::fs::write(
        &manifest_path,
        serde_json::to_vec(&serde_json::json!({"schemaVersion": 1, "runs": [
            {"runId": "mutation-risk-run"}, {"runId": "later-run"}
        ]}))?,
    )?;
    let first = fixture
        .path()
        .join(".enforce/proofs/runs/mutation-risk-run/proof-run.json");
    std::fs::create_dir_all(first.parent().ok_or("first run parent")?)?;
    std::fs::write(first, serde_json::to_vec(&valid_run(&commit))?)?;
    let later = fixture
        .path()
        .join(".enforce/proofs/runs/later-run/proof-run.json");
    std::fs::create_dir_all(later.parent().ok_or("later run parent")?)?;
    std::fs::write(later, b"not-json")?;

    assert!(matches!(
        rejection(validate(fixture.path(), None)),
        Some(MutationRiskProofRejection::RunMalformed(_))
    ));
    Ok(())
}

#[test]
fn wrong_id_status_commit_and_command_states_are_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    let (fixture, commit) = git_fixture()?;
    let cases: [(&str, fn(&mut serde_json::Value)); 6] = [
        ("wrong proof id", |run: &mut serde_json::Value| {
            run["proofId"] = serde_json::json!("OTHER")
        }),
        ("failed status", |run: &mut serde_json::Value| {
            run["status"] = serde_json::json!("failed")
        }),
        ("stale commit", |run: &mut serde_json::Value| {
            run["git"]["commit"] = serde_json::json!("0".repeat(40))
        }),
        ("wrong command", |run: &mut serde_json::Value| {
            run["command"] = serde_json::json!(["node.exe", "scripts/not-ci-local.mjs"])
        }),
        ("extra command argument", |run: &mut serde_json::Value| {
            run["command"] = serde_json::json!(["node.exe", "scripts/ci-local.mjs", "--extra"])
        }),
        ("wrong executable", |run: &mut serde_json::Value| {
            run["command"] = serde_json::json!(["npm", "scripts/ci-local.mjs"])
        }),
    ];
    for (label, mutate) in cases {
        let mut run = valid_run(&commit);
        mutate(&mut run);
        write_manifest_and_run(
            fixture.path(),
            serde_json::json!({"schemaVersion": 1, "runs": [{"runId": "mutation-risk-run"}]}),
            Some(&serde_json::to_vec(&run)?),
        )?;
        assert!(!validate(fixture.path(), None).is_accepted(), "{label}");
    }
    Ok(())
}

#[test]
fn explicit_head_sha_is_the_commit_authority_not_a_branch_label(
) -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, commit) = git_fixture()?;
    write_manifest_and_run(
        fixture.path(),
        serde_json::json!({"schemaVersion": 1, "runs": [{"runId": "mutation-risk-run"}]}),
        Some(&serde_json::to_vec(&valid_run(&commit))?),
    )?;
    assert!(validate(fixture.path(), Some(&commit)).is_accepted());
    assert!(!validate(fixture.path(), Some("not-a-branch-or-sha")).is_accepted());
    Ok(())
}
