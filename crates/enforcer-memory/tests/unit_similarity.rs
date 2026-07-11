//! X06 core parity: `SIMILAR_TO`/`SEMANTICALLY_RELATED` edge
//! materialization ([`enforcer_memory::similarity`]).

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use enforcer_memory::artifacts::{export_graph_artifact, import_graph_artifact, GraphSnapshot};
use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest};
use enforcer_memory::graph_schema::{
    get_graph_schema, get_graph_schema_with_similarity, get_graph_schema_with_similarity_modes,
};
use enforcer_memory::similarity::{
    proximity_multiplier, semantically_related, similar_to, similar_to_body_shingles,
    similar_to_identifier_tokens, tokenize_identifier, SimilarityMode,
    SEMANTICALLY_RELATED_THRESHOLD, SIMILAR_TO_MAX_EDGES_PER_NODE, SIMILAR_TO_THRESHOLD,
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

fn long_rust_fn(name: &str) -> String {
    format!(
        r#"
fn {name}(input: &str) -> usize {{
    let trimmed = input.trim();
    let mut total = 0usize;
    for segment in trimmed.split(',') {{
        let normalized = segment.trim();
        if normalized.is_empty() {{
            continue;
        }}
        total += normalized.len();
        total += normalized.bytes().filter(|b| *b == b'a').count();
        total += normalized.split('_').count();
    }}
    total
}}
"#
    )
}

fn long_js_fn(name: &str) -> String {
    format!(
        r#"
function {name}(input) {{
  const trimmed = input.trim();
  let total = 0;
  for (const segment of trimmed.split(',')) {{
    const normalized = segment.trim();
    if (!normalized) {{
      continue;
    }}
    total += normalized.length;
    total += [...normalized].filter(ch => ch === 'a').length;
    total += normalized.split('_').length;
  }}
  return total;
}}
"#
    )
}

fn function_fp(graph: &CodeGraph, name: &str) -> Option<(String, usize)> {
    graph.nodes().iter().find_map(|node| match node {
        CodeNode::Function(sym)
        | CodeNode::Method(sym)
        | CodeNode::Test(sym)
        | CodeNode::Lambda(sym)
            if sym.name == name =>
        {
            sym.source_body_fingerprint
                .as_ref()
                .and_then(|fp| fp.fp.clone().zip(fp.k))
        }
        _ => None,
    })
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
// SIMILAR_TO baseline MinHash contract
// ---------------------------------------------------------------------

#[test]
fn fingerprint_uses_64_slot_hex_and_enforces_30_token_minimum() -> TestResult {
    let src = format!(
        "{}\nfn short_clone() {{ short_call(); }}\n",
        long_rust_fn("long_clone")
    );
    let (_dir, graph) = index_files(&[("lib.rs", src.as_str())])?;

    let long_fp = function_fp(&graph, "long_clone").ok_or("missing long fingerprint")?;
    assert_eq!(long_fp.0.len(), 512);
    assert_eq!(long_fp.1, 64);
    assert!(long_fp.0.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert!(
        function_fp(&graph, "short_clone").is_none(),
        "functions below the 30-token floor must not get persisted fp evidence"
    );
    Ok(())
}

#[test]
fn fingerprint_persists_through_artifact_roundtrip() -> TestResult {
    let (dir, graph) = index_files(&[("lib.rs", long_rust_fn("persisted_clone").as_str())])?;
    let expected = function_fp(&graph, "persisted_clone").ok_or("missing source fingerprint")?;

    let root = PathBuf::from(dir.path());
    let snapshot = GraphSnapshot::from_code_graph(&graph);
    export_graph_artifact(&root, &snapshot, "demo", None, "2026-07-09T00:00:00Z")?;
    let (imported, _meta) = import_graph_artifact(&root)?;

    let imported_fp = imported
        .symbols
        .iter()
        .find(|symbol| symbol.name == "persisted_clone")
        .and_then(|symbol| symbol.source_body_fingerprint.as_ref())
        .ok_or("missing imported fingerprint")?;
    assert_eq!(imported_fp.fp, Some(expected.0));
    assert_eq!(imported_fp.k, Some(expected.1));
    Ok(())
}

#[test]
fn similar_to_matches_identical_long_bodies_same_extension() -> TestResult {
    let src = format!(
        "{}\n{}",
        long_rust_fn("parse_widget_config"),
        long_rust_fn("load_widget_settings")
    );
    let (_dir, graph) = index_files(&[("lib.rs", src.as_str())])?;
    let edges = similar_to(&graph);
    assert!(
        !edges.is_empty(),
        "expected at least one SIMILAR_TO edge for matching 64-slot MinHash fingerprints"
    );
    for edge in &edges {
        assert_eq!(edge.mode, SimilarityMode::MinHashFingerprint);
        assert!(edge.jaccard >= SIMILAR_TO_THRESHOLD);
        assert!(edge.source_id < edge.target_id);
        assert!(edge.same_file);
    }
    Ok(())
}

#[test]
fn similar_to_requires_same_file_extension() -> TestResult {
    let rs_src = long_rust_fn("parse_widget_config");
    let js_src = long_js_fn("parseWidgetConfig");
    let (_dir, graph) = index_files(&[("a.rs", rs_src.as_str()), ("b.js", js_src.as_str())])?;
    let edges = similar_to(&graph);
    assert!(
        edges.is_empty(),
        "cross-extension pairs must never emit a SIMILAR_TO edge, got {edges:?}"
    );
    Ok(())
}

#[test]
fn similar_to_is_deterministic_across_repeated_calls() -> TestResult {
    let src = format!(
        "{}\n{}\n{}",
        long_rust_fn("parse_json_value"),
        long_rust_fn("parse_json_input"),
        long_rust_fn("parse_json_output")
    );
    let (_dir, graph) = index_files(&[("lib.rs", src.as_str())])?;
    let first = similar_to(&graph);
    let second = similar_to(&graph);
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn similar_to_respects_threshold_cap_and_minhash_contract() -> TestResult {
    let mut files: Vec<(String, String)> = (0..12)
        .map(|idx| {
            (
                format!("clone_{idx}.rs"),
                long_rust_fn(&format!("clone_{idx}")),
            )
        })
        .collect();
    files.push(("clone.js".to_owned(), long_js_fn("cloneJs")));
    let owned: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, source)| (path.as_str(), source.as_str()))
        .collect();

    let (_dir, graph) = index_files(&owned)?;
    let edges = similar_to(&graph);
    assert!(
        !edges.is_empty(),
        "expected MinHash SIMILAR_TO edges for identical long bodies"
    );
    assert!(edges
        .iter()
        .all(|edge| edge.mode == SimilarityMode::MinHashFingerprint));
    assert!(edges
        .iter()
        .all(|edge| edge.jaccard >= SIMILAR_TO_THRESHOLD));
    assert!(edges
        .iter()
        .all(|edge| !edge.source_id.ends_with("cloneJs")));

    let mut counts = HashMap::<&str, usize>::new();
    for edge in &edges {
        *counts.entry(edge.source_id.as_str()).or_insert(0) += 1;
        *counts.entry(edge.target_id.as_str()).or_insert(0) += 1;
    }
    assert!(counts
        .values()
        .all(|count| *count <= SIMILAR_TO_MAX_EDGES_PER_NODE));
    Ok(())
}

#[test]
fn similar_to_empty_graph_yields_no_edges() {
    let graph = CodeGraph::new();
    assert!(similar_to(&graph).is_empty());
}

// ---------------------------------------------------------------------
// Additive similarity modes
// ---------------------------------------------------------------------

#[test]
fn similar_to_body_shingles_matches_identical_short_bodies_with_different_names() -> TestResult {
    let src = r#"
fn parse_widget_config(path: &str) -> String {
    path.trim().to_string()
}

fn parseWidgetConfig(path: &str) -> String {
    path.trim().to_string()
}
"#;
    let (_dir, graph) = index_files(&[("lib.rs", src)])?;
    let edges = similar_to_body_shingles(&graph);
    assert!(
        !edges.is_empty(),
        "expected body-shingle additive edge for equivalent short function bodies"
    );
    assert!(edges
        .iter()
        .all(|edge| edge.mode == SimilarityMode::BodyShingle));
    assert!(edges
        .iter()
        .any(|edge| edge.jaccard >= SIMILAR_TO_THRESHOLD));
    Ok(())
}

#[test]
fn rust_identifier_signal_stays_rust_only() -> TestResult {
    let rust_src = r#"
fn parse_widget_config(path: &str) -> String {
    path.trim().to_string()
}

fn parseWidgetConfig(path: &str) -> String {
    path.trim().to_string()
}
"#;
    let js_src = r#"
function parse_widget_config(path) {
  return path.trim();
}

function parseWidgetConfig(path) {
  return path.trim();
}
"#;
    let (_dir, graph) = index_files(&[("lib.rs", rust_src), ("helpers.js", js_src)])?;
    let edges = similar_to_identifier_tokens(&graph);
    assert_eq!(
        edges.len(),
        1,
        "only the Rust pair should emit identifier-token evidence"
    );
    assert!(edges
        .iter()
        .all(|edge| edge.mode == SimilarityMode::IdentifierToken));
    Ok(())
}

// ---------------------------------------------------------------------
// SEMANTICALLY_RELATED
// ---------------------------------------------------------------------

#[test]
fn semantically_related_excludes_pairs_already_similar_to() -> TestResult {
    let src = format!(
        "{}\n{}",
        long_rust_fn("parse_json_value"),
        long_rust_fn("load_widget_settings")
    );
    let (_dir, graph) = index_files(&[("lib.rs", src.as_str())])?;
    let similar = similar_to(&graph);
    assert_eq!(
        similar.len(),
        1,
        "the two matching long Rust functions must produce exactly one baseline similarity edge"
    );
    let related = semantically_related(&graph);
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
fn graph_schema_surfaces_fp_property_without_fabricating_similarity_row() -> TestResult {
    let (_dir, graph) = index_files(&[("lib.rs", long_rust_fn("schema_probe").as_str())])?;
    let schema = get_graph_schema(&graph);
    let function_row = schema
        .labels
        .iter()
        .find(|row| row.label == "Function")
        .ok_or("missing Function row")?;
    assert!(function_row.properties.iter().any(|prop| prop == "fp"));
    assert!(function_row.properties.iter().any(|prop| prop == "k"));

    let similarity_schema = get_graph_schema_with_similarity(&graph, &[], &[]);
    assert!(
        !similarity_schema
            .edge_types
            .iter()
            .any(|row| row.edge_type == "SIMILAR_TO"),
        "schema must not invent a baseline SIMILAR_TO row from fp evidence alone"
    );
    Ok(())
}

#[test]
fn graph_schema_with_similarity_reports_edge_counts() -> TestResult {
    let src = format!(
        "{}\n{}\nfn read_config_file() {{}}\nfn load_config_data() {{}}\n",
        long_rust_fn("parse_json_value"),
        long_rust_fn("parse_json_input")
    );
    let (_dir, graph) = index_files(&[("lib.rs", src.as_str())])?;
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

#[test]
fn graph_schema_with_similarity_modes_reports_additive_rows_separately() -> TestResult {
    let rust_src = r#"
fn parse_widget_config(path: &str) -> String {
    path.trim().to_string()
}

fn parseWidgetConfig(path: &str) -> String {
    path.trim().to_string()
}
"#;
    let js_src = r#"
function parse_widget_config(path) {
  return path.trim();
}

function parseWidgetConfig(path) {
  return path.trim();
}
"#;
    let (_dir, graph) = index_files(&[("lib.rs", rust_src), ("helpers.js", js_src)])?;
    let body = similar_to_body_shingles(&graph);
    let identifier = similar_to_identifier_tokens(&graph);
    let schema = get_graph_schema_with_similarity_modes(&graph, &body, &identifier, &[]);

    assert!(schema
        .edge_types
        .iter()
        .any(|edge| edge.edge_type == "BODY_SHINGLE_SIMILAR_TO" && edge.count == body.len()));
    assert!(!schema
        .edge_types
        .iter()
        .any(|edge| edge.edge_type == "SIMILAR_TO"));
    assert!(schema.edge_types.iter().any(|edge| {
        edge.edge_type == "RUST_IDENTIFIER_SIMILAR_TO" && edge.count == identifier.len()
    }));
    Ok(())
}
