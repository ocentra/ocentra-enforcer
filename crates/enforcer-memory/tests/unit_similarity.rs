//! X06 core parity: `SIMILAR_TO`/`SEMANTICALLY_RELATED` edge
//! materialization ([`enforcer_memory::similarity`]).

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::graph_schema::get_graph_schema_with_similarity;
use enforcer_memory::similarity::{
    proximity_multiplier, semantically_related, similar_to, tokenize_identifier,
    SEMANTICALLY_RELATED_THRESHOLD, SIMILAR_TO_THRESHOLD,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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

fn index_files(files: &[(&str, &str)]) -> TestResult<(tempfile::TempDir, CodeGraph)> {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let mut paths = Vec::new();
    for (rel_name, source) in files {
        let file_path = dir.path().join(rel_name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, source)?;
        paths.push(file_path);
    }
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &paths, &Manifest::default())?;
    Ok((dir, graph))
}

// ---------------------------------------------------------------------
// tokenize_identifier
// ---------------------------------------------------------------------

#[test]
fn tokenize_identifier_splits_camel_case() {
    assert_eq!(
        tokenize_identifier("parseJsonValue"),
        vec!["parse", "json", "value"]
    );
}

#[test]
fn tokenize_identifier_splits_snake_case() {
    assert_eq!(
        tokenize_identifier("parse_json_value"),
        vec!["parse", "json", "value"]
    );
}

#[test]
fn tokenize_identifier_lowercases_everything() {
    // No camelCase break before a run of consecutive uppercase letters
    // (only an uppercase-after-lowercase transition splits), so
    // "HTTPServer" stays a single lowercased token.
    assert_eq!(tokenize_identifier("HTTPServer"), vec!["httpserver"]);
}

#[test]
fn tokenize_identifier_empty_string_yields_no_tokens() {
    assert!(tokenize_identifier("").is_empty());
}

// ---------------------------------------------------------------------
// proximity_multiplier
// ---------------------------------------------------------------------

#[test]
fn proximity_multiplier_same_file_boosts_score() {
    let boost = proximity_multiplier("src/foo.rs", "src/foo.rs");
    assert!(boost > 1.0);
    assert!(boost <= 1.10 + f64::EPSILON);
}

#[test]
fn proximity_multiplier_distant_paths_no_boost() {
    let boost = proximity_multiplier("src/a/x.rs", "lib/b/y.rs");
    assert!((boost - 1.0).abs() < f64::EPSILON);
}

