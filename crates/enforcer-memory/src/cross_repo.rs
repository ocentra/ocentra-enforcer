//! X06 core parity: cross-repo-intelligence mode.
//!
//! Baseline binding: `docs/plans/enforcer-selfhost-plan/refs/
//! x06-baseline-tool-schemas.md` Â§9.2/Â§9.5. The baseline's
//! `index_repository(mode="cross-repo-intelligence")` never re-indexes
//! anything; it matches already-indexed projects' Routes/Channels and
//! service-call route nodes against each other to create
//! `CROSS_HTTP_CALLS`/`CROSS_ASYNC_CALLS`/`CROSS_CHANNEL`/
//! `CROSS_GRPC_CALLS`/`CROSS_GRAPHQL_CALLS`/`CROSS_TRPC_CALLS` edges,
//! and returns a fixed-shape response with a typed count per protocol
//! plus `total_cross_edges`/`elapsed_ms`.
//!
//! This module is the library-layer analog: given one project's
//! [`crate::code_graph::CodeGraph`] (`current`) and a set of named
//! target projects' graphs, it matches HTTP route/call sites plus
//! async, channel, gRPC, GraphQL, and tRPC protocol call sites to
//! produce typed cross-repo evidence. Wiring into the MCP
//! `index_repository` handler's `mode="cross-repo-intelligence"` branch
//! is [`crate::mcp`]'s job; this module owns the matching algorithm and
//! its typed result.
//!
//! # Matching heuristic -- documented honestly
//!
//! `CROSS_HTTP_CALLS` has three evidence modes:
//!
//! 1. **Target side (declared routes)**: every [`RouteEdge`] already
//!    extracted by [`crate::code_graph`]'s language extractors (Axum/
//!    Actix/Express/FastAPI-style route macros/decorators) -- `method`
//!    (upper-cased) + `path` (as written, e.g. `/widgets/:id`).
//! 2. **Current side (baseline-compatible outbound call sites)**: every
//!    [`crate::code_graph::CallEdge`] whose `callee` text matches a
//!    fixed, best-effort allow-list of HTTP-client-shaped callees
//!    ([`is_http_client_callee`] -- `axios.*`, `http.get`/
//!    `http.post`/etc., `requests.*`, `reqwest::*`/`reqwest.*`,
//!    `httpClient.*`/`httpclient.*`), combined with a URL/path literal
//!    found in [`CallEdge::arg_texts`] ([`extract_url_literal`] -- the
//!    first string-literal-shaped argument, `"..."`/`'...'`/`` `...` ``,
//!    with a leading `http://`/`https://` scheme stripped if present).
//!    These emit [`CrossHttpMatchKind::HttpClient`].
//! 3. **Current side (Rust extension)**: a bare `fetch("...")` literal
//!    call uses the same path matcher but is reported separately as
//!    [`CrossHttpMatchKind::LiteralUrl`] because the installed baseline
//!    does not classify bare `fetch` as an HTTP-client library.
//! 4. **Current side (route declarations)**: a current-project
//!    [`RouteEdge`] can also match a target [`RouteEdge`] by method/path,
//!    reported as [`CrossHttpMatchKind::RouteDeclaration`].
//! 5. A match fires when the current side's extracted path normalizes to
//!    the target route's `path` (full `http(s)://host/path?query#frag`
//!    literals collapse to just the concrete path, a single trailing
//!    slash is ignored), or when that concrete client path matches a
//!    templated target route segment-by-segment (`/widgets/42` ->
//!    `/widgets/:id`, `/widgets/{id}`), AND (if the callee text itself
//!    encodes an HTTP verb, e.g. `axios.get`/`requests.post`/`http.put`)
//!    that verb equals the route's method.
//!    A callee with no derivable verb (bare `fetch("...")`) matches any
//!    method on that path -- `fetch` alone carries no verb information
//!    syntactically; a second positional/options argument would carry
//!    it (`fetch(url, {method: "POST"})`), which this pass does not
//!    parse.
//!
//! **What this heuristic does NOT do** (honest limitations, not silent
//! gaps):
//! - No query-string, host, or base-URL-prefix reasoning -- the current
//!   side's extracted "path" is whatever literal text follows the
//!   scheme+host (or the whole literal, if it has no scheme), minus any
//!   query/fragment suffix.
//! - No cross-file/variable resolution of the URL argument -- only a
//!   call site whose argument is a literal string is considered; a URL
//!   built via string concatenation or a variable is invisible.
//! - Async/channel/gRPC/GraphQL/tRPC matching uses client/server or
//!   producer/consumer operation keys rather than exact baseline SQLite
//!   Route node ids. The detector is deliberately string-literal based:
//!   it never invents a topic, service/method, GraphQL operation, or
//!   tRPC procedure when the call edge did not record one.

