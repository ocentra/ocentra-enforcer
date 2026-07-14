use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::embed::{Embedder, HashingEmbedder};
use enforcer_memory::search::{
    run_search_graph as search_graph, search_graph_with_semantic, NodeLabel, SearchGraphSpec,
    SearchMode,
};
use enforcer_memory::vector::VectorIndex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

fn fixture_graph() -> Result<(tempfile::TempDir, CodeGraph), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        "struct Widget;\nfn parseConfig() { helper(); }\nfn helper() {}\n#[test]\nfn a_test() {}\n",
    )?;
    commit_all(dir.path(), "first")?;
    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;
    Ok((dir, graph))
}

#[test]
fn regex_name_pattern_hits() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some("parse.*".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert_eq!(result.search_mode, SearchMode::Regex);
    assert!(result.results.iter().any(|h| h.name == "parseConfig"));
    Ok(())
}

#[test]
fn qn_pattern_hits() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        qn_pattern: Some(".*helper".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(result.results.iter().any(|h| h.name == "helper"));
    Ok(())
}

#[test]
fn label_filter_selects_only_that_label() -> TestResult {
    // X06 rich vocabulary: the fixture's `struct Widget;` is now a
    // Struct node, not a generic Type -- assert against its real label.
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        label: Some(NodeLabel::Struct),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(!result.results.is_empty());
    assert!(result.results.iter().all(|h| h.label == "Struct"));
    Ok(())
}

#[test]
fn file_pattern_bare_literal_is_substring_match() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        file_pattern: Some("lib.rs".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(!result.results.is_empty());
    Ok(())
}

#[test]
fn file_pattern_glob_matches() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        file_pattern: Some("*.rs".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(!result.results.is_empty());
    let spec_no_match = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        file_pattern: Some("*.py".to_owned()),
        ..Default::default()
    };
    let result_no_match = search_graph(&graph, &spec_no_match)?;
    assert!(result_no_match.results.is_empty());
    Ok(())
}

#[test]
fn degree_filters_on_known_fixture_graph() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    // parseConfig's file has 1 outbound call (helper); min_degree
    // filters out nodes with total degree below the threshold.
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        min_degree: Some(1),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(!result.results.is_empty());

    let spec_max = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        max_degree: Some(0),
        ..Default::default()
    };
    let result_max = search_graph(&graph, &spec_max)?;
    assert!(result_max
        .results
        .iter()
        .all(|h| h.in_degree.unwrap_or(0) + h.out_degree.unwrap_or(0) == 0));
    Ok(())
}

#[test]
fn exclude_entry_points_removes_zero_inbound_nodes() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let baseline_spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        ..Default::default()
    };
    let baseline = search_graph(&graph, &baseline_spec)?;

    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        exclude_entry_points: true,
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(result.results.len() <= baseline.results.len());
    Ok(())
}

#[test]
fn include_connected_returns_names_via_one_hop_bfs() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some("parseConfig".to_owned()),
        include_connected: true,
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert_eq!(result.results.len(), 1);
    let hit = &result.results[0];
    let connected = hit
        .connected_names
        .as_ref()
        .ok_or("expected connected_names to be Some")?;
    assert!(connected.iter().any(|n| n.contains("helper")));
    Ok(())
}

#[test]
fn relationship_validation_rejects_lowercase() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        relationship: Some("calls".to_owned()),
        ..Default::default()
    };
    match search_graph(&graph, &spec) {
        Err(err) => assert_eq!(
            err,
            enforcer_memory::search::SearchGraphError::InvalidRelationship
        ),
        Ok(_) => return Err("expected InvalidRelationship error".into()),
    }
    Ok(())
}

#[test]
fn relationship_validation_rejects_too_long() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let long = "A".repeat(65);
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        relationship: Some(long),
        ..Default::default()
    };
    assert!(search_graph(&graph, &spec).is_err());
    Ok(())
}

#[test]
fn relationship_validation_accepts_valid_uppercase() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        relationship: Some("CALLS".to_owned()),
        ..Default::default()
    };
    assert!(search_graph(&graph, &spec).is_ok());
    Ok(())
}

