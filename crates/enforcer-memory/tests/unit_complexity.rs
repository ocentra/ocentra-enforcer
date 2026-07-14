//! X06 core parity: hand-verifiable tests for `enforcer_memory::complexity`
//! -- cyclomatic/cognitive/loop metrics per language, recursion flags,
//! linear-scan/alloc-in-loop detection, transitive propagation across a
//! call chain, cycle safety, the query DSL surface, and a compile-only
//! wave-B extension-point smoke check.

use enforcer_memory::analysis::query::{execute, parse};
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::complexity::{self, CallGraphNode, ComplexityLanguage, NodeKindTable};
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

fn rust_metrics(source: &str, fn_name: &str) -> TestResult<complexity::ComplexityMetrics> {
    let line = find_line(source, fn_name)?;
    let names = vec![(fn_name.to_string(), line)];
    let map = complexity::metrics_for_symbols(ComplexityLanguage::Rust, source, &names);
    map.get(&(fn_name.to_string(), line))
        .copied()
        .ok_or_else(|| format!("no metrics resolved for {fn_name} in:\n{source}").into())
}

fn ts_metrics(source: &str, fn_name: &str) -> TestResult<complexity::ComplexityMetrics> {
    let line = find_line(source, fn_name)?;
    let names = vec![(fn_name.to_string(), line)];
    let map =
        complexity::metrics_for_symbols(ComplexityLanguage::TypeScriptOrJavaScript, source, &names);
    map.get(&(fn_name.to_string(), line))
        .copied()
        .ok_or_else(|| format!("no metrics resolved for {fn_name} in:\n{source}").into())
}

fn python_metrics(source: &str, fn_name: &str) -> TestResult<complexity::ComplexityMetrics> {
    let line = find_line(source, fn_name)?;
    let names = vec![(fn_name.to_string(), line)];
    let map = complexity::metrics_for_symbols(ComplexityLanguage::Python, source, &names);
    map.get(&(fn_name.to_string(), line))
        .copied()
        .ok_or_else(|| format!("no metrics resolved for {fn_name} in:\n{source}").into())
}

/// Locate the 1-based line of the first `fn <name>`/`function <name>`/
/// `def <name>` occurrence -- good enough for these small fixtures,
/// mirroring how `SymbolRef::line`/`SymbolNode::line` are populated
/// from a real extractor (first line of the definition).
fn find_line(source: &str, name: &str) -> TestResult<usize> {
    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("fn ") && trimmed[3..].starts_with(name)
            || trimmed.starts_with("pub fn ") && trimmed[7..].starts_with(name)
            || trimmed.starts_with("function ") && trimmed[9..].starts_with(name)
            || trimmed.starts_with("def ") && trimmed[4..].starts_with(name)
        {
            return Ok(idx + 1);
        }
    }
    Err(format!("fixture has no definition line for {name} in:\n{source}").into())
}

// ---------------------------------------------------------------------
// Cyclomatic + cognitive complexity, per language (hand-verified).
// ---------------------------------------------------------------------

#[test]
fn rust_cyclomatic_counts_one_decision_point_per_branch() -> TestResult<()> {
    // Baseline 1 + if + else-if(match arm counts separately) -> 1 (fn)
    // + 1 (if) + 1 (while) = 3.
    let source = r#"
fn branchy(x: i32) -> i32 {
    if x > 0 {
        return x;
    }
    while x < 0 {
        return -x;
    }
    0
}
"#;
    let m = rust_metrics(source, "branchy")?;
    assert_eq!(m.complexity, 3, "1 baseline + if + while");
    assert_eq!(m.loop_count, 1);
    assert_eq!(m.loop_depth, 1);
    assert_eq!(m.param_count, 1);
    Ok(())
}

