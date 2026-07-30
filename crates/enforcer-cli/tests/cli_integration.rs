//! The arc-22 proof row: spawn the REAL compiled `enforcer` binary (not an
//! in-process call) and run it end-to-end -- a check/scan on fail/pass
//! fixture trees across all three scope modes with the expected exit
//! codes, a usage-error collision case, the "no override flag" assertion,
//! and a `serve` mode responding to `initialize` over stdio. Mirrors
//! `crates/enforcer-mcp/tests/stdio_smoke.rs`'s pattern of spawning the
//! real binary rather than faking the process boundary.

#[cfg(feature = "full")]
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
#[cfg(feature = "full")]
use std::process::Stdio;

fn binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    let mut dir = exe
        .parent()
        .ok_or("test binary has no parent directory")?
        .to_path_buf();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let candidate = dir.join(if cfg!(windows) {
        "enforcer.exe"
    } else {
        "enforcer"
    });
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(format!("enforcer binary not found at {}", candidate.display()).into())
}

fn write_pass_fixture(root: &std::path::Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("lib.rs"), "fn good() -> i32 { 42 }\n")
}

fn write_fail_fixture(root: &std::path::Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("lib.rs"),
        "fn bad() { let x: Option<i32> = None; x.unwrap(); }\n",
    )
}

fn write_inline_test_fixture(root: &std::path::Path) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("lib.rs"),
        "pub fn answer() -> i32 { 42 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn answer_is_stable() {\n        assert_eq!(super::answer(), 42);\n    }\n}\n",
    )
}

fn write_inline_test_policy(root: &std::path::Path, policy: &str) -> std::io::Result<()> {
    std::fs::write(
        root.join("ocentra-enforcer.config.json"),
        format!(
            "{{\n  \"schemaVersion\": 2,\n  \"profileName\": \"default\",\n  \"inlineTestPolicy\": \"{policy}\"\n}}\n"
        ),
    )
}

fn write_dependency_policy_fixture(
    root: &std::path::Path,
    external_path: bool,
) -> std::io::Result<()> {
    std::fs::create_dir_all(root.join("crates").join("core"))?;
    std::fs::create_dir_all(root.join("crates").join("app"))?;
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/core\", \"crates/app\"]\nresolver = \"2\"\n",
    )?;
    std::fs::write(
        root.join("crates").join("core").join("Cargo.toml"),
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    let dependency_path = if external_path {
        "../outside"
    } else {
        "../core"
    };
    std::fs::write(
        root.join("crates").join("app").join("Cargo.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = {{ path = \"{dependency_path}\" }}\n"
        ),
    )?;
    Ok(())
}

fn write_secret_policy_fixture(
    root: &std::path::Path,
    contains_secret: bool,
) -> std::io::Result<()> {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir)?;
    let source = if contains_secret {
        inline_secret_source()
    } else {
        String::from("const api_key = std::env::var(\"API_KEY\")?;\n")
    };
    std::fs::write(dir.join("config.ts"), source)
}

/// Assemble the detector fixture at test execution time. The compiled CLI must
/// detect this input; the integration-test source itself must not look like a
/// committed credential to the native dogfood scan.
fn inline_secret_source() -> String {
    format!(
        "const api_key = \"{}{}\";\n",
        "sk-proj-", "ABCDEFGHIJKLMNOPQRSTUVWXYZ123456"
    )
}

fn write_secret_policy_scope_config(root: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(
        root.join("ocentra-enforcer.config.json"),
        r#"{"schemaVersion":2,"profileName":"default","ignoreFileGlobs":["**/fixtures/**","vendor/**"]}"#,
    )
}

fn write_ignored_secret_policy_sources(root: &std::path::Path) -> std::io::Result<()> {
    let fixture = root.join("crates/sample/fixtures/fail.ts");
    let vendor = root.join("vendor/example/SKILL.md");
    std::fs::create_dir_all(
        fixture
            .parent()
            .ok_or_else(|| std::io::Error::other("fixture has no parent"))?,
    )?;
    std::fs::create_dir_all(
        vendor
            .parent()
            .ok_or_else(|| std::io::Error::other("vendor file has no parent"))?,
    )?;
    std::fs::write(fixture, inline_secret_source())?;
    std::fs::write(vendor, inline_secret_source())
}

