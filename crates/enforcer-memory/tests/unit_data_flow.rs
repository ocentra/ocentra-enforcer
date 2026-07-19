//! X06 core parity: hard tests for [`enforcer_memory::data_flow`] --
//! `DATA_FLOWS` edge materialization from captured call-argument
//! expressions plus [`enforcer_memory::resolution`]'s resolved callees.
//!
//! These tests build `(CallEdge, ResolvedCall)` pairs directly (via
//! [`enforcer_memory::data_flow::materialize_from`]) rather than
//! indexing real source files -- the post-pass under test is a pure
//! function of exactly those two index-aligned slices, and
//! `resolution::resolve`/language-extractor behavior already has its own
//! dedicated test suites (`unit_resolution.rs`, `unit_languages_*.rs`).
//! A smaller end-to-end check ([`materialize_over_a_real_indexed_graph`])
//! confirms the two layers actually compose through a real
//! `CodeGraph::index_repository` run.

use enforcer_domain::memory_types::{
    MemoryDataFlowSourceSymbolId, MemoryDataFlowTargetSymbolId, ResolutionConfidence,
};
use enforcer_memory::code_graph::{CallEdge, CodeGraph, Manifest};
use enforcer_memory::data_flow::{self, DataFlowGraph};
use enforcer_memory::resolution::ResolvedCall;
use std::error::Error;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn call(callee: &str, line: usize, arg_texts: Vec<&str>) -> CallEdge {
    CallEdge {
        from_file_id: "file:caller.rs".to_owned(),
        callee: callee.to_owned(),
        line: line.into(),
        arg_texts: arg_texts.into_iter().map(str::to_owned).collect(),
        from_symbol: Some("caller_fn".to_owned()),
        from_symbol_line: Some(1usize.into()),
        ..CallEdge::default()
    }
}

fn resolved(
    from_symbol_id: Option<&str>,
    candidates: Vec<&str>,
    confidence: ResolutionConfidence,
) -> ResolvedCall {
    ResolvedCall {
        from_symbol_id: from_symbol_id.map(Into::into),
        candidates: candidates.into_iter().map(Into::into).collect(),
        confidence,
    }
}

// ---------------------------------------------------------------------
// Core materialization contract
// ---------------------------------------------------------------------

#[test]
fn resolved_call_with_args_produces_one_edge() {
    let calls = vec![call("helper", 10, vec!["42", "\"x\""])];
    let resolved_calls = vec![resolved(
        Some("sym:caller.rs:1:caller_fn"),
        vec!["sym:callee.rs:5:helper"],
        ResolutionConfidence::Resolved,
    )];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);

    assert_eq!(graph.edges().len(), 1);
    let edge = &graph.edges()[0];
    assert_eq!(
        edge.from_symbol_id.as_deref(),
        Some("sym:caller.rs:1:caller_fn")
    );
    assert_eq!(edge.to_symbol_id, "sym:callee.rs:5:helper");
    assert_eq!(edge.confidence, ResolutionConfidence::Resolved);
    assert_eq!(
        edge.argument_exprs,
        vec!["42".to_owned(), "\"x\"".to_owned()]
    );
    assert_eq!(edge.line, 10);
}

#[test]
fn call_with_no_arguments_produces_no_edge() {
    let calls = vec![call("helper", 10, vec![])];
    let resolved_calls = vec![resolved(
        Some("caller_fn"),
        vec!["sym:callee.rs:5:helper"],
        ResolutionConfidence::Resolved,
    )];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);

    assert!(graph.edges().is_empty());
}

#[test]
fn unresolved_call_produces_no_edge_even_with_arguments() {
    let calls = vec![call("mystery", 10, vec!["1"])];
    let resolved_calls = vec![resolved(
        Some("caller_fn"),
        vec![],
        ResolutionConfidence::Unresolved,
    )];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);

    assert!(
        graph.edges().is_empty(),
        "an unresolved callee must never fabricate a DataFlowEdge target"
    );
}

#[test]
fn ambiguous_call_produces_one_edge_per_candidate_never_an_arbitrary_pick() {
    let calls = vec![call("overloaded", 10, vec!["1"])];
    let resolved_calls = vec![resolved(
        Some("caller_fn"),
        vec!["sym:a.rs:1:overloaded", "sym:b.rs:2:overloaded"],
        ResolutionConfidence::Ambiguous,
    )];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);

    assert_eq!(graph.edges().len(), 2);
    let mut targets: Vec<&str> = graph
        .edges()
        .iter()
        .map(|e| e.to_symbol_id.as_str())
        .collect();
    targets.sort_unstable();
    assert_eq!(
        targets,
        vec!["sym:a.rs:1:overloaded", "sym:b.rs:2:overloaded"]
    );
    assert!(graph
        .edges()
        .iter()
        .all(|e| e.confidence == ResolutionConfidence::Ambiguous));
    assert!(graph
        .edges()
        .iter()
        .all(|e| e.argument_exprs == vec!["1".to_owned()]));
}

