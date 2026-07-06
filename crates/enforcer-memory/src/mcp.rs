//! X06.7: the MCP stdio JSON-RPC server for `enforcer-memory`.
//!
//! # Charter
//!
//! This module exposes the codebase-memory-mcp 14-tool parity floor
//! (`refs/x06-baseline-tool-schemas.md`) as an MCP `tools/list` +
//! `tools/call` surface. Wire framing (dual `Content-Length:`/NDJSON,
//! auto-detected per message) is reused directly from the arc-21
//! `enforcer-mcp` skeleton's [`enforcer_mcp::transport`] module rather than
//! re-implemented here -- that module is pure I/O-free framing/message-type
//! code with no business logic, so importing it does not pull this crate
//! into arc-21's tool surface, stdio sink, or stale-write gate.
//!
//! # Stateless-per-call dispatch (honest wiring, no invented persistence)
//!
//! Most tools in [`dispatch_tool`] are a pure function of `(tool name,
//! JSON args) -> JSON result`: tools that need a code graph rebuild it
//! fresh from `repoPath` within the same call (no incremental reuse
//! across calls yet, recorded honestly rather than faked); `manage_adr`
//! takes/returns the ADR list inline so the caller owns persistence. The
//! three project-registry tools (`list_projects`/`delete_project`/
//! `index_status`) are the one exception: they operate over a
//! caller-supplied `storesDir` on-disk registry ([`crate::projects`]),
//! not a freshly-rebuilt in-memory graph, since that is what their
//! backing library functions actually persist across calls. This is also
//! what makes the CLI mirror ([`crate::cli`]) trivially call-for-call
//! identical to the MCP surface: both call the exact same
//! [`dispatch_tool`].
//!
//! # Tool wiring
//!
//! All fourteen baseline tools, and every documented mode of each, now
//! dispatch into landed library functions: `index_repository` (->
//! [`code_graph::CodeGraph::index_repository`]), `search_graph` (->
//! [`fulltext::FullTextIndex`] for the minimal bm25-only shape, or
//! [`crate::search::search_graph::search_graph_with_semantic`] for the
//! full bm25/regex/semantic spec), `query_graph` (-> [`analysis::query`]),
//! `trace_path` (-> [`crate::analysis::trace::trace_calls`] /
//! [`crate::analysis::trace::trace_data_flow`] /
//! [`crate::analysis::trace::trace_cross_service`] for its three modes),
//! `get_code_snippet` (-> [`snippet::get_code_snippet`]),
//! `get_graph_schema` (-> [`graph_schema::get_graph_schema`]),
//! `get_architecture` (-> [`architecture::build_report`], full aspect
//! set), `search_code` (-> [`code_search::search_code`]),
//! `list_projects`/`delete_project`/`index_status` (-> [`projects`]),
//! `manage_adr` (-> [`adr::AdrStore`]), `detect_changes` (->
//! [`impact::analyze_diff_impact`]), and `ingest_traces` (->
//! [`crate::traces::TraceStore`]). [`not_wired`]/[`NotWiredError`] remain
//! in this module as the honest shape a genuinely unlanded tool/mode
//! would return -- no arm in [`dispatch_tool`] reaches them today, but
//! removing the mechanism itself would be a regression for the next
//! not-yet-landed capability.
//!
//! # Registry-as-one-line-per-tool wiring
//!
//! [`dispatch_tool`]'s `match` is the single place a tool name resolves to
//! a handler; wiring a not-yet-landed tool at integration is changing its
//! arm from [`not_wired`] to a real handler call -- no other structural
//! change required.
//!
//! # Wire envelope (binding: `refs/x06-baseline-tool-schemas.md` §1)
//!
//! [`dispatch_tool`] returns this crate's own tool-result JSON directly
//! (`{"ok": bool, ...}`); [`wrap_envelope`] is the one place that shape is
//! wrapped into the baseline's exact MCP tool-result envelope, applied
//! identically by both the `tools/call` handler below and
//! [`crate::cli::cli_invoke`] (so CLI mirror parity holds on the
//! envelope-wrapped shape, not just the inner JSON):
//!
//! ```json
//! {
//!   "content": [{ "type": "text", "text": "<the inner JSON, serialized as a string>" }],
//!   "structuredContent": { "...the same JSON, as a real object -- omitted when isError is true..." },
//!   "isError": false
//! }
//! ```
//!
//! `isError` is `true` exactly when the inner JSON's `"ok"` field is not
//! `true` (covers both this crate's [`ToolError`] and [`NotWiredError`]
//! shapes, matching the baseline's own "no single fixed error-object
//! schema, treat `content[0].text` as opaque-or-parseable" posture, §17).
//!
//! # Further binding facts
//!
//! - **Dual framing**: newline-delimited JSON (default) AND LSP-style
//!   `Content-Length:` framing, auto-detected per message -- inherited
//!   for free from [`enforcer_mcp::transport::FrameReader`]/
//!   [`enforcer_mcp::transport::encode_frame`]; a `Content-Length`-framed
//!   reply carries NO trailing newline (also already that module's
//!   behavior).
//! - **`initialize`**: echoes the client's `protocolVersion` if this
//!   server supports it (`SUPPORTED_PROTOCOL_VERSIONS`), else falls back
//!   to the newest supported version; `capabilities: { tools: {
//!   listChanged: false } }` only -- no resources/prompts/logging
//!   capabilities advertised.
//! - **`tools/list` is PAGINATED**: [`TOOLS_LIST_PAGE_SIZE`] (8) tools per
//!   page, `cursor` param is a stringified integer offset,
//!   `nextCursor` present ONLY when more pages remain. Every descriptor
//!   carries `name`/`title`/`description`/`inputSchema`/`outputSchema`
//!   (the last a tool-independent constant); no `$schema` draft URI
//!   anywhere.
//! - **Only two JSON-RPC error codes ever exist**: `-32700` (parse error,
//!   `id` hardcoded to `0`, never `null`) and `-32601` (method not found,
//!   `id` echoed). Every tool-level failure -- including an unknown tool
//!   NAME passed to `tools/call` (`isError:true`, text `"unknown tool:
//!   <name>"` verbatim, see [`call_tool`]) -- is a normal RESULT, never a
//!   JSON-RPC error object. `-32603` (internal error) is never emitted by
//!   this module.
//! - **`notifications/cancelled`**: the one notification the baseline
//!   gives semantic meaning to (cancel an in-flight request by id).
//!   This crate's dispatch is synchronous with no in-flight pipeline to
//!   cancel, so it (like every other notification) is silently ignored,
//!   no response -- a documented no-op rather than a missing feature.
//! - **Per-request diagnostics**: every `tools/call`/`initialize`/`ping`/
//!   `tools/list` request emits one [`crate::diagnostics::RequestRecord`]
//!   (`event: "mcp.request"`, fields `protocol`/`method`/`tool`/`status`/
//!   `duration`) to stderr via [`crate::diagnostics::emit_to_stderr`], at
//!   WARN level on error and INFO otherwise.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::adr::{AdrDocument, AdrRecord, AdrStore};
use crate::analysis::trace::{
    trace_calls, trace_cross_service, trace_data_flow, TraceCallsParams, TraceCrossServiceParams,
};
use crate::analysis::{query as graph_query, CodeAdjacency, TraceDirection};
use crate::architecture::{self, Aspect};
use crate::code_graph::{CodeGraph, Manifest};
use crate::code_search::{self, SearchMode, SearchQuery};
use crate::embed::Embedder;
use crate::fulltext::FullTextIndex;
use crate::graph_schema;
use crate::impact;
use crate::projects;
use crate::search::document::{DocumentKind, SearchDocument};
use crate::search::search_graph::{
    search_graph_with_semantic, NodeLabel as SearchGraphNodeLabel, SearchGraphSpec,
};
use crate::snippet;
use crate::traces::{TraceRecord, TraceStore};

use enforcer_mcp::transport::{
    encode_frame, Frame, FrameReader, Framing, RpcError, RpcMessage, RpcResult,
};

/// The baseline 14-tool parity floor (`refs/x06-baseline-tool-schemas.md`),
/// in the digest's own enumeration order. This is the single source of
/// truth [`tool_descriptors`] and [`dispatch_tool`] both key off.
pub const TOOL_NAMES: &[&str] = &[
    "index_repository",
    "search_graph",
    "query_graph",
    "trace_path",
    "get_code_snippet",
    "get_graph_schema",
    "get_architecture",
    "search_code",
    "list_projects",
    "delete_project",
    "index_status",
    "detect_changes",
    "manage_adr",
    "ingest_traces",
];

/// Tools with a landed library function behind them in this pass. Every
/// other name in [`TOOL_NAMES`] dispatches to [`not_wired`].
const WIRED_TOOLS: &[&str] = &[
    "index_repository",
    "search_graph",
    "query_graph",
    "trace_path",
    "get_code_snippet",
    "get_graph_schema",
    "get_architecture",
    "search_code",
    "list_projects",
    "delete_project",
    "index_status",
    "detect_changes",
    "manage_adr",
    "ingest_traces",
];

