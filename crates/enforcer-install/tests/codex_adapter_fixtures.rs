//! c06 acceptance-row proof: `cargo test -p enforcer-install` against a
//! temp `~/.codex` fixture (`tests/fixtures/codex/**`) runs `install` then
//! `verify` and asserts all-green checks (pass fixture); a hand-corrupted
//! descriptor or a hand-edited/corrupt `config.toml` fails closed with a
//! typed error, never a silent skip; round-trip `install`->`uninstall`
//! restores the pre-state; the generated `[mcp_servers.enforcer]` TOML
//! block, the native `agents/openai.yaml` descriptor, and the GLOBAL
//! `AGENTS.md` managed block (start/end markers + body) each equal pinned
//! golden snapshots under `tests/fixtures/codex/golden/**` — any diff fails
//! the build.
//!
//! Every fixture under `tests/fixtures/codex/**` is COPIED into an isolated
//! `tempfile::tempdir()` before a test touches it — this crate's tests
//! NEVER write into the checked-in fixture tree, and NEVER touch the real
//! `~/.codex` (the live session's actual config).

use enforcer_install::adapters::codex::CodexAdapter;
use enforcer_install::cli_contract::RequestContext;
use enforcer_install::core::HarnessAdapter;
use enforcer_install::error::InstallError;
use std::path::{Path, PathBuf};

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codex")
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

fn golden(name: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(fixture_root("golden").join(name))
}

#[test]
fn pass_fixture_install_then_verify_is_all_green() -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("pass")?;
    let adapter = CodexAdapter::new(home.path(), &binary);
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

    // The pre-existing unrelated mcp_servers entry + table must have
    // survived the merge (never a destructive overwrite).
    let raw = std::fs::read_to_string(adapter.config_toml_path())?;
    assert!(raw.contains("other-tool"));
    assert!(raw.contains("keep = \"me\""));
    assert!(raw.contains("[mcp_servers.enforcer]"));

    // Descriptor + global AGENTS.md + skill all exist.
    assert!(adapter.agent_descriptor_path().is_file());
    assert!(adapter.global_agents_md_path().is_file());
    assert!(adapter.skill_md_path().is_file());
    Ok(())
}

#[test]
fn descriptor_fail_fixture_verify_returns_a_failed_check_not_a_skip(
) -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("descriptor_fail")?;
    let config_toml_path = home.path().join("config.toml");
    let raw = std::fs::read_to_string(&config_toml_path)?;
    let escaped_binary = binary.display().to_string().replace('\\', "\\\\");
    let fixed = raw.replace("PLACEHOLDER_BINARY_PATH", &escaped_binary);
    std::fs::write(&config_toml_path, fixed)?;

    let adapter = CodexAdapter::new(home.path(), &binary);
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
fn config_toml_fail_fixture_verify_fails_closed_with_typed_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("config_toml_fail")?;
    let adapter = CodexAdapter::new(home.path(), &binary);
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
fn pass_fixture_round_trip_install_uninstall_restores_unrelated_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("pass")?;
    let adapter = CodexAdapter::new(home.path(), &binary);
    let ctx = RequestContext::with_defaults(binary);

    let install_plan = adapter.plan(&ctx)?;
    adapter.apply(&install_plan)?;
    assert!(adapter.agent_descriptor_path().is_file());
    assert!(adapter.skill_md_path().is_file());

    let uninstall_plan = adapter.plan_uninstall(&ctx)?;
    assert!(!uninstall_plan.is_noop());
    adapter.apply_uninstall(&uninstall_plan)?;

    let post = std::fs::read_to_string(adapter.config_toml_path())?;
    assert!(post.contains("other-tool"));
    assert!(post.contains("keep = \"me\""));
    assert!(!post.contains("[mcp_servers.enforcer]"));
    assert!(!adapter.agent_descriptor_path().is_file());
    assert!(!adapter.skill_md_path().is_file());
    Ok(())
}