#[test]
fn call_with_no_enclosing_symbol_still_produces_an_edge() {
    // A module-scope call (from_symbol_id: None) still carries real
    // argument data -- it must not be dropped just because the caller
    // side of the edge is unknown.
    let calls = vec![CallEdge {
        from_file_id: "file:top_level.rs".to_owned(),
        callee: "helper".to_owned(),
        line: 3usize.into(),
        arg_texts: vec!["7".to_owned()],
        ..CallEdge::default()
    }];
    let resolved_calls = vec![resolved(
        None,
        vec!["sym:callee.rs:5:helper"],
        ResolutionConfidence::Probable,
    )];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);

    assert_eq!(graph.edges().len(), 1);
    assert_eq!(graph.edges()[0].from_symbol_id, None);
}

#[test]
fn mismatched_length_slices_use_only_the_shared_prefix_never_panics() {
    let calls = vec![call("a", 1, vec!["x"]), call("b", 2, vec!["y"])];
    let resolved_calls = vec![resolved(
        Some("caller_fn"),
        vec!["sym:a.rs:1:a"],
        ResolutionConfidence::Resolved,
    )];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);

    assert_eq!(graph.edges().len(), 1);
    assert_eq!(graph.edges()[0].to_symbol_id, "sym:a.rs:1:a");
}

// ---------------------------------------------------------------------
// Lookup indices
// ---------------------------------------------------------------------

#[test]
fn edges_from_symbol_and_edges_to_symbol_filter_correctly() {
    let calls = vec![
        call("helper_one", 10, vec!["1"]),
        call("helper_two", 20, vec!["2"]),
    ];
    let resolved_calls = vec![
        resolved(
            Some("caller_a"),
            vec!["sym:callee.rs:1:helper_one"],
            ResolutionConfidence::Resolved,
        ),
        resolved(
            Some("caller_b"),
            vec!["sym:callee.rs:2:helper_two"],
            ResolutionConfidence::Resolved,
        ),
    ];

    let graph: DataFlowGraph = data_flow::materialize_from(&calls, &resolved_calls);

    let caller_a = MemoryDataFlowSourceSymbolId::from("caller_a");
    let from_a: Vec<_> = graph.edges_from_symbol(&caller_a).collect();
    assert_eq!(from_a.len(), 1);
    assert_eq!(from_a[0].to_symbol_id, "sym:callee.rs:1:helper_one");

    let helper_two = MemoryDataFlowTargetSymbolId::from("sym:callee.rs:2:helper_two");
    let to_helper_two: Vec<_> = graph.edges_to_symbol(&helper_two).collect();
    assert_eq!(to_helper_two.len(), 1);
    assert_eq!(to_helper_two[0].from_symbol_id.as_deref(), Some("caller_b"));

    let nobody = MemoryDataFlowSourceSymbolId::from("nobody");
    assert!(graph.edges_from_symbol(&nobody).next().is_none());
}

#[test]
fn argument_exprs_by_target_groups_every_argument_under_its_resolved_target() -> TestResult {
    let calls = vec![
        call("helper", 10, vec!["1", "2"]),
        call("helper", 20, vec!["3"]),
    ];
    let resolved_calls = vec![
        resolved(
            Some("caller_a"),
            vec!["sym:callee.rs:1:helper"],
            ResolutionConfidence::Resolved,
        ),
        resolved(
            Some("caller_b"),
            vec!["sym:callee.rs:1:helper"],
            ResolutionConfidence::Resolved,
        ),
    ];

    let graph = data_flow::materialize_from(&calls, &resolved_calls);
    let by_target = data_flow::argument_exprs_by_target(&graph);

    let Some(exprs) = by_target.get("sym:callee.rs:1:helper") else {
        return Err("helper must have collected argument expressions".into());
    };
    assert_eq!(exprs, &vec!["1", "2", "3"]);
    Ok(())
}

// ---------------------------------------------------------------------
// End-to-end: real indexed graph, real resolution::resolve, real
// trace_data_flow wiring.
// ---------------------------------------------------------------------

