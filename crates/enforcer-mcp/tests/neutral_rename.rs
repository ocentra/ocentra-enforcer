//! x01 (neutral rename) proof rows: `neutral-rename-grep-clean` +
//! `neutral-rename-mcp-smoke`.
//!
//! Per `docs/plans/enforcer-selfhost-plan/workpacks/x01-neutral-rename.md`:
//! x01 owns exactly the workspace/crate `[package]`/`[[bin]]` name fields
//! and the two name consts (`crates/enforcer-cli/src/name.rs`,
//! `crates/enforcer-mcp/src/name.rs`). This file is the named `cargo test`
//! proof for that scope — it does NOT scan `enforcer-literal-scan` crate
//! internals or plan-doc prose (explicitly out of the grep gate's scope
//! per the workpack).

use std::path::{Path, PathBuf};

/// The workspace root, resolved from this crate's manifest dir (two levels
/// up: `crates/enforcer-mcp` -> workspace root).
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    match manifest_dir.parent().and_then(Path::parent) {
        Some(root) => root.to_path_buf(),
        None => unreachable!("crates/enforcer-mcp always has two parent components"),
    }
}

/// A legacy token match: which owned file, which line, which token.
struct Hit {
    file: PathBuf,
    line_no: usize,
    line: String,
}

/// Scan one file's full text for `ocentra[-_]enforcer` or `rust_rules`
/// (case-insensitive), returning every matching line. Mirrors the
/// workpack's grep gate exactly (regex shape `ocentra[-_]enforcer` plus
/// the plain literal `rust_rules`).
fn scan_file(path: &Path) -> Vec<Hit> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let lower = line.to_ascii_lowercase();
            lower.contains("ocentra-enforcer")
                || lower.contains("ocentra_enforcer")
                || lower.contains("rust_rules")
        })
        .map(|(idx, line)| Hit {
            file: path.to_path_buf(),
            line_no: idx + 1,
            line: line.to_owned(),
        })
        .collect()
}

/// Every `[package] name =` / `[[bin]] name =` line only (not the whole
/// file) for a member crate's `Cargo.toml` — the workpack scopes the
/// member-crate grep to the name fields specifically (checklist: "a01
/// owns the workspace-root manifest STRUCTURE ... x01 touches only the
/// product NAME strings"), so a stray non-name literal elsewhere in a
/// sibling-owned `Cargo.toml` (e.g. a `description`) is out of this
/// gate's scope.
fn scan_cargo_toml_name_fields(path: &Path) -> Vec<Hit> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim_start();
            trimmed.starts_with("name") && trimmed.contains('=')
        })
        .filter(|(_, line)| {
            let lower = line.to_ascii_lowercase();
            lower.contains("ocentra-enforcer")
                || lower.contains("ocentra_enforcer")
                || lower.contains("rust_rules")
        })
        .map(|(idx, line)| Hit {
            file: path.to_path_buf(),
            line_no: idx + 1,
            line: line.to_owned(),
        })
        .collect()
}

fn member_crate_manifests(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&crates_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        if manifest.is_file() {
            out.push(manifest);
        }
    }
    out.sort();
    out
}

/// `neutral-rename-grep-clean`: the pass condition named in the workpack —
/// a grep gate over the owned shipped/config name surfaces for
/// `ocentra[-_]enforcer` and `rust_rules` returns EMPTY.
#[test]
fn neutral_rename_grep_clean() {
    let root = workspace_root();
    let mut hits: Vec<Hit> = Vec::new();

    // Root Cargo.toml: [workspace.package] metadata — whole-file scope
    // (the workpack's checklist explicitly extends the root manifest's
    // scan to any workspace-level name/description/repository product
    // string, not just the `name` key).
    hits.extend(scan_file(&root.join("Cargo.toml")));

    // Member crate manifests: name/bin-name fields ONLY (not the whole
    // file — sibling-owned fields such as `description` are out of
    // scope per the workpack's "Coordinate only the name field" note).
    for manifest in member_crate_manifests(&root) {
        hits.extend(scan_cargo_toml_name_fields(&manifest));
    }

    // The two owned name consts — whole-file scope.
    hits.extend(scan_file(&root.join("crates/enforcer-cli/src/name.rs")));
    hits.extend(scan_file(&root.join("crates/enforcer-mcp/src/name.rs")));

    let report: Vec<String> = hits
        .iter()
        .map(|hit| {
            format!(
                "{}:{}: {}",
                hit.file.display(),
                hit.line_no,
                hit.line.trim()
            )
        })
        .collect();
    assert!(
        hits.is_empty(),
        "neutral-rename-grep-clean: found {} residual legacy-token match(es) in owned \
         name surfaces:\n{}",
        hits.len(),
        report.join("\n")
    );
}

/// Fail-fixture companion: the scanner itself must actually detect a
/// planted legacy token (proves the gate is not a vacuous always-pass).
#[test]
fn fail_fixture_scanner_detects_a_planted_legacy_token() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let planted = temp.path().join("name.rs");
    std::fs::write(
        &planted,
        "pub const SERVER_NAME: &str = \"ocentra-enforcer\";\n",
    )?;
    let hits = scan_file(&planted);
    assert_eq!(hits.len(), 1, "scanner must flag the planted legacy token");

    let planted_alias = temp.path().join("aliases.rs");
    std::fs::write(&planted_alias, "const P: &str = \"rust_rules\";\n")?;
    let hits2 = scan_file(&planted_alias);
    assert_eq!(
        hits2.len(),
        1,
        "scanner must flag a planted rust_rules token"
    );
    Ok(())
}

/// `neutral-rename-mcp-smoke`: `cargo test -p enforcer-mcp` smoke green
/// post-rename under server name `enforcer`. This asserts the const
/// value directly (the crate's own `crates/enforcer-mcp/tests/stdio_smoke.rs`
/// proves the end-to-end stdio behavior against the real spawned binary;
/// this row asserts the shipped identity value itself is the renamed
/// one, closing the loop between "the const says enforcer" and "the wire
/// smoke test observed enforcer").
#[test]
fn neutral_rename_mcp_smoke_server_name_is_enforcer() {
    assert_eq!(
        enforcer_mcp::name::SERVER_NAME,
        "enforcer",
        "the MCP server-name const must be the neutral product name post-rename"
    );
}

/// The binary-name const (owned by this same workpack, in the sibling
/// `enforcer-cli` crate) must also read the neutral name — asserted here
/// via direct file content, mirroring the same-shape check the grep gate
/// runs, since `enforcer-mcp` cannot depend on `enforcer-cli` (would
/// invert the dependency graph).
#[test]
fn binary_name_const_file_reads_enforcer() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();
    let text = std::fs::read_to_string(root.join("crates/enforcer-cli/src/name.rs"))?;
    assert!(
        text.contains("pub const BINARY_NAME: &str = \"enforcer\";"),
        "BINARY_NAME must be the literal neutral value \"enforcer\""
    );
    Ok(())
}