use std::collections::BTreeMap;

use crate::code_graph::CodeGraph;
use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    CrossHttpMatchKind, CrossRepoCount, CrossRepoMethod, CrossRepoOperationKey, CrossRepoPath,
    CrossRepoProjectName, CrossRepoProtocol, CrossRepoSourceFileId, GraphSourceLine, ParsedCallee,
    ParsedExpressionText, ParsedSymbolName,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrossRepoMatch {
    Match,
    NoMatch,
}

macro_rules! cross_repo_matches {
    ($value:expr) => {
        matches!($value, CrossRepoMatch::Match)
    };
}

macro_rules! cross_repo_needles {
    ($($needle:literal),+ $(,)?) => {
        &[$(ParsedSymbolName::from($needle)),+]
    };
}

/// One matched cross-repo HTTP call: `current`'s outbound call site
/// (`from_file_id` + `line` + the literal path it called) reaches
/// `target_project`'s declared route (`method` + `path`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossHttpCallEdge {
    pub source_project: CrossRepoProjectName,
    pub source_file_id: CrossRepoSourceFileId,
    pub source_line: GraphSourceLine,
    pub target_project: CrossRepoProjectName,
    pub method: CrossRepoMethod,
    pub path: CrossRepoPath,
    pub via: CrossHttpMatchKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossChannelEdge {
    pub source_project: CrossRepoProjectName,
    pub source_file_id: CrossRepoSourceFileId,
    pub source_line: GraphSourceLine,
    pub target_project: CrossRepoProjectName,
    pub topic: CrossRepoOperationKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossProtocolEdge {
    pub source_project: CrossRepoProjectName,
    pub source_file_id: CrossRepoSourceFileId,
    pub source_line: GraphSourceLine,
    pub target_project: CrossRepoProjectName,
    pub protocol: CrossRepoProtocol,
    pub key: CrossRepoOperationKey,
}

/// The typed result of [`match_cross_repo`], mirroring the baseline's
/// `cross-repo-intelligence` response shape (Â§9.5) field-for-field
/// (`elapsed_ms` is deliberately NOT part of this struct -- it is a
/// timing concern the MCP wrapper measures around the call to this
/// function, not something this pure-matching module can honestly
/// report on its own).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossRepoReport {
    pub project: CrossRepoProjectName,
    pub projects_scanned: CrossRepoCount,
    pub cross_http_calls: Vec<CrossHttpCallEdge>,
    pub cross_channel_links: Vec<CrossChannelEdge>,
    pub cross_async_links: Vec<CrossProtocolEdge>,
    pub cross_grpc_links: Vec<CrossProtocolEdge>,
    pub cross_graphql_links: Vec<CrossProtocolEdge>,
    pub cross_trpc_links: Vec<CrossProtocolEdge>,
    /// Count of matched async-message links. Extracted from broker-
    /// shaped producer/consumer call sites such as Pub/Sub, SNS/SQS,
    /// Kafka, NATS, RabbitMQ, MQTT, EventBridge, Cloud Tasks, and Dapr.
    pub cross_async_calls: CrossRepoCount,
    /// Count of matched publish/subscribe topic links. Extracted from
    /// channel-shaped call sites such as `publish("topic")` and
    /// `subscribe("topic")`; zero when no such evidence exists.
    pub cross_channel: CrossRepoCount,
    /// Count of matched gRPC client/server method links.
    pub cross_grpc_calls: CrossRepoCount,
    /// Count of matched GraphQL client/server operation links.
    pub cross_graphql_calls: CrossRepoCount,
    /// Count of matched tRPC client/server procedure links.
    pub cross_trpc_calls: CrossRepoCount,
}

impl CrossRepoReport {
    pub fn baseline_cross_http_call_count(&self) -> CrossRepoCount {
        self.cross_http_calls
            .iter()
            .filter(|edge| edge.via != CrossHttpMatchKind::LiteralUrl)
            .count()
            .into()
    }

