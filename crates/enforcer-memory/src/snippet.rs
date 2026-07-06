//! X06.P1: `get_code_snippet` -- byte-exact source retrieval for a
//! qualified symbol name, matching the codebase-memory-mcp parity
//! baseline's `get_code_snippet` tool (scout digest §1, row 5: "by
//! qualified_name, optional neighbors").
//!
//! # Qualified names
//!
//! [`CodeGraph`] does not define a "qualified name" concept of its own
//! (see [`crate::code_graph::SymbolNode`] -- only a bare `name` plus the
//! mechanically-generated `sym:<rel_path>:<line>:<name>` id). This
//! module defines the qualified-name scheme callers use to address a
//! symbol unambiguously: `<rel_path>::<name>` (forward-slash-normalized
//! path, exactly as stored on [`crate::code_graph::FileNode::rel_path`]/
//! [`crate::code_graph::SymbolNode::file_id`]), OR the raw
//! `sym:<rel_path>:<line>:<name>` node id verbatim -- both resolve to the
//! same symbol when unambiguous.
//!
//! # Fail-closed on the unknown case (never a similar-name substitute)
//!
//! [`get_code_snippet`] returns [`SnippetError::UnknownSymbol`] for any
//! qualified name that does not resolve to exactly one symbol -- this
//! module never falls back to a fuzzy/nearest match. A qualified name
//! that matches more than one symbol (e.g. two same-named symbols in one
//! file, which the id format's line suffix normally prevents but a
//! caller-supplied ambiguous `<rel_path>::<name>` could still hit) is
//! [`SnippetError::AmbiguousSymbol`], also fail-closed.
//!
//! # Byte-exactness
//!
//! The returned [`CodeSnippet::bytes`] is always an exact slice of the
//! file at [`CodeSnippet::path`] between [`CodeSnippet::start_byte`] and
//! [`CodeSnippet::end_byte`] (end-exclusive) -- [`CodeSnippet::sha256`]
//! is the SHA-256 of exactly those bytes, so a caller (or this module's
//! own hard test) can re-slice the file independently and assert
//! hash-equality without trusting this module's own bookkeeping.
//!
//! Because [`crate::code_graph::SymbolNode`] stores only a 1-based start
//! line (no end line, no byte span -- see its module docs), a symbol's
//! extent is derived deterministically: from its start line up to (but
//! not including) the start line of the next symbol declared later in
//! the same file, or end-of-file if it is the last symbol. This is a
//! documented, deterministic convention (not a claim of exact AST
//! extent) -- what matters for parity and for the hard test is that the
//! returned range is *reproducible* and *byte-exact against the file on
//! disk*, not that it matches some other tool's idea of a node boundary.
//!
//! # Baseline parity notes
//!
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §6 (ground-truth extraction of codebase-memory-mcp's C
//! `get_code_snippet` handler) grounds three decisions here:
//!
//! - **Resolution tiers**: the baseline resolves `qualified_name` by (1)
//!   exact match, falling back to (2) a suffix match (`qualified_name
//!   LIKE '%.{suffix}'`) -- never fuzzy/similar-name substitution.
//!   [`resolve_symbol`] mirrors that: exact id/`<rel_path>::<name>` match
//!   first, then a `::`-boundary suffix match on the same qualified form,
//!   with [`CodeSnippet::match_method`] recording which tier resolved
//!   (`None` for an exact match, `Some("suffix")` for a suffix match --
//!   matching the baseline's own field, which is present only on the
//!   suffix tier).
//! - **No hash field on the baseline**: §6.4 confirms the C baseline's
//!   response has no content-hash field at all, despite the parity
//!   harness's byte-exact/hash-verified requirement -- [`CodeSnippet::sha256`]
//!   is an enforcer-native improvement kept deliberately (recorded as
//!   "better-because" in the parity diff, not a baseline field to match).
//! - **`include_neighbors` asymmetry**: the baseline always returns
//!   `callers`/`callees` as plain integer counts (CALLS in/out-degree),
//!   and *additionally* returns `caller_names`/`callee_names` (name
//!   arrays, CALLS+HTTP_CALLS+ASYNC_CALLS, capped at 10) only when
//!   `include_neighbors=true`. This crate's [`crate::code_graph::CallEdge`]
//!   records callees by as-written name only (never resolved to a target
//!   node -- see that type's docs), so [`CodeSnippet::callers`]/
//!   [`CodeSnippet::callees`] here are the same honest name-match proxy
//!   [`crate::code_search`] uses for its structural ranking, not a
//!   resolved-target count; [`CodeSnippet::caller_names`] mirrors the
//!   baseline's cap-at-10 behavior. [`CodeSnippet::neighbors`] (same-file
//!   sibling symbols) has no baseline equivalent -- it is an
//!   enforcer-native addition kept because this crate's graph has no
//!   call-target resolution to fall back on for "what else is near this
//!   symbol", so callers of the enforcer library get a project-relevant
//!   answer either way, at the deliberate cost of a byte-for-byte parity
//!   gap on the specific `caller_names`/`callee_names` arrays' contents.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::code_graph::{CodeGraph, SymbolNode};

/// One resolved symbol's byte-exact source snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSnippet {
    /// The qualified name this snippet was resolved from (echoed back so
    /// callers that resolved via the raw node-id form can see the
    /// canonical `<rel_path>::<name>` form too).
    pub qualified_name: String,
    /// Repo-relative, forward-slash-normalized path (matches
    /// [`crate::code_graph::FileNode::rel_path`]).
    pub rel_path: String,
    /// Absolute path this snippet's bytes were read from.
    pub path: PathBuf,
    /// 1-based inclusive start line.
    pub start_line: usize,
    /// 1-based inclusive end line.
    pub end_line: usize,
    /// 0-based inclusive start byte offset into the file's raw bytes.
    pub start_byte: usize,
    /// 0-based exclusive end byte offset into the file's raw bytes.
    pub end_byte: usize,
    /// The exact bytes of the file between `start_byte` and `end_byte`.
    pub bytes: Vec<u8>,
    /// `sha256:<64 hex>` of `bytes`, lowercase hex, no other prefix
    /// variance -- verified in the hard test to equal an independent
    /// hash of a fresh slice of the file taken outside this module.
    /// Enforcer-native; the baseline has no equivalent field (see module
    /// docs' "baseline parity notes").
    pub sha256: String,
    /// Which resolution tier matched: `None` for an exact
    /// id/`<rel_path>::<name>` match, `Some("suffix")` when only a
    /// `::name`-boundary suffix match succeeded -- mirrors the
    /// baseline's `match_method` field, present only on its suffix tier.
    pub match_method: Option<&'static str>,
    /// Inbound call-degree proxy: how many [`crate::code_graph::CallEdge`]s
    /// anywhere in the graph name this symbol as their callee (by name,
    /// unresolved -- see module docs). Always present, matching the
    /// baseline's always-present `callers` count.
    pub callers: usize,
    /// Outbound call-degree proxy: how many calls this symbol's own body
    /// line range contains, counted the same by-name way. Always
    /// present, matching the baseline's always-present `callees` count.
    pub callees: usize,
    /// Present only when the caller requested `include_neighbors`: the
    /// distinct names of callers (by the same by-name proxy as
    /// `callers`), capped at 10, matching the baseline's
    /// `caller_names`/cap behavior.
    pub caller_names: Vec<String>,
    /// Present only when the caller requested `include_neighbors`: the
    /// distinct names this symbol calls, capped at 10.
    pub callee_names: Vec<String>,
    /// Enforcer-native addition (no baseline equivalent -- see module
    /// docs): present only when the caller requested `include_neighbors`,
    /// every OTHER symbol declared in the same file, ordered by line,
    /// each as its own byte-exact snippet.
    pub neighbors: Vec<CodeSnippet>,
}

