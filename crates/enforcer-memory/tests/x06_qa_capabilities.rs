use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::impact::analyze_diff_impact;
use serde_json::Value;
use std::collections::BTreeSet;
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

fn index_files(root: &Path, files: &[&str]) -> TestResult<CodeGraph> {
    let mut graph = CodeGraph::new();
    let paths = files.iter().map(|file| root.join(file)).collect::<Vec<_>>();
    graph.index_repository(root, &paths, &Manifest::default())?;
    Ok(graph)
}

#[test]
fn checked_in_x06_qa_capabilities_proof_matches_fixture_tests() -> TestResult {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../proof/memory/x06-qa-capabilities.json"
    ))?;
    assert_eq!(proof["schemaVersion"], 1);
    assert_eq!(proof["status"], "degraded-pass-evidence");
    assert_eq!(proof["proofScope"]["ciParity"], false);
    assert_eq!(proof["proofScope"]["portability"], "portable-fixture-proof");
    assert_eq!(proof["proofScope"]["localHardwareRequired"], false);

    let covered = proof["rowsCovered"]
        .as_array()
        .ok_or("rowsCovered must be an array")?;
    let covered_ids = covered
        .iter()
        .map(|row| {
            row["status"]
                .as_str()
                .filter(|status| *status == "degraded-pass")
                .ok_or("covered rows must have degraded-pass status")?;
            row["evidence"]
                .as_str()
                .filter(|evidence| {
                    matches!(
                        *evidence,
                        "x06_qa_diff_rows_can_identify_rules_or_workpacks_affected_by_changed_file"
                            | "x06_qa_hotspot_and_dead_export_rows_have_graph_evidence"
                            | "x06_qa_route_auth_rows_can_pair_routes_with_permission_checks"
                            | "x06_qa_symbol_rows_can_find_zero_callers_and_generic_result_mentions"
                    )
                })
                .ok_or("covered row must point at a focused fixture test")?;
            row["id"].as_str().ok_or("covered row id must be a string")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    assert_eq!(
        covered_ids,
        BTreeSet::from([
            "QA-011", "QA-038", "QA-039", "QA-044", "QA-045", "QA-057", "QA-058", "QA-065",
            "QA-107", "QA-109"
        ])
    );

    let still_needed = proof["rowsStillNeedingRunnerOrCode"]
        .as_array()
        .ok_or("rowsStillNeedingRunnerOrCode must be an array")?
        .iter()
        .map(|row| {
            row["reason"]
                .as_str()
                .filter(|reason| !reason.is_empty())
                .ok_or("still-needed row must have a reason")?;
            row["id"]
                .as_str()
                .ok_or("still-needed row id must be a string")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    assert_eq!(
        still_needed,
        BTreeSet::from(["QA-024", "QA-066", "QA-067", "QA-188"])
    );
    assert!(
        proof["lessons"]
            .as_array()
            .is_some_and(|lessons| !lessons.is_empty()),
        "proof must carry durable learning evidence"
    );
    Ok(())
}

#[test]
fn x06_qa_diff_rows_can_identify_rules_or_workpacks_affected_by_changed_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("rule.rs"),
        "fn check_rule() { shared_policy(); }\n",
    )?;
    fs::write(
        dir.path().join("policy.rs"),
        "fn shared_policy() {}\nfn helper() {}\n",
    )?;
    commit_all(dir.path(), "initial rules fixture")?;

    let graph = index_files(dir.path(), &["rule.rs", "policy.rs"])?;
    let report = analyze_diff_impact(&graph, &["policy.rs".to_owned()], 3);
    let impacted = report
        .impacted
        .iter()
        .find(|row| row.rel_path == "policy.rs")
        .ok_or("expected policy.rs impact row")?;

    assert!(
        impacted
            .affected_node_ids
            .iter()
            .any(|id| id == "file:rule.rs"),
        "changing policy.rs must identify rule.rs as affected, got {:?}",
        impacted.affected_node_ids
    );
    assert!(
        impacted
            .affected_node_ids
            .iter()
            .any(|id| id.contains("check_rule")),
        "changed shared policy should surface rule/workpack symbols, got {:?}",
        impacted.affected_node_ids
    );
    Ok(())
}

#[test]
fn x06_qa_hotspot_and_dead_export_rows_have_graph_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("api.rs"),
        "pub fn hot_entry() { shared(); }\npub fn alternate() { shared(); }\npub fn dead_public_export() {}\n",
    )?;
    fs::write(dir.path().join("core.rs"), "pub fn shared() {}\n")?;
    commit_all(dir.path(), "initial hotspot fixture")?;

    let graph = index_files(dir.path(), &["api.rs", "core.rs"])?;
    let adjacency = CodeAdjacency::build(&graph);
    let hotspots = adjacency.hotspots(5);

    assert!(
        hotspots
            .iter()
            .any(|row| row.node_id == "file:api.rs" && row.total_degree() >= 3),
        "expected api.rs to rank as a high-degree hotspot, got {hotspots:?}"
    );

    graph
        .symbol_nodes()
        .find(|symbol| symbol.name == "dead_public_export")
        .ok_or("expected dead_public_export symbol")?;
    let callers = graph
        .calls()
        .iter()
        .filter(|call| call.callee == "dead_public_export")
        .collect::<Vec<_>>();
    assert!(
        callers.is_empty(),
        "dead_public_export should have no structural callers, got {callers:?}"
    );
    Ok(())
}

#[test]
fn x06_qa_route_auth_rows_can_pair_routes_with_permission_checks() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("routes.ts"),
        "router.get('/admin', requireAuth, listAdmin);\nrouter.get('/public', listPublic);\nfunction requireAuth() {}\nfunction listAdmin() { requireAuth(); }\nfunction listPublic() {}\n",
    )?;
    commit_all(dir.path(), "initial route auth fixture")?;

    let graph = index_files(dir.path(), &["routes.ts"])?;
    let routes = graph.routes();
    assert!(
        routes
            .iter()
            .any(|route| route.method == "GET" && route.path == "/admin"),
        "expected GET /admin route, got {routes:?}"
    );
    assert!(
        routes
            .iter()
            .any(|route| route.method == "GET" && route.path == "/public"),
        "expected GET /public route, got {routes:?}"
    );
    assert!(
        graph
            .calls()
            .iter()
            .any(|call| call.callee == "requireAuth"),
        "expected auth/permission check call evidence, got {:?}",
        graph.calls()
    );
    Ok(())
}

#[test]
fn x06_qa_symbol_rows_can_find_zero_callers_and_generic_result_mentions() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("symbols.rs"),
        "fn used() -> Result<String, String> { Ok(String::new()) }\nfn caller() { let _ = used(); }\nfn zero_callers() -> Result<u32, String> { Ok(1) }\n",
    )?;
    commit_all(dir.path(), "initial symbol fixture")?;

    let graph = index_files(dir.path(), &["symbols.rs"])?;
    graph
        .symbol_nodes()
        .find(|symbol| symbol.name == "zero_callers")
        .ok_or("expected zero_callers symbol")?;
    let callers = graph
        .calls()
        .iter()
        .filter(|call| call.callee == "zero_callers")
        .collect::<Vec<_>>();
    assert!(callers.is_empty(), "expected zero callers, got {callers:?}");
    assert!(
        graph
            .type_refs()
            .iter()
            .any(|type_ref| type_ref.type_name.contains("Result")),
        "expected at least one Result<T> type reference, got {:?}",
        graph.type_refs()
    );
    Ok(())
}