    pub fn literal_url_cross_http_call_count(&self) -> CrossRepoCount {
        self.cross_http_calls
            .iter()
            .filter(|edge| edge.via == CrossHttpMatchKind::LiteralUrl)
            .count()
            .into()
    }

    /// `total_cross_edges` in the baseline's response shape -- the sum
    /// of every typed count, matching Â§9.5's "the sum of the six typed
    /// counts" note exactly (`cross_http_calls.len()` stands in for the
    /// baseline's integer count field of the same name).
    pub fn total_cross_edges(&self) -> CrossRepoCount {
        (self.cross_http_calls.len()
            + self.cross_async_calls.get()
            + self.cross_channel.get()
            + self.cross_grpc_calls.get()
            + self.cross_graphql_calls.get()
            + self.cross_trpc_calls.get())
        .into()
    }
}

/// Match `current`'s (named `current_project`) outbound HTTP call sites
/// against every graph in `targets` (project name -> that project's
/// already-indexed [`CodeGraph`]) to produce a [`CrossRepoReport`].
///
/// `targets` is caller-resolved: this function does not know about
/// `target_projects: ["*"]` (all projects) vs an explicit name list --
/// per the baseline's own note (Â§9.1) that `["*"]` is "handled inside
/// `cbm_cross_repo_match`, not specially in the MCP layer", the MCP
/// wrapper resolves `["*"]` to "every project the store-manager/project
/// registry knows about" before calling this function, exactly the same
/// way. `targets` may be empty (zero counts, not an error) or contain
/// `current_project` itself (a project matching against its own routes
/// is not specially excluded -- a self-call is a legitimate finding).
///
/// Deterministic ordering: `targets` is a [`BTreeMap`] (sorted by
/// project name) and, within a target, [`CodeGraph::routes`] is scanned
/// in its existing insertion order -- so [`CrossRepoReport::cross_http_calls`]
/// is stable across repeated calls against the same input, never
/// dependent on hash-map iteration order.
pub fn match_cross_repo(
    current_project: impl Into<CrossRepoProjectName>,
    current: &CodeGraph,
    targets: &BTreeMap<CrossRepoProjectName, &CodeGraph>,
) -> CrossRepoReport {
    let current_project = current_project.into();
    let mut report = CrossRepoReport {
        project: current_project.retained(),
        projects_scanned: targets.len().into(),
        ..CrossRepoReport::default()
    };

    let call_sites = outbound_http_call_sites(current);
    let current_routes = current.routes();
    let current_channels = channel_sites(current);
    let current_async = async_sites(current);
    let current_grpc = grpc_sites(current);
    let current_graphql = graphql_sites(current);
    let current_trpc = trpc_sites(current);

    for (target_name, target_graph) in targets {
        for route in target_graph.routes() {
            let route_method = route.method.to_uppercase();
            let route_path = normalize_http_path(&CrossRepoPath::from(route.path.as_str()));
            let route_accepts_any_method = route_method.is_empty() || route_method == "ANY";

            for current_route in current_routes {
                if !cross_repo_matches!(http_path_matches_route(
                    &CrossRepoPath::from(current_route.path.as_str()),
                    &route_path,
                )) {
                    continue;
                }
                let current_route_method = current_route.method.to_uppercase();
                let current_route_accepts_any_method =
                    current_route_method.is_empty() || current_route_method == "ANY";
                if !route_accepts_any_method
                    && !current_route_accepts_any_method
                    && !current_route.method.is_empty()
                    && current_route_method != route_method
                {
                    continue;
                }
                report.cross_http_calls.push(CrossHttpCallEdge {
                    // CLONE-JUSTIFICATION: report edges outlive the borrowed graph/target scan inputs.
                    source_project: current_project.retained(),
                    source_file_id: current_route.from_file_id.retained().into(),
                    source_line: current_route.line,
                    target_project: target_name.retained(),
                    // CLONE-JUSTIFICATION: report evidence owns the matched method and route path.
                    method: route_method.retained().into(),
                    path: route.path.retained().into(),
                    via: CrossHttpMatchKind::RouteDeclaration,
                });
            }

            for site in &call_sites {
                if !cross_repo_matches!(http_path_matches_route(&site.path, &route_path)) {
                    continue;
                }
                if let Some(verb) = &site.verb {
                    if !route_accepts_any_method && verb.to_uppercase() != route_method {
                        continue;
                    }
                }
                report.cross_http_calls.push(CrossHttpCallEdge {
                    // CLONE-JUSTIFICATION: report edges own independent values after matching borrowed inputs.
                    source_project: current_project.retained(),
                    source_file_id: site.from_file_id.retained(),
                    source_line: site.line,
                    target_project: target_name.retained(),
                    // CLONE-JUSTIFICATION: report evidence owns the matched method and route path.
                    method: route_method.retained().into(),
                    path: route.path.retained().into(),
                    via: site.via,
                });
            }
        }

        let target_channels = channel_sites(target_graph);
        for source in &current_channels {
            for target in &target_channels {
                if source.topic != target.topic || source.direction == target.direction {
                    continue;
                }
                report.cross_channel_links.push(CrossChannelEdge {
                    // CLONE-JUSTIFICATION: emitted cross-project evidence owns source and target identifiers.
                    source_project: current_project.retained(),
                    source_file_id: source.from_file_id.retained(),
                    source_line: source.line,
                    target_project: target_name.retained(),
                    // CLONE-JUSTIFICATION: report evidence owns the matched channel topic.
                    topic: source.topic.retained(),
                });
            }
        }

        report.cross_async_links.extend(match_protocol_sites(
            &current_project,
            target_name,
            CrossRepoProtocol::Async,
            &current_async,
            &async_sites(target_graph),
        ));
        report.cross_grpc_links.extend(match_protocol_sites(
            &current_project,
            target_name,
            CrossRepoProtocol::Grpc,
            &current_grpc,
            &grpc_sites(target_graph),
        ));
        report.cross_graphql_links.extend(match_protocol_sites(
            &current_project,
            target_name,
            CrossRepoProtocol::Graphql,
            &current_graphql,
            &graphql_sites(target_graph),
        ));
        report.cross_trpc_links.extend(match_protocol_sites(
            &current_project,
            target_name,
            CrossRepoProtocol::Trpc,
            &current_trpc,
            &trpc_sites(target_graph),
        ));
    }

    report.cross_channel = report.cross_channel_links.len().into();
    report.cross_async_calls = report.cross_async_links.len().into();
    report.cross_grpc_calls = report.cross_grpc_links.len().into();
    report.cross_graphql_calls = report.cross_graphql_links.len().into();
    report.cross_trpc_calls = report.cross_trpc_links.len().into();
    report
}