fn run_check(
    root: &std::path::Path,
    extra_args: &[&str],
) -> Result<std::process::ExitStatus, Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let mut cmd = Command::new(binary);
    cmd.current_dir(root).arg("check").args(extra_args);
    Ok(cmd.status()?)
}

#[test]
fn pass_fixture_paths_mode_exits_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &["src/lib.rs"])?;
    assert!(status.success(), "clean tree via <paths...> must exit 0");
    Ok(())
}

#[test]
fn fail_fixture_paths_mode_exits_non_zero_with_violations_class(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let status = run_check(temp.path(), &["src/lib.rs"])?;
    assert_eq!(
        status.code(),
        Some(1),
        "a rule violation must exit with the Violations class (1), not a generic non-zero"
    );
    Ok(())
}

#[test]
fn native_dependency_policy_rejects_external_local_cargo_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_dependency_policy_fixture(temp.path(), true)?;
    let status = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "dependency-policy"])
        .status()?;
    assert_eq!(status.code(), Some(1));
    Ok(())
}

#[test]
fn native_dependency_policy_accepts_declared_workspace_member_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_dependency_policy_fixture(temp.path(), false)?;
    let status = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "dependency-policy"])
        .status()?;
    assert!(status.success());
    Ok(())
}

#[test]
fn native_secrets_policy_rejects_and_redacts_inline_credentials(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_secret_policy_fixture(temp.path(), true)?;
    let output = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "secrets"])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("SEC-1.1"));
    assert!(stdout.contains("Inline secrets are forbidden"));
    assert!(!stdout.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ123456"));
    Ok(())
}

#[test]
fn native_secrets_policy_accepts_runtime_configuration_reference(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_secret_policy_fixture(temp.path(), false)?;
    let status = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "secrets"])
        .status()?;
    assert!(status.success());
    Ok(())
}

#[test]
fn native_secrets_policy_excludes_configured_fixture_and_vendor_sources(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_secret_policy_scope_config(temp.path())?;
    write_ignored_secret_policy_sources(temp.path())?;
    let output = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "secrets"])
        .output()?;
    assert!(
        output.status.success(),
        "configured non-product sources must not fail secret policy: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

#[test]
fn native_secrets_policy_still_rejects_product_source_with_configured_exclusions(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_secret_policy_scope_config(temp.path())?;
    write_ignored_secret_policy_sources(temp.path())?;
    write_secret_policy_fixture(temp.path(), true)?;
    let output = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "secrets"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout.contains("src/config.ts:1 SEC-1.1"));
    assert!(!stdout.contains("crates/sample/fixtures/fail.ts"));
    assert!(!stdout.contains("vendor/example/SKILL.md"));
    Ok(())
}

#[test]
fn native_sbom_policy_writes_a_deterministic_schema_validated_artifact(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\nresolver = \"2\"\n",
    )?;
    std::fs::create_dir_all(temp.path().join("crates/app/src"))?;
    std::fs::write(
        temp.path().join("crates/app/Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    std::fs::write(
        temp.path().join("crates/app/src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )?;
    std::fs::write(
        temp.path().join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"fixture-app\"\nversion = \"0.1.0\"\n",
    )?;
    let output = temp.path().join("sbom-output");
    let status = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "sbom", "--output"])
        .arg(&output)
        .status()?;
    assert!(status.success());
    let artifact = output.join("cargo-sbom.json");
    let first = std::fs::read_to_string(&artifact)?;
    let decoded: enforcer_scan::sbom_policy::CargoSbomDto = serde_json::from_str(&first)?;
    enforcer_scan::sbom_policy::validate(&decoded).map_err(std::io::Error::other)?;
    let status = Command::new(binary_path()?)
        .current_dir(temp.path())
        .args(["policy", "sbom", "--output"])
        .arg(&output)
        .status()?;
    assert!(status.success());
    assert_eq!(first, std::fs::read_to_string(artifact)?);
    Ok(())
}

#[test]
fn inline_test_policy_keeps_rust_unit_tests_exempt_in_the_real_cli_binary(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_inline_test_fixture(temp.path())?;

    let default_status = run_check(temp.path(), &["src/lib.rs"])?;
    assert!(
        default_status.success(),
        "Rust cfg(test) modules are exempt from TEST-2.2"
    );

    for policy in ["forbid", "warn", "allow"] {
        write_inline_test_policy(temp.path(), policy)?;
        let status = run_check(temp.path(), &["src/lib.rs"])?;
        assert!(
            status.success(),
            "Rust cfg(test) remains exempt with {policy} policy"
        );
    }
    Ok(())
}

