//! Integration coverage for X06.P1's parity read tools --
//! [`enforcer_memory::snippet`], [`enforcer_memory::graph_schema`],
//! [`enforcer_memory::code_search`], [`enforcer_memory::projects`] --
//! against a real, throwaway git working tree copied from
//! `tests/fixtures/memory/parity_read_tools/` (content hashes and
//! byte-exact snippet extraction are meaningless without real files on
//! disk, so this test does not mock the filesystem or git).

use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::code_search::{search_code, SearchError, SearchMode};
use enforcer_memory::graph_schema::get_graph_schema;
use enforcer_memory::projects::{
    delete_project, index_status, list_projects, FreshnessState, ProjectsError,
};
use enforcer_memory::snippet::{get_code_snippet, SnippetError};
use enforcer_memory::store::Store;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/parity_read_tools";

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

fn indexed_fixture_repo() -> Result<(tempfile::TempDir, CodeGraph), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial fixture import")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &files, &Manifest::default())?;
    Ok((dir, graph))
}

fn hash_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::from("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

// -- get_code_snippet ------------------------------------------------

#[test]
fn snippet_is_byte_exact_hash_equal_to_an_independent_file_slice() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let snippet = get_code_snippet(&graph, dir.path(), "service.rs::helper", false)?;

    let raw = fs::read(dir.path().join("service.rs"))?;
    let independent_slice = &raw[snippet.start_byte..snippet.end_byte];
    assert_eq!(snippet.bytes, independent_slice);
    assert_eq!(snippet.sha256, hash_hex(independent_slice));
    Ok(())
}

#[test]
fn snippet_unknown_symbol_fails_closed_never_a_similar_name_substitute() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    // "helperx" is a near-miss of the real symbol "helper".
    let outcome = get_code_snippet(&graph, dir.path(), "service.rs::helperx", false);
    assert!(matches!(outcome, Err(SnippetError::UnknownSymbol { .. })));
    Ok(())
}

#[test]
fn snippet_include_neighbors_lists_other_symbols_in_the_same_file() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let snippet = get_code_snippet(&graph, dir.path(), "service.rs::helper", true)?;
    let names: Vec<&str> = snippet
        .neighbors
        .iter()
        .map(|n| n.qualified_name.as_str())
        .collect();
    assert!(names.contains(&"service.rs::caller_one"), "{names:?}");
    assert!(names.contains(&"service.rs::caller_two"), "{names:?}");
    Ok(())
}

// -- get_graph_schema --------------------------------------------------

#[test]
fn graph_schema_counts_labels_and_edge_types_deterministically() -> TestResult {
    let (_dir, graph) = indexed_fixture_repo()?;

    let schema = get_graph_schema(&graph);
    assert_eq!(schema.total_nodes(), graph.nodes().len());
    assert_eq!(
        schema.total_edges(),
        graph.imports().len() + graph.calls().len() + graph.routes().len()
    );

    let labels: Vec<&str> = schema.labels.iter().map(|l| l.label.as_str()).collect();
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted, "labels must be alphabetically ordered");

    assert!(schema
        .edge_types
        .iter()
        .any(|e| e.edge_type == "Route" && e.count >= 1));
    assert!(schema
        .edge_types
        .iter()
        .any(|e| e.edge_type == "Calls" && e.count >= 2));
    Ok(())
}

// -- search_code --------------------------------------------------------

#[test]
fn search_code_enriches_hits_with_containing_symbol_and_ranks_by_inbound_degree() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let outcome = search_code(&graph, dir.path(), "value", SearchMode::Full, 0, 0)?;
    assert!(outcome.total_matches >= 1);
    assert!(outcome
        .hits
        .iter()
        .any(|h| h.containing_symbol.as_deref() == Some("helper")));

    // `helper` is called from two places in the fixture -- confirm the
    // ranking machinery actually sees a positive structural rank for it.
    let helper_hit = outcome
        .hits
        .iter()
        .find(|h| h.containing_symbol.as_deref() == Some("helper"))
        .expect("expected a hit inside helper");
    assert!(helper_hit.structural_rank >= 2, "{:?}", helper_hit);
    Ok(())
}

#[test]
fn search_code_modes_compact_full_files_are_distinct() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let files_mode = search_code(&graph, dir.path(), "helper", SearchMode::Files, 0, 0)?;
    assert!(files_mode.hits.is_empty());
    assert!(files_mode.files.contains(&"service.rs".to_string()));

    let full_mode = search_code(&graph, dir.path(), "helper", SearchMode::Full, 1, 0)?;
    assert!(!full_mode.hits.is_empty());
    Ok(())
}

#[test]
fn search_code_reports_unreadable_files_never_silently_skips() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;
    fs::remove_file(dir.path().join("service.rs"))?;

    let outcome = search_code(&graph, dir.path(), "helper", SearchMode::Full, 0, 0)?;
    assert!(outcome
        .unreadable_files
        .iter()
        .any(|f| f.rel_path == "service.rs"));
    Ok(())
}

#[test]
fn search_code_invalid_pattern_is_a_typed_error() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;
    let outcome = search_code(&graph, dir.path(), "(unterminated[", SearchMode::Full, 0, 0);
    assert!(matches!(outcome, Err(SearchError::InvalidPattern { .. })));
    Ok(())
}

// -- project registry ---------------------------------------------------

fn temp_stores_dir(name: &str) -> PathBuf {
    let unique = format!(
        "enforcer-memory-parity-projects-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

fn parse_repo_root(raw: &str) -> enforcer_domain::paths::RepoRoot {
    raw.parse()
        .unwrap_or_else(|_| unreachable!("test literal {raw:?} must parse as a RepoRoot"))
}

#[test]
fn projects_list_status_delete_round_trip() -> TestResult {
    let stores_dir = temp_stores_dir("roundtrip");
    let root = parse_repo_root("C:/Projects/parity-roundtrip");
    let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
    let project_id = store.project_id().as_str().to_owned();
    drop(store);

    let projects = list_projects(&stores_dir)?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].project_id, project_id);

    let status = index_status(&stores_dir, &project_id)?;
    assert_eq!(status.logs.len(), 2);
    assert!(status
        .logs
        .iter()
        .all(|l| matches!(l.state, FreshnessState::NoIndexBuilt)));

    delete_project(&stores_dir, &project_id)?;
    let projects_after = list_projects(&stores_dir)?;
    assert!(projects_after.is_empty());

    fs::remove_dir_all(&stores_dir)?;
    Ok(())
}

#[test]
fn projects_delete_rejects_path_traversal() -> TestResult {
    let parent = temp_stores_dir("traversal-parent");
    let stores_dir = parent.join("stores");
    fs::create_dir_all(&stores_dir)?;

    let victim = parent.join("victim");
    fs::create_dir_all(&victim)?;
    fs::write(
        victim.join("store.json"),
        r#"{"schema_version":1,"project_id":"victim","repo_root":"C:/victim","initialized_at":"2026-07-05T00:00:00Z"}"#,
    )?;

    let outcome = delete_project(&stores_dir, "../victim");
    assert!(matches!(outcome, Err(ProjectsError::PathTraversal { .. })));
    assert!(victim.exists(), "path traversal must never delete outside stores_dir");

    fs::remove_dir_all(&parent)?;
    Ok(())
}
