//! X06 core parity: cross-repo-intelligence mode.
//!
//! Baseline binding: `docs/plans/enforcer-selfhost-plan/refs/
//! x06-baseline-tool-schemas.md` §9.2/§9.5. The baseline's
//! `index_repository(mode="cross-repo-intelligence")` never re-indexes
//! anything; it matches already-indexed projects' Routes/Channels
//! against each other to create `CROSS_HTTP_CALLS`/`CROSS_ASYNC_CALLS`/
//! `CROSS_CHANNEL`/`CROSS_GRPC_CALLS`/`CROSS_GRAPHQL_CALLS`/
//! `CROSS_TRPC_CALLS` edges, and returns a fixed-shape response with a
//! typed count per protocol plus `total_cross_edges`/`elapsed_ms`.
//!
//! This module is the library-layer analog: given one project's
//! [`crate::code_graph::CodeGraph`] (`current`) and a set of named
//! target projects' graphs, it matches `current`'s outbound-HTTP-call
//! sites against each target's declared [`crate::code_graph::RouteEdge`]s
//! to produce [`CrossHttpCallEdge`]s, and reports zero counts (never an
//! omitted field, never an error) for every other protocol this crate
//! does not yet detect. Wiring into the MCP `index_repository` handler's
//! `mode="cross-repo-intelligence"` branch is [`crate::mcp`]'s job (a
//! shared file this lane only touches minimally); this module owns the
//! matching algorithm and its typed result.
//!
//! # Matching heuristic -- documented honestly
//!
//! `CROSS_HTTP_CALLS` is the only edge kind this module currently
//! detects a real match for:
//!
//! 1. **Target side (declared routes)**: every [`RouteEdge`] already
//!    extracted by [`crate::code_graph`]'s language extractors (Axum/
//!    Actix/Express/FastAPI-style route macros/decorators) -- `method`
//!    (upper-cased) + `path` (as written, e.g. `/widgets/:id`).
//! 2. **Current side (outbound call sites)**: every
//!    [`crate::code_graph::CallEdge`] whose `callee` text matches a
//!    fixed, best-effort allow-list of HTTP-client-shaped callees
//!    ([`is_http_client_callee`] -- `fetch`, `axios.*`, `http.get`/
//!    `http.post`/etc., `requests.*`, `reqwest::*`/`reqwest.*`,
//!    `httpClient.*`/`httpclient.*`), combined with a URL/path literal
//!    found in [`CallEdge::arg_texts`] ([`extract_url_literal`] -- the
//!    first string-literal-shaped argument, `"..."`/`'...'`/`` `...` ``,
//!    with a leading `http://`/`https://` scheme stripped if present).
//! 3. A match fires when the current side's extracted path **equals**
//!    the target route's `path`, ignoring a single trailing slash, AND
//!    (if the callee text itself encodes an HTTP verb, e.g. `axios.get`/
//!    `requests.post`/`http.put`) that verb equals the route's method.
//!    A callee with no derivable verb (bare `fetch("...")`) matches any
//!    method on that path -- `fetch` alone carries no verb information
//!    syntactically; a second positional/options argument would carry
//!    it (`fetch(url, {method: "POST"})`), which this pass does not
//!    parse.
//!
//! **What this heuristic does NOT do** (honest limitations, not silent
//! gaps):
//! - No path *parameter* matching (`/widgets/:id` vs a call site's
//!   `/widgets/42`) -- only byte-equal paths (modulo the one trailing
//!   slash) match. A parameterized route is invisible to a call site
//!   built from a literal id.
//! - No query-string, host, or base-URL-prefix reasoning -- the current
//!   side's extracted "path" is whatever literal text follows the
//!   scheme+host (or the whole literal, if it has no scheme), verbatim.
//! - No cross-file/variable resolution of the URL argument -- only a
//!   call site whose argument is a literal string is considered; a URL
//!   built via string concatenation or a variable is invisible.
//! - `CROSS_ASYNC_CALLS`/`CROSS_CHANNEL`/`CROSS_GRPC_CALLS`/
//!   `CROSS_GRAPHQL_CALLS`/`CROSS_TRPC_CALLS` have no detector at all in
//!   this crate yet (no message-queue/channel/gRPC/GraphQL/tRPC
//!   extraction exists upstream to match against) -- [`CrossRepoReport`]
//!   always reports these as `0`, per the workpack's "emit zeros...
//!   rather than omitting fields" instruction, not as an error or an
//!   omitted key.