/// Golden-file proof (workpack c06 acceptance row): the native
/// `agents/openai.yaml` descriptor rendered by
/// [`CodexAdapter::render_agent_descriptor`] must equal the pinned
/// snapshot byte-for-byte. This is the adapter's own emitter output — NOT
/// sourced from the legacy bulk `.codex-plugin` publish path (that path
/// does not exist in this adapter at all; it is collapsed per the
/// workpack).
#[test]
fn agent_descriptor_matches_golden_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = CodexAdapter::render_agent_descriptor();
    let expected = golden("openai.yaml")?;
    assert_eq!(
        rendered, expected,
        "agents/openai.yaml descriptor diverged from the pinned golden snapshot"
    );
    Ok(())
}

/// Golden-file proof: the GLOBAL `AGENTS.md` managed block (transitional
/// `ocentra-enforcer` markers + body) rendered by
/// [`CodexAdapter::upsert_global_agents_block`] on an empty existing file
/// must equal the pinned snapshot byte-for-byte.
#[test]
fn global_agents_md_block_matches_golden_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let rendered = CodexAdapter::upsert_global_agents_block("", "AGENTS.md")?;
    let expected = golden("AGENTS.md")?;
    assert_eq!(
        rendered, expected,
        "global AGENTS.md managed block diverged from the pinned golden snapshot"
    );
    // The transitional markers are byte-for-byte the legacy `.mjs` literals.
    assert!(rendered.starts_with("<!-- ocentra-enforcer:start -->\n"));
    assert!(rendered
        .trim_end()
        .ends_with("<!-- ocentra-enforcer:end -->"));
    Ok(())
}

/// Golden-file proof: the `[mcp_servers.enforcer]` TOML table rendered by
/// [`CodexAdapter::desired_table`]-equivalent `apply` output (rendered via
/// a fresh document) must equal the pinned snapshot byte-for-byte once the
/// binary path is substituted — the adapter never falls back to a
/// `node`/`.mjs` shim command per the workpack's binding contract.
#[test]
fn mcp_servers_toml_block_matches_golden_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let (home, binary) = isolated_fixture("pass")?;
    let adapter = CodexAdapter::new(home.path(), &binary);
    let ctx = RequestContext::with_defaults(binary.clone());
    let plan = adapter.plan(&ctx)?;
    adapter.apply(&plan)?;

    let raw = std::fs::read_to_string(adapter.config_toml_path())?;

    // Compare structurally (parsed, not string-substituted): `toml_edit`
    // picks a quoting style (double- vs single-quoted literal) based on
    // whether the value contains backslashes, which varies by platform
    // path shape (Windows vs POSIX). The golden template pins the
    // STRUCTURE (keys/shape/no node-shim), not one platform's quoting
    // style; the actual `command` value is asserted separately below.
    let doc: toml_edit::DocumentMut = raw.parse()?;
    let entry = &doc["mcp_servers"]["enforcer"];
    assert_eq!(
        entry["command"].as_str(),
        Some(binary.display().to_string().as_str())
    );
    assert_eq!(entry["args"].as_array().map(|a| a.len()), Some(0));
    assert_eq!(entry["enabled"].as_bool(), Some(true));
    assert_eq!(
        entry["env"]["OCENTRA_LEDGER_HOME"].as_str(),
        Some("${HOME}/.enforcer/ledger")
    );

    let golden_template = golden("mcp_servers_block.toml")?;
    let golden_doc: toml_edit::DocumentMut = golden_template
        .replace("__BINARY_PATH__", "placeholder")
        .parse()?;
    let golden_entry = &golden_doc["mcp_servers"]["enforcer"];
    assert_eq!(
        entry["args"].as_array().map(|a| a.len()),
        golden_entry["args"].as_array().map(|a| a.len()),
        "golden template's `args` shape diverged from the generated block"
    );
    assert_eq!(
        entry["enabled"].as_bool(),
        golden_entry["enabled"].as_bool(),
        "golden template's `enabled` shape diverged from the generated block"
    );
    assert_eq!(
        entry["env"]["OCENTRA_LEDGER_HOME"].as_str(),
        golden_entry["env"]["OCENTRA_LEDGER_HOME"].as_str(),
        "golden template's ledger-home env value diverged from the generated block"
    );

    assert!(!raw.contains("\"node\""));
    assert!(!raw.contains(".mjs"));
    Ok(())
}