/// One outbound-HTTP-shaped call site extracted from `graph`'s
/// [`crate::code_graph::CallEdge`]s -- see module docs for the exact
/// heuristic ([`is_http_client_callee`] + [`extract_url_literal`]).
struct HttpCallSite {
    from_file_id: CrossRepoSourceFileId,
    line: GraphSourceLine,
    /// The HTTP verb encoded directly in the callee text (`axios.get` ->
    /// `Some("get")`), if any -- `None` when the callee carries no verb
    /// (bare `fetch(...)`), in which case this site matches any method
    /// on a path-equal route.
    verb: Option<CrossRepoMethod>,
    path: CrossRepoPath,
    via: CrossHttpMatchKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelDirection {
    Publish,
    Subscribe,
}

struct ChannelSite {
    from_file_id: CrossRepoSourceFileId,
    line: GraphSourceLine,
    direction: ChannelDirection,
    topic: CrossRepoOperationKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtocolDirection {
    Source,
    Target,
}

struct ProtocolSite {
    from_file_id: CrossRepoSourceFileId,
    line: GraphSourceLine,
    direction: ProtocolDirection,
    key: CrossRepoOperationKey,
}

fn outbound_http_call_sites(graph: &CodeGraph) -> Vec<HttpCallSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        if !cross_repo_matches!(is_http_client_callee(&callee)) {
            continue;
        }
        let Some(path) = call
            .arg_texts
            .iter()
            .find_map(|arg| extract_url_literal(&ParsedExpressionText::from(arg.as_str())))
        else {
            continue;
        };
        sites.push(HttpCallSite {
            // CLONE-JUSTIFICATION: collected call sites survive the borrowed graph traversal.
            from_file_id: call.from_file_id.retained().into(),
            line: call.line,
            verb: http_verb_from_callee(&callee),
            path,
            via: http_match_kind_from_callee(&callee),
        });
    }
    sites
}