/// One MCP tool's static descriptor (mirrors `enforcer_mcp::registry`'s
/// shape so both crates' `tools/list` payloads look the same on the wire).
/// Field set and shape match `refs/x06-baseline-tool-schemas.md`'s fully
/// verified `tools/list` contract exactly: `name`/`title`/`description`/
/// `inputSchema` plus a constant `outputSchema` identical for every tool
/// (`{"type":"object","additionalProperties":true}`) -- no `$schema` draft
/// URI anywhere, matching the baseline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Value,
}

/// The constant `outputSchema` every tool descriptor carries (binding:
/// coordinator-verified baseline contract) -- the baseline does not
/// declare a per-tool output shape, just this permissive constant.
fn constant_output_schema() -> Value {
    json!({ "type": "object", "additionalProperties": true })
}

/// Build every tool's descriptor for `tools/list`. All 14 baseline tools
/// are always advertised (parity requirement) regardless of wiring state;
/// a caller discovers wiring state only by calling `tools/call` and reading
/// [`NotWiredError`], never by a tool being missing from this list. Use
/// [`handle_tools_list`] for the actual `tools/list` response -- this
/// function returns the full, unpaginated set (also the seam
/// [`crate::cli`] and tests use directly).
pub fn tool_descriptors() -> Vec<ToolDescriptor> {
    TOOL_NAMES
        .iter()
        .map(|&name| ToolDescriptor {
            name: name.to_owned(),
            title: tool_title(name).to_owned(),
            description: tool_description(name).to_owned(),
            input_schema: tool_input_schema(name),
            output_schema: constant_output_schema(),
        })
        .collect()
}

fn tool_title(name: &str) -> &'static str {
    match name {
        "index_repository" => "Index Repository",
        "search_graph" => "Search Graph",
        "query_graph" => "Query Graph",
        "trace_path" => "Trace Path",
        "get_code_snippet" => "Get Code Snippet",
        "get_graph_schema" => "Get Graph Schema",
        "get_architecture" => "Get Architecture",
        "search_code" => "Search Code",
        "list_projects" => "List Projects",
        "delete_project" => "Delete Project",
        "index_status" => "Index Status",
        "detect_changes" => "Detect Changes",
        "manage_adr" => "Manage ADR",
        "ingest_traces" => "Ingest Traces",
        _ => "",
    }
}

fn tool_description(name: &str) -> &'static str {
    match name {
        "index_repository" => "Index a repository's current working tree into a code knowledge graph (files, symbols, imports, calls, routes).",
        "search_graph" => "Code-aware search over an indexed repository's files/symbols: bm25 full-text, regex (name/qualified-name + label/degree/connectivity filters), and semantic (cosine over keyword embeddings) modes.",
        "query_graph" => "Read-only Cypher-subset query over the code graph (MATCH/WHERE/RETURN/ORDER BY/LIMIT/DISTINCT/COUNT).",
        "trace_path" => "Trace a path from a node: calls (call-graph BFS), data_flow (call-graph edges, honestly labeled call-graph-only approximation), or cross_service (producer/route/consumer paths).",
        "get_code_snippet" => "Fetch a byte-exact code snippet by qualified name, optionally with same-file neighbors.",
        "get_graph_schema" => "Fetch the graph's node labels and edge types, with counts.",
        "get_architecture" => "Architecture overview: structure, dependencies, routes, languages, packages, entry points, hotspots, boundaries, layers, file tree, clusters.",
        "search_code" => "Graph-augmented text search: regex match ranked by structural importance, enriched with containing symbol.",
        "list_projects" => "List every indexed project known to the store registry.",
        "delete_project" => "Delete an indexed project and its stored artifacts.",
        "index_status" => "Report the indexing status/freshness for a project.",
        "detect_changes" => "Impact analysis from a set of changed files: affected symbols + risk classification.",
        "manage_adr" => "Get/update ADR (architecture decision record) sections, linked to graph nodes.",
        "ingest_traces" => "Ingest runtime call traces to enrich CALLS edges with observed frequency.",
        _ => "",
    }
}

fn tool_input_schema(name: &str) -> Value {
    match name {
        "index_repository" => json!({
            "type": "object",
            "required": ["repoPath"],
            "properties": {
                "repoPath": { "type": "string", "description": "Absolute path to the repository root." },
                "mode": { "type": "string", "enum": ["full", "moderate", "fast", "cross-repo-intelligence"], "default": "full", "description": "\"cross-repo-intelligence\" short-circuits before any indexing pipeline (baseline §9.2): matches this project's outbound HTTP call sites against target_projects' declared routes. See crate::cross_repo for the exact heuristic." },
                "targetProjects": { "type": "array", "items": { "type": "string" }, "description": "Required only for mode=\"cross-repo-intelligence\": project ids (resolved via storesDir) or, if storesDir is omitted, literal repo paths to match against. [\"*\"] means every project storesDir knows about." },
                "storesDir": { "type": "string", "description": "cross-repo-intelligence only: the project registry directory (crate::projects) targetProjects ids are resolved against. Omit to pass targetProjects as literal repo paths instead." },
                "name": { "type": "string", "description": "cross-repo-intelligence only: the current project's own name as reported in the response's \"project\" field; defaults to repoPath if omitted." }
            }
        }),
        "search_graph" => json!({
            "type": "object",
            "required": ["repoPath", "query"],
            "properties": {
                "repoPath": {
                    "type": "string",
                    "description": "Deviation from baseline's `project` param (refs/x06-baseline-tool-schemas.md §2): this lane has no project-scoped indexed-graph cache wired into search_graph yet, so the tool takes a filesystem path and indexes it fresh per call rather than resolving a named project."
                },
                "query": { "type": "string" },
                "namePattern": { "type": "string", "description": "Regex mode: matched against node name (baseline §2.1 name_pattern)." },
                "qnPattern": { "type": "string", "description": "Regex mode: matched against qualified name (baseline §2.1 qn_pattern)." },
                "label": { "type": "string", "enum": ["Function", "Type", "Test", "File", "TextOnly"] },
                "filePattern": { "type": "string" },
                "relationship": { "type": "string" },
                "minDegree": { "type": "integer" },
                "maxDegree": { "type": "integer" },
                "excludeEntryPoints": { "type": "boolean", "default": false },
                "includeConnected": { "type": "boolean", "default": false },
                "semanticQuery": { "type": "array", "items": { "type": "string" } },
                "offset": { "type": "integer", "minimum": 0, "default": 0 },
                "mode": { "type": "string", "enum": ["bm25", "regex", "semantic"], "default": "bm25", "description": "All three modes are wired: bm25 (fulltext::FullTextIndex, the minimal {repoPath,query} shape), regex (namePattern/qnPattern + filters), and semantic (semanticQuery, combined with the regex path per crate::search::search_graph's mode-interaction contract)." },
                "limit": { "type": "integer", "minimum": 1, "default": 100, "description": "Matches the baseline's actual BM25_DEFAULT_LIMIT code default (100), not its docstring's claim of 200 -- refs/x06-baseline-tool-schemas.md §2.1." }
            }
        }),
        "query_graph" => json!({
            "type": "object",
            "required": ["repoPath", "query"],
            "properties": {
                "repoPath": { "type": "string" },
                "query": { "type": "string", "description": "Read-only Cypher-subset query string." }
            }
        }),
        "trace_path" => json!({
            "type": "object",
            "required": ["repoPath", "startNodeId"],
            "properties": {
                "repoPath": { "type": "string" },
                "startNodeId": { "type": "string" },
                "mode": { "type": "string", "enum": ["calls", "data_flow", "cross_service"], "default": "calls" },
                "direction": { "type": "string", "enum": ["in", "out", "both"], "default": "out" },
                "depth": { "type": "integer", "minimum": 1, "default": 3 },
                "includeTests": { "type": "boolean", "default": true }
            }
        }),
        "get_code_snippet" => json!({
            "type": "object",
            "required": ["repoPath", "qualifiedName"],
            "properties": {
                "repoPath": { "type": "string" },
                "qualifiedName": { "type": "string" },
                "includeNeighbors": { "type": "boolean", "default": false }
            }
        }),
        "get_graph_schema" => json!({
            "type": "object",
            "required": ["repoPath"],
            "properties": { "repoPath": { "type": "string" } }
        }),
        "get_architecture" => json!({
            "type": "object",
            "required": ["repoPath"],
            "properties": {
                "repoPath": { "type": "string" },
                "aspects": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["all", "overview", "structure", "dependencies", "routes", "languages", "packages", "entry_points", "hotspots", "boundaries", "layers", "file_tree", "clusters"] },
                    "default": ["overview"]
                },
                "path": { "type": "string", "description": "Directory-prefix scope, applied uniformly across every requested aspect." },
                "hotspotLimit": { "type": "integer", "minimum": 1, "default": 10 },
                "maxIterations": { "type": "integer", "minimum": 1, "default": 100 }
            }
        }),
        "search_code" => json!({
            "type": "object",
            "required": ["repoPath", "query"],
            "properties": {
                "repoPath": { "type": "string" },
                "query": { "type": "string" },
                "mode": { "type": "string", "enum": ["compact", "full", "files"], "default": "compact" },
                "context": { "type": "integer", "minimum": 0, "default": 0 },
                "limit": { "type": "integer", "minimum": 1, "default": 10 }
            }
        }),
        "list_projects" => json!({
            "type": "object",
            "required": ["storesDir"],
            "properties": {
                "storesDir": { "type": "string", "description": "Deviation from baseline's implicit cache-directory scan: this lane's project registry (crate::projects) is rooted at a caller-supplied directory, not an auto-resolved cache dir." }
            }
        }),
        "delete_project" => json!({
            "type": "object",
            "required": ["storesDir", "projectId"],
            "properties": {
                "storesDir": { "type": "string" },
                "projectId": { "type": "string" }
            }
        }),
        "index_status" => json!({
            "type": "object",
            "required": ["storesDir", "projectId"],
            "properties": {
                "storesDir": { "type": "string" },
                "projectId": { "type": "string" }
            }
        }),
        "detect_changes" => json!({
            "type": "object",
            "required": ["repoPath", "changedPaths"],
            "properties": {
                "repoPath": { "type": "string" },
                "changedPaths": { "type": "array", "items": { "type": "string" } },
                "baseBranch": { "type": "string" },
                "since": { "type": "string" },
                "maxDepth": { "type": "integer", "minimum": 1, "default": 3 }
            }
        }),
        "manage_adr" => json!({
            "type": "object",
            "required": ["project"],
            "properties": {
                "project": { "type": "string", "description": "Baseline (refs/x06-baseline-tool-schemas.md §14.1) project id the ADR document is scoped to." },
                "mode": {
                    "type": "string",
                    "enum": ["get", "update", "sections"],
                    "default": "get",
                    "description": "Baseline §14.1: defaults to \"get\" when absent; the handler also accepts the undocumented alias \"store\" as a synonym for \"update\" (not in this enum, matching the baseline's hidden extra value)."
                },
                "document": {
                    "type": "string",
                    "description": "Deviation from baseline: this lane has no persistence layer, so the caller round-trips the whole stored document (the baseline's SQLite-backed blob) across calls the same way manage_adr's section-based extension round-trips \"adrs\" below."
                },
                "content": { "type": "string", "description": "Baseline §14.1: the full ADR markdown body, required for mode=\"update\"/\"store\" -- whole-document replace, not a diff/merge." },
                "operation": { "type": "string", "enum": ["get", "update_section", "link_node", "create"], "description": "Extension mode (not in the baseline): section-based ADR API, reached by passing \"operation\" instead of \"mode\"." },
                "adrs": { "type": "array", "description": "Extension mode: current ADR list the caller persists between calls (round-tripped)." },
                "id": { "type": "string" },
                "title": { "type": "string" },
                "section": { "type": "string" },
                "body": { "type": "string" },
                "nodeId": { "type": "string" }
            }
        }),
        "ingest_traces" => json!({
            "type": "object",
            "required": ["repoPath", "traces"],
            "properties": {
                "repoPath": { "type": "string" },
                "traces": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["caller", "callee", "count"],
                        "properties": {
                            "caller": { "type": "string" },
                            "callee": { "type": "string" },
                            "count": { "type": "integer" }
                        }
                    }
                }
            }
        }),
        _ => json!({ "type": "object" }),
    }
}