use std::collections::BTreeMap;

use crate::code_graph::CodeGraph;

/// One matched cross-repo HTTP call: `current`'s outbound call site
/// (`from_file_id` + `line` + the literal path it called) reaches
/// `target_project`'s declared route (`method` + `path`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossHttpCallEdge {
    pub source_project: String,
    pub source_file_id: String,
    pub source_line: usize,
    pub target_project: String,
    pub method: String,
    pub path: String,
}

/// The typed result of [`match_cross_repo`], mirroring the baseline's
/// `cross-repo-intelligence` response shape (§9.5) field-for-field
/// (`elapsed_ms` is deliberately NOT part of this struct -- it is a
/// timing concern the MCP wrapper measures around the call to this
/// function, not something this pure-matching module can honestly
/// report on its own).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossRepoReport {
    pub project: String,
    pub projects_scanned: usize,
    pub cross_http_calls: Vec<CrossHttpCallEdge>,
    /// Always `0` today -- no async-messaging extraction exists
    /// upstream to detect a match against. See module docs.
    pub cross_async_calls: usize,
    /// Always `0` today -- no pub/sub channel extraction exists
    /// upstream. See module docs.
    pub cross_channel: usize,
    /// Always `0` today -- no gRPC extraction exists upstream. See
    /// module docs.
    pub cross_grpc_calls: usize,
    /// Always `0` today -- no GraphQL extraction exists upstream. See
    /// module docs.
    pub cross_graphql_calls: usize,
    /// Always `0` today -- no tRPC extraction exists upstream. See
    /// module docs.
    pub cross_trpc_calls: usize,
}

impl CrossRepoReport {
    /// `total_cross_edges` in the baseline's response shape -- the sum
    /// of every typed count, matching §9.5's "the sum of the six typed
    /// counts" note exactly (`cross_http_calls.len()` stands in for the
    /// baseline's integer count field of the same name).
    pub fn total_cross_edges(&self) -> usize {
        self.cross_http_calls.len()
            + self.cross_async_calls
            + self.cross_channel
            + self.cross_grpc_calls
            + self.cross_graphql_calls
            + self.cross_trpc_calls
    }
}

/// Match `current`'s (named `current_project`) outbound HTTP call sites
/// against every graph in `targets` (project name -> that project's
/// already-indexed [`CodeGraph`]) to produce a [`CrossRepoReport`].
///
/// `targets` is caller-resolved: this function does not know about
/// `target_projects: ["*"]` (all projects) vs an explicit name list --
/// per the baseline's own note (§9.1) that `["*"]` is "handled inside
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
    current_project: &str,
    current: &CodeGraph,
    targets: &BTreeMap<String, &CodeGraph>,
) -> CrossRepoReport {
    let mut report = CrossRepoReport {
        project: current_project.to_owned(),
        projects_scanned: targets.len(),
        ..CrossRepoReport::default()
    };

    let call_sites = outbound_http_call_sites(current);

    for (target_name, target_graph) in targets {
        for route in target_graph.routes() {
            let route_method = route.method.to_uppercase();
            let route_path = trim_trailing_slash(&route.path);

            for site in &call_sites {
                if trim_trailing_slash(&site.path) != route_path {
                    continue;
                }
                if let Some(verb) = &site.verb {
                    if verb.to_uppercase() != route_method {
                        continue;
                    }
                }
                report.cross_http_calls.push(CrossHttpCallEdge {
                    source_project: current_project.to_owned(),
                    source_file_id: site.from_file_id.clone(),
                    source_line: site.line,
                    target_project: target_name.clone(),
                    method: route_method.clone(),
                    path: route.path.clone(),
                });
            }
        }
    }

    report
}

