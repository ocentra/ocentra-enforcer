use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::code_search::{search_code, SearchError, SearchMode, SearchQuery};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = std::result::Result<(), Box<dyn Error>>;

fn init_git_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

/// Test-local shorthand for the common `SearchQuery` shape (a single
/// pattern, given mode, no context lines, no limit) so individual
/// tests don't repeat the struct literal.
fn query(pattern: &str, mode: SearchMode) -> SearchQuery<'_> {
    SearchQuery {
        pattern,
        mode,
        context_lines: 0,
        limit: 0,
    }
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

#[test]
fn finds_matches_and_enriches_with_containing_symbol() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        "fn helper() {\n    let needle = 1;\n}\nfn other() {\n    let needle = 2;\n}\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(&graph, dir.path(), &query("needle", SearchMode::Full))?;
    assert_eq!(outcome.total_matches, 2);
    assert_eq!(outcome.hits.len(), 2);
    assert!(outcome
        .hits
        .iter()
        .any(|h| h.containing_symbol.as_deref() == Some("helper")));
    assert!(outcome
        .hits
        .iter()
        .any(|h| h.containing_symbol.as_deref() == Some("other")));
    assert!(outcome.unreadable_files.is_empty());
    Ok(())
}

#[test]
fn ranks_by_structural_importance_inbound_call_degree() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    // `popular` is called twice; `lonely` is never called. Both
    // contain the needle "MARK".
    fs::write(
        &file_path,
        "fn popular() {\n    // MARK\n}\nfn lonely() {\n    // MARK\n}\nfn caller_a() { popular(); }\nfn caller_b() { popular(); }\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(&graph, dir.path(), &query("MARK", SearchMode::Full))?;
    assert_eq!(outcome.hits.len(), 2);
    assert_eq!(
        outcome.hits[0].containing_symbol.as_deref(),
        Some("popular"),
        "the symbol called twice must rank before the never-called one"
    );
    assert!(outcome.hits[0].structural_rank > outcome.hits[1].structural_rank);
    Ok(())
}

#[test]
fn modes_compact_full_files_shape_the_output_correctly() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "fn a() {\n    let x = 1;\n}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let compact = search_code(&graph, dir.path(), &query("let x", SearchMode::Compact))?;
    assert_eq!(compact.hits.len(), 1);
    assert_eq!(compact.files, vec!["lib.rs".to_string()]);

    let full = search_code(
        &graph,
        dir.path(),
        &SearchQuery {
            pattern: "let x",
            mode: SearchMode::Full,
            context_lines: 1,
            limit: 0,
        },
    )?;
    assert_eq!(full.hits.len(), 1);
    assert_eq!(full.hits[0].context_before, vec!["fn a() {".to_string()]);
    assert_eq!(full.hits[0].context_after, vec!["}".to_string()]);

    let files_mode = search_code(&graph, dir.path(), &query("let x", SearchMode::Files))?;
    assert!(
        files_mode.hits.is_empty(),
        "files mode returns no per-line hits"
    );
    assert_eq!(files_mode.files, vec!["lib.rs".to_string()]);
    assert_eq!(
        files_mode.total_matches, 1,
        "total_matches is populated even in files mode"
    );
    Ok(())
}

#[test]
fn limit_truncates_hits_but_total_matches_reflects_the_untruncated_count() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "// needle\n// needle\n// needle\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(
        &graph,
        dir.path(),
        &SearchQuery {
            pattern: "needle",
            mode: SearchMode::Full,
            context_lines: 0,
            limit: 2,
        },
    )?;
    assert_eq!(outcome.hits.len(), 2);
    assert_eq!(outcome.total_matches, 3);
    Ok(())
}

#[test]
fn unreadable_files_are_reported_never_silently_skipped() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "fn a() { /* needle */ }\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    // Delete the file after indexing so the graph still references
    // it but the read fails at search time.
    fs::remove_file(dir.path().join("lib.rs"))?;

    let outcome = search_code(&graph, dir.path(), &query("needle", SearchMode::Full))?;
    assert_eq!(outcome.hits.len(), 0);
    assert_eq!(outcome.unreadable_files.len(), 1);
    assert_eq!(outcome.unreadable_files[0].rel_path, "lib.rs");
    Ok(())
}

#[test]
fn invalid_regex_pattern_is_a_typed_error() {
    let graph = CodeGraph::new();
    let outcome = search_code(
        &graph,
        Path::new("."),
        &query("(unterminated[", SearchMode::Full),
    );
    assert!(matches!(outcome, Err(SearchError::InvalidPattern { .. })));
}

#[test]
fn regex_pattern_matches_not_just_literal_substrings() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "fn foo_1() {}\nfn foo_2() {}\nfn bar() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(&graph, dir.path(), &query(r"foo_\d", SearchMode::Full))?;
    assert_eq!(outcome.total_matches, 2);
    Ok(())
}

#[test]
fn function_symbols_outrank_type_symbols_at_equal_call_degree() -> TestResult {
    // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
    // §8.3: label boost is +10 for Function/Method, +0 for anything
    // else -- neither symbol here is ever called, so the only
    // difference is the Function boost.
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "struct MarkerType;\nfn marker_fn() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(
        &graph,
        dir.path(),
        &query("Marker|marker", SearchMode::Full),
    )?;
    let fn_rank = outcome
        .hits
        .iter()
        .find(|h| h.containing_symbol.as_deref() == Some("marker_fn"))
        .map(|h| h.structural_rank)
        .ok_or("expected a marker_fn hit")?;
    let type_rank = outcome
        .hits
        .iter()
        .find(|h| h.containing_symbol.as_deref() == Some("MarkerType"))
        .map(|h| h.structural_rank)
        .ok_or("expected a MarkerType hit")?;
    assert!(
        fn_rank > type_rank,
        "fn_rank={fn_rank} type_rank={type_rank}"
    );
    Ok(())
}

#[test]
fn test_symbols_are_penalized_below_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, "#[test]\nfn a_test() {\n    // marker\n}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(&graph, dir.path(), &query("marker", SearchMode::Full))?;
    assert_eq!(outcome.hits.len(), 1);
    assert_eq!(
        outcome.hits[0].structural_rank, -5,
        "an uncalled test symbol scores exactly the -5 test penalty"
    );
    Ok(())
}

#[test]
fn vendored_paths_are_penalized() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let vendor_dir = dir.path().join("vendor");
    fs::create_dir_all(&vendor_dir)?;
    let file_path = vendor_dir.join("lib.rs");
    fs::write(&file_path, "fn vendored_fn() {\n    // marker\n}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let outcome = search_code(&graph, dir.path(), &query("marker", SearchMode::Full))?;
    assert_eq!(outcome.hits.len(), 1);
    // +10 Function boost, 0 in_degree, -50 vendored penalty = -40.
    assert_eq!(outcome.hits[0].structural_rank, -40);
    Ok(())
}