/// The honest "not wired yet" shape every unlanded tool/mode returns.
/// Never a stub result and never fake data (workpack hard requirement) --
/// callers can match on `capabilityState` to distinguish this from a real
/// error.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NotWiredError {
    #[serde(rename = "capabilityState")]
    pub capability_state: &'static str,
    pub tool: String,
    pub reason: String,
}

fn not_wired(tool: &str, reason: impl Into<String>) -> Value {
    let error = NotWiredError {
        capability_state: "not_wired",
        tool: tool.to_owned(),
        reason: reason.into(),
    };
    json!({ "ok": false, "error": error })
}

/// A dispatch-level error: distinct from [`NotWiredError`] (which is a
/// *result value*, not a failure of dispatch itself) -- this covers bad
/// arguments, a repo path that will not index, a query that fails to
/// parse, etc. Always reported as `{"ok": false, "error": {...}}` in MCP
/// tool results (never a raw MCP-protocol-level error, per baseline
/// behavior: a tool call that fails is still a successful `tools/call`
/// RPC carrying a failure payload) and as a nonzero exit from the CLI
/// mirror ([`crate::cli`]).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolError {
    pub tool: String,
    pub message: String,
}

fn tool_error(tool: &str, message: impl Into<String>) -> Value {
    json!({ "ok": false, "error": ToolError { tool: tool.to_owned(), message: message.into() } })
}

/// Dispatch one `tools/call`-shaped request: `name` is the tool name as
/// received, `args` is the raw JSON `arguments` object. Returns a JSON
/// value that is ALWAYS a valid tool result (`{"ok": true, ...}` or
/// `{"ok": false, "error": ...}`) -- this function never panics on
/// malformed input and never returns `Err`, matching the MCP contract
/// that a tool failure is still a successful RPC.
/// [`crate::cli::cli_invoke`] calls this same function so MCP and CLI are
/// call-for-call identical.
pub fn dispatch_tool(name: &str, args: &Value) -> Value {
    if !TOOL_NAMES.contains(&name) {
        return tool_error(name, format!("unknown tool: {name}"));
    }
    if !WIRED_TOOLS.contains(&name) {
        return not_wired(
            name,
            format!(
                "{name} has no landed enforcer-memory library function yet (owned by a parallel lane); returning honest not_wired state rather than a stub result"
            ),
        );
    }
    match name {
        "index_repository" => handle_index_repository(args),
        "search_graph" => handle_search_graph(args),
        "query_graph" => handle_query_graph(args),
        "trace_path" => handle_trace_path(args),
        "get_code_snippet" => handle_get_code_snippet(args),
        "get_graph_schema" => handle_get_graph_schema(args),
        "get_architecture" => handle_get_architecture(args),
        "search_code" => handle_search_code(args),
        "list_projects" => handle_list_projects(args),
        "delete_project" => handle_delete_project(args),
        "index_status" => handle_index_status(args),
        "detect_changes" => handle_detect_changes(args),
        "manage_adr" => handle_manage_adr(args),
        "ingest_traces" => handle_ingest_traces(args),
        other => tool_error(other, "wired-list/dispatch mismatch (registry bug)"),
    }
}

/// Wrap [`dispatch_tool`]'s inner result JSON into the baseline's exact
/// MCP tool-result envelope (binding: `refs/x06-baseline-tool-schemas.md`
/// §1):
///
/// ```json
/// { "content": [{ "type": "text", "text": "<inner JSON as a string>" }],
///   "structuredContent": { ... only when isError is false ... },
///   "isError": bool }
/// ```
///
/// `isError` is derived from the inner JSON's own `"ok"` field (this
/// crate's uniform success/failure marker across every handler above) --
/// `true` whenever `"ok"` is anything other than the literal `true`,
/// matching the baseline's "no single fixed error-object schema" posture
/// (§17): a not-wired result, a tool_error, and any other non-success
/// shape all become `isError: true` the same way. Applied identically by
/// the `tools/call` handler ([`handle_tools_call`]) and
/// [`crate::cli::cli_invoke`] so CLI mirror parity holds on the
/// envelope-wrapped shape, not just the inner JSON.
pub fn wrap_envelope(inner: &Value) -> Value {
    let is_error = inner.get("ok").and_then(Value::as_bool) != Some(true);
    let text = serde_json::to_string(inner).unwrap_or_else(|_| "{}".to_owned());
    if is_error {
        json!({
            "content": [{ "type": "text", "text": text }],
            "isError": true,
        })
    } else {
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": inner,
            "isError": false,
        })
    }
}

/// The full `tools/call` pipeline: unknown-tool-NAME special case (binding
/// spec: `text: "unknown tool: <name>"` verbatim, no JSON-wrapped inner
/// error object) falls through to [`dispatch_tool`] + [`wrap_envelope`]
/// for every recognized name (wired or not-wired). ONE function so the
/// MCP `tools/call` handler ([`handle_tools_call`]) and
/// [`crate::cli::cli_invoke`] produce byte-identical envelopes for the
/// unknown-tool case too, not just the recognized-tool case
/// [`wrap_envelope`] alone already covered.
pub fn call_tool(name: &str, args: &Value) -> Value {
    if !TOOL_NAMES.contains(&name) {
        return json!({
            "content": [{ "type": "text", "text": format!("unknown tool: {name}") }],
            "isError": true,
        });
    }
    wrap_envelope(&dispatch_tool(name, args))
}

// ---------------------------------------------------------------------
// Shared repo-indexing helper
// ---------------------------------------------------------------------

