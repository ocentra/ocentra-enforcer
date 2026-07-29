use enforcer_domain::memory_types::DocumentKind;
use enforcer_domain::memory_types::TraceDirection;
use enforcer_memory::analysis::CodeAdjacency;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::fulltext::FullTextIndex;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::impact::analyze_diff_impact;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::search::document::SearchDocument;
use enforcer_memory::similarity::{similar_to_identifier_tokens, SIMILAR_TO_THRESHOLD};
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
                            | "x06_qa_duplicate_logic_rows_can_find_similar_functions"
                            | "x06_qa_module_rows_can_enumerate_touched_filesystem_paths"
                            | "x06_qa_previous_bug_rows_can_recall_similar_incidents"
                            | "x06_qa_error_change_rows_can_recall_downstream_breakage"
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
            "QA-011", "QA-024", "QA-038", "QA-039", "QA-044", "QA-045", "QA-057", "QA-058",
            "QA-065", "QA-066", "QA-067", "QA-107", "QA-109", "QA-188"
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
    assert!(
        still_needed.is_empty(),
        "all capability rows should be covered"
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
fn x06_qa_error_change_rows_can_recall_downstream_breakage() -> TestResult {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "lesson-decode-error-downstream-break".to_owned().into(),
        date: "2026-07-09".to_owned().into(),
        observed: "changing DecodeError from enum variant to struct broke downstream match arms"
            .to_owned()
            .into(),
        lesson:
            "before changing public error type shape, find downstream constructors and match arms"
                .to_owned()
                .into(),
        landed_at: "commit error-type-breakage-proof".to_owned().into(),
        ships_via: "x06-qa".to_owned().into(),
    });
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: ("lesson-decode-error-downstream-break".to_owned()).into(),
            rule_id: Some("QA-188".to_owned().into()),
            fault_class: Some("error-type-downstream-breakage".to_owned().into()),
            repo_context:
                "DecodeError shape change broke enforcer-mcp match arms and CLI conversion"
                    .to_owned()
                    .into(),
            clean: (false).into(),
            source_surface: ("fixture:error-history".to_owned()).into(),
            ts: ("2026-07-09T00:00:00Z".to_owned()).into(),
        },
    );
    graph.ingest_lesson_row(LessonRow {
        id: "lesson-unrelated-route-policy".to_owned().into(),
        date: "2026-07-09".to_owned().into(),
        observed: "route auth policy missed public endpoint coverage"
            .to_owned()
            .into(),
        lesson: "pair route extraction with auth-call evidence"
            .to_owned()
            .into(),
        landed_at: "commit unrelated-route-proof".to_owned().into(),
        ships_via: "x06-qa".to_owned().into(),
    });

    let docs = graph
        .nodes()
        .iter()
        .map(|node| {
            SearchDocument::new(
                node.id().as_str(),
                DocumentKind::Lesson,
                node.searchable_text().as_str(),
            )
        })
        .collect::<Vec<_>>();
    let index = FullTextIndex::build(&docs)?;
    let hits = index.search(
        &"DecodeError shape broke downstream match arms conversion".into(),
        5.into(),
    )?;

    assert_eq!(
        hits.first().map(|hit| hit.doc_id.as_str()),
        Some("obs-fixture:error-history-0001"),
        "error-change history should retrieve the exact downstream breakage incident first, got {hits:?}"
    );
    assert!(
        hits.iter()
            .any(|hit| hit.doc_id == "lesson-decode-error-downstream-break"),
        "error-change history should also return the landed lesson context, got {hits:?}"
    );
    let incidents = graph.incidents_for_lesson(&"lesson-decode-error-downstream-break".into());
    assert!(
        incidents
            .iter()
            .any(|incident| incident.fault_class.as_deref()
                == Some("error-type-downstream-breakage")),
        "error-change lesson must carry typed fault-class evidence, got {incidents:?}"
    );
    assert!(
        incidents.iter().any(|incident| incident
            .repo_context
            .contains("broke enforcer-mcp match arms")),
        "error-change lesson must name downstream breakage context, got {incidents:?}"
    );
    assert!(
        hits.iter()
            .all(|hit| hit.doc_id != "lesson-unrelated-route-policy"),
        "unrelated route-policy lesson must not satisfy error-change history query, got {hits:?}"
    );
    Ok(())
}