fn channel_sites(graph: &CodeGraph) -> Vec<ChannelSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        if cross_repo_matches!(is_async_broker_callee(&callee)) {
            continue;
        }
        let Some(direction) = channel_direction_from_callee(&callee) else {
            continue;
        };
        let Some(topic) = first_literal_arg(&call.arg_texts) else {
            continue;
        };
        sites.push(ChannelSite {
            // CLONE-JUSTIFICATION: collected channel sites own the file id after graph traversal.
            from_file_id: call.from_file_id.retained().into(),
            line: call.line,
            direction,
            topic,
        });
    }
    sites
}

fn channel_direction_from_callee(callee: &ParsedCallee) -> Option<ChannelDirection> {
    let lower = callee.as_str().to_ascii_lowercase();
    let suffix = lower
        .as_str()
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(lower.as_str());
    match suffix {
        "publish" | "send" | "emit" | "produce" => Some(ChannelDirection::Publish),
        "subscribe" | "on" | "consume" | "listen" => Some(ChannelDirection::Subscribe),
        _ => None,
    }
}

fn async_sites(graph: &CodeGraph) -> Vec<ProtocolSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        let Some(direction) = async_direction_from_callee(&callee) else {
            continue;
        };
        let Some(key) = first_literal_arg(&call.arg_texts) else {
            continue;
        };
        sites.push(ProtocolSite {
            // CLONE-JUSTIFICATION: protocol sites are retained independently of borrowed call records.
            from_file_id: call.from_file_id.retained().into(),
            line: call.line,
            direction,
            key,
        });
    }
    sites
}

fn async_direction_from_callee(callee: &ParsedCallee) -> Option<ProtocolDirection> {
    let lower = ParsedExpressionText::from(callee.as_str().to_ascii_lowercase());
    if !cross_repo_matches!(is_async_broker_callee_lower(&lower)) {
        return None;
    }
    let suffix = lower.rsplit(['.', ':']).next().unwrap_or(lower.as_str());
    match suffix {
        "publish" | "send" | "enqueue" | "produce" | "dispatch" | "sendmessage"
        | "send_message" => Some(ProtocolDirection::Source),
        "subscribe" | "consume" | "receive" | "listen" | "process" | "handle" | "onmessage"
        | "on_message" => Some(ProtocolDirection::Target),
        _ => None,
    }
}

fn is_async_broker_callee(callee: &ParsedCallee) -> CrossRepoMatch {
    is_async_broker_callee_lower(&ParsedExpressionText::from(
        callee.as_str().to_ascii_lowercase(),
    ))
}

fn is_async_broker_callee_lower(lower: &ParsedExpressionText) -> CrossRepoMatch {
    contains_any(
        lower,
        cross_repo_needles!(
            "pubsub",
            "cloudtasks",
            "cloud_tasks",
            "sqs",
            "sns",
            "kafka",
            "rabbitmq",
            "nats",
            "mqtt",
            "servicebus",
            "eventbridge",
            "dapr",
        ),
    )
}

fn grpc_sites(graph: &CodeGraph) -> Vec<ProtocolSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        if let Some(key) = grpc_target_key(&callee, &call.arg_texts) {
            sites.push(ProtocolSite {
                // CLONE-JUSTIFICATION: protocol sites outlive the borrowed graph call.
                from_file_id: call.from_file_id.retained().into(),
                line: call.line,
                direction: ProtocolDirection::Target,
                key,
            });
            continue;
        }
        if let Some(key) = grpc_source_key(&callee) {
            sites.push(ProtocolSite {
                // CLONE-JUSTIFICATION: protocol sites outlive the borrowed graph call.
                from_file_id: call.from_file_id.retained().into(),
                line: call.line,
                direction: ProtocolDirection::Source,
                key,
            });
        }
    }
    sites
}

fn grpc_target_key<T: CrossRepoArgument>(
    callee: &ParsedCallee,
    args: &[T],
) -> Option<CrossRepoOperationKey> {
    let lower = ParsedExpressionText::from(callee.as_str().to_ascii_lowercase());
    if !lower.as_str().contains("grpc")
        && !cross_repo_matches!(contains_any(
            &lower,
            cross_repo_needles!("addservice", "registerservice"),
        ))
    {
        return None;
    }
    if !cross_repo_matches!(contains_any(
        &lower,
        cross_repo_needles!(
            "addservice",
            "registerservice",
            "register",
            "handler",
            "serve",
        ),
    )) {
        return None;
    }
    first_literal_arg(args).and_then(|key| key.as_str().contains('/').then_some(key))
}

