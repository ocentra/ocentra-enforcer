use enforcer_proof::boundary::read_model::read_project_proof_snapshot;

#[test]
fn snapshot_saturates_declared_artifact_bytes_from_a_run_record(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let run_directory = root.path().join(".enforce/proofs/runs/overflow");
    std::fs::create_dir_all(&run_directory)?;
    let run = serde_json::json!({
        "schemaVersion": 1, "proofId": "READ-MODEL-OVERFLOW", "runId": "overflow",
        "title": "Read model overflow boundary", "capability": "local", "status": "passed",
        "startedAt": "2026-07-14T00:00:00Z", "endedAt": "2026-07-14T00:00:01Z",
        "git": {}, "exitCode": 0, "command": [], "diagnosticCount": 0, "pinned": false,
        "claimsProved": [], "claimsNotProved": [], "artifacts": [
            {"name":"largest.bin","path":"largest.bin","sha256":format!("sha256:{}", "0".repeat(64)),"byteLength":u64::MAX},
            {"name":"one-more.bin","path":"one-more.bin","sha256":format!("sha256:{}", "0".repeat(64)),"byteLength":1}
        ]
    });
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
    let directory = root.path().join(".enforce/proofs/runs/boundary");
    std::fs::create_dir_all(&directory)?;
    let run = serde_json::json!({
        "schemaVersion":1, "proofId":"PATH", "runId":"boundary", "title":"Path",
        "capability":"local", "status":"passed", "startedAt":"2026-07-14T00:00:00Z",
        "endedAt":"2026-07-14T00:00:01Z", "git":{}, "exitCode":0, "command":[],
        "diagnosticCount":0, "pinned":false, "claimsProved":[], "claimsNotProved":[],
        "artifacts":[{"name":"outside","path":"/outside","sha256":format!("sha256:{}", "0".repeat(64)),"byteLength":1}]
    });
    std::fs::write(directory.join("proof-run.json"), serde_json::to_vec(&run)?)?;
    let snapshot = read_project_proof_snapshot(root.path())?;
    assert!(snapshot.runs[0].proof_run.is_none());
    assert_eq!(
        snapshot.runs[0].parse_error.as_deref(),
        Some(
            "decode/validation failed at `relPath`: invalid relative path: must be relative (no leading separator or drive letter) at line 1 column 64"
        )
    );
    Ok(())
}
