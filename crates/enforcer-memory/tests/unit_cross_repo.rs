use std::collections::BTreeMap;

use enforcer_memory::code_graph::{CallEdge, CodeGraph, RouteEdge};
use enforcer_memory::cross_repo::match_cross_repo;

fn graph_with_route(method: &str, path: &str) -> CodeGraph {
    let mut graph = CodeGraph::new();
    graph.push_route_for_test(RouteEdge {
        from_file_id: "file:server.rs".to_owned(),
        method: method.to_owned(),
        path: path.to_owned(),
        line: 10,
    });
    graph
}

fn graph_with_call(callee: &str, url_literal: &str) -> CodeGraph {
    let mut graph = CodeGraph::new();
    graph.push_call_for_test(CallEdge {
        from_file_id: "file:client.ts".to_owned(),
        callee: callee.to_owned(),
        line: 20,
        arg_texts: vec![format!("\"{url_literal}\"")],
        ..CallEdge::default()
    });
    graph
}

#[test]
fn matching_route_and_call_produces_exactly_one_cross_http_edge() {
    let current = graph_with_call("axios.get", "http://api.example.com/widgets");
    let target = graph_with_route("GET", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(report.total_cross_edges(), 1);
    let edge = &report.cross_http_calls[0];
    assert_eq!(edge.source_project, "service-a");
    assert_eq!(edge.target_project, "service-b");
    assert_eq!(edge.method, "GET");
    assert_eq!(edge.path, "/widgets");
    assert_eq!(report.projects_scanned, 1);
    assert_eq!(report.cross_async_calls, 0);
    assert_eq!(report.cross_channel, 0);
    assert_eq!(report.cross_grpc_calls, 0);
    assert_eq!(report.cross_graphql_calls, 0);
    assert_eq!(report.cross_trpc_calls, 0);
}

#[test]
fn mismatched_method_does_not_match() {
    let current = graph_with_call("axios.post", "/widgets");
    let target = graph_with_route("GET", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 0);
    assert_eq!(report.total_cross_edges(), 0);
}

#[test]
fn verbless_fetch_matches_any_method_on_matching_path() {
    let current = graph_with_call("fetch", "/widgets");
    let target = graph_with_route("POST", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
}

#[test]
fn no_match_produces_zero_counts_not_an_error() {
    let current = graph_with_call("axios.get", "/nope");
    let target = graph_with_route("GET", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 0);
    assert_eq!(report.total_cross_edges(), 0);
    assert_eq!(report.projects_scanned, 1);
}

#[test]
fn wildcard_target_style_multiple_projects_all_scanned() {
    let current = graph_with_call("axios.get", "/widgets");
    let target_b = graph_with_route("GET", "/widgets");
    let target_c = graph_with_route("GET", "/other");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target_b);
    targets.insert("service-c".to_owned(), &target_c);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.projects_scanned, 2);
    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(report.cross_http_calls[0].target_project, "service-b");
}

#[test]
fn empty_targets_produces_zero_counts_not_an_error() {
    let current = graph_with_call("axios.get", "/widgets");
    let targets = BTreeMap::new();

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.projects_scanned, 0);
    assert_eq!(report.total_cross_edges(), 0);
}

#[test]
fn non_literal_url_argument_is_not_matched() {
    let mut current = CodeGraph::new();
    current.push_call_for_test(CallEdge {
        from_file_id: "file:client.ts".to_owned(),
        callee: "axios.get".to_owned(),
        line: 1,
        arg_texts: vec!["urlVariable".to_owned()],
        ..CallEdge::default()
    });
    let target = graph_with_route("GET", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 0);
}

#[test]
fn trailing_slash_is_ignored_when_matching_paths() {
    let current = graph_with_call("axios.get", "/widgets/");
    let target = graph_with_route("GET", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
}