/// One outbound-HTTP-shaped call site extracted from `graph`'s
/// [`crate::code_graph::CallEdge`]s -- see module docs for the exact
/// heuristic ([`is_http_client_callee`] + [`extract_url_literal`]).
struct HttpCallSite {
    from_file_id: String,
    line: usize,
    /// The HTTP verb encoded directly in the callee text (`axios.get` ->
    /// `Some("get")`), if any -- `None` when the callee carries no verb
    /// (bare `fetch(...)`), in which case this site matches any method
    /// on a path-equal route.
    verb: Option<String>,
    path: String,
}

fn outbound_http_call_sites(graph: &CodeGraph) -> Vec<HttpCallSite> {
    let mut sites = Vec::new();
    for call in graph.calls() {
        if !is_http_client_callee(&call.callee) {
            continue;
        }
        let Some(path) = call
            .arg_texts
            .iter()
            .find_map(|arg| extract_url_literal(arg))
        else {
            continue;
        };
        sites.push(HttpCallSite {
            from_file_id: call.from_file_id.clone(),
            line: call.line,
            verb: http_verb_from_callee(&call.callee),
            path,
        });
    }
    sites
}

/// Fixed, best-effort allow-list of HTTP-client-shaped callee text --
/// see module docs' "Target side"/"Current side" heuristic description
/// for the exact rationale and its limitations.
fn is_http_client_callee(callee: &str) -> bool {
    let lower = callee.to_ascii_lowercase();
    lower == "fetch"
        || lower.starts_with("axios.")
        || lower == "axios"
        || lower.starts_with("http.")
        || lower.starts_with("requests.")
        || lower.starts_with("reqwest::")
        || lower.starts_with("reqwest.")
        || lower.starts_with("httpclient.")
}

/// The HTTP verb a callee's own text encodes, if any (`axios.get` ->
/// `get`, `requests.post` -> `post`, `http.put` -> `put`). `fetch` and
/// bare `axios`/`reqwest::get`-shaped-but-unrecognized-suffix callees
/// return `None` (see [`HttpCallSite::verb`]'s doc for what that means
/// downstream).
fn http_verb_from_callee(callee: &str) -> Option<String> {
    const VERBS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];
    let lower = callee.to_ascii_lowercase();
    let suffix = lower.rsplit(['.', ':']).next()?;
    VERBS
        .iter()
        .find(|verb| **verb == suffix)
        .map(|verb| (*verb).to_owned())
}

/// Extract a URL/path literal from one raw call-argument source text,
/// if that argument is string-literal-shaped (`"..."`, `'...'`, or a
/// backtick template literal with no `${...}` interpolation). Strips a
/// leading `http://`/`https://<host>` prefix down to the path, if
/// present, so `"http://api.example.com/widgets"` and `"/widgets"` both
/// yield `"/widgets"`. Returns `None` for a non-literal argument
/// (variable, concatenation, template literal with interpolation) --
/// see module docs' "no cross-file/variable resolution" limitation.
fn extract_url_literal(arg: &str) -> Option<String> {
    let trimmed = arg.trim();
    let inner = strip_quotes(trimmed)?;
    if inner.contains("${") {
        // A template literal with interpolation is not a literal URL --
        // honest miss, not a guess.
        return None;
    }
    Some(strip_scheme_and_host(inner))
}

fn strip_quotes(text: &str) -> Option<&str> {
    for quote in ['"', '\'', '`'] {
        if text.len() >= 2 && text.starts_with(quote) && text.ends_with(quote) {
            return Some(&text[1..text.len() - 1]);
        }
    }
    None
}

/// `http://host:port/path` / `https://host/path` -> `/path`. A literal
/// with no recognized scheme is returned unchanged (it is already a
/// bare path, e.g. `"/widgets"`).
fn strip_scheme_and_host(literal: &str) -> String {
    for scheme in ["http://", "https://"] {
        if let Some(rest) = literal.strip_prefix(scheme) {
            if let Some(slash_idx) = rest.find('/') {
                return rest[slash_idx..].to_owned();
            }
            // A scheme+host with no path at all -- treat as root.
            return "/".to_owned();
        }
    }
    literal.to_owned()
}

fn trim_trailing_slash(path: &str) -> &str {
    if path.len() > 1 {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CallEdge, RouteEdge};

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
}