#[test]
fn x06_qa_previous_bug_rows_can_recall_similar_incidents() -> TestResult {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "lesson-walk-real-node-not-children".to_owned().into(),
        date: "2026-07-09".to_owned().into(),
        observed: "tree-sitter quirk skipped the actual call node and only walked children"
            .to_owned()
            .into(),
        lesson: "when a field resolves to the call node itself, recurse with walk on that node instead of walk_children"
            .to_owned()
            .into(),
        landed_at: "commit previous-bug-proof".to_owned().into(),
        ships_via: "x06-qa".to_owned().into(),
    });
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: ("lesson-walk-real-node-not-children".to_owned()).into(),
            rule_id: Some("QA-067".to_owned().into()),
            fault_class: Some("previous-bug-similarity".to_owned().into()),
            repo_context:
                "purescript exp_apply bug: walker skipped the real call node and found no callee"
                    .to_owned()
                    .into(),
            clean: (false).into(),
            source_surface: ("fixture:previous-bug".to_owned()).into(),
            ts: ("2026-07-09T00:00:00Z".to_owned()).into(),
        },
    );
    graph.ingest_lesson_row(LessonRow {
        id: "lesson-unrelated-cache-policy".to_owned().into(),
        date: "2026-07-09".to_owned().into(),
        observed: "model cache path policy rejected an absolute path"
            .to_owned()
            .into(),
        lesson: "keep dev cache paths repo-relative and package cache paths app-owned"
            .to_owned()
            .into(),
        landed_at: "commit unrelated-proof".to_owned().into(),
        ships_via: "x06-models".to_owned().into(),
    });

    let docs = graph
        .nodes()
        .iter()
        .map(|node| {
            SearchDocument::new(
                node.id().as_str(),
                DocumentKind::Lesson,
                node.searchable_text().as_str(),
            )
        })
        .collect::<Vec<_>>();
    let index = FullTextIndex::build(&docs)?;
    let hits = index.search(
        &"skipped real call node walk children callee missing".into(),
        5.into(),
    )?;

    assert!(
        hits.iter()
            .any(|hit| hit.doc_id == "lesson-walk-real-node-not-children"),
        "previous bug recall should retrieve the matching lesson, got {hits:?}"
    );
    assert!(
        graph
            .incidents_for_lesson(&"lesson-walk-real-node-not-children".into())
            .iter()
            .any(|incident| incident
                .repo_context
                .contains("walker skipped the real call node")),
        "matching lesson must carry observed incident evidence"
    );
    assert_eq!(
        hits.first().map(|hit| hit.doc_id.as_str()),
        Some("lesson-walk-real-node-not-children"),
        "similar bug lesson should be the top hit, got {hits:?}"
    );
    assert!(
        hits.iter()
            .all(|hit| hit.doc_id != "lesson-unrelated-cache-policy"),
        "unrelated cache-policy lesson must not be returned for call-walker bug recall, got {hits:?}"
    );
    Ok(())
}

#[test]
fn x06_qa_module_rows_can_enumerate_touched_filesystem_paths() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::create_dir_all(dir.path().join("src/orders"))?;
    fs::create_dir_all(dir.path().join("src/shared"))?;
    fs::write(
        dir.path().join("src/orders/mod.rs"),
        "mod writer;\nuse crate::shared::audit;\npub fn save_order() { writer::write_order(); audit::record_order(); }\n",
    )?;
    fs::write(
        dir.path().join("src/orders/writer.rs"),
        "pub fn write_order() {}\n",
    )?;
    fs::write(
        dir.path().join("src/shared/audit.rs"),
        "pub fn record_order() {}\n",
    )?;
    fs::write(
        dir.path().join("src/shared/unrelated.rs"),
        "pub fn ignore_me() {}\n",
    )?;
    commit_all(dir.path(), "initial touched path fixture")?;

    let graph = index_files(
        dir.path(),
        &[
            "src/orders/mod.rs",
            "src/orders/writer.rs",
            "src/shared/audit.rs",
            "src/shared/unrelated.rs",
        ],
    )?;

    let module_paths = graph
        .file_nodes()
        .filter(|file| file.rel_path.starts_with("src/orders/"))
        .map(|file| file.rel_path.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        module_paths,
        BTreeSet::from(["src/orders/mod.rs", "src/orders/writer.rs"])
    );

    let adjacency = CodeAdjacency::build(&graph);
    let touched_node_ids = adjacency.trace_calls("file:src/orders/mod.rs", TraceDirection::Out, 3);
    let trace_paths = touched_node_ids
        .iter()
        .flat_map(|hops| hops.iter())
        .filter_map(|hop| hop.node_id.strip_prefix("file:"))
        .collect::<BTreeSet<_>>();
    let touched_paths = module_paths
        .union(&trace_paths)
        .copied()
        .collect::<BTreeSet<_>>();

    assert!(
        touched_paths.contains("src/orders/writer.rs"),
        "module-prefix evidence should include same-module writer path, got {touched_paths:?}"
    );
    assert!(
        touched_paths.contains("src/shared/audit.rs"),
        "graph traversal should include imported shared audit path, got {touched_paths:?}"
    );
    assert!(
        !touched_paths.contains("src/shared/unrelated.rs"),
        "unrelated files must not be reported as touched paths, got {touched_paths:?}"
    );
    Ok(())
}

#[test]
fn x06_qa_duplicate_logic_rows_can_find_similar_functions() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(
        dir.path().join("similar.rs"),
        "fn parse_policy_rule() { normalize_policy(); }\nfn rule_policy_parse() { normalize_policy(); }\nfn unrelated_export() {}\n",
    )?;
    commit_all(dir.path(), "initial duplicate logic fixture")?;

    let graph = index_files(dir.path(), &["similar.rs"])?;
    let edges = similar_to_identifier_tokens(&graph);
    let duplicate_edge = edges.iter().find(|edge| {
        (edge.source_id.contains("parse_policy_rule")
            && edge.target_id.contains("rule_policy_parse"))
            || (edge.source_id.contains("rule_policy_parse")
                && edge.target_id.contains("parse_policy_rule"))
    });

    let edge = duplicate_edge.ok_or("expected duplicate-logic SIMILAR_TO edge")?;
    assert!(
        edge.jaccard >= SIMILAR_TO_THRESHOLD,
        "duplicate logic edge must clear similarity threshold, got {edge:?}"
    );
    assert!(
        edge.same_file.is_same_file(),
        "duplicate logic fixture should report same-file similarity, got {edge:?}"
    );
    assert!(
        edges
            .iter()
            .all(|edge| !edge.source_id.contains("unrelated_export")
                && !edge.target_id.contains("unrelated_export")),
        "unrelated function must not be returned as duplicate logic, got {edges:?}"
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
    let report = analyze_diff_impact(&graph, &["policy.rs".into()], 3.into());
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
