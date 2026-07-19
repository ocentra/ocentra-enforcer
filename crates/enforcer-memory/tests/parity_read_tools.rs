//! Integration coverage for X06.P1's parity read tools --
//! [`enforcer_memory::snippet`], [`enforcer_memory::graph_schema`],
//! [`enforcer_memory::code_search`], [`enforcer_memory::projects`] --
//! against a real, throwaway git working tree copied from
//! `tests/fixtures/memory/parity_read_tools/` (content hashes and
//! byte-exact snippet extraction are meaningless without real files on
//! disk, so this test does not mock the filesystem or git).

use enforcer_domain::memory_types::{CodeSearchMode, FreshnessState};
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::code_search::{search_code, SearchError, SearchQuery};
use enforcer_memory::graph_schema::get_graph_schema;
use enforcer_memory::projects::{delete_project, index_status, list_projects, ProjectsError};
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

fn search_query(pattern: &str, mode: CodeSearchMode) -> SearchQuery<'_> {
    SearchQuery {
        pattern: pattern.into(),
        mode,
        context_lines: 0.into(),
        limit: 0.into(),
    }
}

// -- get_code_snippet ------------------------------------------------

#[test]
fn snippet_is_byte_exact_hash_equal_to_an_independent_file_slice() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let snippet = get_code_snippet(&graph, dir.path(), "service.rs::helper", false)?;

    let raw = fs::read(dir.path().join("service.rs"))?;
    let independent_slice = &raw[snippet.start_byte.get()..snippet.end_byte.get()];
    assert_eq!(snippet.bytes.as_slice(), independent_slice);
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
    assert_eq!(usize::from(schema.total_nodes()), graph.nodes().len());
    assert_eq!(
        usize::from(schema.total_edges()),
        graph.imports().len() + graph.calls().len() + graph.routes().len()
    );

    // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
    // §3.2: the baseline orders node_labels/edge_types by DESCENDING
    // count, not alphabetically -- verify labels are non-increasing by
    // count (ties may appear in either alphabetical sub-order, which
    // src/graph_schema.rs's own unit tests pin precisely; this
    // integration test only re-confirms the ordering invariant holds
    // end-to-end against the fixture repo).
    let counts: Vec<usize> = schema.labels.iter().map(|l| l.count.get()).collect();
    let mut sorted_desc = counts.clone();
    sorted_desc.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(
        counts, sorted_desc,
        "labels must be ordered by descending count (baseline parity), got {:?}",
        schema.labels
    );

    assert!(schema
        .edge_types
        .iter()
        .any(|e| e.edge_type == "Route" && e.count >= 1));
    assert!(schema
        .edge_types
        .iter()
        .any(|e| e.edge_type == "Calls" && e.count >= 2));

    // edge_types must obey the same descending-by-count ordering as
    // labels (module docs' "same ordering guarantee").
    let edge_counts: Vec<usize> = schema.edge_types.iter().map(|e| e.count.get()).collect();
    let mut edge_sorted_desc = edge_counts.clone();
    edge_sorted_desc.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(
        edge_counts, edge_sorted_desc,
        "edge_types must be ordered by descending count (baseline parity), got {:?}",
        schema.edge_types
    );
    Ok(())
}

// -- search_code --------------------------------------------------------

#[test]
fn search_code_enriches_hits_with_containing_symbol_and_ranks_by_inbound_degree() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let outcome = search_code(
        &graph,
        dir.path(),
        &search_query("value", CodeSearchMode::Full),
    )?;
    assert!(outcome.total_matches.get() >= 1);
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
        .ok_or("expected a hit inside helper")?;
    assert!(helper_hit.structural_rank.get() >= 2, "{:?}", helper_hit);
    Ok(())
}

#[test]
fn search_code_modes_compact_full_files_are_distinct() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;

    let files_mode = search_code(
        &graph,
        dir.path(),
        &search_query("helper", CodeSearchMode::Files),
    )?;
    assert!(files_mode.hits.is_empty());
    assert!(files_mode.files.iter().any(|path| path == "service.rs"));

    let full_mode = search_code(
        &graph,
        dir.path(),
        &SearchQuery {
            pattern: "helper".into(),
            mode: CodeSearchMode::Full,
            context_lines: 1.into(),
            limit: 0.into(),
        },
    )?;
    assert!(full_mode
        .hits
        .iter()
        .any(|hit| hit.containing_symbol.as_deref() == Some("helper")));
    Ok(())
}

#[test]
fn search_code_reports_unreadable_files_never_silently_skips() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;
    fs::remove_file(dir.path().join("service.rs"))?;

    let outcome = search_code(
        &graph,
        dir.path(),
        &search_query("helper", CodeSearchMode::Full),
    )?;
    assert!(outcome
        .unreadable_files
        .iter()
        .any(|f| f.rel_path == "service.rs"));
    Ok(())
}

#[test]
fn search_code_invalid_pattern_is_a_typed_error() -> TestResult {
    let (dir, graph) = indexed_fixture_repo()?;
    let outcome = search_code(
        &graph,
        dir.path(),
        &search_query("(unterminated[", CodeSearchMode::Full),
    );
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

#[test]
fn projects_list_status_delete_round_trip() -> TestResult {
    let stores_dir = temp_stores_dir("roundtrip");
    let root: enforcer_domain::paths::RepoRoot = "C:/Projects/parity-roundtrip".parse()?;
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
    assert!(
        victim.exists(),
        "path traversal must never delete outside stores_dir"
    );

    fs::remove_dir_all(&parent)?;
    Ok(())
}