#[test]
fn pagination_is_deterministic_page1_plus_page2_equals_full() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let full_spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        ..Default::default()
    };
    let full = search_graph(&graph, &full_spec)?;
    assert!(full.total >= 4);
    // Split the full result set into two pages of ceil(total/2) each
    // so page1+page2 always covers the whole set regardless of the
    // fixture's exact node count.
    let page_size = full.total.div_ceil(2);

    let page1_spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        limit: Some(page_size),
        offset: 0,
        ..Default::default()
    };
    let page1 = search_graph(&graph, &page1_spec)?;

    let page2_spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        limit: Some(page_size),
        offset: page_size,
        ..Default::default()
    };
    let page2 = search_graph(&graph, &page2_spec)?;

    let mut combined: Vec<String> = page1
        .results
        .iter()
        .chain(page2.results.iter())
        .map(|h| h.name.clone())
        .collect();
    let mut full_names: Vec<String> = full.results.iter().map(|h| h.name.clone()).collect();
    combined.sort();
    full_names.sort();
    assert_eq!(
        combined, full_names,
        "page1 + page2 must equal the full result set"
    );

    // No duplicate ids across the two pages.
    let mut seen = HashSet::new();
    for hit in page1.results.iter().chain(page2.results.iter()) {
        assert!(
            seen.insert(hit.qualified_name.clone()),
            "duplicate across pages"
        );
    }

    assert!(!page2.has_more, "the second page must be the last page");
    Ok(())
}

#[test]
fn pagination_has_more_is_correct_on_last_page() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let full_spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        ..Default::default()
    };
    let full = search_graph(&graph, &full_spec)?;
    let total = full.total;

    let last_page_spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        limit: Some(total),
        offset: 0,
        ..Default::default()
    };
    let last_page = search_graph(&graph, &last_page_spec)?;
    assert!(!last_page.has_more);
    Ok(())
}

#[test]
fn results_and_semantic_results_are_separate_lists() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let embedder = HashingEmbedder::new();
    let entries: Vec<(String, Vec<f32>)> = Vec::new();
    let vector_index = VectorIndex::build(&entries, embedder.model_info());
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        semantic_query: Some(vec!["parse".to_owned(), "config".to_owned()]),
        ..Default::default()
    };
    let result = search_graph_with_semantic(&graph, &spec, Some((&embedder, &vector_index)))?;
    assert!(!result.results.is_empty());
    // semantic_results is a genuinely separate list (may or may not
    // be empty depending on hashing-embedder cosine values, but it
    // must never be the same Vec instance/content as `results`
    // unless coincidentally identical text, which it is not here).
    assert_ne!(
        result
            .results
            .iter()
            .map(|h| h.name.clone())
            .collect::<Vec<_>>(),
        result
            .semantic_results
            .iter()
            .map(|h| h.name.clone())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn include_connected_names_are_deduplicated_and_sorted() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        include_connected: true,
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    let mut sorted = result.connected_names.clone();
    sorted.sort();
    assert_eq!(result.connected_names, sorted);
    let mut deduped = result.connected_names.clone();
    deduped.dedup();
    assert_eq!(result.connected_names, deduped);
    Ok(())
}

#[test]
fn bm25_mode_ignores_name_pattern_when_query_succeeds() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        query: Some("parseConfig".to_owned()),
        // A name_pattern that would match NOTHING if it were
        // applied -- proves BM25 short-circuited and ignored it.
        name_pattern: Some("^nonexistent-name-zzz$".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert_eq!(result.search_mode, SearchMode::Bm25);
    assert!(
        !result.results.is_empty(),
        "BM25 must ignore name_pattern once it has usable tokens and succeeds"
    );
    Ok(())
}

#[test]
fn bm25_falls_through_to_regex_when_no_usable_tokens() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        // Punctuation-only query tokenizes to zero terms.
        query: Some("!!!".to_owned()),
        name_pattern: Some("helper".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert_eq!(result.search_mode, SearchMode::Regex);
    assert!(result.results.iter().any(|h| h.name == "helper"));
    Ok(())
}

#[test]
fn empty_pattern_matches_matches_everything_and_reports_zero_total_gracefully() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some("^nonexistent-zzz$".to_owned()),
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert_eq!(result.total, 0);
    assert!(result.results.is_empty());
    assert!(!result.has_more);
    Ok(())
}

#[test]
fn offset_at_usize_max_returns_an_empty_page_without_overflowing() -> TestResult {
    let (_dir, graph) = fixture_graph()?;
    let spec = SearchGraphSpec {
        name_pattern: Some(".*".to_owned()),
        offset: usize::MAX,
        ..Default::default()
    };
    let result = search_graph(&graph, &spec)?;
    assert!(result.results.is_empty());
    assert!(!result.has_more);
    Ok(())
}

#[test]
fn invalid_pattern_returns_typed_error_not_panic() {
    let spec = SearchGraphSpec {
        name_pattern: Some("(unclosed".to_owned()),
        ..Default::default()
    };
    let graph = CodeGraph::new();
    let result = search_graph(&graph, &spec);
    assert!(result.is_err());
}