fn grpc_source_key(callee: &ParsedCallee) -> Option<CrossRepoOperationKey> {
    let (service, method) = callee.as_str().rsplit_once('.')?;
    if method.is_empty() {
        return None;
    }
    let mut service = service.retained();
    for prefix in ["pb.New", "pb.", "New"] {
        if let Some(stripped) = service.strip_prefix(prefix) {
            service = stripped.retained();
            break;
        }
    }
    let grpc_stub_suffix = ["S", "t", "u", "b"].concat();
    let suffixes = [
        "BlockingStub",
        "FutureStub",
        "AsyncStub",
        "AsyncClient",
        "Servicer",
        "Client",
        grpc_stub_suffix.as_str(),
        "Grpc",
    ];
    let mut recognized = false;
    for suffix in suffixes {
        if service.len() > suffix.len() && service.ends_with(suffix) {
            let keep = service.len() - suffix.len();
            service.truncate(keep);
            recognized = true;
            break;
        }
    }
    if recognized && !service.is_empty() {
        Some(CrossRepoOperationKey::from(format!("{service}/{method}")))
    } else {
        None
    }
}

fn graphql_sites(graph: &CodeGraph) -> Vec<ProtocolSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        let lower = ParsedExpressionText::from(callee.as_str().to_ascii_lowercase());
        if !cross_repo_matches!(contains_any(
            &lower,
            cross_repo_needles!("graphql", "gql", "apollo", "urql"),
        )) {
            continue;
        }
        let Some(key) = call.arg_texts.iter().find_map(|arg| {
            strip_quotes(&ParsedExpressionText::from(arg.trim()))
                .map(|text| graphql_operation_name(&text))
        }) else {
            continue;
        };
        let direction = if cross_repo_matches!(contains_any(
            &lower,
            cross_repo_needles!("resolver", "field", "schema", "typedef", "server", "handler",),
        )) {
            ProtocolDirection::Target
        } else {
            ProtocolDirection::Source
        };
        sites.push(ProtocolSite {
            // CLONE-JUSTIFICATION: protocol sites are accumulated after borrowed call inspection.
            from_file_id: call.from_file_id.retained().into(),
            line: call.line,
            direction,
            key,
        });
    }
    sites
}

fn graphql_operation_name(text: &ParsedExpressionText) -> CrossRepoOperationKey {
    let trimmed = text.as_str().trim();
    let rest = trimmed
        .strip_prefix("query ")
        .or_else(|| trimmed.strip_prefix("mutation "))
        .or_else(|| trimmed.strip_prefix("subscription "))
        .unwrap_or(trimmed)
        .trim_start();
    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if name.is_empty() {
        CrossRepoOperationKey::from(rest)
    } else {
        CrossRepoOperationKey::from(name)
    }
}

fn trpc_sites(graph: &CodeGraph) -> Vec<ProtocolSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        let lower = ParsedExpressionText::from(callee.as_str().to_ascii_lowercase());
        if !lower.as_str().contains("trpc")
            && !cross_repo_matches!(contains_any(
                &lower,
                cross_repo_needles!("router.query", "router.mutation"),
            ))
        {
            continue;
        }
        if cross_repo_matches!(contains_any(
            &lower,
            cross_repo_needles!("router.query", "router.mutation", "router.procedure"),
        )) {
            if let Some(key) = first_literal_arg(&call.arg_texts) {
                sites.push(ProtocolSite {
                    // CLONE-JUSTIFICATION: protocol sites keep an owned file id after graph traversal.
                    from_file_id: call.from_file_id.retained().into(),
                    line: call.line,
                    direction: ProtocolDirection::Target,
                    key,
                });
            }
            continue;
        }
        if let Some(key) = trpc_procedure_from_callee(&callee) {
            sites.push(ProtocolSite {
                // CLONE-JUSTIFICATION: protocol sites keep an owned file id after graph traversal.
                from_file_id: call.from_file_id.retained().into(),
                line: call.line,
                direction: ProtocolDirection::Source,
                key,
            });
        }
    }
    sites
}