/// Recursively walk `repo_root`, returning every non-ignored file path.
/// [`CodeGraph::index_repository`] deliberately does not walk the
/// filesystem itself (its own module docs: "directory walking is a
/// caller/CLI concern") -- this is that caller. A fixed, small ignore
/// list (`.git`, `target`, `node_modules`, `.enforcer-memory`) keeps this
/// from indexing build output; it is not a full `.gitignore` evaluator
/// (out of scope for this lane).
fn walk_repo_files(repo_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    const IGNORED_DIRS: &[&str] = &[".git", "target", "node_modules", ".enforcer-memory"];
    let mut out = Vec::new();
    let mut stack = vec![repo_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if IGNORED_DIRS.contains(&name.as_ref()) {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Build a fresh [`CodeGraph`] for `repo_path` (string argument, as
/// received over the wire). Shared by every handler below that needs a
/// graph -- see module docs on the stateless-per-call design.
fn build_graph(repo_path: &str) -> Result<CodeGraph, Value> {
    let root = PathBuf::from(repo_path);
    if !root.is_dir() {
        return Err(tool_error(
            "index_repository",
            format!("repoPath {repo_path:?} is not a directory"),
        ));
    }
    let files = walk_repo_files(&root).map_err(|source| {
        tool_error(
            "index_repository",
            format!("failed to walk {repo_path}: {source}"),
        )
    })?;
    let mut graph = CodeGraph::new();
    graph
        .index_repository(&root, &files, &Manifest::default())
        .map_err(|source| tool_error("index_repository", format!("index failed: {source}")))?;
    Ok(graph)
}

fn require_str<'a>(args: &'a Value, field: &str) -> Result<&'a str, Value> {
    args.get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| tool_error(field, format!("missing or empty required field {field:?}")))
}

// ---------------------------------------------------------------------
// index_repository -> code_graph::CodeGraph::index_repository
// ---------------------------------------------------------------------

fn handle_index_repository(args: &Value) -> Value {
    if args.get("mode").and_then(Value::as_str) == Some("cross-repo-intelligence") {
        return handle_cross_repo_mode(args);
    }

    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let root = PathBuf::from(repo_path);
    if !root.is_dir() {
        return tool_error(
            "index_repository",
            format!("repoPath {repo_path:?} is not a directory"),
        );
    }
    let files = match walk_repo_files(&root) {
        Ok(files) => files,
        Err(source) => {
            return tool_error(
                "index_repository",
                format!("failed to walk {repo_path}: {source}"),
            )
        }
    };
    let mut graph = CodeGraph::new();
    let (manifest, report) = match graph.index_repository(&root, &files, &Manifest::default()) {
        Ok(pair) => pair,
        Err(source) => return tool_error("index_repository", format!("index failed: {source}")),
    };
    json!({
        "ok": true,
        "filesIndexed": files.len(),
        "nodeCount": graph.nodes().len(),
        "manifestEntryCount": manifest.entries.len(),
        "report": {
            "unchanged": report.unchanged,
            "changed": report.changed,
            "added": report.added,
            "deleted": report.deleted,
        }
    })
}

// ---------------------------------------------------------------------
// index_repository(mode="cross-repo-intelligence") ->
// crate::cross_repo::match_cross_repo
//
// Baseline binding: refs/x06-baseline-tool-schemas.md §9.2/§9.5. Unlike
// the plain-mode branch above, this never re-extracts a fresh CodeGraph
// through `insert_file_and_chunks`'s full symbol/edge pipeline for its
// own sake -- it builds one stateless graph per project (this crate has
// no persisted graph cache to reuse yet, same "no ghost project
// database"/stateless-per-call posture `build_graph` already documents
// above) purely as the input `crate::cross_repo::match_cross_repo` reads
// routes/calls off of, then matches. `targetProjects: ["*"]` resolves to
// every project `storesDir` knows about via `crate::projects::list_projects`
// (its own `repo_root`), exactly matching the baseline's "[\"*\"] means
// all indexed projects" note (§9.1) -- resolved here in the MCP layer
// for this lane (the baseline resolves it one level deeper, inside its
// own matcher; this crate's project registry is a caller-supplied
// `storesDir`, so this handler is the natural place to read it, still
// "not specially handled by match_cross_repo itself", which only ever
// sees an already-resolved name->graph map).
// ---------------------------------------------------------------------

fn handle_cross_repo_mode(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let Some(target_projects) = args.get("targetProjects").and_then(Value::as_array) else {
        return tool_error(
            "index_repository",
            "target_projects is required for cross-repo-intelligence mode. Use [\"*\"] for all projects. Run list_projects to see available.",
        );
    };
    let target_names: Vec<String> = target_projects
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if target_names.is_empty() {
        return tool_error(
            "index_repository",
            "target_projects is required for cross-repo-intelligence mode. Use [\"*\"] for all projects. Run list_projects to see available.",
        );
    }

    let current_project = args
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(repo_path);

    let current_graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };

    // Resolve target project names (or "*") to (name, repo_root) pairs
    // via the storesDir registry, same as list_projects/index_status
    // above -- `storesDir` is optional: a caller with no project
    // registry at all (every target given as a literal filesystem path
    // instead of a registered project id) can omit it, in which case
    // every entry in `target_names` is tried directly as a `repoPath`.
    let stores_dir = args.get("storesDir").and_then(Value::as_str);
    let resolved_targets: Vec<(String, String)> = if target_names.iter().any(|n| n == "*") {
        match stores_dir {
            Some(dir) => match projects::list_projects(Path::new(dir)) {
                Ok(list) => list
                    .into_iter()
                    .map(|p| (p.project_id, p.repo_root))
                    .collect(),
                Err(source) => {
                    return tool_error("index_repository", format!("{source}"));
                }
            },
            None => Vec::new(),
        }
    } else if let Some(dir) = stores_dir {
        match projects::list_projects(Path::new(dir)) {
            Ok(list) => {
                let by_id: std::collections::HashMap<String, String> = list
                    .into_iter()
                    .map(|p| (p.project_id, p.repo_root))
                    .collect();
                target_names
                    .iter()
                    .filter_map(|name| by_id.get(name).map(|root| (name.clone(), root.clone())))
                    .collect()
            }
            Err(source) => {
                return tool_error("index_repository", format!("{source}"));
            }
        }
    } else {
        // No storesDir: treat each target name as a literal repo path.
        target_names
            .iter()
            .map(|name| (name.clone(), name.clone()))
            .collect()
    };

    let started = std::time::Instant::now();

    let mut target_graphs: std::collections::BTreeMap<String, CodeGraph> =
        std::collections::BTreeMap::new();
    for (name, root) in &resolved_targets {
        match build_graph(root) {
            Ok(graph) => {
                target_graphs.insert(name.clone(), graph);
            }
            Err(_) => {
                // A target that fails to build (missing dir, unreadable
                // files) is skipped, not a hard error for the whole
                // call -- matches this mode's own "no match -> zero
                // counts, not errors" contract for every OTHER kind of
                // miss; a target project that cannot even be opened is
                // the same kind of honest zero-contribution case, not a
                // reason to fail projects_scanned entirely.
                continue;
            }
        }
    }

    let target_refs: std::collections::BTreeMap<String, &CodeGraph> = target_graphs
        .iter()
        .map(|(name, graph)| (name.clone(), graph))
        .collect();

    let report = crate::cross_repo::match_cross_repo(current_project, &current_graph, &target_refs);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    json!({
        "ok": true,
        "status": "success",
        "mode": "cross-repo-intelligence",
        "project": report.project,
        "projects_scanned": report.projects_scanned,
        "cross_http_calls": report.cross_http_calls.len(),
        "cross_async_calls": report.cross_async_calls,
        "cross_channel": report.cross_channel,
        "cross_grpc_calls": report.cross_grpc_calls,
        "cross_graphql_calls": report.cross_graphql_calls,
        "cross_trpc_calls": report.cross_trpc_calls,
        "total_cross_edges": report.total_cross_edges(),
        "elapsed_ms": elapsed_ms,
    })
}

// ---------------------------------------------------------------------
// search_graph -> fulltext::FullTextIndex (minimal bm25-only shape) or
// search::search_graph::search_graph_with_semantic (bm25/regex/semantic)
// ---------------------------------------------------------------------

fn documents_from_graph(graph: &CodeGraph) -> Vec<SearchDocument> {
    let mut docs = Vec::new();
    for file in graph.file_nodes() {
        docs.push(
            SearchDocument::new(file.id.clone(), DocumentKind::File, file.rel_path.clone())
                .with_source_path(file.rel_path.clone()),
        );
    }
    for symbol in graph.symbol_nodes() {
        docs.push(SearchDocument::new(
            symbol.id.clone(),
            DocumentKind::Function,
            symbol.name.clone(),
        ));
    }
    docs
}

fn parse_node_label(raw: &str) -> Option<SearchGraphNodeLabel> {
    match raw {
        "Function" => Some(SearchGraphNodeLabel::Function),
        "Type" => Some(SearchGraphNodeLabel::Type),
        "Test" => Some(SearchGraphNodeLabel::Test),
        "File" => Some(SearchGraphNodeLabel::File),
        "TextOnly" => Some(SearchGraphNodeLabel::TextOnly),
        _ => None,
    }
}

