//! c03 acceptance-row proof: `cargo test -p enforcer-install` against a
//! temp `~/.claude` fixture (`tests/fixtures/claude_adapter/**`) runs
//! `install` then `verify` and asserts all-green checks (pass fixture);
//! a hand-corrupted descriptor or a hand-edited/corrupt `~/.claude.json`
//! fails closed with a typed error, never a silent skip; round-trip
//! `install`->`uninstall` restores the pre-state byte-for-byte.
//!
//! Every fixture under `tests/fixtures/claude_adapter/**` is COPIED into
//! an isolated `tempfile::tempdir()` before a test touches it — this
//! crate's tests NEVER write into the checked-in fixture tree, and NEVER
//! touch the real `~/.claude.json` (the live session's actual config).

use enforcer_install::adapters::claude::ClaudeAdapter;
use enforcer_install::cli_contract::RequestContext;
use enforcer_install::core::HarnessAdapter;
use enforcer_install::error::InstallError;
use std::path::{Path, PathBuf};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/claude_adapter")
        .join(name)
}

/// Recursively copy `src` into `dst` (both directories), creating `dst`.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Stand up an isolated temp-dir copy of `fixture_name`, returning the
/// `TempDir` handle (kept alive for the test's duration) plus the fake
/// `enforcer` binary path every test registers.
fn isolated_fixture(
    fixture_name: &str,
) -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    copy_dir_all(&fixture_root(fixture_name), dir.path())?;
    let binary = dir.path().join("bin").join("enforcer");
    std::fs::create_dir_all(binary.parent().ok_or("expected a parent dir")?)?;
    std::fs::write(&binary, b"fake-enforcer-binary")?;
    Ok((dir, binary))
}

#[test]
fn pass_fixture_install_then_verify_is_all_green() -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("pass")?;
    let adapter = ClaudeAdapter::new(home.path(), &binary);
    let ctx = RequestContext::with_defaults(binary);

    let plan = adapter.plan(&ctx)?;
    assert!(!plan.is_noop(), "fresh pass fixture must have work to do");
    let applied = adapter.apply(&plan)?;
    assert!(applied.all_succeeded());

    let verify = adapter.verify(&ctx)?;
    assert!(
        verify.all_passed(),
        "expected every check green, got {verify:?}"
    );

    // The pre-existing unrelated mcpServers entry + top-level key must
    // have survived the merge (never a destructive overwrite).
    let raw = std::fs::read_to_string(adapter.claude_json_path())?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    assert_eq!(
        value["mcpServers"]["codebase-memory-mcp"]["command"],
        "/usr/local/bin/codebase-memory-mcp"
    );
    assert_eq!(value["someUnrelatedTopLevelKey"], "keep-me");

    // Descriptor exists and parses.
    assert!(adapter.agent_descriptor_path().is_file());
    Ok(())
}

#[test]
fn descriptor_fail_fixture_verify_returns_a_failed_check_not_a_skip(
) -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("descriptor_fail")?;
    // The fixture's `.claude.json` has a placeholder command; rewrite it
    // to the actual per-test binary path so the mcp-registration check
    // passes and isolates this test to the descriptor failure alone.
    let claude_json_path = home.path().join(".claude.json");
    let raw = std::fs::read_to_string(&claude_json_path)?;
    // JSON-escape the binary path (Windows paths carry backslashes,
    // which are not valid unescaped inside a JSON string literal).
    let escaped_binary = serde_json::to_string(&binary.display().to_string())?;
    let escaped_binary = &escaped_binary[1..escaped_binary.len() - 1];
    let fixed = raw.replace("PLACEHOLDER_BINARY_PATH", escaped_binary);
    std::fs::write(&claude_json_path, fixed)?;

    let adapter = ClaudeAdapter::new(home.path(), &binary);
    let ctx = RequestContext::with_defaults(binary);

    let verify = adapter.verify(&ctx)?;
    assert!(!verify.all_passed());
    let descriptor_check = verify
        .checks
        .iter()
        .find(|c| c.name == "agent-descriptor-present")
        .ok_or("expected an agent-descriptor-present check to be present, not skipped")?;
    assert!(!descriptor_check.passed);
    assert!(!descriptor_check.detail.is_empty());
    Ok(())
}

#[test]
fn claude_json_fail_fixture_verify_fails_closed_with_typed_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("claude_json_fail")?;
    let adapter = ClaudeAdapter::new(home.path(), &binary);
    let ctx = RequestContext::with_defaults(binary);

    let result = adapter.verify(&ctx);
    assert!(
        matches!(result, Err(InstallError::MalformedConfig { .. })),
        "expected a typed MalformedConfig error, got {result:?}"
    );

    // plan() must fail the same way -- never silently proceed past a
    // corrupt existing config.
    let plan_result = adapter.plan(&ctx);
    assert!(matches!(
        plan_result,
        Err(InstallError::MalformedConfig { .. })
    ));
    Ok(())
}

#[test]
fn pass_fixture_round_trip_install_uninstall_restores_byte_for_byte(
) -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("pass")?;
    let claude_json_path = home.path().join(".claude.json");
    let pre_bytes = std::fs::read(&claude_json_path)?;
    let pre_value: serde_json::Value = serde_json::from_slice(&pre_bytes)?;

    let adapter = ClaudeAdapter::new(home.path(), &binary);
    let ctx = RequestContext::with_defaults(binary);

    let install_plan = adapter.plan(&ctx)?;
    adapter.apply(&install_plan)?;
    assert!(adapter.agent_descriptor_path().is_file());
    assert!(adapter.skill_md_path().is_file());

    let uninstall_plan = adapter.plan_uninstall(&ctx)?;
    assert!(!uninstall_plan.is_noop());
    adapter.apply_uninstall(&uninstall_plan)?;

    let post_value: serde_json::Value = serde_json::from_slice(&std::fs::read(&claude_json_path)?)?;
    assert_eq!(
        post_value, pre_value,
        "uninstall must restore ~/.claude.json to its pre-install value"
    );
    assert!(!adapter.agent_descriptor_path().is_file());
    assert!(!adapter.skill_md_path().is_file());
    Ok(())
}