#[test]
fn pass_fixture_all_mode_exits_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &["--all"])?;
    assert!(status.success(), "clean tree via --all must exit 0");
    Ok(())
}

#[test]
fn fail_fixture_all_mode_exits_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let status = run_check(temp.path(), &["--all"])?;
    assert_eq!(status.code(), Some(1));
    Ok(())
}

#[test]
fn base_head_mode_parses_and_routes() -> Result<(), Box<dyn std::error::Error>> {
    // `enforcer-scan` does not yet restrict the walked file set by git
    // diff range (see `commands::resolve_files` docs) -- this asserts the
    // mode PARSES AND ROUTES (reaches the engine, produces a Report, exits
    // one of the two documented exit classes) rather than asserting a
    // specific finding count, since true diff-narrowing is a later pack's
    // gap, not this skeleton's.
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &["--base", "HEAD", "--head", "HEAD"])?;
    let code = status.code().ok_or("process terminated by signal")?;
    assert!(
        code == 0 || code == 1,
        "base/head mode must resolve to a normal verdict exit class, got {code}"
    );
    Ok(())
}

#[test]
fn base_head_and_paths_collision_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(
        temp.path(),
        &["--base", "HEAD", "--head", "HEAD", "src/lib.rs"],
    )?;
    assert_eq!(
        status.code(),
        Some(2),
        "a --base/--head + <paths...> collision must be the UsageError class (2)"
    );
    Ok(())
}

#[test]
fn no_scope_at_all_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let status = run_check(temp.path(), &[])?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[test]
fn there_is_no_override_flag_on_the_real_binary() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    for bogus in [
        "--force",
        "--no-verify",
        "--skip",
        "--bypass",
        "--ignore-findings",
    ] {
        let status = run_check(temp.path(), &["--all", bogus])?;
        assert_eq!(
            status.code(),
            Some(2),
            "unknown flag `{bogus}` must be a clap usage error, and specifically it must \
             never suppress the fail fixture's violation exit class"
        );
    }
    Ok(())
}

#[test]
fn help_never_advertises_a_bypass_flag() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let output = Command::new(&binary).args(["check", "--help"]).output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for bad_word in ["--force", "--no-verify", "--skip-checks", "--bypass"] {
        assert!(
            !text.contains(bad_word),
            "check --help must never advertise `{bad_word}`"
        );
    }
    Ok(())
}

fn install_command(binary: &std::path::Path, fixture: &std::path::Path) -> std::process::Command {
    let home = fixture.join("home");
    let app_data = fixture.join("config");
    let mut command = Command::new(binary);
    command
        .current_dir(fixture)
        .arg("install")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("APPDATA", &app_data)
        .env("XDG_CONFIG_HOME", &app_data)
        .env("CODEX_HOME", home.join(".codex"));
    command
}

fn doctor_command(binary: &std::path::Path, fixture: &std::path::Path) -> std::process::Command {
    let home = fixture.join("home");
    let app_data = fixture.join("config");
    let mut command = Command::new(binary);
    command
        .current_dir(fixture)
        .arg("doctor")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("APPDATA", &app_data)
        .env("XDG_CONFIG_HOME", &app_data)
        .env("CODEX_HOME", home.join(".codex"));
    command
}

fn expected_config_root(_home: &std::path::Path, _app_data: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        _home.join("Library").join("Application Support")
    }
    #[cfg(not(target_os = "macos"))]
    {
        _app_data.to_path_buf()
    }
}

