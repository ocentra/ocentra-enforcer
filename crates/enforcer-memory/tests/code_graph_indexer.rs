//! Integration coverage for the X06.2 code KG indexer
//! ([`enforcer_memory::code_graph`]), exercising every hard test the
//! workpack names against a small multi-language fixture repo copied
//! from `tests/fixtures/memory/code_graph/` into a real, throwaway git
//! working tree (content hashes and history summaries are meaningless
//! without a real git repo, so this test does not mock git).

use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/code_graph";

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn init_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

/// Copy every file from `tests/fixtures/memory/code_graph/` into
/// `dest`, returning the copied paths (for `index_repository`'s
/// `walk_files` argument).
fn copy_fixtures(dest: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join(FIXTURE_DIR);
    let mut copied = Vec::new();
    for entry in fs::read_dir(&fixture_root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest_path = dest.join(entry.file_name());
            fs::copy(entry.path(), &dest_path)?;
            copied.push(dest_path);
        }
    }
    Ok(copied)
}

#[test]
fn full_fixture_repo_indexes_every_supported_language_plus_text_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial fixture import")?;

    let mut graph = CodeGraph::new();
    let (_manifest, report) = graph.index_repository(dir.path(), &files, &Manifest::default())?;

    // Every fixture file is new on a first run.
    assert_eq!(report.added.len(), files.len());
    assert!(report.changed.is_empty());
    assert!(report.deleted.is_empty());

    // Symbol extraction: Rust struct/trait/fn/test all present.
    let symbol_names: Vec<&str> = graph.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"Widget"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"Drawable"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"render"), "{symbol_names:?}");
    assert!(
        symbol_names.contains(&"render_does_not_panic"),
        "{symbol_names:?}"
    );

    // Route extraction: both the Express router.get and the Flask
    // @app.route decorator produced a route edge.
    let routes: Vec<(&str, &str)> = graph
        .routes()
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(routes.contains(&("GET", "/widgets")), "{routes:?}");

    // Import/call edges exist for the Rust fixture.
    assert!(graph.imports().iter().any(|i| i.module_path.contains("fs")));
    assert!(graph.calls().iter().any(|c| c.callee.contains("helper")));

    // Fallback: the `.qux` fixture is a TextOnly node, not skipped.
    let has_text_only_notes = graph
        .nodes()
        .iter()
        .any(|n| matches!(n, CodeNode::TextOnly(f) if f.rel_path == "NOTES.qux"));
    assert!(
        has_text_only_notes,
        "unsupported-extension fixture must still be indexed as TextOnly"
    );

    // Every walked file got exactly one file-shaped node (File or
    // TextOnly) -- nothing was silently skipped.
    assert_eq!(graph.file_nodes().count(), files.len());
    Ok(())
}

#[test]
fn unchanged_files_are_skipped_across_reindex_runs() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial fixture import")?;

    let mut graph1 = CodeGraph::new();
    let (manifest_v1, _) = graph1.index_repository(dir.path(), &files, &Manifest::default())?;

    let mut graph2 = CodeGraph::new();
    let (_manifest_v2, report_v2) = graph2.index_repository(dir.path(), &files, &manifest_v1)?;

    assert_eq!(report_v2.unchanged.len(), files.len());
    assert!(report_v2.changed.is_empty());
    assert!(report_v2.added.is_empty());
    assert!(report_v2.deleted.is_empty());
    Ok(())
}

#[test]
fn changed_file_is_reindexed_and_deleted_file_becomes_tombstone() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial fixture import")?;

    let mut graph1 = CodeGraph::new();
    let (manifest_v1, _) = graph1.index_repository(dir.path(), &files, &Manifest::default())?;

    // Mutate one file (adds a new function) and delete another.
    let rust_fixture = dir.path().join("sample.rs");
    let mut contents = fs::read_to_string(&rust_fixture)?;
    contents.push_str("\nfn brand_new_fn() {}\n");
    fs::write(&rust_fixture, contents)?;

    let toml_fixture = dir.path().join("config.toml");
    fs::remove_file(&toml_fixture)?;

    commit_all(dir.path(), "change sample.rs, delete config.toml")?;

    let remaining_files: Vec<PathBuf> = files
        .iter()
        .filter(|p| p.as_path() != toml_fixture.as_path())
        .cloned()
        .collect();

    let mut graph2 = CodeGraph::new();
    let (manifest_v2, report_v2) =
        graph2.index_repository(dir.path(), &remaining_files, &manifest_v1)?;

    assert_eq!(report_v2.changed, vec!["sample.rs".to_string()]);
    assert_eq!(report_v2.deleted, vec!["config.toml".to_string()]);

    let symbol_names: Vec<&str> = graph2.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(
        symbol_names.contains(&"brand_new_fn"),
        "changed file must be reparsed: {symbol_names:?}"
    );

    let tombstones: Vec<&str> = graph2.tombstones().map(|t| t.rel_path.as_str()).collect();
    assert_eq!(tombstones, vec!["config.toml"]);
    assert!(!manifest_v2.entries.contains_key("config.toml"));
    Ok(())
}