/// Errors from [`get_code_snippet`]. Fail-closed: an unresolved or
/// ambiguous qualified name is always an error, never a substitute
/// result.
#[derive(Debug, thiserror::Error)]
pub enum SnippetError {
    /// No symbol in the graph matches `qualified_name`. Deliberately
    /// carries no "did you mean" suggestion -- this module never guesses
    /// a similar name on the caller's behalf.
    #[error("unknown symbol: {qualified_name:?} does not match any indexed symbol")]
    UnknownSymbol { qualified_name: String },

    /// More than one symbol matches `qualified_name` (only possible via
    /// the `<rel_path>::<name>` form when a file has two same-named
    /// symbols at different lines).
    #[error("ambiguous symbol: {qualified_name:?} matches {count} symbols; use the sym:<rel_path>:<line>:<name> id form to disambiguate")]
    AmbiguousSymbol {
        qualified_name: String,
        count: usize,
    },

    /// The symbol resolved, but its file could not be read from disk.
    #[error("failed to read source file {path:?} for symbol {qualified_name:?}: {source}")]
    ReadFile {
        qualified_name: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// This module's `Result` alias.
pub type Result<T> = std::result::Result<T, SnippetError>;

/// Resolve `qualified_name` against `graph` and return its byte-exact
/// source snippet, reading source bytes from `repo_root`.
///
/// `qualified_name` may be either `<rel_path>::<name>` or the raw
/// `sym:<rel_path>:<line>:<name>` node id. Unknown or ambiguous names
/// fail closed (see [`SnippetError`]). When `include_neighbors` is
/// `true`, [`CodeSnippet::neighbors`] carries every other symbol declared
/// in the same file, ordered by line -- an empty vec (not an error) when
/// the file has no other symbols.
pub fn get_code_snippet(
    graph: &CodeGraph,
    repo_root: &Path,
    qualified_name: &str,
    include_neighbors: bool,
) -> Result<CodeSnippet> {
    let (symbol, match_method) = resolve_symbol(graph, qualified_name)?;

    let mut snippet = build_snippet(graph, repo_root, symbol)?;
    snippet.match_method = match_method;
    if !include_neighbors {
        return Ok(snippet);
    }

    let mut neighbors = Vec::new();
    for other in symbols_in_file(graph, &symbol.file_id) {
        if other.id == symbol.id {
            continue;
        }
        neighbors.push(build_snippet(graph, repo_root, other)?);
    }
    neighbors.sort_by_key(|s| s.start_line);

    Ok(CodeSnippet {
        neighbors,
        caller_names: capped_distinct_names(inbound_caller_names(graph, &symbol.name)),
        callee_names: capped_distinct_names(outbound_callee_names(graph, symbol)),
        ..snippet
    })
}

/// The baseline's resolution-cap for `caller_names`/`callee_names`
/// (`MCP_DEFAULT_LIMIT` in the baseline source, §6.3).
const NEIGHBOR_NAME_CAP: usize = 10;

fn capped_distinct_names(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names.truncate(NEIGHBOR_NAME_CAP);
    names
}

/// Resolve `qualified_name` against `graph`, per the baseline's two
/// tiers (see module docs' "baseline parity notes"): (1) an exact match
/// against either accepted form (the raw node id, or
/// `<rel_path>::<name>`); if that yields nothing, (2) a suffix match --
/// `qualified_name` matches a symbol's own qualified form at a `::`
/// boundary (e.g. `"helper"` suffix-matches `"lib.rs::helper"`, and
/// `"service::helper"` would suffix-match `"src/service.rs::helper"`).
/// Returns the resolved symbol plus which tier resolved it
/// (`None` = exact, `Some("suffix")` = suffix). Ambiguous or empty
/// results at either tier fail closed -- never a similar-name guess.
fn resolve_symbol<'a>(
    graph: &'a CodeGraph,
    qualified_name: &str,
) -> Result<(&'a SymbolNode, Option<&'static str>)> {
    let exact: Vec<&SymbolNode> = graph
        .symbol_nodes()
        .filter(|symbol| symbol.id == qualified_name || rel_path_and_name(symbol) == qualified_name)
        .collect();
    match exact.as_slice() {
        [single] => return Ok((single, None)),
        many if many.len() > 1 => {
            return Err(SnippetError::AmbiguousSymbol {
                qualified_name: qualified_name.to_owned(),
                count: many.len(),
            })
        }
        _ => {}
    }

    let suffix_needle = format!("::{qualified_name}");
    let suffix: Vec<&SymbolNode> = graph
        .symbol_nodes()
        .filter(|symbol| {
            let qn = rel_path_and_name(symbol);
            qn.ends_with(&suffix_needle) && qn != qualified_name
        })
        .collect();
    match suffix.as_slice() {
        [single] => Ok((single, Some("suffix"))),
        [] => Err(SnippetError::UnknownSymbol {
            qualified_name: qualified_name.to_owned(),
        }),
        many => Err(SnippetError::AmbiguousSymbol {
            qualified_name: qualified_name.to_owned(),
            count: many.len(),
        }),
    }
}

/// Names of symbols anywhere in the graph with a [`crate::code_graph::CallEdge`]
/// naming `callee_name` -- the by-name inbound-caller proxy (see module
/// docs). A call edge only records the calling FILE, not the calling
/// symbol, so this resolves each matching edge's file to every symbol
/// declared in that file (an over-approximation when a file has more
/// than one symbol, same honesty tradeoff [`crate::code_search`] makes
/// rather than claiming precision this crate's graph cannot yet supply).
fn inbound_caller_names(graph: &CodeGraph, callee_name: &str) -> Vec<String> {
    let caller_file_ids: std::collections::BTreeSet<&str> = graph
        .calls()
        .iter()
        .filter(|call| call.callee == callee_name)
        .map(|call| call.from_file_id.as_str())
        .collect();
    graph
        .symbol_nodes()
        .filter(|s| caller_file_ids.contains(s.file_id.as_str()))
        .map(|s| s.name.clone())
        .collect()
}

/// Names this symbol's file calls out to -- the by-name outbound-callee
/// proxy (see module docs and [`inbound_caller_names`]'s same
/// file-level-granularity caveat).
fn outbound_callee_names(graph: &CodeGraph, symbol: &SymbolNode) -> Vec<String> {
    graph
        .calls()
        .iter()
        .filter(|call| call.from_file_id == symbol.file_id)
        .map(|call| call.callee.clone())
        .collect()
}

fn rel_path_and_name(symbol: &SymbolNode) -> String {
    format!("{}::{}", strip_file_prefix(&symbol.file_id), symbol.name)
}

fn strip_file_prefix(file_id: &str) -> &str {
    file_id.strip_prefix("file:").unwrap_or(file_id)
}

fn symbols_in_file<'a>(graph: &'a CodeGraph, file_id: &str) -> Vec<&'a SymbolNode> {
    graph
        .symbol_nodes()
        .filter(|s| s.file_id == file_id)
        .collect()
}