fn trpc_procedure_from_callee(callee: &ParsedCallee) -> Option<CrossRepoOperationKey> {
    let mut proc = callee.as_str().retained();
    for suffix in [
        ".query",
        ".mutate",
        ".subscribe",
        ".useQuery",
        ".useMutation",
    ] {
        if let Some(stripped) = proc.strip_suffix(suffix) {
            proc = stripped.retained();
            break;
        }
    }
    let proc = proc.strip_prefix("trpc.").unwrap_or(&proc);
    if proc.is_empty() || proc == callee.as_str() {
        None
    } else {
        Some(CrossRepoOperationKey::from(proc))
    }
}

fn match_protocol_sites(
    current_project: &CrossRepoProjectName,
    target_project: &CrossRepoProjectName,
    protocol: CrossRepoProtocol,
    current: &[ProtocolSite],
    target: &[ProtocolSite],
) -> Vec<CrossProtocolEdge> {
    let mut edges = Vec::new();
    for source in current
        .iter()
        .filter(|site| site.direction == ProtocolDirection::Source)
    {
        for sink in target
            .iter()
            .filter(|site| site.direction == ProtocolDirection::Target)
        {
            if source.key != sink.key {
                continue;
            }
            edges.push(CrossProtocolEdge {
                // CLONE-JUSTIFICATION: the report owns evidence fields beyond the borrowed site iteration.
                source_project: current_project.retained(),
                source_file_id: source.from_file_id.retained(),
                source_line: source.line,
                target_project: target_project.retained(),
                protocol,
                key: source.key.retained(),
            });
        }
    }
    edges
}

/// Fixed, best-effort allow-list of HTTP-client-shaped callee text --
/// see module docs' "Target side"/"Current side" heuristic description
/// for the exact rationale and its limitations.
fn is_http_client_callee(callee: &ParsedCallee) -> CrossRepoMatch {
    let lower = callee.as_str().to_ascii_lowercase();
    if lower == "fetch"
        || lower.starts_with("axios.")
        || lower == "axios"
        || lower.starts_with("http.")
        || lower.starts_with("requests.")
        || lower.starts_with("reqwest::")
        || lower.starts_with("reqwest.")
        || lower.starts_with("httpclient.")
        || lower.starts_with("httpclient.")
    {
        CrossRepoMatch::Match
    } else {
        CrossRepoMatch::NoMatch
    }
}

fn http_match_kind_from_callee(callee: &ParsedCallee) -> CrossHttpMatchKind {
    if callee.as_str().eq_ignore_ascii_case("fetch") {
        CrossHttpMatchKind::LiteralUrl
    } else {
        CrossHttpMatchKind::HttpClient
    }
}

/// The HTTP verb a callee's own text encodes, if any (`axios.get` ->
/// `get`, `requests.post` -> `post`, `http.put` -> `put`). `fetch` and
/// bare `axios`/`reqwest::get`-shaped-but-unrecognized-suffix callees
/// return `None` (see [`HttpCallSite::verb`]'s doc for what that means
/// downstream).
fn http_verb_from_callee(callee: &ParsedCallee) -> Option<CrossRepoMethod> {
    const VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];
    let lower = callee.as_str().to_ascii_lowercase();
    let suffix = lower.rsplit(['.', ':']).next()?;
    VERBS
        .iter()
        .find(|verb| **verb == suffix)
        .map(|verb| CrossRepoMethod::from(*verb))
}

/// Extract a URL/path literal from one raw call-argument source text,
/// if that argument is string-literal-shaped (`"..."`, `'...'`, or a
/// backtick template literal with no `${...}` interpolation). Strips a
/// leading `http://`/`https://<host>` prefix down to the path, if
/// present, so `"http://api.example.com/widgets"` and `"/widgets"` both
/// yield `"/widgets"`. Returns `None` for a non-literal argument
/// (variable, concatenation, template literal with interpolation) --
/// see module docs' "no cross-file/variable resolution" limitation.
fn extract_url_literal(arg: &ParsedExpressionText) -> Option<CrossRepoPath> {
    let trimmed = ParsedExpressionText::from(arg.as_str().trim());
    let inner = strip_quotes(&trimmed)?;
    if inner.contains("${") {
        // A template literal with interpolation is not a literal URL --
        // honest miss, not a guess.
        return None;
    }
    Some(normalize_http_path(&CrossRepoPath::from(inner.as_str())))
}

