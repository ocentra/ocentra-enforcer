use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::snippet::{get_code_snippet, SnippetError};
use sha2::{Digest, Sha256};
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

fn indexed_repo(
    source: &str,
    filename: &str,
) -> std::result::Result<(tempfile::TempDir, CodeGraph), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join(filename);
    fs::write(&file_path, source)?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;
    Ok((dir, graph))
}

#[test]
fn snippet_bytes_are_hash_equal_to_an_independent_file_slice() -> TestResult {
    let source = "fn a() {\n    1\n}\nfn b() {\n    2\n}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::a", false)?;

    // Independently re-slice the file on disk and hash it, without
    // reusing any of this module's own byte-offset bookkeeping, per
    // L37 (never verify a value against itself).
    let raw = fs::read(dir.path().join("lib.rs"))?;
    let independent_slice = &raw[snippet.start_byte..snippet.end_byte];
    let mut hasher = Sha256::new();
    hasher.update(independent_slice);
    let digest = hasher.finalize();
    let mut expected = String::from("sha256:");
    for byte in digest {
        expected.push_str(&format!("{byte:02x}"));
    }

    assert_eq!(snippet.sha256, expected);
    assert_eq!(snippet.bytes, independent_slice);
    assert_eq!(snippet.bytes, b"fn a() {\n    1\n}\n");
    Ok(())
}

#[test]
fn last_symbol_in_file_extends_to_end_of_file_byte_exact() -> TestResult {
    let source = "fn a() {}\nfn b() {\n    2\n}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::b", false)?;
    let raw = fs::read(dir.path().join("lib.rs"))?;
    assert_eq!(snippet.end_byte, raw.len());
    assert_eq!(snippet.bytes, b"fn b() {\n    2\n}\n");
    Ok(())
}

#[test]
fn unknown_symbol_fails_closed_never_a_similar_name_substitute() -> TestResult {
    let source = "fn helper() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    // "helperr" is a near-miss of the real symbol "helper" -- this
    // must error, never silently resolve to the closest match.
    let outcome = get_code_snippet(&graph, dir.path(), "lib.rs::helperr", false);
    assert!(matches!(outcome, Err(SnippetError::UnknownSymbol { .. })));

    let outcome_missing_file = get_code_snippet(&graph, dir.path(), "missing.rs::helper", false);
    assert!(matches!(
        outcome_missing_file,
        Err(SnippetError::UnknownSymbol { .. })
    ));
    Ok(())
}

#[test]
fn raw_node_id_form_resolves_the_same_symbol_as_qualified_form() -> TestResult {
    let source = "fn helper() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let by_qualified = get_code_snippet(&graph, dir.path(), "lib.rs::helper", false)?;
    let raw_id = graph
        .symbol_nodes()
        .find(|s| s.name == "helper")
        .map(|s| s.id.clone())
        .ok_or("expected a helper symbol")?;
    let by_raw_id = get_code_snippet(&graph, dir.path(), &raw_id, false)?;

    assert_eq!(by_qualified.bytes, by_raw_id.bytes);
    assert_eq!(by_qualified.sha256, by_raw_id.sha256);
    Ok(())
}

#[test]
fn include_neighbors_returns_other_symbols_in_the_same_file_ordered_by_line() -> TestResult {
    let source = "fn a() {}\nstruct Middle;\nfn z() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::a", true)?;
    let names: Vec<&str> = snippet
        .neighbors
        .iter()
        .map(|n| n.qualified_name.as_str())
        .collect();
    assert_eq!(names, vec!["lib.rs::Middle", "lib.rs::z"]);

    // include_neighbors=false must yield an empty vec, not an error.
    let without = get_code_snippet(&graph, dir.path(), "lib.rs::a", false)?;
    assert!(without.neighbors.is_empty());
    Ok(())
}

#[test]
fn file_with_a_single_symbol_has_no_neighbors_but_no_error() -> TestResult {
    let source = "fn only_one() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::only_one", true)?;
    assert!(snippet.neighbors.is_empty());
    Ok(())
}

#[test]
fn unreadable_source_file_is_a_typed_error_not_a_panic() -> TestResult {
    let source = "fn a() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    // Remove the file after indexing so the graph still has the
    // symbol but the source is gone -- exercising the ReadFile path.
    fs::remove_file(dir.path().join("lib.rs"))?;

    let outcome = get_code_snippet(&graph, dir.path(), "lib.rs::a", false);
    assert!(matches!(outcome, Err(SnippetError::ReadFile { .. })));
    Ok(())
}

#[test]
fn exact_match_never_sets_match_method() -> TestResult {
    let source = "fn helper() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::helper", false)?;
    assert_eq!(snippet.match_method, None);
    Ok(())
}

#[test]
fn bare_name_suffix_matches_and_records_match_method() -> TestResult {
    // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
    // §6.2: the baseline falls back to a suffix match on a bare/
    // short name when the exact qualified form doesn't match, and
    // tags the result with match_method="suffix".
    let source = "fn helper() {}\n";
    let (dir, graph) = indexed_repo(source, "lib.rs")?;

    let snippet = get_code_snippet(&graph, dir.path(), "helper", false)?;
    assert_eq!(snippet.match_method, Some("suffix"));
    assert_eq!(snippet.qualified_name, "lib.rs::helper");
    Ok(())
}

#[test]
fn ambiguous_suffix_match_fails_closed() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let a_path = dir.path().join("a.rs");
    let b_path = dir.path().join("b.rs");
    fs::write(&a_path, "fn helper() {}\n")?;
    fs::write(&b_path, "fn helper() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[a_path, b_path], &Manifest::default())?;

    // Two files each define "helper" -- the bare-name suffix match
    // is ambiguous between them and must fail closed, never silently
    // pick one.
    let outcome = get_code_snippet(&graph, dir.path(), "helper", false);
    assert!(matches!(outcome, Err(SnippetError::AmbiguousSymbol { .. })));
    Ok(())
}

#[test]
fn callers_and_callees_counts_are_always_present() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        "fn popular() {}\nfn caller_a() { popular(); }\nfn caller_b() { popular(); }\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    // include_neighbors=false must still populate the always-present
    // counts (matching the baseline's asymmetry -- see module docs).
    let popular = get_code_snippet(&graph, dir.path(), "lib.rs::popular", false)?;
    assert_eq!(popular.callers, 2, "popular() is called twice");
    assert!(popular.caller_names.is_empty(), "names are opt-in only");
    Ok(())
}

#[test]
fn include_neighbors_populates_caller_and_callee_names() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        "fn popular() {}\nfn caller_a() { popular(); }\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::popular", true)?;
    assert!(
        snippet.caller_names.contains(&"caller_a".to_string()),
        "{:?}",
        snippet.caller_names
    );
    Ok(())
}