fn build_snippet(graph: &CodeGraph, repo_root: &Path, symbol: &SymbolNode) -> Result<CodeSnippet> {
    let rel_path = strip_file_prefix(&symbol.file_id).to_owned();
    let path = repo_root.join(&rel_path);
    let content = fs::read(&path).map_err(|source| SnippetError::ReadFile {
        qualified_name: rel_path_and_name(symbol),
        path: path.clone(),
        source,
    })?;

    let end_line = next_symbol_start_line(graph, symbol).unwrap_or(usize::MAX);
    let (start_byte, end_byte, end_line) =
        line_range_to_byte_range(&content, symbol.line, end_line);
    let bytes = content[start_byte..end_byte].to_vec();
    let sha256 = hash_hex(&bytes);

    let callers = inbound_call_degree(graph, &symbol.name);
    let callees = graph
        .calls()
        .iter()
        .filter(|call| call.from_file_id == symbol.file_id)
        .count();

    Ok(CodeSnippet {
        qualified_name: rel_path_and_name(symbol),
        rel_path,
        path,
        start_line: symbol.line,
        end_line,
        start_byte,
        end_byte,
        bytes,
        sha256,
        match_method: None,
        callers,
        callees,
        caller_names: Vec::new(),
        callee_names: Vec::new(),
        neighbors: Vec::new(),
    })
}