fn init_git_repo(dir: &std::path::Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &std::path::Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

fn run_git(dir: &std::path::Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

/// `graph.symbol_nodes()`'s id for the first symbol named `name`, or an
/// `Err` (never a panic) if this fixture's own source did not produce
/// one -- a missing symbol here is a fixture bug, not a thing this test
/// should crash on.
fn symbol_id(graph: &CodeGraph, name: &str) -> TestResult<String> {
    graph
        .symbol_nodes()
        .find(|s| s.name == name)
        .map(|s| s.id.clone())
        .ok_or_else(|| format!("no symbol named `{name}` was indexed").into())
}

#[test]
fn materialize_over_a_real_indexed_graph() -> TestResult {
    let dir = tempdir()?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        r#"
fn helper(value: i32) -> i32 {
    value + 1
}

fn caller() -> i32 {
    helper(41)
}
"#,
    )?;
    init_git_repo(dir.path())?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let data_flow_graph = data_flow::materialize(&graph);
    let helper_id = symbol_id(&graph, "helper")?;

    let helper_id = MemoryDataFlowTargetSymbolId::from(helper_id);
    let edges_to_helper: Vec<_> = data_flow_graph.edges_to_symbol(&helper_id).collect();
    assert_eq!(
        edges_to_helper.len(),
        1,
        "the single `helper(41)` call site must materialize exactly one DataFlowEdge"
    );
    assert_eq!(edges_to_helper[0].argument_exprs, vec!["41".to_owned()]);

    Ok(())
}

#[test]
fn trace_data_flow_populates_param_link_from_the_materialized_edge() -> TestResult {
    use enforcer_memory::analysis::trace::{trace_data_flow, TraceCallsParams};
    use enforcer_memory::analysis::CodeAdjacency;

    let dir = tempdir()?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        r#"
fn helper(value: i32) -> i32 {
    value + 1
}

fn caller() -> i32 {
    helper(41)
}
"#,
    )?;
    init_git_repo(dir.path())?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let caller_id = symbol_id(&graph, "caller")?;
    let helper_id = symbol_id(&graph, "helper")?;

    let adjacency = CodeAdjacency::build(&graph);
    let report = trace_data_flow(&adjacency, &graph, &caller_id, &TraceCallsParams::default());

    let Some(helper_hop) = report
        .paths
        .iter()
        .flat_map(|p| p.hops.iter())
        .find(|hop| hop.hop.node_id == helper_id)
    else {
        return Err("trace_data_flow must reach the helper hop via the Calls edge".into());
    };

    let Some(param_link) = helper_hop.param_link.as_ref() else {
        return Err("a resolved call with a captured argument must populate param_link".into());
    };
    assert_eq!(param_link.argument_expr, "41");
    assert_eq!(param_link.parameter_name, None);

    Ok(())
}

#[test]
fn code_adjacency_build_inserts_a_data_flows_edge_alongside_the_calls_edge() -> TestResult {
    use enforcer_domain::memory_types::{MemoryEdgeKind, TraceDirection};
    use enforcer_memory::analysis::CodeAdjacency;

    let dir = tempdir()?;
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        r#"
fn helper(value: i32) -> i32 {
    value + 1
}

fn caller() -> i32 {
    helper(41)
}
"#,
    )?;
    init_git_repo(dir.path())?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

    let caller_id = symbol_id(&graph, "caller")?;

    let adjacency = CodeAdjacency::build(&graph);

    // `related()` dedups by node (first edge kind encountered per node
    // wins -- see its own doc comment), so it cannot prove two distinct
    // edge kinds coexist between the same pair. `trace_calls` filtered
    // to exactly one edge kind at a time can: if the caller->helper hop
    // survives an `edge_types: [DataFlows]`-only filter AND an
    // `edge_types: [Calls]`-only filter, both edges independently exist
    // between the two nodes.
    use enforcer_memory::analysis::trace::{trace_calls, TraceCallsParams};

    let data_flows_only = [MemoryEdgeKind::DataFlows];
    let data_flows_params = TraceCallsParams {
        direction: TraceDirection::Out,
        edge_types: Some(&data_flows_only),
        ..TraceCallsParams::default()
    };
    let data_flows_report = trace_calls(&adjacency, &graph, &caller_id, &data_flows_params);
    assert!(
        !data_flows_report.paths.is_empty(),
        "materialize()'s DataFlowEdge must surface as a first-class MemoryEdgeKind::DataFlows edge \
         reachable on its own, independent of the Calls edge"
    );

    let calls_only = [MemoryEdgeKind::Calls];
    let calls_params = TraceCallsParams {
        direction: TraceDirection::Out,
        edge_types: Some(&calls_only),
        ..TraceCallsParams::default()
    };
    let calls_report = trace_calls(&adjacency, &graph, &caller_id, &calls_params);
    assert!(
        !calls_report.paths.is_empty(),
        "the pre-existing Calls edge must still be reachable -- DataFlows is additive, never a \
         replacement"
    );

    // `data_flow` mode restricted to edge_types=[Calls, DataFlows] must
    // still reach the helper hop -- mirrors the baseline's
    // `mode_data_flow = {"CALLS", "DATA_FLOWS"}` allow-list
    // (src/mcp/mcp.c ~L2659).
    let edge_types = [MemoryEdgeKind::Calls, MemoryEdgeKind::DataFlows];
    let params = TraceCallsParams {
        direction: TraceDirection::Out,
        edge_types: Some(&edge_types),
        ..TraceCallsParams::default()
    };
    let report = trace_calls(&adjacency, &graph, &caller_id, &params);
    assert!(
        !report.paths.is_empty(),
        "restricting edge_types to [Calls, DataFlows] must not drop the caller->helper path"
    );

    Ok(())
}
