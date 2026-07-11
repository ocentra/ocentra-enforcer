use std::collections::BTreeMap;
use std::fs;

use enforcer_memory::code_graph::{CallEdge, CodeGraph, RouteEdge};
use enforcer_memory::cross_repo::{match_cross_repo, CrossHttpMatchKind, CrossRepoProtocol};
use enforcer_memory::mcp::dispatch_tool;
use serde_json::json;

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

fn graph_with_channel_call(callee: &str, topic: &str) -> CodeGraph {
    let mut graph = CodeGraph::new();
    graph.push_call_for_test(CallEdge {
        from_file_id: "file:worker.ts".to_owned(),
        callee: callee.to_owned(),
        line: 30,
        arg_texts: vec![format!("\"{topic}\"")],
        ..CallEdge::default()
    });
    graph
}

fn graph_with_literal_call(callee: &str, literal: &str) -> CodeGraph {
    let mut graph = CodeGraph::new();
    graph.push_call_for_test(CallEdge {
        from_file_id: "file:protocol.ts".to_owned(),
        callee: callee.to_owned(),
        line: 40,
        arg_texts: vec![format!("\"{literal}\"")],
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
    assert_eq!(edge.via, CrossHttpMatchKind::HttpClient);
    assert_eq!(report.projects_scanned, 1);
    assert_eq!(report.cross_async_calls, 0);
    assert_eq!(report.cross_channel, 0);
    assert_eq!(report.cross_grpc_calls, 0);
    assert_eq!(report.cross_graphql_calls, 0);
    assert_eq!(report.cross_trpc_calls, 0);
}

#[test]
fn matching_route_declarations_produce_route_declaration_cross_http_edge() {
    let current = graph_with_route("GET", "/widgets");
    let target = graph_with_route("GET", "/widgets");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(
        report.cross_http_calls[0].via,
        CrossHttpMatchKind::RouteDeclaration
    );
    assert_eq!(report.total_cross_edges(), 1);
}

#[test]
fn matching_publish_subscribe_topics_increment_cross_channel() {
    let current = graph_with_channel_call("events.publish", "widgets.created");
    let target = graph_with_channel_call("bus.subscribe", "widgets.created");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_channel, 1);
    assert_eq!(report.cross_channel_links.len(), 1);
    assert_eq!(report.cross_channel_links[0].topic, "widgets.created");
    assert_eq!(report.total_cross_edges(), 1);
}

#[test]
fn matching_async_broker_topics_increment_cross_async_calls() {
    let current = graph_with_literal_call("pubsub.publish", "widgets.async");
    let target = graph_with_literal_call("pubsub.subscribe", "widgets.async");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_async_calls, 1);
    assert_eq!(report.cross_async_links.len(), 1);
    assert_eq!(
        report.cross_async_links[0].protocol,
        CrossRepoProtocol::Async
    );
    assert_eq!(report.cross_async_links[0].key, "widgets.async");
    assert_eq!(report.cross_channel, 0);
    assert_eq!(report.total_cross_edges(), 1);
}

#[test]
fn matching_grpc_client_to_registered_service_increments_cross_grpc_calls() {
    let current = graph_with_literal_call("pb.NewWidgetServiceClient.GetWidget", "ignored");
    let target = graph_with_literal_call("grpcServer.addService", "WidgetService/GetWidget");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_grpc_calls, 1);
    assert_eq!(report.cross_grpc_links[0].protocol, CrossRepoProtocol::Grpc);
    assert_eq!(report.cross_grpc_links[0].key, "WidgetService/GetWidget");
}

#[test]
fn matching_graphql_operation_increments_cross_graphql_calls() {
    let current = graph_with_literal_call("graphqlClient.request", "query GetWidget { widget }");
    let target = graph_with_literal_call("graphqlSchema.resolver", "GetWidget");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_graphql_calls, 1);
    assert_eq!(
        report.cross_graphql_links[0].protocol,
        CrossRepoProtocol::Graphql
    );
    assert_eq!(report.cross_graphql_links[0].key, "GetWidget");
}

#[test]
fn matching_trpc_procedure_increments_cross_trpc_calls() {
    let current = graph_with_literal_call("trpc.widget.byId.query", "ignored");
    let target = graph_with_literal_call("router.query", "widget.byId");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_trpc_calls, 1);
    assert_eq!(report.cross_trpc_links[0].protocol, CrossRepoProtocol::Trpc);
    assert_eq!(report.cross_trpc_links[0].key, "widget.byId");
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
    assert_eq!(
        report.cross_http_calls[0].via,
        CrossHttpMatchKind::LiteralUrl
    );
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