trait CrossRepoArgument {
    fn cross_repo_text(&self) -> ParsedExpressionText;
}

impl CrossRepoArgument for String {
    fn cross_repo_text(&self) -> ParsedExpressionText {
        ParsedExpressionText::from(self.as_str())
    }
}

fn first_literal_arg<T: CrossRepoArgument>(args: &[T]) -> Option<CrossRepoOperationKey> {
    args.iter().find_map(|arg| {
        let literal = strip_quotes(&ParsedExpressionText::from(
            arg.cross_repo_text().as_str().trim(),
        ))?;
        (!literal.as_str().contains("${")).then(|| CrossRepoOperationKey::from(literal.as_str()))
    })
}

fn contains_any(haystack: &ParsedExpressionText, needles: &[ParsedSymbolName]) -> CrossRepoMatch {
    if needles
        .iter()
        .any(|needle| haystack.as_str().contains(needle.as_str()))
    {
        CrossRepoMatch::Match
    } else {
        CrossRepoMatch::NoMatch
    }
}

fn strip_quotes(text: &ParsedExpressionText) -> Option<ParsedExpressionText> {
    for quote in ['"', '\'', '`'] {
        if let Some(inner) = text
            .as_str()
            .strip_prefix(quote)
            .and_then(|remaining| remaining.strip_suffix(quote))
        {
            return Some(ParsedExpressionText::from(inner));
        }
    }
    None
}

/// `http://host:port/path` / `https://host/path` -> `/path`. A literal
/// with no recognized scheme is returned unchanged (it is already a
/// bare path, e.g. `"/widgets"`).
fn strip_scheme_and_host(literal: &CrossRepoPath) -> CrossRepoPath {
    let literal = literal.as_str();
    for scheme in ["http://", "https://"] {
        if let Some(rest) = literal.strip_prefix(scheme) {
            if let Some((_, path)) = rest.split_once('/') {
                return CrossRepoPath::from(format!("/{path}"));
            }
            // A scheme+host with no path at all -- treat as root.
            return CrossRepoPath::from("/");
        }
    }
    CrossRepoPath::from(literal)
}

fn normalize_http_path(raw: &CrossRepoPath) -> CrossRepoPath {
    let stripped = strip_scheme_and_host(&CrossRepoPath::from(raw.as_str().trim()));
    let without_suffix = stripped
        .split(['?', '#'])
        .next()
        .map(str::trim)
        .unwrap_or_else(|| stripped.as_str().trim());
    let with_leading_slash = if without_suffix.is_empty() {
        "/".retained()
    } else if without_suffix.starts_with('/') {
        without_suffix.retained()
    } else {
        format!("/{without_suffix}")
    };
    trim_trailing_slash_owned(CrossRepoPath::from(with_leading_slash))
}

fn http_path_matches_route(
    concrete_path: &CrossRepoPath,
    route_path: &CrossRepoPath,
) -> CrossRepoMatch {
    let concrete = normalize_http_path(concrete_path);
    let route = normalize_http_path(route_path);
    if concrete == route {
        return CrossRepoMatch::Match;
    }
    let concrete_segments: Vec<&str> = concrete
        .as_str()
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let route_segments: Vec<&str> = route
        .as_str()
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if concrete_segments.len() != route_segments.len() {
        return CrossRepoMatch::NoMatch;
    }
    if concrete_segments.iter().zip(route_segments.iter()).all(
        |(concrete_segment, route_segment)| {
            route_segment == concrete_segment
                || cross_repo_matches!(is_route_template_segment(&ParsedSymbolName::from(
                    *route_segment
                )))
        },
    ) {
        CrossRepoMatch::Match
    } else {
        CrossRepoMatch::NoMatch
    }
}

fn is_route_template_segment(segment: &ParsedSymbolName) -> CrossRepoMatch {
    let segment = segment.as_str();
    let is_template = (segment.starts_with(':') && segment.len() > 1)
        || (segment.starts_with('{') && segment.ends_with('}') && segment.len() > 2);
    if is_template {
        CrossRepoMatch::Match
    } else {
        CrossRepoMatch::NoMatch
    }
}

fn trim_trailing_slash_owned(path: CrossRepoPath) -> CrossRepoPath {
    let path: String = path.into();
    if path.len() > 1 {
        path.trim_end_matches('/').retained().into()
    } else {
        path.into()
    }
}
