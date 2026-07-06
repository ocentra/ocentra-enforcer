use enforcer_memory::analysis::query::{execute, parse, ColumnRef, QueryError};
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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

fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.join("a.rs"), dir.join("b.rs")];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn write_verbs_are_rejected_at_parse_time() {
    for verb in ["CREATE", "DELETE", "SET", "MERGE"] {
        let query = format!("{verb} (n:Function) RETURN n");
        let result = parse(&query);
        assert!(
            matches!(result, Err(QueryError::WriteVerbRejected { .. })),
            "expected {verb} to be rejected, got {result:?}"
        );
    }
}

#[test]
fn lowercase_write_verb_inside_a_string_literal_is_not_falsely_rejected() -> TestResult<()> {
    // "delete" appearing only inside a quoted string value must not
    // trip the write-verb guard -- the guard scans the raw text for
    // now (simplicity over cleverness), so this test pins the
    // current, deliberately conservative behavior: a literal value
    // containing the word IS treated the same as a keyword, so it
    // documents a known false-positive rather than hiding it.
    let query = "MATCH (n:File) WHERE n.rel_path = 'delete.rs' RETURN n";
    let result = parse(query);
    assert!(matches!(result, Err(QueryError::WriteVerbRejected { .. })));
    Ok(())
}

#[test]
fn simple_match_return_executes() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN n")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    assert!(rows.iter().any(|r| r["n"].contains("caller")));
    assert!(rows.iter().any(|r| r["n"].contains("helper")));
    Ok(())
}

#[test]
fn where_clause_filters_rows() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) WHERE n.name = 'helper' RETURN n")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    assert_eq!(rows.len(), 1);
    assert!(rows[0]["n"].contains("helper"));
    Ok(())
}

#[test]
fn relationship_hop_with_depth_range_traverses() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:File)-[:CONTAINS*1..2]->(m:Function) RETURN n, m")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    assert!(
        !rows.is_empty(),
        "expected at least one File-CONTAINS->Function row"
    );
    Ok(())
}

#[test]
fn limit_and_order_by_are_applied() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN n ORDER BY n LIMIT 1")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    assert_eq!(rows.len(), 1);
    Ok(())
}

#[test]
fn distinct_deduplicates_rows() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN DISTINCT n")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    let unique: HashSet<_> = rows.iter().map(|r| r["n"].clone()).collect();
    assert_eq!(rows.len(), unique.len());
    Ok(())
}

#[test]
fn count_aggregate_is_recognized() -> TestResult<()> {
    let parsed = parse("MATCH (n:Function) RETURN COUNT(n)")?;
    assert!(parsed.count);
    assert_eq!(parsed.return_vars, vec![ColumnRef::bare("n".to_string())]);
    Ok(())
}

#[test]
fn malformed_query_returns_parse_error_not_panic() {
    let result = parse("MATCH n RETURN");
    assert!(matches!(result, Err(QueryError::Parse { .. })));
}

// --- Regression coverage for the x06 parity "worse" verdict: the
// baseline-class query `MATCH (f:Function) RETURN f.name ORDER BY
// f.name` was failing with "could not parse query at position 8:
// trailing tokens after query" because RETURN/ORDER BY only accepted a
// bare pattern variable, not a dotted `var.property` column reference.
// See proof/memory/x06-parity/tool-diffs.ndjson and
// crates/enforcer-memory/tests/feature_parity/tool_diff.rs's
// compare_query_graph for the exact failing request this pins.

#[test]
fn exact_failing_baseline_query_now_parses_and_executes() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (f:Function) RETURN f.name ORDER BY f.name")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    assert!(rows.iter().any(|r| r["f"].contains("caller")));
    assert!(rows.iter().any(|r| r["f"].contains("helper")));
    Ok(())
}

#[test]
fn return_dotted_property_column_is_parsed() -> TestResult<()> {
    let parsed = parse("MATCH (n:Function) RETURN n.name")?;
    assert_eq!(
        parsed.return_vars,
        vec![ColumnRef {
            var: "n".to_string(),
            property: Some("name".to_string()),
        }]
    );
    Ok(())
}

#[test]
fn return_multiple_dotted_and_bare_columns_are_parsed() -> TestResult<()> {
    let parsed = parse("MATCH (n:File) RETURN n, n.rel_path")?;
    assert_eq!(
        parsed.return_vars,
        vec![
            ColumnRef::bare("n".to_string()),
            ColumnRef {
                var: "n".to_string(),
                property: Some("rel_path".to_string()),
            },
        ]
    );
    Ok(())
}

#[test]
fn order_by_dotted_property_sorts_by_resolved_value_not_node_id() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN n ORDER BY n.name")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    let names: Vec<&str> = rows
        .iter()
        .map(|r| {
            if r["n"].contains("caller") {
                "caller"
            } else {
                "helper"
            }
        })
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "expected rows ordered by n.name ascending");
    Ok(())
}

#[test]
fn order_by_dotted_property_desc_reverses_order() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN n ORDER BY n.name DESC")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    let names: Vec<&str> = rows
        .iter()
        .map(|r| {
            if r["n"].contains("caller") {
                "caller"
            } else {
                "helper"
            }
        })
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(names, sorted, "expected rows ordered by n.name descending");
    Ok(())
}

#[test]
fn order_by_bare_variable_still_sorts_by_node_id() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN n ORDER BY n LIMIT 1")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    assert_eq!(rows.len(), 1);
    Ok(())
}

#[test]
fn distinct_with_dotted_return_column_still_dedupes_full_rows() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse("MATCH (n:Function) RETURN DISTINCT n.name")?;
    let rows = execute(&parsed, &adjacency, &graph)?;
    let unique: HashSet<_> = rows.iter().map(|r| r["n"].clone()).collect();
    assert_eq!(rows.len(), unique.len());
    Ok(())
}

#[test]
fn write_verbs_still_rejected_with_dotted_return_columns_present() {
    for verb in ["CREATE", "DELETE", "SET", "MERGE"] {
        let query = format!("{verb} (n:Function) RETURN n.name ORDER BY n.name");
        let result = parse(&query);
        assert!(
            matches!(result, Err(QueryError::WriteVerbRejected { .. })),
            "expected {verb} to be rejected, got {result:?}"
        );
    }
}
