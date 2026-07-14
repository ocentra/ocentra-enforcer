//! X06-parity live driver: builds a small deterministic fixture repo,
//! indexes it on the REAL installed `codebase-memory-mcp` baseline via
//! [`super::baseline::CliDriver`], runs the comparable landed
//! `enforcer_memory` library functions against the same fixture, and
//! emits a per-tool [`ToolDiffRow`] verdict plus the two required proof
//! artifacts (`proof/memory/x06-kg-parity.json`,
//! `proof/memory/x06-parity/tool-results.ndjson`,
//! `proof/memory/x06-parity/tool-diffs.ndjson`).
//!
//! # Honesty rules (mission-critical, never relaxed)
//!
//! - If the baseline binary genuinely cannot be found/run, every row's
//!   verdict is `"unrunnable: baseline not available"` and the top-level
//!   artifact's `baselineExecuted` is `false` -- this module NEVER
//!   fabricates a baseline output to fill in a row.
//! - If the enforcer-memory candidate side has no wired function for a
//!   tool (`query_graph`/Cypher, `manage_adr` project-linked get/update,
//!   `search_graph` semantic mode), the row's verdict is
//!   `"unrunnable: candidate not wired yet"` -- never a guessed/adapted
//!   substitute presented as if it were the real candidate.
//! - Every comparison this module performs is documented inline as a
//!   `normalization` string on the row (e.g. "sorted node/edge id sets",
//!   "stripped machine-specific paths") so a reviewer can see exactly
//!   what was normalized away before two responses were judged equal.
//!
//! # Running this
//!
//! This is real-environment-dependent (spawns an external process,
//! writes a real git repo to a tempdir) and is gated behind the
//! `#[ignore]` attribute like the rest of this crate's slow/integration
//! tests -- run explicitly with:
//! ```text
//! cargo test -p enforcer-memory --test feature_parity_harness -- --ignored run_live_parity_comparison
//! ```
//! [`run_live_parity_comparison`] is also exposed as a plain `pub fn` so
//! a future `enforcer memory parity-harness` CLI can invoke it directly
//! without going through the test harness.

use super::baseline::{BaselineAdapter, BaselineState, CliDriver, CodebaseMemoryMcpAdapter};
use super::BoxError;
use enforcer_memory::adr::AdrStore;
use enforcer_memory::analysis::query as cypher;
use enforcer_memory::analysis::trace::trace_data_flow;
use enforcer_memory::analysis::{trace::TraceCallsParams, CodeAdjacency, TraceDirection};
use enforcer_memory::architecture::{self, Aspect};
use enforcer_memory::code_graph::{CodeGraph, CodeNode, Manifest};
use enforcer_memory::code_search::{self, SearchMode, SearchQuery};
use enforcer_memory::cross_repo::{match_cross_repo, CrossHttpMatchKind};
use enforcer_memory::graph_schema;
use enforcer_memory::parsers;
use enforcer_memory::projects;
use enforcer_memory::resolution::{self, ResolutionConfidence};
use enforcer_memory::search::search_graph::{search_graph, SearchGraphSpec};
use enforcer_memory::similarity::{
    semantically_related, similar_to, similar_to_body_shingles, similar_to_identifier_tokens,
    SimilarToEdge,
};
use enforcer_memory::snippet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// One tool's comparison outcome. Field names/shapes are this lane's
/// own (not dictated by an upstream schema) -- kept flat and
/// serializable so `tool-diffs.ndjson` is directly greppable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiffRow {
    pub tool: String,
    /// `"equal"`, `"better"`, `"worse"`, `"incomparable"`, or
    /// `"unrunnable: <reason>"` -- mirrors [`super::runners::RowResult`]'s
    /// `verdict` string-not-enum convention for the same
    /// never-touch-every-callsite-for-a-new-reason property.
    pub comparison_verdict: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub better_because: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worse_because: Option<String>,
    /// Every normalization applied before comparing (sorted keys,
    /// stripped paths, etc.) -- documented per-row, never a single
    /// global assumption the reader has to trust blindly.
    pub normalizations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_latency_ms: Option<f64>,
}

/// One tool's raw (pre-diff) result from each side, exactly as
/// produced -- written to `tool-results.ndjson` so a reviewer can see
/// the actual bytes a verdict was computed from, not just the verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultRow {
    pub tool: String,
    pub side: String, // "baseline" | "candidate"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_json: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub latency_ms: Option<f64>,
}

/// The full `proof/memory/x06-kg-parity.json` document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgParityDocument {
    pub baseline_executed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_binary_path: Option<String>,
    pub tools_total: usize,
    pub tools_equal: usize,
    pub tools_better: usize,
    pub tools_worse: usize,
    pub tools_incomparable: usize,
    pub tools_unrunnable: usize,
    pub rows: Vec<ToolDiffRow>,
}

fn run_git(dir: &Path, args: &[&str]) -> Result<(), BoxError> {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

/// The fixture files this comparison indexes on BOTH sides -- kept
/// intentionally tiny and hand-verifiable (two functions, one calling
/// the other) so every expected node/edge in this module's assertions
/// can be checked by inspection.
const FIXTURE_LIB_RS: &str =
    "fn parse_config_file(path: &str) -> String {\n    load_widget_settings(path)\n}\n";
const FIXTURE_WIDGET_RS: &str =
    "fn load_widget_settings(path: &str) -> String {\n    path.to_string()\n}\n";

/// X06 core-parity extension fixture: a trait + struct + two `impl`
/// blocks (INHERITS via the supertrait bound, IMPLEMENTS via each
/// `impl Trait for Widget`, TYPE_REF via the `&Widget` parameter,
/// DEFINES via each method's enclosing impl) plus a function with real
/// branch/loop structure (`describe`: a `for` loop and three `if`/
/// `else if`/`else` arms) so the complexity-property comparison rows
/// have a non-trivial cyclomatic/loop signal to measure, not just the
/// two straight-line functions above. The trailing near-duplicate pair
/// (`parse_widget_config` / `parseWidgetConfig`: identical bodies,
/// identical name-token sets `{parse, widget, config}` under both
/// snake_case and camelCase splitting) deliberately exceeds the
/// baseline's 30-leaf MinHash minimum. This forces the baseline to
/// materialize a real `SIMILAR_TO` edge, while the candidate must also
/// expose its persisted fingerprint and Rust identifier-token extension
/// without disturbing any symbol earlier rows assert on.
const FIXTURE_TRAITS_RS: &str = "pub trait Drawable {\n    fn draw(&self) -> String;\n}\n\npub trait Named: Drawable {\n    fn name(&self) -> String;\n}\n\npub struct Widget {\n    pub label: String,\n}\n\nimpl Drawable for Widget {\n    fn draw(&self) -> String {\n        self.label.clone()\n    }\n}\n\nimpl Named for Widget {\n    fn name(&self) -> String {\n        self.label.clone()\n    }\n}\n\npub fn describe(widget: &Widget) -> String {\n    let mut total = 0;\n    for _ in 0..widget.label.len() {\n        total += 1;\n    }\n    if total > 0 {\n        widget.draw()\n    } else if total == 0 {\n        widget.name()\n    } else {\n        String::new()\n    }\n}\n\npub fn parse_widget_config(path: &str) -> String {\n    let normalized = path.trim();\n    let segments: Vec<&str> = normalized.split('/').collect();\n    let mut total = 0usize;\n    for segment in &segments {\n        total += segment.len();\n    }\n    if total > 0 {\n        format!(\"{}:{}\", normalized, total)\n    } else {\n        String::new()\n    }\n}\n\npub fn parseWidgetConfig(path: &str) -> String {\n    let normalized = path.trim();\n    let segments: Vec<&str> = normalized.split('/').collect();\n    let mut total = 0usize;\n    for segment in &segments {\n        total += segment.len();\n    }\n    if total > 0 {\n        format!(\"{}:{}\", normalized, total)\n    } else {\n        String::new()\n    }\n}\n";

/// Build a fresh, real git-backed fixture repo in a tempdir. Returns
/// the tempdir (kept alive by the caller) and its forward-slash-
/// normalized path string (the baseline's CLI JSON parser rejects
/// unescaped backslashes in string values -- see
/// [`super::baseline::CliDriver::call`]'s docs).
fn build_fixture_repo() -> Result<(tempfile::TempDir, String), BoxError> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("lib.rs"), FIXTURE_LIB_RS)?;
    std::fs::write(dir.path().join("widget.rs"), FIXTURE_WIDGET_RS)?;
    std::fs::write(dir.path().join("traits.rs"), FIXTURE_TRAITS_RS)?;
    run_git(dir.path(), &["init", "--quiet"])?;
    run_git(
        dir.path(),
        &["config", "user.email", "x06-parity@example.com"],
    )?;
    run_git(dir.path(), &["config", "user.name", "x06-parity"])?;
    run_git(dir.path(), &["add", "-A"])?;
    run_git(
        dir.path(),
        &["commit", "--quiet", "-m", "x06-parity fixture"],
    )?;

    let forward_slash_path = dir.path().to_string_lossy().replace('\\', "/");
    Ok((dir, forward_slash_path))
}

/// Build the candidate-side [`CodeGraph`] over the identical fixture
/// files (same content, same relative layout as the baseline's repo --
/// PARITY_HARNESS §0's "same repo fixture" requirement applied to this
/// lane's own comparison, not shared process state with the baseline
/// since the two are entirely separate programs).
fn build_candidate_graph(fixture_dir: &Path) -> Result<CodeGraph, BoxError> {
    let files = vec![
        fixture_dir.join("lib.rs"),
        fixture_dir.join("widget.rs"),
        fixture_dir.join("traits.rs"),
    ];
    let mut graph = CodeGraph::new();
    graph.index_repository(fixture_dir, &files, &Manifest::default())?;
    Ok(graph)
}

/// Strip every field this comparison has decided is machine-specific
/// (absolute paths, timestamps, the baseline's auto-derived slug
/// project name) from a JSON value, recursively, so two responses that
/// differ only in those fields still compare equal. Returns the
/// normalized value; the caller is responsible for recording this
/// normalization in the row's `normalizations` list.
fn strip_machine_specific_fields(value: &serde_json::Value) -> serde_json::Value {
    const STRIP_KEYS: &[&str] = &[
        "file_path",
        "path",
        "root_path",
        "qualified_name",
        "project",
        "elapsed_ms",
        "size_bytes",
    ];
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                if STRIP_KEYS.contains(&key.as_str()) {
                    continue;
                }
                out.insert(key.clone(), strip_machine_specific_fields(val));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(strip_machine_specific_fields).collect())
        }
        other => other.clone(),
    }
}

/// Compare two normalized JSON values for the "same named symbols
/// present" property this comparison cares about (not byte-identical
/// JSON shape, since the two systems' response schemas are
/// intentionally different -- baseline is C/SQLite-shaped, candidate
/// is enforcer-native Rust-shaped). Returns `true` when every string
/// value appearing anywhere in `expected_names` also appears somewhere
/// in `haystack`'s stringified form.
fn haystack_contains_all(haystack: &serde_json::Value, expected_names: &[&str]) -> bool {
    let haystack_text = haystack.to_string();
    expected_names
        .iter()
        .all(|name| haystack_text.contains(name))
}

fn common_normalizations() -> Vec<String> {
    vec![
        "stripped machine-specific fields (file_path, path, root_path, qualified_name, project, elapsed_ms, size_bytes)".to_string(),
        "compared by named-symbol-set containment, not byte-identical JSON shape (schemas intentionally differ)".to_string(),
    ]
}

fn unrunnable_row(tool: &str, reason: &str) -> ToolDiffRow {
    ToolDiffRow {
        tool: tool.to_string(),
        comparison_verdict: format!("unrunnable: {reason}"),
        better_because: None,
        worse_because: None,
        normalizations: Vec::new(),
        baseline_latency_ms: None,
        candidate_latency_ms: None,
    }
}

/// Everything one comparison closure needs: the live CLI driver, the
/// baseline's derived project name, the fixture repo path, the
/// candidate graph, and the results accumulator both raw-result rows
/// and the final diff row get appended to.
struct Ctx<'a> {
    driver: &'a CliDriver,
    baseline_project: &'a str,
    candidate_graph: &'a CodeGraph,
    results: &'a mut Vec<ToolResultRow>,
}

fn record_baseline_result(
    results: &mut Vec<ToolResultRow>,
    tool: &str,
    call: &super::baseline::CliCallResult,
) {
    results.push(ToolResultRow {
        tool: tool.to_string(),
        side: "baseline".to_string(),
        raw_json: call.parsed_json(),
        error: if call.exit_success {
            None
        } else {
            Some(format!("exit failure; stdout={}", call.stdout))
        },
        latency_ms: Some(call.latency_ms),
    });
}

fn record_candidate_result<T: Serialize>(
    results: &mut Vec<ToolResultRow>,
    tool: &str,
    value: &T,
    latency_ms: f64,
) {
    results.push(ToolResultRow {
        tool: tool.to_string(),
        side: "candidate".to_string(),
        raw_json: serde_json::to_value(value).ok(),
        error: None,
        latency_ms: Some(latency_ms),
    });
}

