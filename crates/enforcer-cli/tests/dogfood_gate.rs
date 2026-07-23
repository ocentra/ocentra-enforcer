//! z01 integration proof: the terminal composing dogfood-proof gate runs
//! against the LIVE workspace and records a durable PASS manifest.
//!
//! Invokes `xtask dogfood-gate --no-toolchain` (the z01 gate composes
//! a10's dogfood loop + the e01 literal-scan floor against its committed
//! T2 ceiling + the b02 PLAN-* structure report -- see
//! `xtask/src/dogfood_gate.rs`) and asserts, per the z01 acceptance row
//! (`dogfood-self-zero-violations`):
//! - the run completes and exits zero on the clean (baseline-covered,
//!   ceiling-covered) workspace;
//! - the manifest records a nonzero ran-count (a09 coverage -- a hollow
//!   zero-ran self-scan would have failed upstream);
//! - the manifest records the PASS verdict, zero NEW rust-rule
//!   violations, a well-formed ruleset fingerprint, and per-family
//!   counts;
//! - the `enforcer-proof` hash-chained journal received a tamper-evident
//!   record (every line carries its chain digest).
//!
//! The seeded-self-violation fail-fixture (the gate BITING) is proven at
//! the xtask unit level (`xtask/src/dogfood_gate/boundary.rs::tests::
//! seeded_self_violation_fails_the_gate` plants an unbaselined unwrap()
//! in a fixture repo and asserts the FAIL verdict) -- planting a
//! violation in the LIVE tree from an integration test would race every
//! other concurrently-running test over shared source files.
//!
//! The test executes the already-built `xtask` binary directly instead of
//! invoking `cargo run` from inside the workspace `cargo test` process. This
//! avoids cross-platform Cargo target-directory lock contention while still
//! exercising the real terminal gate. `--no-toolchain` skips xtask's nested
//! `cargo fmt`/`clippy`/`deny`/`audit` subprocesses.

use std::process::Command;

#[test]
fn dogfood_gate_passes_live_workspace_and_emits_manifest_and_journal() -> Result<(), std::io::Error>
{
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| {
            std::io::Error::other("expected crates/enforcer-cli two levels under the root")
        })?;
    let proof_output = tempfile::tempdir()?;
    // Cargo places integration-test binaries under `target/<profile>/deps`.
    // The sibling `xtask` executable is built by the same workspace test
    // command, so execute it directly rather than starting nested Cargo.
    let test_binary = std::env::current_exe()?;
    let profile_dir = test_binary
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| std::io::Error::other("test binary is not under target/<profile>/deps"))?;
    let xtask_binary = profile_dir.join(format!("xtask{}", std::env::consts::EXE_SUFFIX));
    if !xtask_binary.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("workspace xtask binary was not built: {}", xtask_binary.display()),
        ));
    }
    let output = Command::new(xtask_binary)
        .args([
            "dogfood-gate",
            "--proof-output-dir",
            proof_output
                .path()
                .to_str()
                .ok_or_else(|| std::io::Error::other("temporary proof path was not UTF-8"))?,
            "--no-toolchain",
        ])
        .current_dir(workspace_root)
        .output()?;
    let rendered_stdout = String::from_utf8_lossy(&output.stdout);
    let rendered_stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "xtask dogfood-gate must PASS on the live workspace; stdout: {rendered_stdout} stderr: {rendered_stderr}"
    );

    // The manifest is the durable proof artifact.
    let manifest_raw = std::fs::read(proof_output.path().join("dogfood-manifest.json"))?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_raw).map_err(std::io::Error::other)?;

    assert_eq!(
        manifest.get("verdict").and_then(serde_json::Value::as_str),
        Some("pass"),
        "the live workspace must record a PASS verdict, got: {manifest}"
    );
    let ran_count = manifest
        .get("ranCount")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| std::io::Error::other("manifest.ranCount missing or not a number"))?;
    assert!(
        ran_count > 0,
        "a09 coverage: the manifest must record a nonzero ran-count"
    );
    let fingerprint = manifest
        .get("rulesetFingerprint")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| std::io::Error::other("manifest.rulesetFingerprint missing"))?;
    assert!(
        fingerprint.starts_with("sha256:") && fingerprint.len() == "sha256:".len() + 64,
        "ruleset fingerprint must be a branded sha256 digest, got: {fingerprint}"
    );
    let families = manifest
        .get("familyCounts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("manifest.familyCounts missing"))?;
    assert!(
        !families.is_empty(),
        "the manifest must carry per-family finding counts"
    );
    let new_violations = families
        .iter()
        .find(|row| {
            row.get("family").and_then(serde_json::Value::as_str)
                == Some("rust-rules-new-violations")
        })
        .and_then(|row| row.get("count"))
        .and_then(serde_json::Value::as_u64);
    assert_eq!(
        new_violations,
        Some(0),
        "dogfood-self-zero-violations: the PASS manifest must record zero NEW violations"
    );

    // The hash-chained journal received a tamper-evident record: every
    // line carries its chain digest, and at least one line exists.
    let journal = std::fs::read_to_string(proof_output.path().join("dogfood-journal.ndjson"))?;
    let lines: Vec<&str> = journal
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert!(
        !lines.is_empty(),
        "the proof journal must contain at least one dogfood-gate record"
    );
    for line in &lines {
        let record: serde_json::Value =
            serde_json::from_slice(line.as_bytes()).map_err(std::io::Error::other)?;
        let digest = record
            .get("digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| std::io::Error::other("journal line missing its hash-chain digest"))?;
        assert!(
            digest.starts_with("sha256:"),
            "journal digest must be a branded sha256 link, got: {digest}"
        );
    }
    Ok(())
}