fn search_graph_hit_json(hit: &crate::search::search_graph::SearchGraphHit) -> Value {
    json!({
        "name": hit.name,
        "qualifiedName": hit.qualified_name,
        "label": hit.label,
        "filePath": hit.file_path,
        "rank": hit.rank,
        "inDegree": hit.in_degree,
        "outDegree": hit.out_degree,
        "connectedNames": hit.connected_names,
        "score": hit.score,
    })
}

fn handle_search_graph(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("bm25");

    // Legacy bm25-only fast path (kept for the existing minimal
    // {repoPath, query} shape): only taken when the caller has not
    // opted into the richer search_graph::SearchGraphSpec fields
    // (name_pattern/qn_pattern/semantic_query/etc). Any of those present
    // routes through the full spec-driven path below, mode name aside.
    let has_spec_fields = args.get("namePattern").is_some()
        || args.get("qnPattern").is_some()
        || args.get("semanticQuery").is_some();
    if mode == "bm25" && !has_spec_fields {
        let query = match require_str(args, "query") {
            Ok(value) => value,
            Err(err) => return err,
        };
        // Default 100, matching the baseline's actual BM25_DEFAULT_LIMIT
        // code default (refs/x06-baseline-tool-schemas.md §2.1) rather
        // than its own docstring's claim of 200 for all modes -- see the
        // inputSchema description for this param.
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(100)
            .max(1) as usize;

        let graph = match build_graph(repo_path) {
            Ok(graph) => graph,
            Err(err) => return err,
        };
        let documents = documents_from_graph(&graph);
        let index = match FullTextIndex::build(&documents) {
            Ok(index) => index,
            Err(source) => {
                return tool_error(
                    "search_graph",
                    format!("failed to build fulltext index: {source}"),
                )
            }
        };
        let hits = match index.search(query, limit) {
            Ok(hits) => hits,
            Err(source) => return tool_error("search_graph", format!("search failed: {source}")),
        };
        return json!({
            "ok": true,
            "mode": "bm25",
            "hits": hits.into_iter().map(|hit| json!({
                "docId": hit.doc_id,
                "score": hit.score,
            })).collect::<Vec<_>>(),
        });
    }

    // Full spec-driven path: regex/semantic modes plus a query-driven
    // BM25 call when the caller supplied any richer spec field alongside
    // `query` -- see crate::search::search_graph module docs for the
    // mode-interaction contract (BM25 short-circuits when it has usable
    // tokens; regex/label/degree filters and semantic_query combine
    // otherwise).
    let mut label = None;
    if let Some(raw) = args.get("label").and_then(Value::as_str) {
        label = match parse_node_label(raw) {
            Some(label) => Some(label),
            None => return tool_error("search_graph", format!("unknown label {raw:?}")),
        };
    }
    let semantic_query: Option<Vec<String>> = args
        .get("semanticQuery")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        });

    let spec = SearchGraphSpec {
        query: args.get("query").and_then(Value::as_str).map(str::to_owned),
        name_pattern: args
            .get("namePattern")
            .and_then(Value::as_str)
            .map(str::to_owned),
        qn_pattern: args
            .get("qnPattern")
            .and_then(Value::as_str)
            .map(str::to_owned),
        label,
        file_pattern: args
            .get("filePattern")
            .and_then(Value::as_str)
            .map(str::to_owned),
        relationship: args
            .get("relationship")
            .and_then(Value::as_str)
            .map(str::to_owned),
        min_degree: args.get("minDegree").and_then(Value::as_i64),
        max_degree: args.get("maxDegree").and_then(Value::as_i64),
        exclude_entry_points: args
            .get("excludeEntryPoints")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        include_connected: args
            .get("includeConnected")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        semantic_query,
        limit: args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v as usize),
        offset: args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize,
        label_affects_bm25: false,
        label_affects_semantic: false,
    };

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };

    let embedder = crate::embed::HashingEmbedder::new();
    let entries: Vec<(String, Vec<f32>)> = Vec::new();
    let vector_index = crate::vector::VectorIndex::build(&entries, embedder.model_info());
    let semantic: Option<(&dyn Embedder, &crate::vector::VectorIndex)> = spec
        .semantic_query
        .is_some()
        .then_some((&embedder as &dyn Embedder, &vector_index));
    let result = search_graph_with_semantic(&graph, &spec, semantic);

    match result {
        Ok(result) => json!({
            "ok": true,
            "mode": match result.search_mode {
                crate::search::search_graph::SearchMode::Bm25 => "bm25",
                crate::search::search_graph::SearchMode::Regex => "regex",
            },
            "results": result.results.iter().map(search_graph_hit_json).collect::<Vec<_>>(),
            "semanticResults": result.semantic_results.iter().map(search_graph_hit_json).collect::<Vec<_>>(),
            "total": result.total,
            "hasMore": result.has_more,
            "connectedNames": result.connected_names,
        }),
        Err(source) => tool_error("search_graph", format!("{source}")),
    }
}

// ---------------------------------------------------------------------
// query_graph -> analysis::query (D-05 read-only Cypher-subset DSL)
// ---------------------------------------------------------------------