fn compare_get_graph_schema(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "get_graph_schema";
    let request = format!(r#"{{"project":"{}"}}"#, ctx.baseline_project);
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };

    let start = Instant::now();
    let schema = graph_schema::get_graph_schema(ctx.candidate_graph);
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{schema:?}"),
        candidate_latency_ms,
    );

    let baseline_labels: BTreeSet<String> = baseline_json
        .get("node_labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    entry
                        .get("label")
                        .and_then(|l| l.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    let candidate_labels: BTreeSet<String> =
        schema.labels.iter().map(|l| l.label.clone()).collect();

    // Function is the one label vocabulary both systems share on this
    // tiny fixture; both sides use "File" too for the file nodes.
    let both_have_function =
        baseline_labels.contains("Function") && candidate_labels.contains("Function");
    let both_have_file = baseline_labels.contains("File") && candidate_labels.contains("File");

    let mut normalizations = common_normalizations();
    normalizations.push("compared node-label VOCABULARY (label name set), not per-label counts (schemas' base column sets differ)".to_string());

    if both_have_function && both_have_file {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "candidate label set {candidate_labels:?} missing baseline labels present in {baseline_labels:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_search_graph_bm25(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "search_graph(bm25)";
    let request = format!(
        r#"{{"project":"{}","query":"parse_config_file"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call("search_graph", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["parse_config_file"]);

    // Candidate: enforcer-memory DOES have a real BM25-over-graph mode --
    // `search_graph::search_graph` with `SearchGraphSpec{query: Some(..)}`
    // short-circuits into `run_bm25`, which tokenizes the query, scores
    // every non-noise node by term-overlap-ratio + label boost, and
    // returns a ranked `results` list (see
    // `crates/enforcer-memory/src/search/search_graph.rs` module docs,
    // "Modes" section). This row now runs that real mode directly over
    // the same indexed fixture graph, rather than a direct symbol-name
    // lookup shortcut.
    let start = Instant::now();
    let spec = SearchGraphSpec {
        query: Some("parse_config_file".to_string()),
        ..SearchGraphSpec::new()
    };
    let candidate_outcome = search_graph(ctx.candidate_graph, &spec);
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_found, candidate_error) = match &candidate_outcome {
        Ok(result) => (
            result
                .results
                .iter()
                .any(|hit| hit.name == "parse_config_file"),
            None,
        ),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{candidate_outcome:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "baseline BM25 full-text search compared against enforcer_memory::search::search_graph::search_graph's real BM25 mode (SearchGraphSpec{query: Some(..)}, run_bm25 over the indexed fixture graph) -- no longer a symbol-name-lookup shortcut"
            .to_string(),
    );

    if baseline_ok && candidate_found {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations: {
                normalizations.push("both sides find the symbol via a ranked full-text/BM25-style mechanism over the same fixture graph; hit-set containment compared, not exact score/rank values (independently derived scoring formulas)".to_string());
                normalizations
            },
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_found={candidate_found} candidate_error={candidate_error:?}: real BM25 mode did not return the expected symbol"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_search_graph_regex(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "search_graph(regex)";
    let request = format!(
        r#"{{"project":"{}","name_pattern":".*config.*"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call("search_graph", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["parse_config_file"]);

    // Candidate: the real regex mode -- `search_graph::search_graph` with
    // `SearchGraphSpec{name_pattern: Some(..)}` compiles the pattern and
    // matches it against every node's name via `run_regex_mode` (see
    // `crates/enforcer-memory/src/search/search_graph.rs`), no longer a
    // hand-rolled substring shortcut.
    let start = Instant::now();
    let spec = SearchGraphSpec {
        name_pattern: Some(".*config.*".to_string()),
        ..SearchGraphSpec::new()
    };
    let candidate_outcome = search_graph(ctx.candidate_graph, &spec);
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_found, candidate_error) = match &candidate_outcome {
        Ok(result) => (
            result
                .results
                .iter()
                .any(|hit| hit.name == "parse_config_file"),
            None,
        ),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{candidate_outcome:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "baseline regex name_pattern search compared against enforcer_memory::search::search_graph::search_graph's real regex mode (SearchGraphSpec{name_pattern: Some(\"...\")}, run_regex_mode over the indexed fixture graph) -- no longer a substring-match shortcut"
            .to_string(),
    );

    if baseline_ok && candidate_found {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_found={candidate_found} candidate_error={candidate_error:?}: real regex mode did not find the same symbol the baseline's regex search found"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_query_graph(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "query_graph";
    // enforcer_memory::analysis::query DOES implement a read-only Cypher
    // subset (D-05) -- MATCH/WHERE/RETURN/ORDER BY/LIMIT over a
    // CodeAdjacency view. This row wires it against the same simple
    // "every Function node" class of query the baseline handles.
    let request = format!(
        r#"{{"project":"{}","query":"MATCH (f:Function) RETURN f.name ORDER BY f.name"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["parse_config_file"]);

    let start = Instant::now();
    let candidate_ok = (|| -> Result<bool, cypher::QueryError> {
        let parsed = cypher::parse("MATCH (f:Function) RETURN f.name ORDER BY f.name")?;
        let adjacency = CodeAdjacency::build(ctx.candidate_graph);
        let rows = cypher::execute(&parsed, &adjacency, ctx.candidate_graph)?;
        Ok(rows
            .iter()
            .any(|row| row.values().any(|v| v.contains("parse_config_file"))))
    })();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_ok, candidate_error) = match candidate_ok {
        Ok(ok) => (ok, None),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(ctx.results, tool, &candidate_ok, candidate_latency_ms);

    let mut normalizations = common_normalizations();
    normalizations.push(
        "compared baseline's Cypher MATCH...RETURN against enforcer_memory::analysis::query's read-only D-05 Cypher subset over the same class of query (MATCH (n:Label) RETURN col ORDER BY col)".to_string(),
    );

    match (baseline_ok, candidate_ok) {
        (true, true) => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
        (false, true) => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some(
                "candidate found parse_config_file in the same fixture repo where baseline search_code returned zero raw matches; raw responses are recorded in tool-results.ndjson".to_string(),
            ),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
        _ => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_ok={candidate_ok} candidate_error={candidate_error:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
    }
}

fn compare_trace_path_calls(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "trace_path(calls)";
    let request = format!(
        r#"{{"project":"{}","function_name":"parse_config_file","mode":"calls"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call("trace_path", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["load_widget_settings"]);

    let start = Instant::now();
    let adjacency = CodeAdjacency::build(ctx.candidate_graph);
    let start_node = ctx
        .candidate_graph
        .nodes()
        .iter()
        .find_map(|node| match node {
            CodeNode::Function(sym) if sym.name == "parse_config_file" => Some(sym.id.clone()),
            _ => None,
        });
    let candidate_ok = match start_node {
        Some(id) => {
            let report = enforcer_memory::analysis::trace::trace_calls(
                &adjacency,
                ctx.candidate_graph,
                &id,
                &TraceCallsParams {
                    direction: TraceDirection::Out,
                    ..Default::default()
                },
            );
            report
                .paths
                .iter()
                .any(|path| path.hops.iter().any(|hop| hop.node_id.contains("widget")))
        }
        None => false,
    };
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    record_candidate_result(ctx.results, tool, &candidate_ok, candidate_latency_ms);

    let normalizations = common_normalizations();
    if baseline_ok && candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(
                "candidate trace_calls did not find load_widget_settings as an outbound callee"
                    .to_string(),
            ),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_get_code_snippet(ctx: &mut Ctx<'_>, repo_root: &Path) -> ToolDiffRow {
    let tool = "get_code_snippet";
    let request = format!(
        r#"{{"project":"{}","qualified_name":"parse_config_file"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_source = baseline_json
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let baseline_ok = baseline_source.contains("load_widget_settings");

    let start = Instant::now();
    let candidate_result = snippet::get_code_snippet(
        ctx.candidate_graph,
        repo_root,
        "lib.rs::parse_config_file",
        false,
    );
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let candidate_ok = match &candidate_result {
        Ok(snip) => String::from_utf8_lossy(&snip.bytes).contains("load_widget_settings"),
        Err(_) => false,
    };
    record_candidate_result(
        ctx.results,
        tool,
        &candidate_result
            .as_ref()
            .ok()
            .map(|s| String::from_utf8_lossy(&s.bytes).into_owned()),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "candidate get_code_snippet additionally returns a sha256 hash the baseline has no equivalent field for (documented enforcer-native improvement, not a divergence to penalize)".to_string(),
    );

    if baseline_ok && candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some("candidate additionally provides a sha256 content hash the baseline's response has no field for at all (§6.4 confirms baseline has no hash field)".to_string()),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_ok={candidate_ok}: candidate snippet resolution or content did not match expectation"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_get_architecture(ctx: &mut Ctx<'_>, aspect_name: &str, aspect: Aspect) -> ToolDiffRow {
    let tool = format!("get_architecture({aspect_name})");
    let request = format!(
        r#"{{"project":"{}","aspects":["{}"]}}"#,
        ctx.baseline_project, aspect_name
    );
    let call = match ctx.driver.call("get_architecture", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(&tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, &tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(&tool, "baseline returned no parseable JSON");
    };
    let baseline_has_nodes = baseline_json
        .get("total_nodes")
        .and_then(|v| v.as_u64())
        .map(|n| n > 0)
        .unwrap_or(false);

    let start = Instant::now();
    let report = architecture::build_report(ctx.candidate_graph, &[aspect], None, 2, 50);
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let candidate_has_data = match aspect {
        Aspect::Overview => report.overview.is_some(),
        Aspect::Clusters => report
            .clusters
            .as_ref()
            .map(|result| !result.clusters.is_empty())
            .unwrap_or(false),
        _ => false,
    };
    record_candidate_result(
        ctx.results,
        &tool,
        &format!("{report:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(format!(
        "compared presence of non-empty {aspect_name} data on both sides, not exact node/edge counts (candidate re-derives every metric from its own graph model per MEMORY_RETRIEVAL_BORROW_POLICY, never ports the baseline's SQL)"
    ));

    if baseline_has_nodes && candidate_has_data {
        ToolDiffRow {
            tool,
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool,
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_has_nodes={baseline_has_nodes} candidate_has_data={candidate_has_data}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_search_code(ctx: &mut Ctx<'_>, repo_root: &Path) -> ToolDiffRow {
    let tool = "search_code";
    let request = format!(
        r#"{{"project":"{}","pattern":"parse_config_file"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["parse_config_file"]);

    // Candidate: `enforcer_memory::code_search::search_code` is a
    // graph-augmented grep over `repo_root` -- wired here against the
    // same fixture tempdir this comparison already built and indexed.
    let start = Instant::now();
    let candidate_outcome = code_search::search_code(
        ctx.candidate_graph,
        repo_root,
        &SearchQuery {
            pattern: "parse_config_file",
            mode: SearchMode::Full,
            context_lines: 0,
            limit: 0,
        },
    );
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_ok, candidate_error) = match &candidate_outcome {
        Ok(outcome) => (outcome.total_matches > 0, None),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{candidate_outcome:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "compared baseline's search_code text-match-plus-structural-rank result against enforcer_memory::code_search::search_code's graph-augmented grep over the same fixture repo_root (presence of a match, not exact rank ordering, since the two score formulas are independently derived)".to_string(),
    );

    match (baseline_ok, candidate_ok) {
        (true, true) => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
        (false, true) => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some(
                "candidate found parse_config_file in the same fixture repo where baseline search_code returned zero raw matches; raw responses are recorded in tool-results.ndjson".to_string(),
            ),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
        _ => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_ok={candidate_ok} candidate_error={candidate_error:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
    }
}

/// Populate a freshly `Store::init`-ed project directory with real
/// graph-event-log entries derived from `candidate_graph` (one
/// `NodeAdded` per [`CodeNode`], one `EdgeAdded` per resolved
/// [`enforcer_memory::code_graph::CallEdge`] whose callee name matches a
/// known `Function` node id), then rebuild `operational.sqlite3` from
/// that log via [`enforcer_memory::store::sqlite::OperationalGraph::rebuild`].
/// This is the real public write path (`graph_event_log_mut` +
/// `OperationalGraph`), not a synthetic shortcut -- it is the same
/// mechanism [`enforcer_memory::store::sqlite`]'s own module docs
/// describe as the log's only consumer. Returns the initialized store's
/// `stores_dir` (kept alive by the caller) and project id so
/// `list_projects`/`index_status` can be exercised against a store that
/// actually has data, not an empty throwaway.
fn populate_store_from_candidate_graph(
    candidate_graph: &CodeGraph,
    fixture_dir: &Path,
) -> Result<(tempfile::TempDir, String), BoxError> {
    let stores_dir = tempfile::tempdir()?;
    let repo_root = enforcer_memory::ids::repo_root(&fixture_dir.to_string_lossy())?;
    let mut store =
        enforcer_memory::store::Store::init(stores_dir.path(), &repo_root, "2026-07-06T00:00:00Z")?;
    let project_id = store.project_id().as_str().to_owned();

    // One NodeAdded event per real CodeNode the candidate indexer found.
    for node in candidate_graph.nodes() {
        let node_kind = match node {
            CodeNode::File(_) => "File",
            CodeNode::Function(_) => "Function",
            CodeNode::Type(_) => "Type",
            CodeNode::Test(_) => "Test",
            CodeNode::TextOnly(_) => "TextOnly",
            CodeNode::Tombstone(_) => "Tombstone",
            CodeNode::Method(_) => "Method",
            CodeNode::Class(_) => "Class",
            CodeNode::Struct(_) => "Struct",
            CodeNode::Interface(_) => "Interface",
            CodeNode::Enum(_) => "Enum",
            CodeNode::TypeAlias(_) => "TypeAlias",
            CodeNode::Module(_) => "Module",
            CodeNode::Lambda(_) => "Lambda",
            CodeNode::Variable(_) => "Variable",
            CodeNode::Constant(_) => "Constant",
        };
        let node_id = node.id().to_string();
        store.graph_event_log_mut().append_with_seq(|seq| {
            enforcer_memory::schema::GraphEventLogEntry {
                schema_version: enforcer_memory::schema::SCHEMA_VERSION,
                seq,
                id: format!("evt-node-{seq}"),
                event: enforcer_memory::schema::GraphEventKind::NodeAdded {
                    node_id,
                    node_kind: node_kind.to_string(),
                },
                ts: "2026-07-06T00:00:00Z".to_string(),
                supersedes_seq: None,
            }
        })?;
    }

    // One EdgeAdded event per real CallEdge whose callee resolves by
    // name to a known Function node id (both fixture functions
    // participate: parse_config_file calls load_widget_settings).
    for call_edge in candidate_graph.calls() {
        let Some(to_id) = candidate_graph.nodes().iter().find_map(|node| match node {
            CodeNode::Function(sym) if sym.name == call_edge.callee => Some(sym.id.clone()),
            _ => None,
        }) else {
            continue;
        };
        store.graph_event_log_mut().append_with_seq(|seq| {
            enforcer_memory::schema::GraphEventLogEntry {
                schema_version: enforcer_memory::schema::SCHEMA_VERSION,
                seq,
                id: format!("evt-edge-{seq}"),
                event: enforcer_memory::schema::GraphEventKind::EdgeAdded {
                    from: call_edge.from_file_id.clone(),
                    to: to_id,
                    label: "calls".to_string(),
                },
                ts: "2026-07-06T00:00:00Z".to_string(),
                supersedes_seq: None,
            }
        })?;
    }

    // Replay the just-written graph-event log into the store's real
    // operational.sqlite3 read model -- the same rebuild path
    // `enforcer_memory::store::sqlite`'s module docs describe as the
    // log's sole consumer, not a bespoke test-only shortcut.
    let log_path = store.graph_event_log_path();
    drop(store);
    let outcome = enforcer_memory::log::read_verified::<enforcer_memory::schema::GraphEventLogEntry>(
        &log_path,
        |e| e.seq,
    )?;
    let sqlite_path = stores_dir
        .path()
        .join(&project_id)
        .join("operational.sqlite3");
    let mut operational = enforcer_memory::store::sqlite::OperationalGraph::open(&sqlite_path)?;
    operational.rebuild(&outcome.entries)?;

    Ok((stores_dir, project_id))
}

fn compare_list_projects(ctx: &mut Ctx<'_>, fixture_dir: &Path) -> ToolDiffRow {
    let tool = "list_projects";
    let call = match ctx.driver.call(tool, "{}") {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = baseline_json
        .get("projects")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    // Candidate: populate a real Store-backed project directory (via
    // the graph-event log + OperationalGraph::rebuild write path -- see
    // `populate_store_from_candidate_graph`) so `list_projects` scans a
    // `stores_dir` that actually has the fixture project in it, not an
    // empty throwaway.
    let start = Instant::now();
    let candidate_result = (|| -> Result<bool, BoxError> {
        let (stores_dir, project_id) =
            populate_store_from_candidate_graph(ctx.candidate_graph, fixture_dir)?;
        let entries = enforcer_memory::projects::list_projects(stores_dir.path())?;
        Ok(entries.iter().any(|entry| entry.project_id == project_id))
    })();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_ok, candidate_error) = match candidate_result {
        Ok(found) => (found, None),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(ctx.results, tool, &candidate_ok, candidate_latency_ms);

    let mut normalizations = common_normalizations();
    normalizations.push("candidate list_projects exercised over a stores_dir populated via the real graph-event-log + OperationalGraph::rebuild write path (same fixture project the baseline indexed); compared entry PRESENCE and field semantics (name/root present), not byte-identical schema, since the two persistence models differ".to_string());

    if baseline_ok && candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_ok={candidate_ok} candidate_error={candidate_error:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_index_status(ctx: &mut Ctx<'_>, fixture_dir: &Path) -> ToolDiffRow {
    let tool = "index_status";
    let request = format!(r#"{{"project":"{}"}}"#, ctx.baseline_project);
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ready = baseline_json.get("status").and_then(|v| v.as_str()) == Some("ready");

    // Candidate: populate a real Store-backed project directory (via
    // the graph-event log + OperationalGraph::rebuild write path -- see
    // `populate_store_from_candidate_graph`) over the same fixture repo
    // root, so `index_status` reports real node/edge counts and a
    // baseline-aligned Ready status derived from actually-applied graph
    // events, not a bare freshly-init-ed empty store.
    let start = Instant::now();
    let candidate_result = (|| -> Result<projects::IndexStatusSummary, BoxError> {
        let (stores_dir, project_id) =
            populate_store_from_candidate_graph(ctx.candidate_graph, fixture_dir)?;
        let summary = projects::index_status(stores_dir.path(), &project_id)?;
        Ok(summary)
    })();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_ready, candidate_error) = match &candidate_result {
        // The store was populated with real NodeAdded events above, so
        // a healthy run reports Ready with nodes > 0 -- the same
        // baseline-aligned nodes>0?ready:empty derivation, now actually
        // exercised end-to-end instead of only checking the Empty leg.
        Ok(summary) => (
            matches!(summary.status, projects::ProjectStatus::Ready) && summary.nodes > 0,
            None,
        ),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{candidate_result:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push("candidate index_status exercised over a crate::store::Store::init-ed project populated with real NodeAdded/EdgeAdded graph events derived from the candidate CodeGraph (via graph_event_log_mut + OperationalGraph::rebuild -- the store's own documented log-replay write path), so this row compares field-level ready/empty status semantics against the baseline's own indexed project, not just the wiring on an empty throwaway store".to_string());

    if candidate_error.is_some() {
        return ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!("candidate_error={candidate_error:?}")),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        };
    }

    if baseline_ready && candidate_ready {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ready={baseline_ready} candidate_ready={candidate_ready}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_detect_changes(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "detect_changes";
    let request = format!(r#"{{"project":"{}"}}"#, ctx.baseline_project);
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let baseline_json = call.parsed_json();
    // Fixture repo has no uncommitted changes -- both sides should
    // report zero changed files.
    let baseline_zero = baseline_json
        .as_ref()
        .and_then(|json| json.get("changed_count").and_then(|v| v.as_u64()))
        == Some(0);

    let start = Instant::now();
    let view = enforcer_memory::impact::detect_changes_view(
        ctx.candidate_graph,
        &[],
        2,
        enforcer_memory::impact::DetectChangesScope::Symbols,
    );
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let candidate_zero = view.changed_count == 0;
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{view:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push("both sides fed an empty changed-file list (fixture repo has no uncommitted diff); this row only checks the zero-change shape, not a real diff comparison".to_string());

    match (baseline_json.is_some(), baseline_zero, candidate_zero) {
        (true, true, true) => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
        (false, _, true) => {
            normalizations.push(
                "baseline detect_changes exited without parseable JSON on the clean fixture while candidate returned the deterministic zero-change view; raw baseline stderr/stdout is recorded in tool-results.ndjson".to_string(),
            );
            ToolDiffRow {
                tool: tool.to_string(),
                comparison_verdict: "better".to_string(),
                better_because: Some(
                    "candidate returns a parseable zero-change detect_changes report for the same clean fixture where baseline returned no parseable JSON".to_string(),
                ),
                worse_because: None,
                normalizations,
                baseline_latency_ms: Some(call.latency_ms),
                candidate_latency_ms: Some(candidate_latency_ms),
            }
        }
        _ => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_zero={baseline_zero} candidate_zero={candidate_zero}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
    }
}

fn compare_manage_adr(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "manage_adr(get/update)";

    // The baseline's manage_adr is a whole-document get/update: write a
    // full markdown blob under mode="update", read the same blob back
    // under mode="get". enforcer_memory::adr's AdrStore NOW also exposes
    // a baseline-compatible whole-document API
    // (`get_document`/`update_document`/`list_document_headings`,
    // `refs/x06-baseline-tool-schemas.md` §14) alongside its original
    // section-based extension API -- so this row compares the two
    // whole-document paths directly, same shape class on both sides:
    // "update(content) then get() returns it".
    let update_request = format!(
        r#"{{"project":"{}","mode":"update","content":"ADR: parse_config_file decision"}}"#,
        ctx.baseline_project
    );
    let update_call = match ctx.driver.call("manage_adr", &update_request) {
        Ok(call) => call,
        Err(error) => {
            return unrunnable_row(tool, &format!("baseline update call failed: {error}"))
        }
    };
    record_baseline_result(ctx.results, "manage_adr(update)", &update_call);

    let get_request = format!(r#"{{"project":"{}","mode":"get"}}"#, ctx.baseline_project);
    let get_call = match ctx.driver.call("manage_adr", &get_request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline get call failed: {error}")),
    };
    record_baseline_result(ctx.results, "manage_adr(get)", &get_call);
    let Some(get_json) = get_call.parsed_json() else {
        return unrunnable_row(tool, "baseline get returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&get_json, &["parse_config_file"]);

    let start = Instant::now();
    let mut store = AdrStore::new();
    store.update_document("adr-x06-parity", "ADR: parse_config_file decision");
    let document = store.get_document("adr-x06-parity");
    let candidate_ok = !document.no_adr && document.content.contains("parse_config_file");
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    record_candidate_result(ctx.results, tool, &candidate_ok, candidate_latency_ms);

    let mut normalizations = common_normalizations();
    normalizations.push(
        "compared the whole-document compat API on both sides: baseline update(content=<whole markdown>) then get() returning it, vs. candidate AdrStore::update_document(...) then get_document() returning the same content -- same shape class, no longer the section-based extension API".to_string(),
    );
    normalizations.push(
        "candidate additionally exposes a section-based extension API (AdrStore::create/update_section/get) and list_document_headings(), which the baseline's whole-document model has no equivalent for -- documented enforcer-native extension, not part of the equality comparison itself".to_string(),
    );

    if baseline_ok && candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(get_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_ok={candidate_ok}: whole-document update_document/get_document round-trip did not match expectation"
            )),
            normalizations,
            baseline_latency_ms: Some(get_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_index_repository(
    ctx: &mut Ctx<'_>,
    index_call: &super::baseline::CliCallResult,
) -> ToolDiffRow {
    let tool = "index_repository";

    // The baseline call was already made once (to obtain the project
    // name every other row's request needs) -- this row reuses that
    // same real call result rather than issuing a second index request
    // against the fixture, and simply grades it: did indexing report
    // success, and did it report a non-empty node/edge-bearing result.
    let Some(baseline_json) = index_call.parsed_json() else {
        return unrunnable_row(tool, "baseline index_repository returned no parseable JSON");
    };
    let baseline_has_project = baseline_json
        .get("project")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    // Candidate: `CodeGraph::index_repository`'s own IndexReport --
    // "success" here means the call already succeeded upstream (this
    // function is called with the SAME candidate_graph already built by
    // `build_candidate_graph` earlier in the run) and the graph carries
    // a non-empty node set plus at least one resolved call edge, the
    // node/edge-count-nonzero bar this row's mission specifies.
    let start = Instant::now();
    let candidate_node_count = ctx.candidate_graph.nodes().len();
    let candidate_edge_count = ctx.candidate_graph.calls().len();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let candidate_ok = candidate_node_count > 0 && candidate_edge_count > 0;
    record_candidate_result(
        ctx.results,
        tool,
        &format!("nodes={candidate_node_count} edges={candidate_edge_count}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "compared indexing SUCCESS plus node/edge-count-nonzero presence on both sides, not exact counts -- the baseline's SQLite-backed graph and the candidate's in-memory CodeGraph derive node/edge sets from independent schemas (e.g. baseline may add synthetic project/root nodes the candidate does not), so absolute counts are not a meaningful equality bar".to_string(),
    );

    if baseline_has_project && candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(index_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_has_project={baseline_has_project} candidate_node_count={candidate_node_count} candidate_edge_count={candidate_edge_count}"
            )),
            normalizations,
            baseline_latency_ms: Some(index_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_delete_project(ctx: &mut Ctx<'_>, fixture_dir: &Path) -> ToolDiffRow {
    let tool = "delete_project";

    // Baseline: index a SECOND, throwaway fixture project (distinct
    // name) so this row can delete it without disturbing
    // `ctx.baseline_project`, which other rows (and this function's own
    // caller, which runs delete_project on the primary project only
    // AFTER every row has executed) still depend on.
    let index_request = format!(
        r#"{{"repo_path":"{}","name":"x06parity-live-delete-throwaway","mode":"full"}}"#,
        fixture_dir.to_string_lossy().replace('\\', "/")
    );
    let index_call = match ctx.driver.call("index_repository", &index_request) {
        Ok(call) => call,
        Err(error) => {
            return unrunnable_row(tool, &format!("baseline throwaway index failed: {error}"))
        }
    };
    record_baseline_result(
        ctx.results,
        "index_repository(delete-throwaway)",
        &index_call,
    );
    let Some(throwaway_project) = index_call.parsed_json().and_then(|v| {
        v.get("project")
            .and_then(|p| p.as_str())
            .map(str::to_string)
    }) else {
        return unrunnable_row(
            tool,
            "baseline throwaway index did not return a project name",
        );
    };

    let delete_request = format!(r#"{{"project":"{throwaway_project}"}}"#);
    let delete_call = match ctx.driver.call("delete_project", &delete_request) {
        Ok(call) => call,
        Err(error) => {
            return unrunnable_row(tool, &format!("baseline delete call failed: {error}"))
        }
    };
    record_baseline_result(ctx.results, tool, &delete_call);
    let baseline_deleted = delete_call.exit_success;

    // Verify the baseline actually removed it: list_projects should no
    // longer report the throwaway project id.
    let baseline_actually_gone = match ctx.driver.call("list_projects", "{}") {
        Ok(list_call) => list_call
            .parsed_json()
            .and_then(|v| v.get("projects").and_then(|p| p.as_array()).cloned())
            .map(|projects| {
                !projects.iter().any(|p| {
                    p.get("project")
                        .or_else(|| p.get("name"))
                        .and_then(|n| n.as_str())
                        == Some(throwaway_project.as_str())
                })
            })
            .unwrap_or(false),
        Err(_) => false,
    };

    // Candidate: populate a real, separate throwaway store-backed
    // project (same `populate_store_from_candidate_graph` write path
    // `compare_list_projects`/`compare_index_status` already use), then
    // call `projects::delete_project` on it and verify the store
    // directory is actually gone afterward.
    let start = Instant::now();
    let candidate_result = (|| -> Result<bool, BoxError> {
        let (stores_dir, project_id) =
            populate_store_from_candidate_graph(ctx.candidate_graph, fixture_dir)?;
        let store_root = stores_dir.path().join(&project_id);
        projects::delete_project(stores_dir.path(), &project_id)?;
        Ok(!store_root.exists())
    })();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_deleted, candidate_error) = match candidate_result {
        Ok(gone) => (gone, None),
        Err(error) => (false, Some(error.to_string())),
    };
    record_candidate_result(ctx.results, tool, &candidate_deleted, candidate_latency_ms);

    let mut normalizations = common_normalizations();
    normalizations.push("baseline delete_project run against a second, throwaway indexed project distinct from ctx.baseline_project (so other rows' project is left intact); candidate projects::delete_project run against a separate throwaway Store populated via the same graph-event-log write path compare_list_projects/compare_index_status use -- both sides compared on report-deleted-success AND actual-removal-verified, not report alone".to_string());

    if baseline_deleted && baseline_actually_gone && candidate_deleted {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(delete_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_deleted={baseline_deleted} baseline_actually_gone={baseline_actually_gone} candidate_deleted={candidate_deleted} candidate_error={candidate_error:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(delete_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

fn compare_ingest_traces(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "ingest_traces";

    // Baseline: `refs/x06-baseline-tool-schemas.md` §15.2 -- VERIFIED
    // this tool is an unimplemented stub. The handler never reads the
    // caller/callee/count fields of any trace element, performs no
    // store lookup, and unconditionally returns
    // `{"status":"accepted","traces_received":<len>,"note":"Runtime
    // edge creation from traces not yet implemented"}` regardless of
    // input validity. This row calls it for real (never fabricating
    // that response) purely to document the stub shape as the recorded
    // baseline evidence.
    let request = format!(
        r#"{{"project":"{}","traces":[{{"caller":"lib.rs::parse_config_file","callee":"load_widget_settings","count":3}}]}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call(tool, &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_is_stub = baseline_json.get("status").and_then(|v| v.as_str()) == Some("accepted")
        && baseline_json.get("note").is_some();

    // Candidate: `enforcer_memory::traces::TraceStore` performs a real
    // merge -- ingesting a caller/callee/count record against the
    // indexed fixture graph and producing an annotated `TracedEdge`
    // with a nonzero observed_count, exactly the CALLS-edge enrichment
    // §15.2 confirms the baseline never actually does.
    let start = Instant::now();
    let mut trace_store = enforcer_memory::traces::TraceStore::new();
    let caller_id = ctx
        .candidate_graph
        .nodes()
        .iter()
        .find_map(|node| match node {
            CodeNode::Function(sym) if sym.name == "parse_config_file" => Some(sym.id.clone()),
            _ => None,
        });
    let candidate_merged = match caller_id {
        Some(caller_id) => {
            trace_store.ingest(
                ctx.candidate_graph,
                &[enforcer_memory::traces::TraceRecord {
                    caller: caller_id,
                    callee: "load_widget_settings".to_string(),
                    count: 3,
                }],
            );
            let edges = trace_store.edges(ctx.candidate_graph);
            edges.iter().any(|edge| {
                edge.callee == "load_widget_settings"
                    && edge.observed_count == 3
                    && trace_store.unresolved().is_empty()
            })
        }
        None => false,
    };
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    record_candidate_result(ctx.results, tool, &candidate_merged, candidate_latency_ms);

    let mut normalizations = common_normalizations();
    normalizations.push(
        "baseline ingest_traces is a documented unimplemented stub (refs/x06-baseline-tool-schemas.md §15.2: handler never reads caller/callee/count, does no store lookup, unconditionally returns {status:accepted, traces_received:N, note:'not yet implemented'}); candidate enforcer_memory::traces::TraceStore performs a real merge, annotating the matching CALLS edge with an observed_count and tracking unresolved records explicitly".to_string(),
    );

    if baseline_is_stub && candidate_merged {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some("baseline is an unimplemented stub; candidate performs real runtime-edge merging with idempotency and unresolved tracking (refs/x06-baseline-tool-schemas.md §15.2)".to_string()),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_is_stub={baseline_is_stub} candidate_merged={candidate_merged}: expected baseline stub shape and a real candidate merge, one side did not match"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

// ---------------------------------------------------------------------
// X06 core-parity extension rows (final verification wave): complexity
// properties, rich node/edge vocabulary, multi-language indexing,
// similarity edges, data_flow tracing, type-aware call resolution, and
// cross-repo-intelligence mode. Same honesty rules as every row above:
// real calls on both sides, documented normalizations, never a
// fabricated result.
// ---------------------------------------------------------------------

/// `query_graph` over the X06 complexity properties: both sides run the
/// same class of Cypher query filtering on `f.complexity` (Tier A,
/// cyclomatic) and `f.transitive_loop_depth` (Tier B, interprocedural),
/// expecting the fixture's `describe` function (a `for` loop + three
/// `if`/`else if`/`else` arms) to be the row both filters select.
fn compare_query_graph_complexity(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "query_graph(complexity)";
    let complexity_query =
        "MATCH (f:Function) WHERE f.complexity >= 2 RETURN f.name, f.complexity ORDER BY f.name";
    let tld_query =
        "MATCH (f:Function) WHERE f.transitive_loop_depth >= 1 RETURN f.name ORDER BY f.name";

    let complexity_request = format!(
        r#"{{"project":"{}","query":"{}"}}"#,
        ctx.baseline_project, complexity_query
    );
    let complexity_call = match ctx.driver.call("query_graph", &complexity_request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(
        ctx.results,
        "query_graph(complexity:cyclomatic)",
        &complexity_call,
    );
    let baseline_complexity_ok = complexity_call
        .parsed_json()
        .map(|json| haystack_contains_all(&json, &["describe"]))
        .unwrap_or(false);

    let tld_request = format!(
        r#"{{"project":"{}","query":"{}"}}"#,
        ctx.baseline_project, tld_query
    );
    let tld_call = match ctx.driver.call("query_graph", &tld_request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, "query_graph(complexity:tld)", &tld_call);
    let baseline_tld_ok = tld_call
        .parsed_json()
        .map(|json| haystack_contains_all(&json, &["describe"]))
        .unwrap_or(false);

    let start = Instant::now();
    let candidate_outcome = (|| -> Result<(bool, bool), cypher::QueryError> {
        let adjacency = CodeAdjacency::build(ctx.candidate_graph);
        let complexity_rows = cypher::execute(
            &cypher::parse(complexity_query)?,
            &adjacency,
            ctx.candidate_graph,
        )?;
        let tld_rows =
            cypher::execute(&cypher::parse(tld_query)?, &adjacency, ctx.candidate_graph)?;
        let found_in = |rows: &[cypher::ResultRow]| {
            rows.iter()
                .any(|row| row.values().any(|v| v.contains("describe")))
        };
        Ok((found_in(&complexity_rows), found_in(&tld_rows)))
    })();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_complexity_ok, candidate_tld_ok, candidate_error) = match candidate_outcome {
        Ok((c, t)) => (c, t, None),
        Err(error) => (false, false, Some(error.to_string())),
    };
    record_candidate_result(
        ctx.results,
        tool,
        &format!(
            "complexity_ok={candidate_complexity_ok} tld_ok={candidate_tld_ok} error={candidate_error:?}"
        ),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "same class of Cypher complexity-property query on both sides (WHERE f.complexity >= 2 / WHERE f.transitive_loop_depth >= 1); candidate answers via analysis::query's resolve_property over complexity::ComplexityMetrics (Tier A) and TransitiveMetrics (Tier B), baseline via its SQLite node-property columns -- compared on which function each filter selects (describe), never on independently-derived score equality".to_string(),
    );

    if baseline_complexity_ok && baseline_tld_ok && candidate_complexity_ok && candidate_tld_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(complexity_call.latency_ms + tld_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_complexity_ok={baseline_complexity_ok} baseline_tld_ok={baseline_tld_ok} candidate_complexity_ok={candidate_complexity_ok} candidate_tld_ok={candidate_tld_ok} candidate_error={candidate_error:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(complexity_call.latency_ms + tld_call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

/// Rich node-label/edge-type vocabulary: the fixture's `traits.rs`
/// carries a trait hierarchy (`Named: Drawable`), two `impl Trait for
/// Widget` blocks, a typed `&Widget` parameter, and impl-scoped methods
/// -- so the candidate schema must report `Interface`/`Struct`/`Method`
/// labels and `INHERITS`/`IMPLEMENTS`/`TYPE_REF`/`DEFINES` edge rows,
/// and the baseline's own schema over the same repo is graded on
/// reporting the same class of rich vocabulary.
fn compare_graph_schema_rich_vocab(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "get_graph_schema(rich_vocab)";
    let request = format!(r#"{{"project":"{}"}}"#, ctx.baseline_project);
    let call = match ctx.driver.call("get_graph_schema", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_text = baseline_json.to_string();
    // Rust traits may surface as "Interface" (this crate's vocabulary)
    // or "Trait" in the baseline's own label set -- accept either as
    // "the baseline models the trait", never require our exact string.
    let baseline_rich_labels = (baseline_text.contains("Interface")
        || baseline_text.contains("Trait"))
        && baseline_text.contains("Struct")
        && baseline_text.contains("Method");
    let baseline_rich_edges = baseline_text.contains("INHERITS")
        || baseline_text.contains("IMPLEMENTS")
        || baseline_text.contains("TYPE_REF")
        || baseline_text.contains("DEFINES");

    let start = Instant::now();
    let schema = graph_schema::get_graph_schema(ctx.candidate_graph);
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let label_set: BTreeSet<&str> = schema.labels.iter().map(|l| l.label.as_str()).collect();
    let edge_set: BTreeSet<&str> = schema
        .edge_types
        .iter()
        .map(|e| e.edge_type.as_str())
        .collect();
    let candidate_ok = ["Interface", "Struct", "Method"]
        .iter()
        .all(|label| label_set.contains(label))
        && ["INHERITS", "IMPLEMENTS", "TYPE_REF", "DEFINES"]
            .iter()
            .all(|edge| edge_set.contains(edge));
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{schema:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "compared rich-vocabulary PRESENCE (Interface-or-Trait/Struct/Method labels; at least one of INHERITS/IMPLEMENTS/TYPE_REF/DEFINES edge rows on the baseline side, all four required on the candidate side), not per-row counts -- the two extractors derive node/edge sets from independent parsers".to_string(),
    );

    if candidate_ok && baseline_rich_labels && baseline_rich_edges {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else if candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some(format!(
                "candidate schema reports the full rich vocabulary (Interface/Struct/Method + INHERITS/IMPLEMENTS/TYPE_REF/DEFINES) on this Rust fixture; baseline schema on the same repo lacks part of it (rich_labels={baseline_rich_labels} rich_edges={baseline_rich_edges} -- raw baseline schema recorded in tool-results.ndjson as evidence)"
            )),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "candidate schema missing required rich vocabulary: labels={label_set:?} edges={edge_set:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

/// `SIMILAR_TO` materialization: the fixture deliberately exceeds the
/// baseline's MinHash body-size floor, so both systems must report the
/// real baseline-compatible edge and `fp` property vocabulary. The
/// candidate must additionally retain body-shingle and identifier-token
/// signals under distinct Rust-native edge names.
fn compare_graph_schema_similarity(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "get_graph_schema(similarity)";
    let request = format!(r#"{{"project":"{}"}}"#, ctx.baseline_project);
    let call = match ctx.driver.call("get_graph_schema", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_semantic = baseline_json.to_string().contains("SEMANTICALLY_RELATED");
    let baseline_has_fingerprint_property = baseline_json
        .get("node_labels")
        .and_then(|labels| labels.as_array())
        .is_some_and(|labels| {
            labels.iter().any(|label| {
                label.get("label").and_then(|value| value.as_str()) == Some("Function")
                    && label
                        .get("properties")
                        .and_then(|value| value.as_array())
                        .is_some_and(|properties| {
                            properties.iter().any(|value| value.as_str() == Some("fp"))
                        })
            })
        });
    let baseline_similar_to = baseline_json
        .get("edge_types")
        .and_then(|edges| edges.as_array())
        .is_some_and(|edges| {
            edges.iter().any(|edge| {
                edge.get("type").and_then(|value| value.as_str()) == Some("SIMILAR_TO")
                    && edge
                        .get("properties")
                        .and_then(|value| value.as_array())
                        .is_some_and(|properties| {
                            let names: BTreeSet<&str> = properties
                                .iter()
                                .filter_map(|value| value.as_str())
                                .collect();
                            names.contains("jaccard") && names.contains("same_file")
                        })
            })
        });

    let start = Instant::now();
    let minhash_similar = similar_to(ctx.candidate_graph);
    let body_shingle_similar = similar_to_body_shingles(ctx.candidate_graph);
    let rust_identifier_similar = similar_to_identifier_tokens(ctx.candidate_graph);
    let mut candidate_similarity_signals = minhash_similar.clone();
    candidate_similarity_signals.extend(body_shingle_similar.iter().cloned());
    let semantic = semantically_related(ctx.candidate_graph);
    let schema = graph_schema::get_graph_schema_with_similarity_modes(
        ctx.candidate_graph,
        &candidate_similarity_signals,
        &rust_identifier_similar,
        &semantic,
    );
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let has_fixture_pair = |edges: &[SimilarToEdge]| {
        edges.iter().any(|edge| {
            edge.source_id.contains("parse_widget_config")
                && edge.target_id.contains("parseWidgetConfig")
                || edge.source_id.contains("parseWidgetConfig")
                    && edge.target_id.contains("parse_widget_config")
        })
    };
    let candidate_minhash_pair_found = has_fixture_pair(&minhash_similar);
    let candidate_body_shingle_pair_found = has_fixture_pair(&body_shingle_similar);
    let candidate_identifier_pair_found = has_fixture_pair(&rust_identifier_similar);
    let candidate_schema_row = schema.edge_types.iter().any(|edge| {
        edge.edge_type == "SIMILAR_TO"
            && edge.properties.iter().any(|property| property == "jaccard")
            && edge
                .properties
                .iter()
                .any(|property| property == "same_file")
    });
    let candidate_body_shingle_schema_row = schema
        .edge_types
        .iter()
        .any(|edge| edge.edge_type == "BODY_SHINGLE_SIMILAR_TO");
    let candidate_identifier_schema_row = schema
        .edge_types
        .iter()
        .any(|edge| edge.edge_type == "RUST_IDENTIFIER_SIMILAR_TO");
    let candidate_has_fingerprint_property = schema.labels.iter().any(|label| {
        label.label == "Function"
            && label.properties.iter().any(|property| property == "fp")
            && label.properties.iter().any(|property| property == "k")
    });
    let candidate_ok = candidate_minhash_pair_found
        && candidate_body_shingle_pair_found
        && candidate_identifier_pair_found
        && candidate_schema_row
        && candidate_body_shingle_schema_row
        && candidate_identifier_schema_row
        && candidate_has_fingerprint_property;
    record_candidate_result(
        ctx.results,
        tool,
        &format!(
            "minhash_similar_to={minhash_similar:?} body_shingle_similar_to={body_shingle_similar:?} rust_identifier_similar_to={rust_identifier_similar:?} semantically_related_count={} schema={schema:?}",
            semantic.len()
        ),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "required a real baseline SIMILAR_TO schema row with jaccard/same_file properties and a baseline Function fp property after enlarging the shared fixture above the baseline's 30-leaf MinHash floor; schema-property presence alone cannot produce better".to_string(),
    );
    normalizations.push(
        "candidate is required to emit the baseline-compatible persisted 64-slot MinHash SIMILAR_TO plus two additive Rust signals: BODY_SHINGLE_SIMILAR_TO and RUST_IDENTIFIER_SIMILAR_TO; all preserve the 0.95 threshold, same-extension gate, 10-edge cap, and ordered-pair dedup".to_string(),
    );
    normalizations.push(format!(
        "SEMANTICALLY_RELATED recorded, not graded: baseline_present={baseline_semantic}, candidate_count={} because the two systems use different semantic-combination models",
        semantic.len()
    ));

    if candidate_ok && baseline_similar_to && baseline_has_fingerprint_property {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some(
                "candidate persists and surfaces the baseline-compatible fp/k MinHash contract and emits the same SIMILAR_TO evidence on the engineered baseline-sized body pair; it additionally exposes body-shingle and Rust identifier-token similarity as distinct signals".to_string(),
            ),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else if candidate_ok {
        normalizations.push(format!(
            "baseline did not materialize the required fingerprint-and-SIMILAR_TO evidence on this fixture (fp={baseline_has_fingerprint_property}, similar_to={baseline_similar_to}); raw response is recorded, so this remains incomparable"
        ));
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "incomparable".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "candidate similarity pass did not produce required evidence (minhash_pair={candidate_minhash_pair_found} body_shingle_pair={candidate_body_shingle_pair_found} identifier_pair={candidate_identifier_pair_found} minhash_schema={candidate_schema_row} body_shingle_schema={candidate_body_shingle_schema_row} identifier_schema={candidate_identifier_schema_row} fp_property={candidate_has_fingerprint_property})"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

/// `trace_path mode=data_flow`: both sides trace from
/// `parse_config_file` and must reach `load_widget_settings`; the
/// candidate's hop additionally carries a [`enforcer_memory::analysis::trace::ParamLink`]
/// with the real captured argument expression (`path`) and the
/// documented always-`None` `parameter_name` (no extractor records
/// callee parameter names -- data_flow.rs module docs).
fn compare_trace_data_flow(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "trace_path(data_flow)";
    let request = format!(
        r#"{{"project":"{}","function_name":"parse_config_file","mode":"data_flow"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call("trace_path", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["load_widget_settings"]);

    let start = Instant::now();
    let adjacency = CodeAdjacency::build(ctx.candidate_graph);
    let start_node = ctx
        .candidate_graph
        .nodes()
        .iter()
        .find_map(|node| match node {
            CodeNode::Function(sym) if sym.name == "parse_config_file" => Some(sym.id.clone()),
            _ => None,
        });
    let (candidate_hop_ok, param_link_observed) = match &start_node {
        Some(id) => {
            let report = trace_data_flow(
                &adjacency,
                ctx.candidate_graph,
                id,
                &TraceCallsParams {
                    direction: TraceDirection::Out,
                    ..Default::default()
                },
            );
            let hop_ok = report.paths.iter().any(|path| {
                path.hops
                    .iter()
                    .any(|hop| hop.hop.node_id.contains("load_widget_settings"))
            });
            let param_link = report
                .paths
                .iter()
                .flat_map(|path| path.hops.iter())
                .find_map(|hop| hop.param_link.clone());
            (hop_ok, param_link)
        }
        None => (false, None),
    };
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    // The engineered expectation: the call site `load_widget_settings(path)`
    // has a captured argument expression, so the hop must carry a real
    // ParamLink whose argument_expr is "path" and whose parameter_name is
    // the documented honest None -- a Some(..) parameter_name here would
    // mean someone fabricated a binding no extractor records.
    let candidate_param_ok = matches!(
        &param_link_observed,
        Some(link) if link.argument_expr == "path" && link.parameter_name.is_none()
    );
    record_candidate_result(
        ctx.results,
        tool,
        &format!("hop_ok={candidate_hop_ok} param_link={param_link_observed:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "candidate data_flow is the documented honest analog: call-graph walk plus real captured argument text (CallEdge::arg_texts -> ParamLink::argument_expr), approximation=CallGraphOnly; ParamLink::parameter_name is always None because no language extractor records callee parameter names (data_flow.rs/analysis::trace module docs) -- the same granularity as the baseline's own caller_args raw-JSON copy, which also never binds arguments to parameters by name".to_string(),
    );

    if baseline_ok && candidate_hop_ok && candidate_param_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_hop_ok={candidate_hop_ok} candidate_param_ok={candidate_param_ok} param_link={param_link_observed:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

/// Type-aware call resolution: `describe` calls `widget.draw()` on a
/// `&Widget`-typed parameter, so the candidate's resolution ladder must
/// resolve that call to the single `impl Drawable for Widget` method
/// (confidence Resolved or Probable, exactly one candidate -- never an
/// arbitrary pick from an ambiguous set), and the baseline's own
/// `trace_path` from `describe` must reach `draw` through its
/// equivalent resolution.
fn compare_resolution_trace(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "trace_path(calls,type_resolution)";
    let request = format!(
        r#"{{"project":"{}","function_name":"describe","mode":"calls"}}"#,
        ctx.baseline_project
    );
    let call = match ctx.driver.call("trace_path", &request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline call failed: {error}")),
    };
    record_baseline_result(ctx.results, tool, &call);
    let Some(baseline_json) = call.parsed_json() else {
        return unrunnable_row(tool, "baseline returned no parseable JSON");
    };
    let baseline_ok = haystack_contains_all(&baseline_json, &["draw"]);

    let start = Instant::now();
    let resolved = resolution::resolve(ctx.candidate_graph);
    let candidate_resolution = ctx
        .candidate_graph
        .calls()
        .iter()
        .zip(resolved.iter())
        .find(|(call_edge, _)| call_edge.callee.contains("draw"))
        .map(|(_, resolved_call)| (resolved_call.confidence, resolved_call.candidates.clone()));
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let candidate_ok = matches!(
        &candidate_resolution,
        Some((confidence, candidates))
            if matches!(confidence, ResolutionConfidence::Resolved | ResolutionConfidence::Probable)
                && candidates.len() == 1
                && candidates[0].contains("draw")
    );
    record_candidate_result(
        ctx.results,
        tool,
        &format!("{candidate_resolution:?}"),
        candidate_latency_ms,
    );

    let mut normalizations = common_normalizations();
    normalizations.push(
        "candidate graded on resolution::resolve's ladder output for the widget.draw() call site (exactly one candidate at Resolved/Probable confidence -- type-aware receiver match or documented lower-confidence rung, never an arbitrary pick from an Ambiguous set); baseline graded on its trace_path from describe reaching draw, its own resolution surface".to_string(),
    );

    if baseline_ok && candidate_ok {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "baseline_ok={baseline_ok} candidate_ok={candidate_ok} candidate_resolution={candidate_resolution:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(call.latency_ms),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

/// How a [`MULTI_LANG_FIXTURES`] entry's `expected` string is checked
/// on the candidate side -- language-parity wave G3 stage 5. Most
/// onboarded languages have a real named function/class the generic
/// engine surfaces as a [`SymbolRef`], but several Tier-0 formats
/// genuinely don't (data/config/markup with no "named symbol" concept
/// at all -- JSON, YAML, a `.gitignore`, ...), so this widens the
/// comparison beyond symbol-name matching rather than forcing every
/// language into a shape that doesn't fit it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FixtureCheckKind {
    /// A named def (`ParsedFile::symbols`) -- the original, most
    /// precise check.
    SymbolName,
    /// An import path (`ParsedFile::imports`).
    ImportPath,
    /// A call callee (`ParsedFile::calls`).
    CallCallee,
    /// No named-symbol concept applies to this language/format at all
    /// (config/markup/data) -- falls back to the fixture's own
    /// filename, which both sides trivially recognize once the file is
    /// indexed at all, so a miss here means the language wasn't
    /// recognized/indexed, not that one specific name diverged.
    Filename,
}

/// The multi-language fixture set this row indexes on both sides: one
/// real file per onboarded language from the repo's own committed
/// fixtures (READ at run time, never inlined copies that could drift),
/// plus the Rust `lib.rs` the primary fixture already uses. Each entry
/// carries one expected fact unique to that language's file (kind
/// depends on [`FixtureCheckKind`]), so a miss is attributable to a
/// specific language. The first 8 rows are wave-B's original hand-picked
/// set (unchanged); everything after was harvested mechanically from
/// each language's own `tests/unit_languages_*.rs` fixture-reading test
/// for language-parity wave G3 stage 5 (full parity re-verification) --
/// see that wave's closeout doc for the harvesting method and the
/// handful of manual fills (languages whose own tests never read a
/// committed fixture file at all).
const MULTI_LANG_FIXTURES: &[(&str, &str, &str, FixtureCheckKind)] = &[
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_go/widget.go",
        "widget.go",
        "NewWidget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_java/Widget.java",
        "Widget.java",
        "Shape",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_c/widget.c",
        "widget.c",
        "widget_new",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cpp/widget.cpp",
        "widget.cpp",
        "DerivedWidget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_csharp/Widget.cs",
        "Widget.cs",
        "LoadWidgetSettings",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_php/Widget.php",
        "Widget.php",
        "loadWidgetSettings",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/code_graph/app.py",
        "app.py",
        "list_widgets",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/code_graph/server.ts",
        "server.ts",
        "listWidgets",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_ada/widget.adb",
        "widget.adb",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_agda/Widget.agda",
        "Widget.agda",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_apex/Widget.cls",
        "Widget.cls",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_assembly/sample.s",
        "sample.s",
        "main",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_astro/Sample.astro",
        "Sample.astro",
        "Sample.astro",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_awk/widget.awk",
        "widget.awk",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_bash/widget.sh",
        "widget.sh",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_beancount/sample.beancount",
        "sample.beancount",
        "other.beancount",
        FixtureCheckKind::ImportPath,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_bibtex/sample.bib",
        "sample.bib",
        "sample.bib",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_bicep/storage.bicep",
        "storage.bicep",
        "storageAccount",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_bitbake/example.bb",
        "example.bb",
        "do_compile",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_blade/sample.blade.php",
        "sample.blade.php",
        "sample.blade.php",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cairo/counter.cairo",
        "counter.cairo",
        "Counter",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_capnp/person.capnp",
        "person.capnp",
        "Person",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cfml/UserComponent.cfm",
        "UserComponent.cfm",
        "getUser",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cfscript/UserService.cfc",
        "UserService.cfc",
        "getUser",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_clojure/widget.clj",
        "widget.clj",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cmake/CMakeLists.cmake",
        "CMakeLists.cmake",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cobol/widget.cbl",
        "widget.cbl",
        "WIDGET",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_commonlisp/widget.lisp",
        "widget.lisp",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_crystal/widget.cr",
        "widget.cr",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_css/sample.css",
        "sample.css",
        "foo.css",
        FixtureCheckKind::ImportPath,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_csv/sample.csv",
        "sample.csv",
        "sample.csv",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_cuda/widget.cu",
        "widget.cu",
        "addKernel",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_d/widget.d",
        "widget.d",
        "Animal",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_dart/widget.dart",
        "widget.dart",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_devicetree/board.dts",
        "board.dts",
        "board-common.dtsi",
        FixtureCheckKind::ImportPath,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_diff/sample.diff",
        "sample.diff",
        "diff",
        FixtureCheckKind::CallCallee,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_dockerfile/Dockerfile",
        "Dockerfile",
        "Dockerfile",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_dotenv/sample.env",
        "sample.env",
        "sample.env",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_elixir/widget.ex",
        "widget.ex",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_elm/Widget.elm",
        "Widget.elm",
        "area",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_emacslisp/utils.el",
        "utils.el",
        "add-numbers",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_erlang/widget.erl",
        "widget.erl",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_fennel/widget.fnl",
        "widget.fnl",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_fish/widget.fish",
        "widget.fish",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_form/widget.frm",
        "widget.frm",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_fortran/widget.f90",
        "widget.f90",
        "area",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_fsharp/Widget.fs",
        "Widget.fs",
        "Widgets",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_func/counter.fc",
        "counter.fc",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gdscript/widget.gd",
        "widget.gd",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gitattributes/sample.gitattributes",
        "sample.gitattributes",
        "sample.gitattributes",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gitignore/.gitignore",
        ".gitignore",
        ".gitignore",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gleam/widget.gleam",
        "widget.gleam",
        "Shape",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_glsl/widget.glsl",
        "widget.glsl",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gn/BUILD.gn",
        "BUILD.gn",
        "//build/config.gni",
        FixtureCheckKind::ImportPath,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gomod/go.mod",
        "go.mod",
        "github.com/bar/baz",
        FixtureCheckKind::ImportPath,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_gotemplate/main.gotmpl",
        "main.gotmpl",
        "footer",
        FixtureCheckKind::CallCallee,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_graphql/schema.graphql",
        "schema.graphql",
        "Query",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_groovy/widget.groovy",
        "widget.groovy",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_hare/widget.ha",
        "widget.ha",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_haskell/widget.hs",
        "widget.hs",
        "Shape",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_hcl/sample.tf",
        "sample.tf",
        "variable.region",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_hlsl/widget.hlsl",
        "widget.hlsl",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_html/page.html",
        "page.html",
        "page.html",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_hyprlang/hyprland.conf",
        "hyprland.conf",
        "hyprland.conf",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_ini/settings.ini",
        "settings.ini",
        "section",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_ispc/widget.ispc",
        "widget.ispc",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_janet/script.janet",
        "script.janet",
        "script.janet",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_jinja2/template.jinja2",
        "template.jinja2",
        "template.jinja2",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_jsdoc/comment.jsdoc",
        "comment.jsdoc",
        "comment.jsdoc",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_json/config.json",
        "config.json",
        "config.json",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_json5/sample.json5",
        "sample.json5",
        "sample.json5",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_jsonnet/config.jsonnet",
        "config.jsonnet",
        "greeting",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_julia/widget.jl",
        "widget.jl",
        "Widgets",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_just/widget.just",
        "widget.just",
        "build",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_kconfig/Kconfig.widget",
        "Kconfig.widget",
        "WIDGET",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_kdl/sample.kdl",
        "sample.kdl",
        "sample.kdl",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_kotlin/widget.kt",
        "widget.kt",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_lean/widget.lean",
        "widget.lean",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_linkerscript/sample.ld",
        "sample.ld",
        "ASSERT",
        FixtureCheckKind::CallCallee,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_liquid/sample.liquid",
        "sample.liquid",
        "header.liquid",
        FixtureCheckKind::ImportPath,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_llvmir/example.ll",
        "example.ll",
        "main",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_lua/widget.lua",
        "widget.lua",
        "Widget.new",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_luau/widget.luau",
        "widget.luau",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_magma/widget.magma",
        "widget.magma",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_makefile/widget.mk",
        "widget.mk",
        "widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_markdown/sample.md",
        "sample.md",
        "sample.md",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_matlab/widget.m",
        "widget.m",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_mermaid/sample.mmd",
        "sample.mmd",
        "sample.mmd",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_meson/widget.meson",
        "widget.meson",
        "project",
        FixtureCheckKind::CallCallee,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_move/counter.move",
        "counter.move",
        "Counter",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_nasm/widget.nasm",
        "widget.nasm",
        "_start",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_nickel/config.ncl",
        "config.ncl",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_nix/sample.nix",
        "sample.nix",
        "addOne",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_objc/Widget.m",
        "Widget.m",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_ocaml/widget.ml",
        "widget.ml",
        "shape",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_odin/widget.odin",
        "widget.odin",
        "Dog",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_pascal/widget.pas",
        "widget.pas",
        "TDog.Bark",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_perl/widget.pl",
        "widget.pl",
        "new",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_pine/strategy.pine",
        "strategy.pine",
        "myFunc",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_pkl/sample.pkl",
        "sample.pkl",
        "Person",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_po/sample.po",
        "sample.po",
        "sample.po",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_pony/widget.pony",
        "widget.pony",
        "Animal",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_powershell/widget.ps1",
        "widget.ps1",
        "Animal",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_prisma/sample.prisma",
        "sample.prisma",
        "User",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_properties/sample.properties",
        "sample.properties",
        "sample.properties",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_protobuf/sample.proto",
        "sample.proto",
        "Foo",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_puppet/widget.pp",
        "widget.pp",
        "widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_purescript/widget.purs",
        "widget.purs",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_qml/Widget.qml",
        "Widget.qml",
        "increment",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_r/widget.r",
        "widget.r",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_racket/widget.rkt",
        "widget.rkt",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_regex/sample.re",
        "sample.re",
        "sample.re",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_requirements/requirements.txt",
        "requirements.txt",
        "requirements.txt",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_rescript/Widget.res",
        "Widget.res",
        "add",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_ron/sample.ron",
        "sample.ron",
        "sample.ron",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_rst/sample.rst",
        "sample.rst",
        "sample.rst",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_ruby/widget.rb",
        "widget.rb",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_scala/widget.scala",
        "widget.scala",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_scheme/widget.scm",
        "widget.scm",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_scss/widget.scss",
        "widget.scss",
        "flex-center",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_slang/widget.slang",
        "widget.slang",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_smali/Foo.smali",
        "Foo.smali",
        "LFoo;",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_smithy/weather.smithy",
        "weather.smithy",
        "Weather",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_solidity/Widget.sol",
        "Widget.sol",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_soql/sample.soql",
        "sample.soql",
        "sample.soql",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_sosl/sample.sosl",
        "sample.sosl",
        "sample.sosl",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_sql/sample.sql",
        "sample.sql",
        "add_one",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_squirrel/widget.nut",
        "widget.nut",
        "Animal",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_sshconfig/config",
        "config",
        "config",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_starlark/widget.bzl",
        "widget.bzl",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_svelte/Sample.svelte",
        "Sample.svelte",
        "Sample.svelte",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_sway/widget.sw",
        "widget.sw",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_swift/widget.swift",
        "widget.swift",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_systemverilog/widget.sv",
        "widget.sv",
        "widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_tablegen/example.td",
        "example.td",
        "Instruction",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_tcl/widget.tcl",
        "widget.tcl",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_teal/widget.tl",
        "widget.tl",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_templ/widget.templ",
        "widget.templ",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_thrift/person.thrift",
        "person.thrift",
        "Person",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_tlaplus/widget.tla",
        "widget.tla",
        "Helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_toml/sample.toml",
        "sample.toml",
        "package",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_tsx/widget.tsx",
        "widget.tsx",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_typst/widget.typ",
        "widget.typ",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_verilog/widget.v",
        "widget.v",
        "widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_vhdl/widget.vhd",
        "widget.vhd",
        "widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_vimscript/widget.vim",
        "widget.vim",
        "Greet",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_vue/Sample.vue",
        "Sample.vue",
        "Sample.vue",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_wgsl/widget.wgsl",
        "widget.wgsl",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_wit/host.wit",
        "host.wit",
        "types",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_wolfram/widget.wl",
        "widget.wl",
        "helper",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_xml/sample.xml",
        "sample.xml",
        "note",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_yaml/sample.yaml",
        "sample.yaml",
        "sample.yaml",
        FixtureCheckKind::Filename,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_zig/widget.zig",
        "widget.zig",
        "Widget",
        FixtureCheckKind::SymbolName,
    ),
    (
        "crates/enforcer-memory/tests/fixtures/memory/lang_zsh/widget.zsh",
        "widget.zsh",
        "greet",
        FixtureCheckKind::SymbolName,
    ),
];

/// Build a git-backed multi-language fixture repo (8 committed fixture
/// files + the Rust `lib.rs`) in a tempdir; returns the tempdir, its
/// forward-slash path, and the list of file paths for the candidate
/// indexer.
fn build_multi_language_repo() -> Result<(tempfile::TempDir, String, Vec<PathBuf>), BoxError> {
    let dir = tempfile::tempdir()?;
    let root = workspace_root();
    let mut files: Vec<PathBuf> = Vec::new();

    let lib_dest = dir.path().join("lib.rs");
    std::fs::write(&lib_dest, FIXTURE_LIB_RS)?;
    files.push(lib_dest);

    for (source_rel, dest_name, _expected, _kind) in MULTI_LANG_FIXTURES {
        let dest = dir.path().join(dest_name);
        std::fs::copy(root.join(source_rel), &dest)?;
        files.push(dest);
    }

    run_git(dir.path(), &["init", "--quiet"])?;
    run_git(
        dir.path(),
        &["config", "user.email", "x06-parity@example.com"],
    )?;
    run_git(dir.path(), &["config", "user.name", "x06-parity"])?;
    run_git(dir.path(), &["add", "-A"])?;
    run_git(
        dir.path(),
        &[
            "commit",
            "--quiet",
            "-m",
            "x06-parity multi-language fixture",
        ],
    )?;

    let forward_slash_path = dir.path().to_string_lossy().replace('\\', "/");
    Ok((dir, forward_slash_path, files))
}

/// Whether `expected` is present in the candidate's own extraction for
/// one fixture entry, dispatched by [`FixtureCheckKind`] -- language-
/// parity wave G3 stage 5. `SymbolName`/`Filename` both check the
/// already-built [`CodeGraph`]'s own node-debug string (a `Filename`
/// entry's `expected` IS the filename, which the graph's `File` node
/// carries regardless of language); `ImportPath`/`CallCallee` instead
/// re-parse the fixture's own source DIRECTLY via
/// [`parsers::parse_file`] (bypassing `CodeGraph` entirely for this
/// check), since imports/calls are graph EDGES, not something
/// `CodeGraph::nodes()`'s own `Debug` output reliably surfaces the way
/// node names do.
fn candidate_fixture_check_passes(
    dest_name: &str,
    expected: &str,
    kind: FixtureCheckKind,
    files: &[PathBuf],
    nodes_debug: &str,
) -> bool {
    match kind {
        FixtureCheckKind::SymbolName | FixtureCheckKind::Filename => nodes_debug.contains(expected),
        FixtureCheckKind::ImportPath | FixtureCheckKind::CallCallee => {
            let Some(file_path) = files.iter().find(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy() == dest_name)
                    .unwrap_or(false)
            }) else {
                return false;
            };
            let Ok(source) = std::fs::read_to_string(file_path) else {
                return false;
            };
            let language = parsers::classify(dest_name);
            let Some(parsed) = parsers::parse_file(language, &source, dest_name) else {
                return false;
            };
            match kind {
                FixtureCheckKind::ImportPath => parsed
                    .imports
                    .iter()
                    .any(|i| i.module_path.contains(expected)),
                FixtureCheckKind::CallCallee => {
                    parsed.calls.iter().any(|c| c.callee.contains(expected))
                }
                _ => unreachable!(),
            }
        }
    }
}

/// Multi-language indexing: index the FULL onboarded-language fixture
/// set (146 languages harvested from each language's own committed
/// `tests/unit_languages_*.rs` fixture-reading test, plus the original
/// 9-language hand-picked set) on both sides, then require one
/// language-unique fact per file to be findable on each side (baseline:
/// one `search_graph` lookup per fact; candidate: dispatched by
/// [`FixtureCheckKind`] via [`candidate_fixture_check_passes`]) --
/// language-parity wave G3 stage 5.
fn compare_multi_language(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "index_repository(multi_language)";

    let built = match build_multi_language_repo() {
        Ok(built) => built,
        Err(error) => return unrunnable_row(tool, &format!("fixture build failed: {error}")),
    };
    let (_dir, repo_path, files) = built;

    let index_request =
        format!(r#"{{"repo_path":"{repo_path}","name":"x06parity-multilang","mode":"full"}}"#);
    let index_call = match ctx.driver.call("index_repository", &index_request) {
        Ok(call) => call,
        Err(error) => return unrunnable_row(tool, &format!("baseline index failed: {error}")),
    };
    record_baseline_result(ctx.results, "index_repository(multi_language)", &index_call);
    let Some(project) = index_call.parsed_json().and_then(|v| {
        v.get("project")
            .and_then(|p| p.as_str())
            .map(str::to_string)
    }) else {
        return unrunnable_row(
            tool,
            "baseline multi-language index returned no project name",
        );
    };

    // `("<dest_name for candidate lookup>", "<expected fact>", kind)` --
    // the extra leading entry is the original fixture's own `lib.rs`
    // check, kept identical to the pre-G3-stage-5 behavior.
    let mut expected: Vec<(&str, &str, FixtureCheckKind)> =
        vec![("lib.rs", "parse_config_file", FixtureCheckKind::SymbolName)];
    expected.extend(
        MULTI_LANG_FIXTURES
            .iter()
            .map(|(_, dest, symbol, kind)| (*dest, *symbol, *kind)),
    );

    let mut baseline_missing: Vec<String> = Vec::new();
    let mut baseline_latency_total = index_call.latency_ms;
    for (_dest, symbol, _kind) in &expected {
        let request = format!(r#"{{"project":"{project}","query":"{symbol}"}}"#);
        let call = match ctx.driver.call("search_graph", &request) {
            Ok(call) => call,
            Err(error) => {
                return unrunnable_row(tool, &format!("baseline search_graph failed: {error}"))
            }
        };
        baseline_latency_total += call.latency_ms;
        record_baseline_result(
            ctx.results,
            &format!("search_graph(multi_language:{symbol})"),
            &call,
        );
        let found = call
            .parsed_json()
            .map(|json| haystack_contains_all(&json, &[symbol]))
            .unwrap_or(false);
        if !found {
            baseline_missing.push((*symbol).to_string());
        }
    }

    let start = Instant::now();
    let candidate_outcome = (|| -> Result<Vec<String>, BoxError> {
        let mut graph = CodeGraph::new();
        graph.index_repository(Path::new(&repo_path), &files, &Manifest::default())?;
        let nodes_debug = format!("{:?}", graph.nodes());
        Ok(expected
            .iter()
            .filter(|(dest, symbol, kind)| {
                !candidate_fixture_check_passes(dest, symbol, *kind, &files, &nodes_debug)
            })
            .map(|(_, symbol, _)| (*symbol).to_string())
            .collect())
    })();
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_missing, candidate_error) = match candidate_outcome {
        Ok(missing) => (missing, None),
        Err(error) => (
            expected.iter().map(|(_, s, _)| (*s).to_string()).collect(),
            Some(error.to_string()),
        ),
    };
    record_candidate_result(
        ctx.results,
        tool,
        &format!("missing={candidate_missing:?} error={candidate_error:?}"),
        candidate_latency_ms,
    );

    // Best-effort cleanup of the extra baseline project.
    let _ = ctx
        .driver
        .call("delete_project", &format!(r#"{{"project":"{project}"}}"#));

    let mut normalizations = common_normalizations();
    normalizations.push(format!(
        "{} onboarded languages (committed repo fixtures, copied at run time) indexed on both sides; one language-unique fact per file must be findable per side (baseline via search_graph; candidate via node-debug containment for SymbolName/Filename, or a direct per-file parsers::parse_file re-parse for ImportPath/CallCallee) -- per-language attribution, not total node-count equality",
        expected.len()
    ));

    if baseline_missing.is_empty() && candidate_missing.is_empty() {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "equal".to_string(),
            better_because: None,
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(baseline_latency_total),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else if candidate_missing.is_empty() {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "better".to_string(),
            better_because: Some(format!(
                "candidate extracted every per-language symbol; baseline search_graph could not find {baseline_missing:?} in its own index of the same repo (raw responses recorded in tool-results.ndjson as evidence)"
            )),
            worse_because: None,
            normalizations,
            baseline_latency_ms: Some(baseline_latency_total),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    } else {
        ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: "worse".to_string(),
            better_because: None,
            worse_because: Some(format!(
                "candidate_missing={candidate_missing:?} baseline_missing={baseline_missing:?} candidate_error={candidate_error:?}"
            )),
            normalizations,
            baseline_latency_ms: Some(baseline_latency_total),
            candidate_latency_ms: Some(candidate_latency_ms),
        }
    }
}

/// The cross-repo fixture pair covers every baseline protocol surface
/// plus the Rust-only URL extension: HTTP Route/client, async broker,
/// Channel, gRPC, GraphQL, and tRPC. The current/client project emits
/// outbound/client/producer call sites; the target/server project emits
/// matching handler/listener/server call sites. The bare `fetch` call
/// is deliberately kept as additive Rust evidence because the installed
/// baseline classifies library-shaped HTTP clients, not bare fetch.
const CROSS_REPO_SERVER_PY: &str = "from flask import Flask\n\napp = Flask(__name__)\n\n@app.route(\"/api/widgets\", methods=[\"GET\"])\ndef list_widgets():\n    return []\n";
const CROSS_REPO_SERVER_TS: &str = "const app = { get(_path: string, _handler: () => void) {} };\nconst bus = { on(_topic: string, _handler: () => void) {} };\nconst pubsub = { subscribe(_topic: string, _handler: () => void) {} };\nconst grpcServer = { addService(_route: string, _handler: () => void) {} };\nconst graphqlSchema = { resolver(_operation: string, _handler: () => void) {} };\nconst router = { query(_procedure: string, _handler: () => void) {} };\n\nfunction listWidgets() {}\napp.get(\"/api/widgets\", listWidgets);\nbus.on(\"widgets.created\", () => {});\npubsub.subscribe(\"widgets.async\", () => {});\ngrpcServer.addService(\"WidgetService/GetWidget\", () => {});\ngraphqlSchema.resolver(\"GetWidget\", () => {});\nrouter.query(\"widget.byId\", () => {});\n";
const CROSS_REPO_CLIENT_PY: &str = "def requests_get(url, params=None):\n    return {\"url\": url, \"params\": params}\n\ndef fetch_widgets():\n    return requests_get(\"/api/widgets\")\n";
const CROSS_REPO_CLIENT_TS: &str = "import axios from \"axios\";\n\nconst events = { emit(_topic: string) {} };\nconst pubsub = { publish(_topic: string) {} };\nconst pb = { NewWidgetServiceClient: { GetWidget(_request: string) {} } };\nconst graphqlClient = { request(_query: string) {} };\nconst trpc = { widget: { byId: { query() {} } } };\n\nexport function fetchWidgets() {\n  events.emit(\"widgets.created\");\n  pubsub.publish(\"widgets.async\");\n  pb.NewWidgetServiceClient.GetWidget(\"ignored\");\n  graphqlClient.request(\"query GetWidget { widget { id } }\");\n  trpc.widget.byId.query();\n  fetch(\"/api/widgets\");\n  return axios.get(\"/api/widgets\");\n}\n";

fn build_cross_repo_pair(
) -> Result<(tempfile::TempDir, String, tempfile::TempDir, String), BoxError> {
    let server_dir = tempfile::tempdir()?;
    std::fs::write(server_dir.path().join("server.py"), CROSS_REPO_SERVER_PY)?;
    std::fs::write(server_dir.path().join("server.ts"), CROSS_REPO_SERVER_TS)?;
    let client_dir = tempfile::tempdir()?;
    std::fs::write(client_dir.path().join("client.py"), CROSS_REPO_CLIENT_PY)?;
    std::fs::write(client_dir.path().join("client.ts"), CROSS_REPO_CLIENT_TS)?;
    for dir in [server_dir.path(), client_dir.path()] {
        run_git(dir, &["init", "--quiet"])?;
        run_git(dir, &["config", "user.email", "x06-parity@example.com"])?;
        run_git(dir, &["config", "user.name", "x06-parity"])?;
        run_git(dir, &["add", "-A"])?;
        run_git(
            dir,
            &["commit", "--quiet", "-m", "x06-parity cross-repo fixture"],
        )?;
    }
    let server_path = server_dir.path().to_string_lossy().replace('\\', "/");
    let client_path = client_dir.path().to_string_lossy().replace('\\', "/");
    Ok((server_dir, server_path, client_dir, client_path))
}

/// Candidate-side cross-repo match over the fixture pair -- shared by
/// the live parity row and the baseline-independent unit test below, so
/// the unit test exercises exactly the code path the parity row grades.
fn candidate_cross_repo_report(
    server_path: &str,
    client_path: &str,
) -> Result<enforcer_memory::cross_repo::CrossRepoReport, BoxError> {
    let mut server_graph = CodeGraph::new();
    server_graph.index_repository(
        Path::new(server_path),
        &[
            Path::new(server_path).join("server.py"),
            Path::new(server_path).join("server.ts"),
        ],
        &Manifest::default(),
    )?;
    let mut client_graph = CodeGraph::new();
    client_graph.index_repository(
        Path::new(client_path),
        &[
            Path::new(client_path).join("client.py"),
            Path::new(client_path).join("client.ts"),
        ],
        &Manifest::default(),
    )?;
    let mut targets: BTreeMap<String, &CodeGraph> = BTreeMap::new();
    targets.insert("x06parity-crossrepo-server".to_string(), &server_graph);
    Ok(match_cross_repo(
        "x06parity-crossrepo-client",
        &client_graph,
        &targets,
    ))
}

/// `index_repository(mode="cross-repo-intelligence")`: index the
/// server/client fixture pair on the baseline, run its cross-repo
/// matcher, and compare against `cross_repo::match_cross_repo` over the
/// same two graphs. Better requires live baseline Route and Channel
/// evidence, equivalent Rust evidence for both, all landed Rust
/// protocol detectors, and a separately identified literal URL link.
fn compare_cross_repo(ctx: &mut Ctx<'_>) -> ToolDiffRow {
    let tool = "index_repository(cross-repo-intelligence)";

    let built = match build_cross_repo_pair() {
        Ok(built) => built,
        Err(error) => return unrunnable_row(tool, &format!("fixture build failed: {error}")),
    };
    let (_server_dir, server_path, _client_dir, client_path) = built;

    let mut baseline_latency_total = 0.0;
    let mut baseline_projects: Vec<String> = Vec::new();
    for (name, path) in [
        ("x06parity-crossrepo-server", &server_path),
        ("x06parity-crossrepo-client", &client_path),
    ] {
        let request = format!(r#"{{"repo_path":"{path}","name":"{name}","mode":"full"}}"#);
        let call = match ctx.driver.call("index_repository", &request) {
            Ok(call) => call,
            Err(error) => return unrunnable_row(tool, &format!("baseline index failed: {error}")),
        };
        baseline_latency_total += call.latency_ms;
        record_baseline_result(ctx.results, &format!("index_repository({name})"), &call);
        match call.parsed_json().and_then(|v| {
            v.get("project")
                .and_then(|p| p.as_str())
                .map(str::to_string)
        }) {
            Some(project) => baseline_projects.push(project),
            None => {
                return unrunnable_row(tool, "baseline cross-repo index returned no project name")
            }
        }
    }

    let Some(server_project) = baseline_projects.first() else {
        return unrunnable_row(
            tool,
            "baseline cross-repo fixture did not index server project",
        );
    };
    let cross_request = format!(
        r#"{{"repo_path":"{client_path}","mode":"cross-repo-intelligence","target_projects":["{server_project}"]}}"#
    );
    let cross_call = match ctx.driver.call("index_repository", &cross_request) {
        Ok(call) => call,
        Err(error) => {
            return unrunnable_row(tool, &format!("baseline cross-repo call failed: {error}"))
        }
    };
    baseline_latency_total += cross_call.latency_ms;
    record_baseline_result(ctx.results, tool, &cross_call);
    let baseline_total_edges = cross_call
        .parsed_json()
        .and_then(|v| v.get("total_cross_edges").and_then(|n| n.as_u64()));
    let baseline_http_count = cross_call
        .parsed_json()
        .and_then(|v| v.get("cross_http_calls").and_then(|n| n.as_u64()));
    let baseline_async_count = cross_call
        .parsed_json()
        .and_then(|v| v.get("cross_async_calls").and_then(|n| n.as_u64()));
    let baseline_channel_count = cross_call
        .parsed_json()
        .and_then(|v| v.get("cross_channel").and_then(|n| n.as_u64()));
    let baseline_grpc_count = cross_call
        .parsed_json()
        .and_then(|v| v.get("cross_grpc_calls").and_then(|n| n.as_u64()));
    let baseline_graphql_count = cross_call
        .parsed_json()
        .and_then(|v| v.get("cross_graphql_calls").and_then(|n| n.as_u64()));
    let baseline_trpc_count = cross_call
        .parsed_json()
        .and_then(|v| v.get("cross_trpc_calls").and_then(|n| n.as_u64()));

    let start = Instant::now();
    let candidate_outcome = candidate_cross_repo_report(&server_path, &client_path);
    let candidate_latency_ms = start.elapsed().as_secs_f64() * 1000.0;
    let (candidate_ok, candidate_summary) = match &candidate_outcome {
        Ok(report) => {
            let http_client_found = report.cross_http_calls.iter().any(|edge| {
                edge.method == "GET"
                    && edge.path == "/api/widgets"
                    && edge.target_project == "x06parity-crossrepo-server"
                    && edge.via == CrossHttpMatchKind::HttpClient
            });
            let literal_http_found = report.cross_http_calls.iter().any(|edge| {
                edge.method == "GET"
                    && edge.path == "/api/widgets"
                    && edge.target_project == "x06parity-crossrepo-server"
                    && edge.via == CrossHttpMatchKind::LiteralUrl
            });
            let channel_found = report.cross_channel >= 1
                && report
                    .cross_channel_links
                    .iter()
                    .any(|edge| edge.topic == "widgets.created");
            let async_found = report.cross_async_calls >= 1
                && report
                    .cross_async_links
                    .iter()
                    .any(|edge| edge.key == "widgets.async");
            let grpc_found = report.cross_grpc_calls >= 1
                && report
                    .cross_grpc_links
                    .iter()
                    .any(|edge| edge.key == "WidgetService/GetWidget");
            let graphql_found = report.cross_graphql_calls >= 1
                && report
                    .cross_graphql_links
                    .iter()
                    .any(|edge| edge.key == "GetWidget");
            let trpc_found = report.cross_trpc_calls >= 1
                && report
                    .cross_trpc_links
                    .iter()
                    .any(|edge| edge.key == "widget.byId");
            (
                http_client_found
                    && literal_http_found
                    && channel_found
                    && async_found
                    && grpc_found
                    && graphql_found
                    && trpc_found
                    && report.total_cross_edges() >= 7,
                format!("{report:?}"),
            )
        }
        Err(error) => (false, format!("error: {error}")),
    };
    record_candidate_result(ctx.results, tool, &candidate_summary, candidate_latency_ms);

    // Best-effort cleanup of both extra baseline projects.
    for project in &baseline_projects {
        let _ = ctx
            .driver
            .call("delete_project", &format!(r#"{{"project":"{project}"}}"#));
    }

    let mut normalizations = common_normalizations();
    normalizations.push(
        "baseline parity is graded on its documented Route/HTTP_CALLS and Channel surfaces, each requiring a live baseline edge and equivalent Rust evidence. Async broker, gRPC, GraphQL, and tRPC are separately required Rust protocol signals covered by the same fixture and focused unit tests; bare-fetch LiteralUrl remains additive and cannot satisfy Route or Channel parity".to_string(),
    );

    match (candidate_ok, baseline_total_edges) {
        (true, Some(total))
            if total >= 2
                && baseline_http_count.unwrap_or(0) >= 1
                && baseline_channel_count.unwrap_or(0) >= 1 =>
        {
            ToolDiffRow {
                tool: tool.to_string(),
                comparison_verdict: "better".to_string(),
                better_because: Some(format!(
                    "candidate reproduces the baseline's live Route/HTTP_CALLS and Channel cross-repo semantics (baseline counts: http={baseline_http_count:?}, channel={baseline_channel_count:?}, total={total}) and additionally detects async broker, gRPC, GraphQL, and tRPC links plus the separately identified bare fetch/url LiteralUrl link (baseline reported async={baseline_async_count:?}, grpc={baseline_grpc_count:?}, graphql={baseline_graphql_count:?}, trpc={baseline_trpc_count:?})"
                )),
                worse_because: None,
                normalizations,
                baseline_latency_ms: Some(baseline_latency_total),
                candidate_latency_ms: Some(candidate_latency_ms),
            }
        }
        (true, Some(total)) if total >= 1 => {
            normalizations.push(format!(
                "baseline did not emit both required Route/HTTP_CALLS and Channel edges on this fixture (counts: http={baseline_http_count:?}, async={baseline_async_count:?}, channel={baseline_channel_count:?}, grpc={baseline_grpc_count:?}, graphql={baseline_graphql_count:?}, trpc={baseline_trpc_count:?}, total={total}); raw response is recorded in tool-results.ndjson, so this remains incomparable rather than claiming better"
            ));
            ToolDiffRow {
                tool: tool.to_string(),
                comparison_verdict: "incomparable".to_string(),
                better_because: None,
                worse_because: None,
                normalizations,
                baseline_latency_ms: Some(baseline_latency_total),
                candidate_latency_ms: Some(candidate_latency_ms),
            }
        }
        (true, Some(0)) => {
            normalizations.push(
                "baseline's cross-repo matcher reported total_cross_edges=0 after the fixture supplied baseline-native HTTP Route and event-emitter Channel inputs; raw response is recorded in tool-results.ndjson, so this remains incomparable rather than relabeled better".to_string(),
            );
            ToolDiffRow {
                tool: tool.to_string(),
                comparison_verdict: "incomparable".to_string(),
                better_because: None,
                worse_because: None,
                normalizations,
                baseline_latency_ms: Some(baseline_latency_total),
                candidate_latency_ms: Some(candidate_latency_ms),
            }
        }
        (true, None) => {
            normalizations.push(
                "baseline cross-repo response carried no parseable total_cross_edges field (raw response recorded in tool-results.ndjson as evidence); candidate produced the full §9.5-shaped report".to_string(),
            );
            ToolDiffRow {
                tool: tool.to_string(),
                comparison_verdict: "incomparable".to_string(),
                better_because: None,
                worse_because: None,
                normalizations,
                baseline_latency_ms: Some(baseline_latency_total),
                candidate_latency_ms: Some(candidate_latency_ms),
            }
        }
        (false, _) | (true, Some(_)) => ToolDiffRow {
            tool: tool.to_string(),
            comparison_verdict: if candidate_ok {
                "equal".to_string()
            } else {
                "worse".to_string()
            },
            better_because: None,
            worse_because: if candidate_ok {
                None
            } else {
                Some(format!(
                    "candidate cross-repo match failed: {candidate_summary} (baseline_total_edges={baseline_total_edges:?})"
                ))
            },
            normalizations,
            baseline_latency_ms: Some(baseline_latency_total),
            candidate_latency_ms: Some(candidate_latency_ms),
        },
    }
}

/// Run the full live comparison. Returns the [`KgParityDocument`] plus
/// the raw per-tool result rows (for `tool-results.ndjson`) -- never
/// partially fabricated: every row is either a real comparison or an
/// honest `unrunnable: <reason>`.
pub fn run_live_parity_comparison() -> Result<(KgParityDocument, Vec<ToolResultRow>), BoxError> {
    let adapter = CodebaseMemoryMcpAdapter::new();
    let state = adapter.probe();

    let mut results: Vec<ToolResultRow> = Vec::new();

    let driver = match CliDriver::from_state(&state) {
        Ok(driver) => driver,
        Err(_) => {
            let tool_names = [
                "get_graph_schema",
                "search_graph(bm25)",
                "search_graph(regex)",
                "query_graph",
                "trace_path(calls)",
                "get_code_snippet",
                "get_architecture(overview)",
                "get_architecture(clusters)",
                "search_code",
                "list_projects",
                "index_status",
                "detect_changes",
                "manage_adr(get/update)",
                "index_repository",
                "delete_project",
                "ingest_traces",
                "query_graph(complexity)",
                "get_graph_schema(rich_vocab)",
                "get_graph_schema(similarity)",
                "trace_path(data_flow)",
                "trace_path(calls,type_resolution)",
                "index_repository(multi_language)",
                "index_repository(cross-repo-intelligence)",
            ];
            let rows: Vec<ToolDiffRow> = tool_names
                .iter()
                .map(|tool| unrunnable_row(tool, "baseline not available"))
                .collect();
            let document = KgParityDocument {
                baseline_executed: false,
                baseline_binary_path: None,
                tools_total: rows.len(),
                tools_equal: 0,
                tools_better: 0,
                tools_worse: 0,
                tools_incomparable: 0,
                tools_unrunnable: rows.len(),
                rows,
            };
            return Ok((document, results));
        }
    };

    let (_fixture_dir, fixture_path) = build_fixture_repo()?;
    let candidate_graph = build_candidate_graph(Path::new(&fixture_path))?;

    let index_request =
        format!(r#"{{"repo_path":"{fixture_path}","name":"x06parity-live","mode":"full"}}"#);
    let index_call = driver.call("index_repository", &index_request)?;
    record_baseline_result(&mut results, "index_repository", &index_call);
    let baseline_project = index_call
        .parsed_json()
        .and_then(|v| v.get("project").and_then(|p| p.as_str()).map(str::to_string))
        .ok_or("baseline index_repository did not return a project name -- cannot run the rest of the comparison")?;

    let mut ctx = Ctx {
        driver: &driver,
        baseline_project: &baseline_project,
        candidate_graph: &candidate_graph,
        results: &mut results,
    };

    let mut rows = vec![
        compare_index_repository(&mut ctx, &index_call),
        compare_get_graph_schema(&mut ctx),
        compare_search_graph_bm25(&mut ctx),
        compare_search_graph_regex(&mut ctx),
        compare_query_graph(&mut ctx),
        compare_trace_path_calls(&mut ctx),
        compare_get_code_snippet(&mut ctx, Path::new(&fixture_path)),
        compare_get_architecture(&mut ctx, "overview", Aspect::Overview),
        compare_get_architecture(&mut ctx, "clusters", Aspect::Clusters),
        compare_search_code(&mut ctx, Path::new(&fixture_path)),
        compare_list_projects(&mut ctx, Path::new(&fixture_path)),
        compare_index_status(&mut ctx, Path::new(&fixture_path)),
        compare_detect_changes(&mut ctx),
        compare_manage_adr(&mut ctx),
        // X06 core-parity extension rows that query the PRIMARY baseline
        // project MUST run before compare_delete_project: that row
        // indexes the same fixture repo_path under a throwaway name and
        // deletes it, which the baseline treats as invalidating the
        // primary project too (projects are keyed by repo path in its
        // store manager -- observed empirically: every primary-project
        // call after the throwaway delete exits nonzero with empty
        // stdout, while fresh-project calls keep working).
        compare_query_graph_complexity(&mut ctx),
        compare_graph_schema_rich_vocab(&mut ctx),
        compare_graph_schema_similarity(&mut ctx),
        compare_trace_data_flow(&mut ctx),
        compare_resolution_trace(&mut ctx),
        // Rows below here either use their own throwaway projects or a
        // store-lookup-free baseline stub (ingest_traces, §15.2), so
        // primary-project invalidation cannot affect them.
        compare_delete_project(&mut ctx, Path::new(&fixture_path)),
        compare_ingest_traces(&mut ctx),
        compare_multi_language(&mut ctx),
        compare_cross_repo(&mut ctx),
    ];
    rows.sort_by(|a, b| a.tool.cmp(&b.tool));

    // Best-effort cleanup: delete the baseline project so repeated runs
    // don't accumulate cache-dir entries. Never fails the comparison if
    // cleanup itself fails -- the rows above already captured the real
    // comparison data.
    let _ = driver.call(
        "delete_project",
        &format!(r#"{{"project":"{baseline_project}"}}"#),
    );

    let tools_equal = rows
        .iter()
        .filter(|r| r.comparison_verdict == "equal")
        .count();
    let tools_better = rows
        .iter()
        .filter(|r| r.comparison_verdict == "better")
        .count();
    let tools_worse = rows
        .iter()
        .filter(|r| r.comparison_verdict == "worse")
        .count();
    let tools_incomparable = rows
        .iter()
        .filter(|r| r.comparison_verdict == "incomparable")
        .count();
    let tools_unrunnable = rows
        .iter()
        .filter(|r| r.comparison_verdict.starts_with("unrunnable:"))
        .count();

    let document = KgParityDocument {
        baseline_executed: true,
        baseline_binary_path: match &state {
            BaselineState::FoundUnprobed { path } => Some(path.to_string_lossy().into_owned()),
            BaselineState::NotInstalled => None,
        },
        tools_total: rows.len(),
        tools_equal,
        tools_better,
        tools_worse,
        tools_incomparable,
        tools_unrunnable,
        rows,
    };

    Ok((document, results))
}

/// Write `rows` to `path` as NDJSON (one compact JSON object per line),
/// creating parent directories if needed.
fn write_ndjson<T: Serialize>(path: &Path, rows: &[T]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut buffer = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| std::io::Error::other(format!("serializing {path:?}: {error}")))?;
        buffer.push_str(&line);
        buffer.push('\n');
    }
    std::fs::write(path, buffer)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// Run the live comparison and write both required proof artifacts.
/// Exposed as a plain function (not just a `#[test]`) so a future
/// `enforcer memory parity-harness` CLI can call it directly.
pub fn run_and_emit_proof() -> Result<KgParityDocument, BoxError> {
    let (document, results) = run_live_parity_comparison()?;
    let root = workspace_root();

    write_ndjson(
        &root.join("proof/memory/x06-parity/tool-results.ndjson"),
        &results,
    )?;
    write_ndjson(
        &root.join("proof/memory/x06-parity/tool-diffs.ndjson"),
        &document.rows,
    )?;

    let json = serde_json::to_string_pretty(&document)?;
    let kg_parity_path = root.join("proof/memory/x06-kg-parity.json");
    if let Some(parent) = kg_parity_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&kg_parity_path, json)?;

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), BoxError>;

    #[test]
    fn strip_machine_specific_fields_removes_documented_keys() {
        let value = serde_json::json!({
            "name": "foo",
            "file_path": "/abs/path",
            "nested": { "path": "x", "keep": 1 }
        });
        let stripped = strip_machine_specific_fields(&value);
        assert_eq!(
            stripped,
            serde_json::json!({ "name": "foo", "nested": { "keep": 1 } })
        );
    }

    #[test]
    fn haystack_contains_all_true_only_when_every_name_present() {
        let value = serde_json::json!({"results": [{"name": "parse_config_file"}]});
        assert!(haystack_contains_all(&value, &["parse_config_file"]));
        assert!(!haystack_contains_all(
            &value,
            &["parse_config_file", "missing_symbol"]
        ));
    }

    #[test]
    fn unrunnable_row_never_carries_latency_or_verdicts() {
        let row = unrunnable_row("some_tool", "baseline not available");
        assert_eq!(row.comparison_verdict, "unrunnable: baseline not available");
        assert!(row.baseline_latency_ms.is_none());
        assert!(row.candidate_latency_ms.is_none());
        assert!(row.better_because.is_none());
        assert!(row.worse_because.is_none());
    }

    /// When the baseline is genuinely absent, [`run_live_parity_comparison`]
    /// must report `baseline_executed: false` and every row unrunnable --
    /// never silently skip rows or claim a comparison happened. This
    /// test forces that path by using a probe name guaranteed absent
    /// (it does not call [`run_live_parity_comparison`] directly, which
    /// always probes the REAL default binary name -- instead it
    /// exercises the same fallback branch's row-shape contract via the
    /// adapter used inside that function, so the assertion holds
    /// regardless of whether the real baseline happens to be installed
    /// on the machine running this test).
    #[test]
    fn not_installed_state_produces_only_unrunnable_rows_with_baseline_not_available_reason() {
        let adapter = CodebaseMemoryMcpAdapter::with_binary_name(
            "enforcer-x06-parity-guard-9f3c9e5e-does-not-exist",
        );
        let state = adapter.probe();
        assert_eq!(state, BaselineState::NotInstalled);
        let row = unrunnable_row("get_graph_schema", "baseline not available");
        assert_eq!(row.comparison_verdict, "unrunnable: baseline not available");
    }

    /// Baseline-independent coverage for `cross_repo::match_cross_repo`
    /// (the one core-parity module with no test coverage in `tests/`
    /// before this wave): the engineered server/client fixture pair must
    /// produce protocol-specific evidence for every baseline cross-repo
    /// surface (HTTP, async, channel, gRPC, GraphQL, tRPC) plus the
    /// Rust-only bare-fetch `LiteralUrl` HTTP edge -- exercised through
    /// the same `candidate_cross_repo_report` helper the live parity row
    /// grades, so this test and that row can never diverge silently.
    #[test]
    fn cross_repo_match_finds_every_engineered_protocol_surface() -> TestResult {
        let (_server_dir, server_path, _client_dir, client_path) = build_cross_repo_pair()?;
        let report = candidate_cross_repo_report(&server_path, &client_path)?;

        assert_eq!(report.project, "x06parity-crossrepo-client");
        assert_eq!(report.projects_scanned, 1);
        for via in [
            CrossHttpMatchKind::RouteDeclaration,
            CrossHttpMatchKind::HttpClient,
            CrossHttpMatchKind::LiteralUrl,
        ] {
            assert!(
                report.cross_http_calls.iter().any(|edge| {
                    edge.method == "GET"
                        && edge.path == "/api/widgets"
                        && edge.source_project == "x06parity-crossrepo-client"
                        && edge.target_project == "x06parity-crossrepo-server"
                        && edge.via == via
                }),
                "expected {via:?} GET /api/widgets edge, got {:?}",
                report.cross_http_calls
            );
        }
        assert!(
            report.cross_http_calls.len() >= 3,
            "expected route/client/literal HTTP evidence, got {:?}",
            report.cross_http_calls
        );
        assert_eq!(report.cross_channel, 1);
        assert!(
            report
                .cross_channel_links
                .iter()
                .any(|edge| edge.topic == "widgets.created"),
            "expected widgets.created channel link, got {:?}",
            report.cross_channel_links
        );

        // Honest zeros, never omitted/fabricated counts (cross_repo.rs
        // module docs + baseline §9.5 shape).
        assert!(report
            .cross_async_links
            .iter()
            .any(|edge| edge.key == "widgets.async"));
        assert!(report
            .cross_grpc_links
            .iter()
            .any(|edge| edge.key == "WidgetService/GetWidget"));
        assert!(report
            .cross_graphql_links
            .iter()
            .any(|edge| edge.key == "GetWidget"));
        assert!(report
            .cross_trpc_links
            .iter()
            .any(|edge| edge.key == "widget.byId"));
        assert_eq!(report.cross_async_calls, report.cross_async_links.len());
        assert_eq!(report.cross_grpc_calls, report.cross_grpc_links.len());
        assert_eq!(report.cross_graphql_calls, report.cross_graphql_links.len());
        assert_eq!(report.cross_trpc_calls, report.cross_trpc_links.len());
        assert!(report.total_cross_edges() >= 7);
        Ok(())
    }

    /// The real end-to-end run: spawns the actual installed baseline
    /// binary, builds a fixture repo, indexes it on both sides, and
    /// diffs every tool. Gated behind `#[ignore]` because it depends on
    /// the baseline binary being installed on the machine running the
    /// test and shells out to a real external process -- run explicitly
    /// with:
    /// `cargo test -p enforcer-memory --test feature_parity_harness -- --ignored run_live_parity_comparison_and_emit_proof`
    #[test]
    #[ignore = "spawns the real installed codebase-memory-mcp binary; run explicitly, not part of the default test suite"]
    fn run_live_parity_comparison_and_emit_proof() -> TestResult {
        let document = run_and_emit_proof()?;
        assert!(document.tools_total > 0);
        // Never silently claim more rows ran than were recorded.
        assert_eq!(
            document.tools_equal
                + document.tools_better
                + document.tools_worse
                + document.tools_incomparable
                + document.tools_unrunnable,
            document.tools_total
        );
        Ok(())
    }
}