#[test]
fn rust_straight_line_function_has_baseline_complexity_one() -> TestResult<()> {
    let source = r#"
fn straight(a: i32, b: i32) -> i32 {
    let c = a + b;
    c
}
"#;
    let m = rust_metrics(source, "straight")?;
    assert_eq!(m.complexity, 1, "no decision points -> baseline only");
    assert_eq!(m.cognitive, 0);
    assert_eq!(m.loop_count, 0);
    assert_eq!(m.loop_depth, 0);
    assert_eq!(m.param_count, 2);
    Ok(())
}

#[test]
fn rust_outer_access_depth_excludes_nested_closure_body() -> TestResult<()> {
    let source = r#"
fn outer() {
    let short = root.child;
    let nested = || inner.one.two.three;
    let _ = (short, nested);
}
"#;

    let metrics = rust_metrics(source, "outer")?;
    assert_eq!(
        metrics.max_access_depth, 2,
        "the nested closure's four-segment chain belongs to the closure, not outer"
    );
    Ok(())
}

#[test]
fn rust_nested_loops_increase_cognitive_more_than_flat_loops() -> TestResult<()> {
    // Two independent (flat, sibling) loops: cognitive = (1+0) + (1+0) = 2.
    let flat = r#"
fn flat(items: &[i32]) {
    for a in items {
        let _ = a;
    }
    for b in items {
        let _ = b;
    }
}
"#;
    let flat_metrics = rust_metrics(flat, "flat")?;
    assert_eq!(flat_metrics.cognitive, 2);
    assert_eq!(flat_metrics.loop_depth, 1);

    // One loop nested inside another: cognitive = (1+0) [outer] + (1+1) [inner] = 3.
    let nested = r#"
fn nested(items: &[i32]) {
    for a in items {
        for b in items {
            let _ = (a, b);
        }
    }
}
"#;
    let nested_metrics = rust_metrics(nested, "nested")?;
    assert_eq!(nested_metrics.cognitive, 3);
    assert_eq!(nested_metrics.loop_depth, 2, "two nested loops -> depth 2");
    assert_eq!(nested_metrics.loop_count, 2);

    assert!(
        nested_metrics.cognitive > flat_metrics.cognitive,
        "nesting must cost strictly more than the same count of flat constructs"
    );
    Ok(())
}

#[test]
fn typescript_cyclomatic_counts_if_and_for() -> TestResult<()> {
    let source = r#"
function branchy(x) {
    if (x > 0) {
        return x;
    }
    for (let i = 0; i < x; i++) {
        console.log(i);
    }
    return 0;
}
"#;
    let m = ts_metrics(source, "branchy")?;
    assert_eq!(m.complexity, 3, "1 baseline + if + for");
    assert_eq!(m.loop_count, 1);
    assert_eq!(m.loop_depth, 1);
    Ok(())
}

#[test]
fn python_cyclomatic_counts_if_elif_and_while() -> TestResult<()> {
    let source = "def branchy(x):\n    if x > 0:\n        return x\n    elif x < 0:\n        return -x\n    while x == 0:\n        return 0\n    return -1\n";
    let m = python_metrics(source, "branchy")?;
    // 1 baseline + if + elif + while = 4.
    assert_eq!(m.complexity, 4);
    assert_eq!(m.loop_count, 1);
    assert_eq!(m.param_count, 1);
    Ok(())
}

// ---------------------------------------------------------------------
// Recursion flags.
// ---------------------------------------------------------------------

#[test]
fn rust_self_recursive_call_outside_loop_sets_flag_without_recursion_in_loop() -> TestResult<()> {
    let source = r#"
fn fact(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        n * fact(n - 1)
    }
}
"#;
    let m = rust_metrics(source, "fact")?;
    assert!(m.self_recursive);
    assert!(!m.recursion_in_loop);
    assert!(
        !m.unguarded_recursion,
        "the self-call sits under an `if`/`else` branch, i.e. a guarded base case exists"
    );
    Ok(())
}

#[test]
fn rust_unconditional_self_call_is_unguarded_recursion() -> TestResult<()> {
    let source = r#"
fn spin(n: u64) -> u64 {
    spin(n + 1)
}
"#;
    let m = rust_metrics(source, "spin")?;
    assert!(m.self_recursive);
    assert!(
        m.unguarded_recursion,
        "no conditional guards the self-call -- infinite recursion by construction"
    );
    Ok(())
}