#[test]
fn proximity_multiplier_no_directory_components_no_boost() {
    let boost = proximity_multiplier("a.rs", "b.rs");
    assert!((boost - 1.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------
// similar_to
// ---------------------------------------------------------------------

#[test]
fn similar_to_matches_near_duplicate_names_same_extension() -> TestResult {
    // Same token multiset (order differs only), so name-token Jaccard is
    // exactly 1.0 -- comfortably above SIMILAR_TO_THRESHOLD (0.95).
    let src = r#"
fn parse_json_value() { }
fn value_json_parse() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let edges = similar_to(&graph);
    assert!(
        !edges.is_empty(),
        "expected at least one SIMILAR_TO edge for near-duplicate names"
    );
    for edge in &edges {
        assert!(edge.jaccard >= SIMILAR_TO_THRESHOLD);
        assert!(edge.source_id < edge.target_id);
        assert!(edge.same_file);
    }
    Ok(())
}

#[test]
fn similar_to_unrelated_names_produce_no_edges() -> TestResult {
    let src = r#"
fn alpha() { }
fn totally_different_thing() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let edges = similar_to(&graph);
    assert!(edges.is_empty());
    Ok(())
}

#[test]
fn similar_to_requires_same_file_extension() -> TestResult {
    let rs_src = "fn parse_json_value() { }\n";
    let js_src = "function parse_json_value() { }\n";
    let (_dir, graph) = index_files(&[("a.rs", rs_src), ("b.js", js_src)])?;
    let edges = similar_to(&graph);
    assert!(
        edges.is_empty(),
        "cross-extension pairs must never emit a SIMILAR_TO edge, got {edges:?}"
    );
    Ok(())
}

#[test]
fn similar_to_is_deterministic_across_repeated_calls() -> TestResult {
    let src = r#"
fn parse_json_value() { }
fn parse_json_input() { }
fn parse_json_output() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let first = similar_to(&graph);
    let second = similar_to(&graph);
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn similar_to_empty_graph_yields_no_edges() {
    let graph = CodeGraph::new();
    assert!(similar_to(&graph).is_empty());
}

// ---------------------------------------------------------------------
// semantically_related
// ---------------------------------------------------------------------

#[test]
fn semantically_related_excludes_pairs_already_similar_to() -> TestResult {
    let src = r#"
fn parse_json_value() { }
fn value_json_parse() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let similar = similar_to(&graph);
    assert!(!similar.is_empty());
    let related = semantically_related(&graph);
    // The near-duplicate pair cleared SIMILAR_TO's threshold, so the
    // early-exit rule must keep it out of SEMANTICALLY_RELATED.
    for edge in &related {
        let is_same_pair = similar
            .iter()
            .any(|s| s.source_id == edge.source_id && s.target_id == edge.target_id);
        assert!(!is_same_pair);
    }
    Ok(())
}

#[test]
fn semantically_related_scores_meet_threshold() -> TestResult {
    let src = r#"
fn read_config_file() { }
fn load_config_data() { }
fn unrelated_thing_entirely() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let edges = semantically_related(&graph);
    for edge in &edges {
        assert!(edge.score >= SEMANTICALLY_RELATED_THRESHOLD);
        assert!(edge.score <= 1.0);
        assert!(edge.source_id < edge.target_id);
    }
    Ok(())
}

#[test]
fn semantically_related_requires_same_file_extension() -> TestResult {
    let rs_src = "fn read_config_file() { }\n";
    let js_src = "function load_config_data() { }\n";
    let (_dir, graph) = index_files(&[("a.rs", rs_src), ("b.js", js_src)])?;
    let edges = semantically_related(&graph);
    assert!(
        edges.is_empty(),
        "cross-extension pairs must never emit a SEMANTICALLY_RELATED edge, got {edges:?}"
    );
    Ok(())
}

#[test]
fn semantically_related_is_deterministic_across_repeated_calls() -> TestResult {
    let src = r#"
fn read_config_file() { }
fn load_config_data() { }
fn parse_config_entry() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let first = semantically_related(&graph);
    let second = semantically_related(&graph);
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn semantically_related_empty_graph_yields_no_edges() {
    let graph = CodeGraph::new();
    assert!(semantically_related(&graph).is_empty());
}

// ---------------------------------------------------------------------
// graph_schema integration
// ---------------------------------------------------------------------

#[test]
fn graph_schema_with_similarity_reports_edge_counts() -> TestResult {
    let src = r#"
fn parse_json_value() { }
fn parse_json_input() { }
fn read_config_file() { }
fn load_config_data() { }
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let similar = similar_to(&graph);
    let related = semantically_related(&graph);
    let schema = get_graph_schema_with_similarity(&graph, &similar, &related);

    if !similar.is_empty() {
        let row_count = schema
            .edge_types
            .iter()
            .find(|e| e.edge_type == "SIMILAR_TO")
            .map(|row| row.count);
        assert_eq!(row_count, Some(similar.len()));
    }
    if !related.is_empty() {
        let row_count = schema
            .edge_types
            .iter()
            .find(|e| e.edge_type == "SEMANTICALLY_RELATED")
            .map(|row| row.count);
        assert_eq!(row_count, Some(related.len()));
    }
    Ok(())
}

#[test]
fn graph_schema_with_similarity_omits_zero_count_rows() {
    let graph = CodeGraph::new();
    let schema = get_graph_schema_with_similarity(&graph, &[], &[]);
    assert!(!schema
        .edge_types
        .iter()
        .any(|e| e.edge_type == "SIMILAR_TO" || e.edge_type == "SEMANTICALLY_RELATED"));
}