#[test]
fn install_registers_every_native_harness_and_is_idempotent(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let binary = binary_path()?;
    let first = install_command(&binary, fixture.path()).status()?;
    assert_eq!(
        first.code(),
        Some(0),
        "a complete native install plus doctor verification must exit 0"
    );

    let home = fixture.path().join("home");
    let config_root = expected_config_root(&home, &fixture.path().join("config"));
    let json_paths = [
        home.join(".claude.json"),
        home.join(".gemini").join("settings.json"),
        home.join(".gemini").join("config").join("mcp_config.json"),
        home.join(".cursor").join("mcp.json"),
        home.join(".kiro").join("settings").join("mcp.json"),
        home.join(".codeium")
            .join("windsurf")
            .join("mcp_config.json"),
        config_root
            .join("Code")
            .join("User")
            .join("globalStorage")
            .join("kilocode.kilo-code")
            .join("settings")
            .join("mcp_settings.json"),
        config_root.join("Zed").join("settings.json"),
    ];
    for path in &json_paths {
        assert!(
            path.is_file(),
            "native install must create `{}`",
            path.display()
        );
    }
    let codex_config = home.join(".codex").join("config.toml");
    assert!(
        codex_config.is_file(),
        "native install must create `{}`",
        codex_config.display()
    );

    let gemini: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_paths[1])?)?;
    assert_eq!(
        gemini["mcpServers"][enforcer_mcp::name::SERVER_NAME]["command"],
        serde_json::json!(binary.display().to_string())
    );
    let zed: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json_paths[7])?)?;
    assert_eq!(
        zed["context_servers"][enforcer_mcp::name::SERVER_NAME]["command"],
        serde_json::json!(binary.display().to_string())
    );
    let codex_before = std::fs::read(&codex_config)?;
    let json_before = json_paths
        .iter()
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()?;

    let second = install_command(&binary, fixture.path()).status()?;
    assert_eq!(
        second.code(),
        Some(0),
        "an idempotent reinstall plus doctor verification must exit 0"
    );
    assert_eq!(std::fs::read(&codex_config)?, codex_before);
    let json_after = json_paths
        .iter()
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(json_after, json_before);
    Ok(())
}

#[test]
fn doctor_verifies_the_native_install_without_mutating_it() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = tempfile::tempdir()?;
    let binary = binary_path()?;
    assert!(install_command(&binary, fixture.path()).status()?.success());

    let codex_config = fixture
        .path()
        .join("home")
        .join(".codex")
        .join("config.toml");
    let before = std::fs::read(&codex_config)?;
    let status = doctor_command(&binary, fixture.path()).status()?;
    assert!(status.success(), "doctor must pass after a native install");
    assert_eq!(
        std::fs::read(&codex_config)?,
        before,
        "doctor must be read-only"
    );
    Ok(())
}

/// Send one NDJSON request and read exactly one NDJSON reply line -- the
/// same pattern `enforcer-mcp`'s own stdio smoke test uses against its
/// throwaway smoke binary, run here against the PRODUCTION `enforcer
/// serve` mode.
#[cfg(feature = "full")]
fn round_trip(
    stdin: &mut impl Write,
    stdout: &mut impl BufRead,
    request: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let line = format!("{request}\n");
    stdin.write_all(line.as_bytes())?;
    stdin.flush()?;
    let mut reply_line = String::new();
    stdout.read_line(&mut reply_line)?;
    Ok(serde_json::from_str(reply_line.trim_end())?)
}

/// `lite` builds must not merely HIDE `serve`/`coordination`/`ledger`
/// from `--help` -- they must be absent from the compiled dependency
/// graph. This test only proves the `--help` text (the cheapest
/// process-boundary check available); the Cargo-graph exclusion itself is
/// proven by the fact that this whole `tests/cli_integration.rs` binary
/// compiles under `--no-default-features --features lite` with zero
/// `enforcer-coordination` reference reachable (see `Cargo.toml`'s
/// `full = ["enforcer-coordination"]` gate) -- if `coordination`/`ledger`
/// leaked into the lite grammar, this crate would fail to link without
/// the optional dependency.
#[cfg(not(feature = "full"))]
#[test]
fn lite_help_omits_full_only_subcommands() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let output = Command::new(&binary).arg("--help").output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Match each subcommand's own listing line (two-space-indented name at
    // the start of a line, as clap renders its `Commands:` block), not a
    // bare substring -- "serve" is also a substring of prose like
    // "...only reserves the subcommand name...", which would otherwise
    // false-positive this assertion against the `install` entry's help
    // text.
    for full_only in ["serve", "ui", "coordination", "ledger"] {
        let listing_prefix = format!("  {full_only} ");
        let listing_line = format!("  {full_only}\n");
        assert!(
            !text.contains(&listing_prefix) && !text.contains(&listing_line),
            "lite build's top-level --help must never list the `{full_only}` subcommand \
             (full-only subcommand leaked into a lite binary); help was:\n{text}"
        );
    }
    Ok(())
}