#[test]
fn concrete_client_path_matches_templated_route() {
    let current = graph_with_call("axios.get", "/widgets/42");
    let target = graph_with_route("GET", "/widgets/:id");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(report.cross_http_calls[0].path, "/widgets/:id");
    assert_eq!(
        report.cross_http_calls[0].via,
        CrossHttpMatchKind::HttpClient
    );
}

#[test]
fn full_url_normalization_ignores_query_and_fragment() {
    let current = graph_with_call(
        "axios.get",
        "https://api.example.com/widgets/42/?expand=true#details",
    );
    let target = graph_with_route("GET", "/widgets/{id}");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(report.cross_http_calls[0].path, "/widgets/{id}");
}

#[test]
fn literal_url_remains_distinct_additive_evidence_for_templated_routes() {
    let current = graph_with_call("fetch", "https://api.example.com/widgets/42?expand=true");
    let target = graph_with_route("POST", "/widgets/{id}");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(
        report.cross_http_calls[0].via,
        CrossHttpMatchKind::LiteralUrl
    );
    assert_eq!(report.baseline_cross_http_call_count(), 0);
    assert_eq!(report.literal_url_cross_http_call_count(), 1);
}

#[test]
fn route_any_method_accepts_baseline_http_client_match() {
    let current = graph_with_call("axios.delete", "/widgets/42");
    let target = graph_with_route("ANY", "/widgets/{id}");
    let mut targets = BTreeMap::new();
    targets.insert("service-b".to_owned(), &target);

    let report = match_cross_repo("service-a", &current, &targets);

    assert_eq!(report.cross_http_calls.len(), 1);
    assert_eq!(report.baseline_cross_http_call_count(), 1);
}

#[test]
fn cross_repo_mode_accepts_snake_case_wire_aliases() {
    let server_dir = tempfile::tempdir().expect("server tempdir");
    let client_dir = tempfile::tempdir().expect("client tempdir");

    fs::write(
        server_dir.path().join("server.ts"),
        "const app = { get(_path: string, _handler: () => void) {} };\nfunction showWidget() {}\napp.get(\"/widgets/:id\", showWidget);\n",
    )
    .expect("write server fixture");
    fs::write(
        client_dir.path().join("client.ts"),
        "import axios from \"axios\";\nexport function fetchWidget() { return axios.get(\"https://api.example.com/widgets/42?expand=true#details\"); }\n",
    )
    .expect("write client fixture");

    let result = dispatch_tool(
        "index_repository",
        &json!({
            "repo_path": client_dir.path().to_string_lossy().replace('\\', "/"),
            "mode": "cross-repo-intelligence",
            "target_projects": [server_dir.path().to_string_lossy().replace('\\', "/")],
            "name": "client-service",
        }),
    );

    assert_eq!(result["ok"].as_bool(), Some(true), "{result}");
    assert_eq!(result["project"].as_str(), Some("client-service"));
    assert_eq!(result["cross_http_calls"].as_u64(), Some(1));
    assert_eq!(result["total_cross_edges"].as_u64(), Some(1));
    assert_eq!(result["cross_literal_url_calls"].as_u64(), Some(0));
}

#[test]
fn literal_fetch_is_reported_as_extension_not_baseline_http_count() {
    let server_dir = tempfile::tempdir().expect("server tempdir");
    let client_dir = tempfile::tempdir().expect("client tempdir");

    fs::write(
        server_dir.path().join("server.ts"),
        "const app = { get(_path: string, _handler: () => void) {} };\nfunction showWidget() {}\napp.get(\"/widgets/{id}\", showWidget);\n",
    )
    .expect("write server fixture");
    fs::write(
        client_dir.path().join("client.ts"),
        "export function fetchWidget() { return fetch(\"https://api.example.com/widgets/42?expand=true\"); }\n",
    )
    .expect("write client fixture");

    let result = dispatch_tool(
        "index_repository",
        &json!({
            "repo_path": client_dir.path().to_string_lossy().replace('\\', "/"),
            "mode": "cross-repo-intelligence",
            "target_projects": [server_dir.path().to_string_lossy().replace('\\', "/")],
            "name": "client-service",
        }),
    );

    assert_eq!(result["ok"].as_bool(), Some(true), "{result}");
    assert_eq!(result["cross_http_calls"].as_u64(), Some(0));
    assert_eq!(result["cross_literal_url_calls"].as_u64(), Some(1));
    assert_eq!(result["total_cross_edges"].as_u64(), Some(0));
    assert_eq!(
        result["total_cross_edges_including_extensions"].as_u64(),
        Some(1)
    );
}
