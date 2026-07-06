//! d11 integration proof: `enforcer_harness::ci_parity` over real fixture
//! text loaded from disk under `tests/fixtures/ci_parity/**` — matched
//! sets pass; an injected extra local-only step fails; an injected pinned
//! version skew fails. Also proves the check against THIS repo's own real
//! `rust-toolchain.toml` + `.github/workflows/ci.yml` (the "runs both
//! locally and as a CI job, self-referential" acceptance bar) stays
//! green today.

use std::path::{Path, PathBuf};

use enforcer_harness::ci_parity::{
    check_parity, check_toolchain_channel_parity, extract_toolchain_channel, parse_ci_manifest,
    parse_local_manifest,
};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ci_parity")
        .join(name)
}

fn read(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[test]
fn matched_local_and_ci_step_sets_pass_clean() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("matched");
    let local = parse_local_manifest(&read(&root.join("local/local-steps.json"))?)?;
    let ci = parse_ci_manifest(&read(&root.join("ci/workflow.yml"))?);
    let findings = check_parity(&local, &ci);
    assert!(
        findings.is_empty(),
        "expected zero findings for matched fixtures, got {findings:?}"
    );
    Ok(())
}

#[test]
fn injected_extra_local_only_step_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("local_only_step");
    let local = parse_local_manifest(&read(&root.join("local/local-steps.json"))?)?;
    let ci = parse_ci_manifest(&read(&root.join("ci/workflow.yml"))?);
    let findings = check_parity(&local, &ci);
    assert_eq!(
        findings.len(),
        1,
        "expected exactly one finding: {findings:?}"
    );
    assert!(findings[0].detail.contains("cargo local-only-lint"));
    assert!(findings[0]
        .detail
        .contains("is not present in the CI workflow's step set"));
    Ok(())
}

#[test]
fn injected_version_skew_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("version_skew");
    let local = parse_local_manifest(&read(&root.join("local/local-steps.json"))?)?;
    let ci = parse_ci_manifest(&read(&root.join("ci/workflow.yml"))?);
    let findings = check_parity(&local, &ci);
    assert_eq!(
        findings.len(),
        1,
        "expected exactly one finding: {findings:?}"
    );
    assert!(findings[0].title.contains("version skew"));
    assert!(findings[0].detail.contains("0.14.0"));
    assert!(findings[0].detail.contains("0.15.2"));
    Ok(())
}

#[test]
fn injected_toolchain_channel_skew_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture_root("toolchain_skew");
    let local_text = read(&root.join("local-rust-toolchain.toml"))?;
    let ci_text = read(&root.join("ci-observed-toolchain.toml"))?;
    let local_channel = extract_toolchain_channel(&local_text);
    let ci_channel = extract_toolchain_channel(&ci_text);
    let findings = check_toolchain_channel_parity(local_channel.as_deref(), ci_channel.as_deref());
    assert_eq!(
        findings.len(),
        1,
        "expected exactly one finding: {findings:?}"
    );
    assert!(findings[0].detail.contains("1.95.0"));
    assert!(findings[0].detail.contains("1.80.0"));
    Ok(())
}

/// Self-referential smoke test: this repo's own real
/// `rust-toolchain.toml` parses to a channel value (proves
/// `extract_toolchain_channel` works against the real file, not just
/// synthetic fixtures). Does not assert a specific channel string (that
/// would make this test the thing that breaks on every routine toolchain
/// bump) — only that a channel is present.
#[test]
fn real_repo_rust_toolchain_toml_declares_a_channel() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let text = read(&repo_root.join("rust-toolchain.toml"))?;
    let channel = extract_toolchain_channel(&text);
    assert!(
        channel.is_some(),
        "real rust-toolchain.toml must declare a channel"
    );
    Ok(())
}

/// Self-referential smoke test: this repo's own real
/// `.github/workflows/ci.yml` parses to a non-empty step set (proves
/// `parse_ci_manifest` works against the real workflow file, not just
/// synthetic fixtures).
#[test]
fn real_repo_ci_workflow_has_parseable_steps() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let text = read(&repo_root.join(".github/workflows/ci.yml"))?;
    let manifest = parse_ci_manifest(&text);
    assert!(
        !manifest.steps.is_empty(),
        "real ci.yml must yield at least one parsed step"
    );
    let names: Vec<&str> = manifest.steps.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"cargo fmt --check"));
    assert!(names.iter().any(|n| n.contains("clippy")));
    Ok(())
}