#[cfg(feature = "full")]
#[test]
fn serve_mode_responds_to_initialize_over_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let mut child = Command::new(binary)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
    let stdout = child.stdout.take().ok_or("child has no stdout")?;
    let mut reader = BufReader::new(stdout);

    let reply = round_trip(
        &mut stdin,
        &mut reader,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05" },
        }),
    )?;
    assert_eq!(
        reply["result"]["serverInfo"]["name"],
        serde_json::json!(enforcer_mcp::name::SERVER_NAME),
        "enforcer serve must speak the same MCP server identity as enforcer-mcp::sink"
    );

    drop(stdin);
    let status = child.wait()?;
    assert!(status.success(), "enforcer serve must exit 0 on stdin EOF");
    Ok(())
}

/// `serve-remote-no-token` fail-fixture at the real-binary boundary:
/// `enforcer ui --host 0.0.0.0` with no `--token` must refuse to start
/// (`InternalError`, never a silently-clean exit 0) -- the g01 workpack's
/// fail-closed host-bind requirement, proven against the real `enforcer`
/// process rather than only the in-crate unit test.
#[cfg(feature = "full")]
#[test]
fn ui_remote_host_without_token_refuses_to_start() -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let status = Command::new(binary)
        .args(["ui", "--host", "0.0.0.0"])
        .status()?;
    assert_eq!(
        status.code(),
        Some(70),
        "a non-loopback --host with no --token must be the InternalError exit class (70), \
         never a silent success"
    );
    Ok(())
}

/// Both CLI spellings (`enforcer serve --ui` and `enforcer ui`) resolve
/// to the same surface and honor the loopback default -- exercised here
/// by asserting NEITHER exits with the fail-closed refusal code when no
/// `--host` override is given (they bind loopback and would otherwise
/// run until killed, so this test only asserts the refusal path is
/// specific to a non-loopback host, matching the unit-level
/// `serve_surface_contract_loopback_default_holds` proof; a full bind+
/// connect round trip against the real binary is covered by
/// `enforcer_ui::serve`'s own `run_binds_loopback_and_serves_shell_with_mount_registry`).
#[cfg(feature = "full")]
#[test]
fn serve_ui_flag_and_ui_alias_both_reject_remote_without_token_identically(
) -> Result<(), Box<dyn std::error::Error>> {
    let binary = binary_path()?;
    let via_serve_ui_flag = Command::new(&binary)
        .args(["serve", "--ui", "--host", "0.0.0.0"])
        .status()?;
    let via_ui_alias = Command::new(&binary)
        .args(["ui", "--host", "0.0.0.0"])
        .status()?;
    assert_eq!(via_serve_ui_flag.code(), via_ui_alias.code());
    assert_eq!(via_serve_ui_flag.code(), Some(70));
    Ok(())
}

#[test]
fn advise_literals_parses_and_routes() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["advise", "literals"])
        .status()?;
    let code = status.code().ok_or("process terminated by signal")?;
    assert!(
        code == 0 || code == 1,
        "advise literals must reach a normal verdict, got {code}"
    );
    Ok(())
}

#[test]
fn advise_unknown_target_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["advise", "somethingElse"])
        .status()?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[test]
fn verify_mode_bogus_is_a_usage_error_not_a_finding_exit_code(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_fail_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["verify", "--all", "--mode", "bogus"])
        .status()?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[test]
fn architecture_bare_without_check_is_a_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    write_pass_fixture(temp.path())?;
    let binary = binary_path()?;
    let status = Command::new(binary)
        .current_dir(temp.path())
        .args(["architecture"])
        .status()?;
    assert_eq!(status.code(), Some(2));
    Ok(())
}

#[cfg(feature = "full")]
#[test]
fn memory_cli_forwards_hyphenated_index_flags_to_the_real_binary(
) -> Result<(), Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    std::fs::write(repo.path().join("sample.rs"), "pub fn sample() {}\n")?;
    let stores_dir = repo.path().join(".enforce").join("ci-memory");
    let output = Command::new(binary_path()?)
        .args([
            "memory",
            "cli",
            "--json",
            "index_repository",
            "--repo-path",
            repo.path()
                .to_str()
                .ok_or("temp repository path was not UTF-8")?,
            "--stores-dir",
            stores_dir.to_str().ok_or("stores path was not UTF-8")?,
            "--mode",
            "fast",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("\"isError\": false"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
