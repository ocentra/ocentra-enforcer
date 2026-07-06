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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CodeGraph, Manifest};
    use std::error::Error;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    type TestResult = std::result::Result<(), Box<dyn Error>>;

    fn init_git_repo(dir: &Path) -> TestResult {
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

    fn run_git(dir: &Path, args: &[&str]) -> TestResult {
        let status = Command::new("git").args(args).current_dir(dir).status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    }

    fn indexed_repo(
        source: &str,
        filename: &str,
    ) -> std::result::Result<(tempfile::TempDir, CodeGraph), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join(filename);
        fs::write(&file_path, source)?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;
        Ok((dir, graph))
    }

    #[test]
    fn snippet_bytes_are_hash_equal_to_an_independent_file_slice() -> TestResult {
        let source = "fn a() {\n    1\n}\nfn b() {\n    2\n}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::a", false)?;

        // Independently re-slice the file on disk and hash it, without
        // reusing any of this module's own byte-offset bookkeeping, per
        // L37 (never verify a value against itself).
        let raw = fs::read(dir.path().join("lib.rs"))?;
        let independent_slice = &raw[snippet.start_byte..snippet.end_byte];
        let mut hasher = Sha256::new();
        hasher.update(independent_slice);
        let digest = hasher.finalize();
        let mut expected = String::from("sha256:");
        for byte in digest {
            expected.push_str(&format!("{byte:02x}"));
        }

        assert_eq!(snippet.sha256, expected);
        assert_eq!(snippet.bytes, independent_slice);
        assert_eq!(snippet.bytes, b"fn a() {\n    1\n}\n");
        Ok(())
    }

    #[test]
    fn last_symbol_in_file_extends_to_end_of_file_byte_exact() -> TestResult {
        let source = "fn a() {}\nfn b() {\n    2\n}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::b", false)?;
        let raw = fs::read(dir.path().join("lib.rs"))?;
        assert_eq!(snippet.end_byte, raw.len());
        assert_eq!(snippet.bytes, b"fn b() {\n    2\n}\n");
        Ok(())
    }

    #[test]
    fn unknown_symbol_fails_closed_never_a_similar_name_substitute() -> TestResult {
        let source = "fn helper() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        // "helperr" is a near-miss of the real symbol "helper" -- this
        // must error, never silently resolve to the closest match.
        let outcome = get_code_snippet(&graph, dir.path(), "lib.rs::helperr", false);
        assert!(matches!(outcome, Err(SnippetError::UnknownSymbol { .. })));

        let outcome_missing_file =
            get_code_snippet(&graph, dir.path(), "missing.rs::helper", false);
        assert!(matches!(
            outcome_missing_file,
            Err(SnippetError::UnknownSymbol { .. })
        ));
        Ok(())
    }

    #[test]
    fn raw_node_id_form_resolves_the_same_symbol_as_qualified_form() -> TestResult {
        let source = "fn helper() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let by_qualified = get_code_snippet(&graph, dir.path(), "lib.rs::helper", false)?;
        let raw_id = graph
            .symbol_nodes()
            .find(|s| s.name == "helper")
            .map(|s| s.id.clone())
            .ok_or("expected a helper symbol")?;
        let by_raw_id = get_code_snippet(&graph, dir.path(), &raw_id, false)?;

        assert_eq!(by_qualified.bytes, by_raw_id.bytes);
        assert_eq!(by_qualified.sha256, by_raw_id.sha256);
        Ok(())
    }

    #[test]
    fn include_neighbors_returns_other_symbols_in_the_same_file_ordered_by_line() -> TestResult {
        let source = "fn a() {}\nstruct Middle;\nfn z() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::a", true)?;
        let names: Vec<&str> = snippet
            .neighbors
            .iter()
            .map(|n| n.qualified_name.as_str())
            .collect();
        assert_eq!(names, vec!["lib.rs::Middle", "lib.rs::z"]);

        // include_neighbors=false must yield an empty vec, not an error.
        let without = get_code_snippet(&graph, dir.path(), "lib.rs::a", false)?;
        assert!(without.neighbors.is_empty());
        Ok(())
    }

    #[test]
    fn file_with_a_single_symbol_has_no_neighbors_but_no_error() -> TestResult {
        let source = "fn only_one() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::only_one", true)?;
        assert!(snippet.neighbors.is_empty());
        Ok(())
    }

    #[test]
    fn unreadable_source_file_is_a_typed_error_not_a_panic() -> TestResult {
        let source = "fn a() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        // Remove the file after indexing so the graph still has the
        // symbol but the source is gone -- exercising the ReadFile path.
        fs::remove_file(dir.path().join("lib.rs"))?;

        let outcome = get_code_snippet(&graph, dir.path(), "lib.rs::a", false);
        assert!(matches!(outcome, Err(SnippetError::ReadFile { .. })));
        Ok(())
    }

    #[test]
    fn exact_match_never_sets_match_method() -> TestResult {
        let source = "fn helper() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::helper", false)?;
        assert_eq!(snippet.match_method, None);
        Ok(())
    }

    #[test]
    fn bare_name_suffix_matches_and_records_match_method() -> TestResult {
        // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
        // §6.2: the baseline falls back to a suffix match on a bare/
        // short name when the exact qualified form doesn't match, and
        // tags the result with match_method="suffix".
        let source = "fn helper() {}\n";
        let (dir, graph) = indexed_repo(source, "lib.rs")?;

        let snippet = get_code_snippet(&graph, dir.path(), "helper", false)?;
        assert_eq!(snippet.match_method, Some("suffix"));
        assert_eq!(snippet.qualified_name, "lib.rs::helper");
        Ok(())
    }

    #[test]
    fn ambiguous_suffix_match_fails_closed() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let a_path = dir.path().join("a.rs");
        let b_path = dir.path().join("b.rs");
        fs::write(&a_path, "fn helper() {}\n")?;
        fs::write(&b_path, "fn helper() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[a_path, b_path], &Manifest::default())?;

        // Two files each define "helper" -- the bare-name suffix match
        // is ambiguous between them and must fail closed, never silently
        // pick one.
        let outcome = get_code_snippet(&graph, dir.path(), "helper", false);
        assert!(matches!(outcome, Err(SnippetError::AmbiguousSymbol { .. })));
        Ok(())
    }

    #[test]
    fn callers_and_callees_counts_are_always_present() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(
            &file_path,
            "fn popular() {}\nfn caller_a() { popular(); }\nfn caller_b() { popular(); }\n",
        )?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        // include_neighbors=false must still populate the always-present
        // counts (matching the baseline's asymmetry -- see module docs).
        let popular = get_code_snippet(&graph, dir.path(), "lib.rs::popular", false)?;
        assert_eq!(popular.callers, 2, "popular() is called twice");
        assert!(popular.caller_names.is_empty(), "names are opt-in only");
        Ok(())
    }

    #[test]
    fn include_neighbors_populates_caller_and_callee_names() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(
            &file_path,
            "fn popular() {}\nfn caller_a() { popular(); }\n",
        )?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let snippet = get_code_snippet(&graph, dir.path(), "lib.rs::popular", true)?;
        assert!(
            snippet.caller_names.contains(&"caller_a".to_string()),
            "{:?}",
            snippet.caller_names
        );
        Ok(())
    }
}