fn handle_query_graph(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let query = match require_str(args, "query") {
        Ok(value) => value,
        Err(err) => return err,
    };

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let parsed = match graph_query::parse(query) {
        Ok(parsed) => parsed,
        Err(source) => return tool_error("query_graph", format!("parse error: {source}")),
    };
    let adjacency = CodeAdjacency::build(&graph);
    let rows = match graph_query::execute(&parsed, &adjacency, &graph) {
        Ok(rows) => rows,
        Err(source) => return tool_error("query_graph", format!("execution error: {source}")),
    };
    json!({
        "ok": true,
        "rowCount": rows.len(),
        "rows": rows.into_iter().map(|row| {
            let map: BTreeMap<String, String> = row.into_iter().collect();
            json!(map)
        }).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------
// trace_path -> analysis::trace::{trace_calls, trace_data_flow,
// trace_cross_service} (all three modes landed)
// ---------------------------------------------------------------------

fn parse_trace_direction(raw: &str) -> TraceDirection {
    match raw {
        "in" => TraceDirection::In,
        "both" => TraceDirection::Both,
        _ => TraceDirection::Out,
    }
}

fn call_hop_json(hop: &crate::analysis::PathHop) -> Value {
    json!({ "nodeId": hop.node_id, "via": format!("{:?}", hop.via) })
}

fn handle_trace_path(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let start_node_id = match require_str(args, "startNodeId") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("calls");
    let direction = parse_trace_direction(
        args.get("direction")
            .and_then(Value::as_str)
            .unwrap_or("out"),
    );
    let depth = args
        .get("depth")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .max(1) as usize;
    let include_tests = args
        .get("includeTests")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let adjacency = CodeAdjacency::build(&graph);

    match mode {
        "calls" => {
            let params = TraceCallsParams {
                direction,
                depth,
                include_tests,
                edge_types: None,
                risk_labels: false,
            };
            let report = trace_calls(&adjacency, &graph, start_node_id, &params);
            json!({
                "ok": true,
                "mode": "calls",
                "pathCount": report.paths.len(),
                "paths": report.paths.iter().map(|path| json!({
                    "startNodeId": path.start_node_id,
                    "hops": path.hops.iter().map(call_hop_json).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        }
        "data_flow" => {
            let params = TraceCallsParams {
                direction,
                depth,
                include_tests,
                edge_types: None,
                risk_labels: false,
            };
            let report = trace_data_flow(&adjacency, &graph, start_node_id, &params);
            json!({
                "ok": true,
                "mode": "data_flow",
                "approximation": format!("{:?}", report.approximation),
                "pathCount": report.paths.len(),
                "paths": report.paths.iter().map(|path| json!({
                    "startNodeId": path.start_node_id,
                    "hops": path.hops.iter().map(|hop| json!({
                        "hop": call_hop_json(&hop.hop),
                        "paramLink": hop.param_link.as_ref().map(|link| json!({
                            "argumentExpr": link.argument_expr,
                            "parameterName": link.parameter_name,
                        })),
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        }
        "cross_service" => {
            let report = trace_cross_service(
                &adjacency,
                &graph,
                start_node_id,
                TraceCrossServiceParams {
                    direction,
                    depth,
                    include_tests,
                },
            );
            json!({
                "ok": true,
                "mode": "cross_service",
                "pathCount": report.paths.len(),
                "paths": report.paths.iter().map(|path| json!({
                    "mediator": {
                        "method": path.mediator.method,
                        "path": path.mediator.path,
                        "producerNodeId": path.mediator.producer_node_id,
                    },
                    "consumerNodeId": path.consumer_node_id,
                    "hops": path.hops.iter().map(call_hop_json).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        }
        other => tool_error(
            "trace_path",
            format!("unknown mode {other:?}. Valid: calls, data_flow, cross_service."),
        ),
    }
}

// ---------------------------------------------------------------------
// get_code_snippet -> snippet::get_code_snippet
// ---------------------------------------------------------------------

fn handle_get_code_snippet(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let qualified_name = match require_str(args, "qualifiedName") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let include_neighbors = args
        .get("includeNeighbors")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let root = PathBuf::from(repo_path);
    match snippet::get_code_snippet(&graph, &root, qualified_name, include_neighbors) {
        Ok(snip) => json!({
            "ok": true,
            "qualifiedName": snip.qualified_name,
            "relPath": snip.rel_path,
            "startLine": snip.start_line,
            "endLine": snip.end_line,
            "source": String::from_utf8_lossy(&snip.bytes),
            "sha256": snip.sha256,
            "matchMethod": snip.match_method,
            "callers": snip.callers,
            "callees": snip.callees,
            "callerNames": snip.caller_names,
            "calleeNames": snip.callee_names,
            "neighbors": snip.neighbors.into_iter().map(|n| json!({
                "qualifiedName": n.qualified_name,
                "relPath": n.rel_path,
                "startLine": n.start_line,
                "endLine": n.end_line,
                "source": String::from_utf8_lossy(&n.bytes),
                "sha256": n.sha256,
            })).collect::<Vec<_>>(),
        }),
        Err(source) => tool_error("get_code_snippet", format!("{source}")),
    }
}

// ---------------------------------------------------------------------
// get_graph_schema -> graph_schema::get_graph_schema
// ---------------------------------------------------------------------

fn handle_get_graph_schema(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let schema = graph_schema::get_graph_schema(&graph);
    json!({
        "ok": true,
        "nodeLabels": schema.labels.iter().map(|l| json!({ "label": l.label, "count": l.count })).collect::<Vec<_>>(),
        "edgeTypes": schema.edge_types.iter().map(|e| json!({ "type": e.edge_type, "count": e.count })).collect::<Vec<_>>(),
        "totalNodes": schema.total_nodes(),
        "totalEdges": schema.total_edges(),
    })
}

// ---------------------------------------------------------------------
// get_architecture -> architecture::build_report (full aspect set)
// ---------------------------------------------------------------------

fn parse_aspect(raw: &str) -> Option<Aspect> {
    match raw {
        "all" => Some(Aspect::All),
        "overview" => Some(Aspect::Overview),
        "structure" => Some(Aspect::Structure),
        "dependencies" => Some(Aspect::Dependencies),
        "routes" => Some(Aspect::Routes),
        "languages" => Some(Aspect::Languages),
        "packages" => Some(Aspect::Packages),
        "entry_points" => Some(Aspect::EntryPoints),
        "hotspots" => Some(Aspect::Hotspots),
        "boundaries" => Some(Aspect::Boundaries),
        "layers" => Some(Aspect::Layers),
        "file_tree" => Some(Aspect::FileTree),
        "clusters" => Some(Aspect::Clusters),
        _ => None,
    }
}

fn handle_get_architecture(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let hotspot_limit = args
        .get("hotspotLimit")
        .and_then(Value::as_u64)
        .unwrap_or(10)
        .max(1) as usize;
    let max_iterations = args
        .get("maxIterations")
        .and_then(Value::as_u64)
        .unwrap_or(100)
        .max(1) as usize;
    let path_prefix = args.get("path").and_then(Value::as_str);

    let mut aspects: Vec<Aspect> = Vec::new();
    if let Some(raw_aspects) = args.get("aspects").and_then(Value::as_array) {
        for raw in raw_aspects {
            let Some(raw) = raw.as_str() else {
                return tool_error("get_architecture", "aspects entries must be strings");
            };
            match parse_aspect(raw) {
                Some(aspect) => aspects.push(aspect),
                None => {
                    return tool_error(
                        "get_architecture",
                        format!("unknown aspect {raw:?}. Valid: all, overview, structure, dependencies, routes, languages, packages, entry_points, hotspots, boundaries, layers, file_tree, clusters."),
                    )
                }
            }
        }
    }
    if aspects.is_empty() {
        aspects.push(Aspect::Overview);
    }

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let report =
        architecture::build_report(&graph, &aspects, path_prefix, hotspot_limit, max_iterations);
    json!({
        "ok": true,
        "structure": report.structure.map(|sections| sections.into_iter().map(|s| json!({
            "name": s.name,
            "fileCount": s.file_count,
            "symbolCount": s.symbol_count,
            "relPaths": s.rel_paths,
        })).collect::<Vec<_>>()),
        "dependencies": report.dependencies.map(|edges| edges.into_iter().map(|e| json!({
            "from": e.from, "to": e.to, "count": e.count,
        })).collect::<Vec<_>>()),
        "routes": report.routes.map(|routes| routes.into_iter().map(|r| json!({
            "method": r.method, "path": r.path, "declaredIn": r.declared_in, "line": r.line,
        })).collect::<Vec<_>>()),
        "languages": report.languages.map(|langs| langs.into_iter().map(|(lang, count)| json!({
            "language": lang, "fileCount": count,
        })).collect::<Vec<_>>()),
        "packages": report.packages.map(|packages| packages.into_iter().map(|p| json!({
            "dir": p.dir,
            "manifestRelPath": p.manifest_rel_path,
            "memberFileCount": p.member_file_count,
            "memberRelPaths": p.member_rel_paths,
            "fanIn": p.fan_in,
            "fanOut": p.fan_out,
        })).collect::<Vec<_>>()),
        "entryPoints": report.entry_points.map(|entries| entries.into_iter().map(|e| json!({
            "relPath": e.rel_path, "kind": format!("{:?}", e.kind),
        })).collect::<Vec<_>>()),
        "hotspotEntries": report.hotspot_entries.map(|entries| entries.into_iter().map(|h| json!({
            "name": h.name, "nodeId": h.node_id, "fanIn": h.fan_in,
        })).collect::<Vec<_>>()),
        "boundaries": report.boundaries.map(|boundaries| boundaries.into_iter().map(|b| json!({
            "from": b.from, "to": b.to, "callCount": b.call_count,
        })).collect::<Vec<_>>()),
        "layerClassification": report.layer_classification.map(|layers| layers.into_iter().map(|l| json!({
            "name": l.name, "layer": format!("{:?}", l.layer), "reason": l.reason,
        })).collect::<Vec<_>>()),
        "fileTree": report.file_tree.map(|tree| file_tree_to_json(&tree.root)),
        "clusterCohesion": report.cluster_cohesion.map(|clusters| clusters.into_iter().map(|c| json!({
            "clusterId": c.cluster_id, "memberCount": c.member_count, "cohesion": c.cohesion,
        })).collect::<Vec<_>>()),
        "overview": report.overview.map(|overview| json!({
            "totalFiles": overview.total_files,
            "totalSymbols": overview.total_symbols,
            "languageCounts": overview.language_counts,
        })),
    })
}

fn file_tree_to_json(node: &architecture::FileTreeNode) -> Value {
    json!({
        "dir": node.dir,
        "directFileCount": node.direct_file_count,
        "directSymbolCount": node.direct_symbol_count,
        "totalFileCount": node.total_file_count,
        "totalSymbolCount": node.total_symbol_count,
        "children": node.children.iter().map(file_tree_to_json).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------
// search_code -> code_search::search_code
// ---------------------------------------------------------------------

fn parse_search_mode(raw: &str) -> SearchMode {
    match raw {
        "full" => SearchMode::Full,
        "files" => SearchMode::Files,
        _ => SearchMode::Compact,
    }
}

fn handle_search_code(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let pattern = match require_str(args, "query") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let mode = parse_search_mode(
        args.get("mode")
            .and_then(Value::as_str)
            .unwrap_or("compact"),
    );
    let context_lines = args.get("context").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let root = PathBuf::from(repo_path);
    let query = SearchQuery {
        pattern,
        mode,
        context_lines,
        limit,
    };
    match code_search::search_code(&graph, &root, &query) {
        Ok(outcome) => json!({
            "ok": true,
            "hits": outcome.hits.into_iter().map(|hit| json!({
                "relPath": hit.rel_path,
                "line": hit.line,
                "text": hit.text,
                "containingSymbol": hit.containing_symbol,
                "contextBefore": hit.context_before,
                "contextAfter": hit.context_after,
            })).collect::<Vec<_>>(),
            "files": outcome.files,
            "totalMatches": outcome.total_matches,
            "unreadableFiles": outcome.unreadable_files.into_iter().map(|f| json!({
                "relPath": f.rel_path, "reason": f.reason,
            })).collect::<Vec<_>>(),
        }),
        Err(source) => tool_error("search_code", format!("{source}")),
    }
}

// ---------------------------------------------------------------------
// list_projects / delete_project / index_status -> crate::projects
// ---------------------------------------------------------------------

fn handle_list_projects(args: &Value) -> Value {
    let stores_dir = match require_str(args, "storesDir") {
        Ok(value) => value,
        Err(err) => return err,
    };
    match projects::list_projects(Path::new(stores_dir)) {
        Ok(list) => json!({
            "ok": true,
            "projects": list.into_iter().map(|p| json!({
                "projectId": p.project_id,
                "repoRoot": p.repo_root,
                "initializedAt": p.initialized_at,
            })).collect::<Vec<_>>(),
        }),
        Err(source) => tool_error("list_projects", format!("{source}")),
    }
}

fn handle_delete_project(args: &Value) -> Value {
    let stores_dir = match require_str(args, "storesDir") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let project_id = match require_str(args, "projectId") {
        Ok(value) => value,
        Err(err) => return err,
    };
    match projects::delete_project(Path::new(stores_dir), project_id) {
        Ok(()) => json!({ "ok": true, "projectId": project_id, "status": "deleted" }),
        Err(source) => tool_error("delete_project", format!("{source}")),
    }
}

fn handle_index_status(args: &Value) -> Value {
    let stores_dir = match require_str(args, "storesDir") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let project_id = match require_str(args, "projectId") {
        Ok(value) => value,
        Err(err) => return err,
    };
    match projects::index_status(Path::new(stores_dir), project_id) {
        Ok(summary) => json!({
            "ok": true,
            "projectId": summary.project_id,
            "nodes": summary.nodes,
            "edges": summary.edges,
            "status": format!("{:?}", summary.status),
            "logs": summary.logs.into_iter().map(|log| json!({
                "logName": log.log_name,
                "logLength": log.log_length,
                "state": format!("{:?}", log.state),
            })).collect::<Vec<_>>(),
        }),
        Err(source) => tool_error("index_status", format!("{source}")),
    }
}

// ---------------------------------------------------------------------
// detect_changes -> impact::analyze_diff_impact
// ---------------------------------------------------------------------

fn handle_detect_changes(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let Some(changed_paths) = args.get("changedPaths").and_then(Value::as_array) else {
        return tool_error("detect_changes", "missing required field \"changedPaths\"");
    };
    let changed_paths: Vec<String> = changed_paths
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let max_depth = args
        .get("maxDepth")
        .and_then(Value::as_u64)
        .unwrap_or(3)
        .max(1) as usize;

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let report = impact::analyze_diff_impact(&graph, &changed_paths, max_depth);
    json!({
        "ok": true,
        "changedPaths": report.changed_paths,
        "totalAffectedNodeIds": report.total_affected_node_ids,
        "impacted": report.impacted.into_iter().map(|file| json!({
            "relPath": file.rel_path,
            "affectedNodeIds": file.affected_node_ids,
            "risk": format!("{:?}", file.risk),
        })).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------
// manage_adr -> adr::AdrStore (stateless round-trip: caller owns the
// `adrs` list across calls, since there is no landed persistence layer)
// ---------------------------------------------------------------------

fn adr_store_from_json(adrs: &Value) -> AdrStore {
    let mut store = AdrStore::new();
    let Some(items) = adrs.as_array() else {
        return store;
    };
    for item in items {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let title = item.get("title").and_then(Value::as_str).unwrap_or("");
        let mut record = AdrRecord::new(id, title);
        if let Some(sections) = item.get("sections").and_then(Value::as_object) {
            for (name, body) in sections {
                if let Some(body) = body.as_str() {
                    record = record.with_section(name.clone(), body.to_owned());
                }
            }
        }
        if let Some(linked) = item.get("linkedNodeIds").and_then(Value::as_array) {
            for node_id in linked.iter().filter_map(Value::as_str) {
                record = record.with_linked_node(node_id);
            }
        }
        // `create` on an already-populated store only happens when
        // replaying the caller's own round-tripped list, so a duplicate id
        // here is expected on every call after the first -- ignore
        // AlreadyExists rather than surfacing it as a dispatch error.
        let _ = store.create(record);
    }
    store
}

fn adr_store_to_json(store: &AdrStore) -> Value {
    let mut records: Vec<&AdrRecord> = store.all().collect();
    records.sort_by(|a, b| a.id.cmp(&b.id));
    json!(records
        .into_iter()
        .map(|record| json!({
            "id": record.id,
            "title": record.title,
            "sections": record.sections,
            "linkedNodeIds": record.linked_node_ids,
        }))
        .collect::<Vec<_>>())
}

/// Baseline whole-document `manage_adr` (`refs/x06-baseline-tool-schemas.md`
/// §14): `mode` defaults to `"get"`; `"store"` is an undocumented alias for
/// `"update"`; `mode="update"`/`"store"` with no `content` silently degrades
/// to a `get`-shaped response (§14.1: `content` is "required for
/// mode=\"update\"/\"store\" only" but the baseline has no distinct
/// missing-content error path -- it just falls through as if `get` had been
/// requested); an empty/never-stored document returns `content:""` with
/// `status:"no_adr"` plus the baseline's exact hint text.
///
/// Deviation from baseline: since this lane has no persistence layer behind
/// `manage_adr` (recorded honestly in the module doc, matching every other
/// stateless-per-call tool here), the caller round-trips the current
/// document via the `document` argument the same way the section-based
/// extension API round-trips `adrs` -- there is no `project`-keyed database
/// held across calls in-process.
fn handle_manage_adr_document(args: &Value, project: &str) -> Value {
    let mode = args
        .get("mode")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("get");
    let document = args.get("document").and_then(Value::as_str).unwrap_or("");

    let mut store = AdrStore::new();
    if !document.is_empty() {
        store.update_document(project, document);
    }

    match mode {
        "update" | "store" => match args.get("content").and_then(Value::as_str) {
            Some(content) => {
                store.update_document(project, content);
                json!({ "ok": true, "status": "updated", "document": content })
            }
            // Baseline §14.1/§14.4: missing content on update/store has no
            // distinct error path -- it silently degrades to a get-shaped
            // response over whatever was already stored.
            None => adr_document_get_response(&store, project),
        },
        "sections" => {
            let headings = store.list_document_headings(project);
            json!({ "ok": true, "sections": headings })
        }
        _ => adr_document_get_response(&store, project),
    }
}

fn adr_document_get_response(store: &AdrStore, project: &str) -> Value {
    let AdrDocument { content, no_adr } = store.get_document(project);
    if no_adr {
        json!({
            "ok": true,
            "content": "",
            "status": "no_adr",
            "adr_hint": "No ADR yet. Create one with manage_adr(mode='update', content='## PURPOSE\\n...\\n\\n## STACK\\n...\\n\\n## ARCHITECTURE\\n...\\n\\n## PATTERNS\\n...\\n\\n## TRADEOFFS\\n...\\n\\n## PHILOSOPHY\\n...'). For guided creation: explore the codebase with get_architecture, then draft and store. Sections: PURPOSE, STACK, ARCHITECTURE, PATTERNS, TRADEOFFS, PHILOSOPHY."
        })
    } else {
        json!({ "ok": true, "content": content })
    }
}

fn handle_manage_adr(args: &Value) -> Value {
    // Baseline dispatch: a `project` argument selects the whole-document
    // API (`refs/x06-baseline-tool-schemas.md` §14). The pre-existing
    // section-based `operation` argument remains reachable as an extension
    // mode for callers that want richer per-section CRUD than the baseline
    // exposes.
    if let Some(project) = args.get("project").and_then(Value::as_str) {
        if !project.is_empty() {
            return handle_manage_adr_document(args, project);
        }
    }

    let operation = match require_str(args, "operation") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let adrs = args.get("adrs").cloned().unwrap_or_else(|| json!([]));
    let mut store = adr_store_from_json(&adrs);

    match operation {
        "create" => {
            let Ok(id) = require_str(args, "id") else {
                return tool_error("manage_adr", "create requires \"id\"");
            };
            let title = args.get("title").and_then(Value::as_str).unwrap_or("");
            if let Err(source) = store.create(AdrRecord::new(id, title)) {
                return tool_error("manage_adr", format!("{source}"));
            }
        }
        "get" => {
            let Ok(id) = require_str(args, "id") else {
                return tool_error("manage_adr", "get requires \"id\"");
            };
            return match store.get(id) {
                Ok(record) => json!({
                    "ok": true,
                    "adr": {
                        "id": record.id,
                        "title": record.title,
                        "sections": record.sections,
                        "linkedNodeIds": record.linked_node_ids,
                    },
                }),
                Err(source) => tool_error("manage_adr", format!("{source}")),
            };
        }
        "update_section" => {
            let (Ok(id), Ok(section), Ok(body)) = (
                require_str(args, "id"),
                require_str(args, "section"),
                require_str(args, "body"),
            ) else {
                return tool_error(
                    "manage_adr",
                    "update_section requires \"id\", \"section\", \"body\"",
                );
            };
            if let Err(source) = store.update_section(id, section, body) {
                return tool_error("manage_adr", format!("{source}"));
            }
        }
        "link_node" => {
            let (Ok(id), Ok(node_id)) = (require_str(args, "id"), require_str(args, "nodeId"))
            else {
                return tool_error("manage_adr", "link_node requires \"id\", \"nodeId\"");
            };
            if let Err(source) = store.link_node(id, node_id) {
                return tool_error("manage_adr", format!("{source}"));
            }
        }
        other => return tool_error("manage_adr", format!("unknown operation: {other}")),
    }

    json!({ "ok": true, "adrs": adr_store_to_json(&store) })
}

// ---------------------------------------------------------------------
// ingest_traces -> traces::TraceStore
// ---------------------------------------------------------------------

fn handle_ingest_traces(args: &Value) -> Value {
    let repo_path = match require_str(args, "repoPath") {
        Ok(value) => value,
        Err(err) => return err,
    };
    let Some(traces) = args.get("traces").and_then(Value::as_array) else {
        return tool_error("ingest_traces", "missing required field \"traces\"");
    };

    let mut records = Vec::with_capacity(traces.len());
    for entry in traces {
        let (Some(caller), Some(callee), Some(count)) = (
            entry.get("caller").and_then(Value::as_str),
            entry.get("callee").and_then(Value::as_str),
            entry.get("count").and_then(Value::as_u64),
        ) else {
            return tool_error(
                "ingest_traces",
                "each trace entry requires \"caller\", \"callee\", \"count\"",
            );
        };
        records.push(TraceRecord {
            caller: caller.to_owned(),
            callee: callee.to_owned(),
            count,
        });
    }

    let graph = match build_graph(repo_path) {
        Ok(graph) => graph,
        Err(err) => return err,
    };
    let mut store = TraceStore::new();
    store.ingest(&graph, &records);
    let edges = store.edges(&graph);
    json!({
        "ok": true,
        "ingestedCount": records.len(),
        "unresolvedCount": store.unresolved().len(),
        "unresolved": store.unresolved().iter().map(|u| json!({
            "caller": u.record.caller,
            "callee": u.record.callee,
            "count": u.record.count,
            "unresolvedCaller": u.unresolved_caller,
            "unresolvedCallee": u.unresolved_callee,
        })).collect::<Vec<_>>(),
        "edges": edges.iter().map(|edge| json!({
            "caller": edge.caller,
            "callee": edge.callee,
            "provenance": format!("{:?}", edge.provenance),
            "observedCount": edge.observed_count,
        })).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------
// MCP stdio server loop (reuses enforcer_mcp::transport for framing)
// ---------------------------------------------------------------------

/// Run the MCP stdio server loop against `input`/`output`, returning once
/// `input` reaches EOF. Generic over `Read`/`Write` (rather than
/// hardcoding real stdio handles) so tests can drive it against
/// in-memory buffers.
pub fn run_stdio_session(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let mut reader = FrameReader::new();
    let mut buf = [0_u8; 4096];
    loop {
        let read = input.read(&mut buf)?;
        if read == 0 {
            return Ok(());
        }
        for frame in reader.push(&buf[..read]) {
            handle_frame(&frame, output)?;
        }
    }
}

/// Handle one already-framed JSON-RPC message, writing the reply (if any)
/// in the same framing it arrived in. `pub(crate)` so [`crate::cli`] and
/// the test harness can drive it directly against in-memory buffers
/// without a real stdio handle.
pub(crate) fn handle_frame(frame: &Frame, out: &mut impl std::io::Write) -> std::io::Result<()> {
    let message: RpcMessage = match serde_json::from_str(&frame.body) {
        Ok(message) => message,
        Err(err) => {
            // Binding: refs/x06-baseline-tool-schemas.md -- only two
            // JSON-RPC error codes exist in the baseline; a parse failure
            // has no request id to echo, so the baseline hardcodes `0`
            // (never `null`) for this one case.
            let error = RpcError::new(
                json!(0),
                RpcError::PARSE_ERROR,
                format!("Parse error: {err}"),
            );
            return write_reply(out, &error, frame.framing);
        }
    };
    if message.is_notification() {
        // `notifications/cancelled` is the one notification the baseline
        // gives semantic meaning to (cancel an in-flight request by id);
        // every other notification (including this one, in this crate's
        // synchronous dispatch -- there is no in-flight pipeline to
        // cancel) is silently ignored, no response, matching the binding
        // spec exactly.
        return Ok(());
    }
    let Some(id) = message.id.clone() else {
        return Ok(());
    };
    let tool_name = tool_name_from_params(&message);
    let started = std::time::Instant::now();
    let outcome = handle_method(&message);
    let is_error = match &outcome {
        Ok(result) => result_is_error(result),
        Err(_) => true,
    };
    let record = crate::diagnostics::RequestRecord {
        protocol: "mcp",
        method: message.method,
        tool: tool_name,
        duration: started.elapsed(),
        is_error,
    };
    let diagnostics = crate::diagnostics::Diagnostics::from_env();
    crate::diagnostics::emit_to_stderr(&diagnostics, record.level(), &record);
    match outcome {
        Ok(result) => write_reply(out, &RpcResult::new(id, result), frame.framing),
        Err((code, msg)) => write_reply(out, &RpcError::new(id, code, msg), frame.framing),
    }
}

/// Extract the tool name from a `tools/call` message's params, for the
/// per-request diagnostic record; `None` for every other method.
fn tool_name_from_params(message: &RpcMessage) -> Option<String> {
    if message.method != "tools/call" {
        return None;
    }
    message
        .params
        .as_ref()?
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Whether a successful `handle_method` result represents a tool-level
/// (or `tools/list`-shape-mismatch) error, for the per-request diagnostic
/// record's `status` field. `initialize`/`ping`/`tools/list` results are
/// never errors at this layer (a JSON-RPC-level failure already routed
/// through the `Err` arm instead); a `tools/call` result is an error
/// exactly when its envelope's `isError` field is `true`.
fn result_is_error(result: &Value) -> bool {
    result.get("isError").and_then(Value::as_bool) == Some(true)
}

fn handle_method(message: &RpcMessage) -> Result<Value, (i64, String)> {
    let params = message.params.clone().unwrap_or(Value::Null);
    match message.method.as_str() {
        "initialize" => Ok(initialize_result(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(handle_tools_list(&params)),
        "tools/call" => Ok(handle_tools_call(&params)),
        // Binding spec: only -32700 (parse error) and -32601 (method not
        // found) exist as JSON-RPC-level errors; every tool-level failure
        // (bad params, unknown project, unknown tool NAME passed to
        // tools/call) is a normal result with isError:true, never a
        // JSON-RPC error -- see handle_tools_call/dispatch_tool/
        // wrap_envelope, which never return through this Err path.
        other => Err((
            RpcError::METHOD_NOT_FOUND,
            format!("Unknown method: {other}"),
        )),
    }
}

/// Server name advertised in `initialize`'s `serverInfo.name`. This is
/// `enforcer-memory`'s own MCP identity -- distinct from and unrelated to
/// arc-21's `enforcer_mcp::name::SERVER_NAME` (that crate's consolidated
/// scan/check/proof/coordination surface is a different MCP server
/// entirely; this crate is never a client or a delegate of it).
pub const SERVER_NAME: &str = "enforcer-memory";

/// Protocol versions this server understands, newest first (binding:
/// coordinator-verified baseline contract).
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Negotiate the `initialize` response's `protocolVersion`: echo the
/// client's requested version if this server supports it, otherwise fall
/// back to the newest version this server supports (binding spec).
fn negotiate_protocol_version(requested: Option<&str>) -> &'static str {
    if let Some(requested) = requested {
        if let Some(&matched) = SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .find(|&&version| version == requested)
        {
            return matched;
        }
    }
    SUPPORTED_PROTOCOL_VERSIONS[0]
}

fn initialize_result(params: &Value) -> Value {
    let protocol_version =
        negotiate_protocol_version(params.get("protocolVersion").and_then(Value::as_str));
    json!({
        "protocolVersion": protocol_version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
}

/// `tools/list` page size (binding: coordinator-verified baseline
/// contract -- 8 tools per page).
const TOOLS_LIST_PAGE_SIZE: usize = 8;

/// Paginate `tool_descriptors()` starting at `cursor` (a stringified
/// integer offset, matching the baseline's own cursor encoding).
/// `nextCursor` is present in the returned object ONLY when more pages
/// remain past this one -- omitted entirely on the last page, never an
/// empty string or null.
fn handle_tools_list(params: &Value) -> Value {
    let cursor: usize = params
        .get("cursor")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let all = tool_descriptors();
    let page: Vec<&ToolDescriptor> = all.iter().skip(cursor).take(TOOLS_LIST_PAGE_SIZE).collect();
    let mut result = json!({ "tools": page });
    let next_offset = cursor + page.len();
    if next_offset < all.len() {
        result["nextCursor"] = json!(next_offset.to_string());
    }
    result
}

fn handle_tools_call(params: &Value) -> Value {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let empty_args = Value::Object(serde_json::Map::new());
    let args = params.get("arguments").unwrap_or(&empty_args);
    call_tool(name, args)
}

fn write_reply(
    out: &mut impl std::io::Write,
    reply: &impl serde::Serialize,
    framing: Framing,
) -> std::io::Result<()> {
    let body = serde_json::to_string(reply).unwrap_or_else(|_| {
        "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32603,\"message\":\"encode failure\"}}"
            .to_owned()
    });
    out.write_all(&encode_frame(&body, framing))?;
    out.flush()
}