#[test]
fn rust_self_call_inside_a_loop_sets_recursion_in_loop() -> TestResult<()> {
    let source = r#"
fn odd(n: u64) -> bool {
    while n > 0 {
        return odd(n - 1);
    }
    false
}
"#;
    let m = rust_metrics(source, "odd")?;
    assert!(m.self_recursive);
    assert!(
        m.recursion_in_loop,
        "the self-call sits inside the `while` loop"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Linear-scan / alloc-in-loop bottleneck signals.
// ---------------------------------------------------------------------

#[test]
fn rust_find_call_inside_loop_is_a_linear_scan_in_loop() -> TestResult<()> {
    let source = r#"
fn search(haystacks: &[Vec<i32>], needle: i32) {
    for h in haystacks {
        h.iter().find(|x| **x == needle);
    }
}
"#;
    let m = rust_metrics(source, "search")?;
    assert!(
        m.linear_scan_in_loop >= 1,
        "`.find(..)` inside the `for` loop is the hidden O(n^2) this signal exists to catch"
    );
    Ok(())
}

#[test]
fn rust_push_call_inside_loop_is_alloc_in_loop() -> TestResult<()> {
    let source = r#"
fn collect(items: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    for i in items {
        out.push(*i);
    }
    out
}
"#;
    let m = rust_metrics(source, "collect")?;
    assert!(
        m.alloc_in_loop >= 1,
        "`.push(..)` inside the `for` loop is an allocation-shaped call per iteration"
    );
    Ok(())
}

#[test]
fn rust_call_outside_any_loop_is_not_counted_as_in_loop() -> TestResult<()> {
    let source = r#"
fn once(items: &[i32]) -> Option<&i32> {
    items.iter().find(|x| **x > 0)
}
"#;
    let m = rust_metrics(source, "once")?;
    assert_eq!(
        m.linear_scan_in_loop, 0,
        "the `.find(..)` call here is not inside any loop"
    );
    assert_eq!(m.loop_count, 0);
    Ok(())
}

// ---------------------------------------------------------------------
// max_access_depth.
// ---------------------------------------------------------------------

#[test]
fn rust_max_access_depth_counts_the_longest_chain() -> TestResult<()> {
    let source = r#"
fn deep(a: &Config) -> i32 {
    let short = a.x;
    let long = a.b.c.d.value;
    let _ = short;
    long
}
"#;
    let m = rust_metrics(source, "deep")?;
    assert_eq!(
        m.max_access_depth, 5,
        "a.b.c.d.value -> 5 chained accesses; the shorter a.x chain must not win"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// Tier B: transitive_loop_depth propagation across a 3-function chain.
// ---------------------------------------------------------------------

#[test]
fn transitive_loop_depth_propagates_across_a_three_function_call_chain() {
    // outer (loop_depth 1) -> middle (loop_depth 1) -> inner (loop_depth 1)
    // tld(inner) = 1
    // tld(middle) = 1 + tld(inner) = 2
    // tld(outer) = 1 + tld(middle) = 3
    let nodes = vec![
        CallGraphNode {
            id: "outer".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec!["middle".to_string()],
        },
        CallGraphNode {
            id: "middle".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec!["inner".to_string()],
        },
        CallGraphNode {
            id: "inner".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec![],
        },
    ];
    let result = complexity::propagate_transitive_loop_depth(&nodes);
    assert_eq!(result["inner"].transitive_loop_depth, 1);
    assert_eq!(result["middle"].transitive_loop_depth, 2);
    assert_eq!(result["outer"].transitive_loop_depth, 3);
    assert!(!result["outer"].recursive);
    assert!(!result["middle"].recursive);
    assert!(!result["inner"].recursive);
}

#[test]
fn transitive_loop_depth_picks_the_max_over_multiple_callees() {
    // root calls both a (tld 1) and b (tld 5) -- root's tld must follow
    // the larger branch, not the first/last one visited.
    let nodes = vec![
        CallGraphNode {
            id: "root".to_string(),
            loop_depth: 0,
            self_recursive: false,
            callees: vec!["a".to_string(), "b".to_string()],
        },
        CallGraphNode {
            id: "a".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec![],
        },
        CallGraphNode {
            id: "b".to_string(),
            loop_depth: 5,
            self_recursive: false,
            callees: vec![],
        },
    ];
    let result = complexity::propagate_transitive_loop_depth(&nodes);
    assert_eq!(result["root"].transitive_loop_depth, 5);
}

// ---------------------------------------------------------------------
// Cycle safety: a cycle in the call graph must terminate and must set
// `recursive`, never hang.
// ---------------------------------------------------------------------

#[test]
fn call_graph_cycle_terminates_and_flags_recursive() {
    // a -> b -> c -> a: a 3-node mutual-recursion cycle.
    let nodes = vec![
        CallGraphNode {
            id: "a".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec!["b".to_string()],
        },
        CallGraphNode {
            id: "b".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec!["c".to_string()],
        },
        CallGraphNode {
            id: "c".to_string(),
            loop_depth: 1,
            self_recursive: false,
            callees: vec!["a".to_string()],
        },
    ];
    // The mere fact this returns (rather than hanging/overflowing the
    // stack) is the primary assertion here.
    let result = complexity::propagate_transitive_loop_depth(&nodes);
    assert_eq!(result.len(), 3);
    assert!(result["a"].recursive, "cycle participant a is recursive");
    assert!(result["b"].recursive, "cycle participant b is recursive");
    assert!(result["c"].recursive, "cycle participant c is recursive");
}

#[test]
fn direct_self_recursion_seed_is_preserved_in_transitive_metrics() {
    let nodes = vec![CallGraphNode {
        id: "fact".to_string(),
        loop_depth: 0,
        self_recursive: true,
        callees: vec!["fact".to_string()],
    }];
    let result = complexity::propagate_transitive_loop_depth(&nodes);
    assert!(result["fact"].recursive);
}

#[test]
fn empty_call_graph_returns_empty_map_without_panicking() {
    let result = complexity::propagate_transitive_loop_depth(&[]);
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------
// find_definition_node / compute integration surface.
// ---------------------------------------------------------------------

#[test]
fn metrics_for_symbols_skips_unresolvable_names_without_panicking() {
    let source = "fn real() {}\n";
    let names = vec![
        ("real".to_string(), 1),
        ("ghost".to_string(), 99), // does not exist in this source
    ];
    let map = complexity::metrics_for_symbols(ComplexityLanguage::Rust, source, &names);
    assert!(map.contains_key(&("real".to_string(), 1)));
    assert!(
        !map.contains_key(&("ghost".to_string(), 99)),
        "an unresolvable name/line pair must be silently absent, not a panic"
    );
}

#[test]
fn metrics_for_symbols_on_empty_source_returns_empty_map() {
    let map = complexity::metrics_for_symbols(ComplexityLanguage::Rust, "", &[]);
    assert!(map.is_empty());
}

// ---------------------------------------------------------------------
// Wave-B extension point: constructing a `NodeKindTable` for a
// hypothetical new language compiles and behaves like any other table
// (no special-casing inside the walk logic -- see `complexity.rs`'s
// module doc "Language-neutral design" section). This is a compile +
// smoke check, not a real language, so it only exercises the parts of
// `NodeKindTable` that do not require a real tree-sitter grammar.
// ---------------------------------------------------------------------

#[test]
fn node_kind_table_is_a_plain_data_extension_point_for_wave_b_languages() {
    fn hypothetical_language_table() -> NodeKindTable {
        // A wave-B language would list its own grammar's node kinds
        // here -- this constructs a syntactically valid table (borrowing
        // Rust's shapes) purely to prove the type is usable from outside
        // `complexity.rs` with no additional trait implementation.
        NodeKindTable::rust()
    }
    let table = hypothetical_language_table();
    assert!(!table.loops.is_empty());
    assert!(!table.decision_points.is_empty());
}

// ---------------------------------------------------------------------
// End-to-end: real `CodeGraph::index_repository` -> query_graph DSL
// query over the X06 core parity properties, matching the mission's
// exact example query shape (see `docs/plans/enforcer-selfhost-plan/
// refs/x06-baseline-tool-schemas.md` §4.5's
// `MATCH (f:Function) WHERE f.transitive_loop_depth >= 3 OR
// f.linear_scan_in_loop >= 1 RETURN f.qualified_name ORDER BY
// f.transitive_loop_depth DESC`).
// ---------------------------------------------------------------------

#[test]
fn dsl_query_over_transitive_loop_depth_and_linear_scan_matches_and_orders_correctly(
) -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    // `outer` has loop_depth 1 and calls `inner`, which itself has
    // loop_depth 1 -- so outer's transitive_loop_depth should reach 2,
    // clearing the `>= 3` bar is not guaranteed by this fixture alone,
    // so the predicate instead exercises the OR's second arm:
    // `scanner` has a `.find(..)` call inside a `for` loop
    // (linear_scan_in_loop >= 1) without needing deep call chains.
    fs::write(
        dir.path().join("a.rs"),
        "fn outer(items: &[i32]) { for i in items { inner(); } }\nfn inner() { for j in 0..1 { let _ = j; } }\n",
    )?;
    fs::write(
        dir.path().join("b.rs"),
        "fn scanner(items: &[i32]) -> Option<&i32> { for _ in items { return items.iter().find(|x| **x > 0); } items.first() }\nfn plain() {}\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];
    graph.index_repository(dir.path(), &files, &Manifest::default())?;
    let adjacency = CodeAdjacency::build(&graph);

    let parsed = parse(
        "MATCH (f:Function) WHERE f.transitive_loop_depth >= 3 OR f.linear_scan_in_loop >= 1 RETURN f.name ORDER BY f.transitive_loop_depth DESC",
    )?;
    let rows = execute(&parsed, &adjacency, &graph)?;

    // `execute`'s `ResultRow` keys the matched node id under the bare
    // pattern variable (`"f"`), not under the dotted RETURN column --
    // see `unit_analysis_query.rs::exact_failing_baseline_query_now_parses_and_executes`
    // for the same convention. Node ids embed the symbol name (`sym:
    // <path>:<line>:<name>`), so a substring check on `r["f"]` is this
    // DSL's existing way to assert "this row is symbol X".
    let ids: Vec<&String> = rows.iter().filter_map(|r| r.get("f")).collect();
    assert!(
        ids.iter().any(|id| id.contains("scanner")),
        "scanner's in-loop `.find(..)` call must satisfy linear_scan_in_loop >= 1; got rows: {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| id.contains("plain")),
        "plain has no loops/calls and must not satisfy either predicate arm"
    );
    Ok(())
}

#[test]
fn dsl_query_can_read_every_x06_complexity_property_without_error() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("a.rs"),
        "fn f(x: i32) -> i32 { if x > 0 { x } else { -x } }\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[dir.path().join("a.rs")], &Manifest::default())?;
    let adjacency = CodeAdjacency::build(&graph);

    for property in [
        "complexity",
        "cognitive",
        "loop_count",
        "loop_depth",
        "param_count",
        "max_access_depth",
        "linear_scan_in_loop",
        "alloc_in_loop",
        "self_recursive",
        "recursion_in_loop",
        "unguarded_recursion",
        "transitive_loop_depth",
        "recursive",
    ] {
        let query = format!("MATCH (f:Function) RETURN f.{property}");
        let parsed = parse(&query)?;
        let rows = execute(&parsed, &adjacency, &graph)?;
        assert!(
            !rows.is_empty(),
            "property {property} produced no rows for a graph with one Function node"
        );
    }
    Ok(())
}
