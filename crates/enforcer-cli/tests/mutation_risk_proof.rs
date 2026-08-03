//! Real CLI proof-awareness boundary tests.

use std::path::{Path, PathBuf};
use std::process::Command;

fn binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let mut directory = exe
        .parent()
        .ok_or("test binary has no parent directory")?
        .to_path_buf();
    if directory.ends_with("deps") {
        directory.pop();
    }
    let binary = directory.join(if cfg!(windows) {
        "enforcer.exe"
    } else {
        "enforcer"
    });
    if binary.exists() {
        Ok(binary)
    } else {
        Err(format!("enforcer binary not found at {}", binary.display()).into())
    }
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new("git").args(args).current_dir(root).output()?)
}

fn git_fixture() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    std::fs::create_dir_all(fixture.path().join("scripts"))?;
    std::fs::create_dir_all(fixture.path().join("docs"))?;
    std::fs::write(fixture.path().join("Cargo.lock"), "version = 4\n")?;
    std::fs::write(
        fixture.path().join("scripts/ci-local.mjs"),
        "console.log('ci-local');\n",
    )?;
    std::fs::write(fixture.path().join("docs/readme.md"), "# docs\n")?;
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
            "cli proof fixture",
        ],
    )?
    .status
    .success());
    let output = git(fixture.path(), &["rev-parse", "HEAD"])?;
    assert!(output.status.success());
    Ok((fixture, String::from_utf8(output.stdout)?.trim().to_owned()))
}

fn write_proof(root: &Path, run: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = root.join(".enforce/proofs/db/proof-manifest.json");
    let run_path = root.join(".enforce/proofs/runs/mutation-risk-run/proof-run.json");
    std::fs::create_dir_all(manifest.parent().ok_or("manifest parent")?)?;
    std::fs::create_dir_all(run_path.parent().ok_or("run parent")?)?;
    std::fs::write(
        manifest,
        serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 1,
            "runs": [{"runId": "mutation-risk-run"}]
        }))?,
    )?;
    std::fs::write(run_path, serde_json::to_vec(&run)?)?;
    Ok(())
}

fn valid_run(commit: &str) -> serde_json::Value {
    serde_json::json!({
        "runId": "mutation-risk-run",
        "proofId": "PROOF-MUTATION-RISK-CI",
        "status": "passed",
        "git": {"commit": commit},
        "command": ["node.exe", "scripts/ci-local.mjs"]
    })
}

fn run_mutation_risk(
    binary: &Path,
    root: &Path,
    path: &str,
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    Ok(Command::new(binary)
        .current_dir(root)
        .args(["policy", "mutation-risk", path])
        .status()?)
}

#[test]
fn native_cli_requires_current_canonical_proof_and_keeps_noncritical_clean(
) -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, commit) = git_fixture()?;
    let binary = binary_path()?;

    assert_eq!(
        run_mutation_risk(&binary, fixture.path(), "Cargo.lock")?.code(),
        Some(1),
        "critical path without proof must remain ENF-2.1"
    );

    write_proof(fixture.path(), valid_run(&commit))?;
    assert!(
        run_mutation_risk(&binary, fixture.path(), "Cargo.lock")?.success(),
        "current commit and canonical command must clear the native gate"
    );

    let mut stale = valid_run(&commit);
    stale["git"]["commit"] = serde_json::json!("0".repeat(40));
    write_proof(fixture.path(), stale)?;
    assert_eq!(
        run_mutation_risk(&binary, fixture.path(), "Cargo.lock")?.code(),
        Some(1)
    );

    let mut failed = valid_run(&commit);
    failed["status"] = serde_json::json!("failed");
    write_proof(fixture.path(), failed)?;
    assert_eq!(
        run_mutation_risk(&binary, fixture.path(), "Cargo.lock")?.code(),
        Some(1)
    );

    let mut wrong_command = valid_run(&commit);
    wrong_command["command"] = serde_json::json!(["npm", "scripts/ci-local.mjs"]);
    write_proof(fixture.path(), wrong_command)?;
    assert_eq!(
        run_mutation_risk(&binary, fixture.path(), "Cargo.lock")?.code(),
        Some(1)
    );

    assert!(
        run_mutation_risk(&binary, fixture.path(), "docs/readme.md")?.success(),
        "ordinary noncritical paths remain clean without an accepted proof"
    );
    Ok(())
}