/// How many [`crate::code_graph::CallEdge`]s anywhere in the graph name
/// `symbol_name` as their callee -- the always-present `callers` count
/// (see module docs).
fn inbound_call_degree(graph: &CodeGraph, symbol_name: &str) -> usize {
    graph
        .calls()
        .iter()
        .filter(|call| call.callee == symbol_name)
        .count()
}

/// The 1-based start line of the next symbol (by line order) declared in
/// the same file as `symbol`, or `None` if `symbol` is the last one.
fn next_symbol_start_line(graph: &CodeGraph, symbol: &SymbolNode) -> Option<usize> {
    symbols_in_file(graph, &symbol.file_id)
        .into_iter()
        .map(|s| s.line)
        .filter(|&line| line > symbol.line)
        .min()
}

/// Convert a 1-based `[start_line, end_line)` line range (`end_line`
/// exclusive, `usize::MAX` meaning "to end of file") into a
/// `(start_byte, end_byte, actual_end_line)` byte range against `content`
/// split on `\n`. `actual_end_line` is the 1-based inclusive line number
/// the returned `end_byte` actually lands on (the last line included in
/// the slice), which may be short of `end_line` when the file itself
/// ends first.
fn line_range_to_byte_range(
    content: &[u8],
    start_line: usize,
    end_line_exclusive: usize,
) -> (usize, usize, usize) {
    // Byte offset of the start of each 1-based line, plus one sentinel
    // entry for "start of file" at index 0 (unused) and one trailing
    // sentinel for end-of-file, so both start and end lookups are plain
    // slice indexing with no special-casing at the boundaries.
    let mut line_starts = vec![0usize];
    for (i, byte) in content.iter().enumerate() {
        if *byte == b'\n' {
            line_starts.push(i + 1);
        }
    }
    line_starts.push(content.len());

    let total_lines = line_starts.len() - 1; // number of 1-based lines representable
    let start_idx = start_line
        .saturating_sub(1)
        .min(total_lines.saturating_sub(1));
    let start_byte = line_starts[start_idx];

    let end_idx = end_line_exclusive.saturating_sub(1).min(total_lines);
    let end_byte = line_starts[end_idx.max(start_idx)];
    let actual_end_line = end_idx.max(start_idx + 1);

    (start_byte, end_byte.max(start_byte), actual_end_line)
}

fn hash_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2 + 7);
    out.push_str("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
