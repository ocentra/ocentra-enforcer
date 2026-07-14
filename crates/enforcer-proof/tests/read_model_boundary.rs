use enforcer_proof::envelope::{ArtifactRecord, GitState, ProofRun, ProofStatus};
use enforcer_proof::read_model::read_project_proof_snapshot;

#[test]
fn snapshot_saturates_declared_artifact_bytes_from_a_run_record(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let run_directory = root.path().join(".enforce/proofs/runs/overflow");
    std::fs::create_dir_all(&run_directory)?;
    let run = ProofRun {
        schema_version: 1,
        proof_id: "READ-MODEL-OVERFLOW".to_owned(),
        run_id: "overflow".to_owned(),
        title: "Read model overflow boundary".to_owned(),
        capability: "local".to_owned(),
        git: GitState::default(),
        status: ProofStatus::Passed,
        exit_code: Some(0),
        started_at: "2026-07-14T00:00:00Z".to_owned(),
        ended_at: "2026-07-14T00:00:01Z".to_owned(),
        command: Vec::new(),
        diagnostic_count: 0,
        pinned: false,
        artifacts: vec![
            ArtifactRecord {
                name: "largest.bin".to_owned(),
                path: "largest.bin".to_owned(),
                sha256: format!("sha256:{}", "0".repeat(64)).parse()?,
                byte_length: u64::MAX,
            },
            ArtifactRecord {
                name: "one-more.bin".to_owned(),
                path: "one-more.bin".to_owned(),
                sha256: format!("sha256:{}", "0".repeat(64)).parse()?,
                byte_length: 1,
            },
        ],
        claims_proved: Vec::new(),
        claims_not_proved: Vec::new(),
    };
    std::fs::write(
        run_directory.join("proof-run.json"),
        serde_json::to_vec(&run)?,
    )?;

    let snapshot = read_project_proof_snapshot(root.path())?;
    assert_eq!(snapshot.runs.len(), 1);
    assert_eq!(snapshot.runs[0].artifacts.declared, 2);
    assert_eq!(snapshot.runs[0].artifacts.total_bytes, u64::MAX);
    Ok(())
}

#[test]
fn snapshot_rejects_an_absolute_artifact_path() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let outside = tempfile::NamedTempFile::new()?;
    let directory = root.path().join(".enforce/proofs/runs/boundary");
    std::fs::create_dir_all(&directory)?;
    let run = ProofRun {
        schema_version: 1, proof_id: "PATH".to_owned(), run_id: "boundary".to_owned(), title: "Path".to_owned(), capability: "local".to_owned(), git: GitState::default(), status: ProofStatus::Passed, exit_code: Some(0), started_at: "2026-07-14T00:00:00Z".to_owned(), ended_at: "2026-07-14T00:00:01Z".to_owned(), command: Vec::new(), diagnostic_count: 0, pinned: false,
        artifacts: vec![ArtifactRecord { name: "outside".to_owned(), path: outside.path().to_string_lossy().into_owned(), sha256: format!("sha256:{}", "0".repeat(64)).parse()?, byte_length: 1 }], claims_proved: Vec::new(), claims_not_proved: Vec::new(),
    };
    std::fs::write(directory.join("proof-run.json"), serde_json::to_vec(&run)?)?;
    let snapshot = read_project_proof_snapshot(root.path())?;
    assert_eq!(snapshot.runs[0].artifacts.present, 0);
    assert_eq!(snapshot.runs[0].artifacts.missing, 1);
    Ok(())
}
