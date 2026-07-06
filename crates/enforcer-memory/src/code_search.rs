//! X06.P1: `search_code` -- graph-augmented grep, matching the
//! codebase-memory-mcp parity baseline's `search_code` tool (scout
//! digest §1, row 8: "text match -> containing function -> rank by
//! structural importance; modes compact/full/files").
//!
//! # Pipeline
//!
//! 1. Regex (or plain-substring) match over every indexed file's raw
//!    content, read fresh from disk under `repo_root` (this module does
//!    not maintain its own persisted full-text index -- see
//!    [`crate::fulltext`] for that; this is graph-augmented *grep*, a
//!    line-oriented text scan, not a ranked BM25 query).
//! 2. Each matching line is enriched with its containing symbol: the
//!    nearest [`crate::code_graph::SymbolNode`] in the same file whose
//!    start line is `<=` the hit's line (the same "nearest preceding
//!    symbol" convention [`crate::snippet`] uses for extent, applied
//!    here for containment instead).
//! 3. Hits are ranked by structural importance using the same score
//!    formula the baseline uses (see "baseline parity" below) -- a
//!    higher-scoring symbol's hits sort first. Hits with no containing
//!    symbol (a match outside any indexed symbol, e.g. in a `TextOnly`
//!    file or between symbols) rank last, in file/line order.
//!
//! # Never a silent skip
//!
//! A file that cannot be read (permissions, race with deletion, etc.)
//! never disappears from the result silently -- it is reported in
//! [`SearchOutcome::unreadable_files`], and the search continues over
//! every other file.
//!
//! # Baseline parity: the ranking formula
//!
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §8.3 (ground-truth extraction of codebase-memory-mcp's C
//! `handle_search_code`) gives the exact score:
//!
//! ```text
//! score = in_degree
//!       + (label in {Function, Method} ? 10 : 0)
//!       + (label == Route ? 15 : 0)
//!       + (path contains vendored/|vendor/|node_modules/ ? -50 : 0)
//!       + (path contains test|spec|_test. ? -5 : 0)
//! ```
//!
//! [`inbound_call_degree`] supplies `in_degree` (the same by-name
//! [`crate::code_graph::CallEdge`] proxy [`crate::snippet`] uses, since
//! this crate's call edges are unresolved -- see that type's docs).
//! [`crate::code_graph::CodeNode::Function`] gets the baseline's
//! Function/Method boost (this crate has no separate Method label --
//! see [`crate::graph_schema`]'s label vocabulary); this crate's graph
//! has no node-level Route label (routes are edges, not nodes -- see
//! [`crate::code_graph::RouteEdge`]), so the Route boost has no
//! containing-symbol equivalent here and is omitted rather than
//! guessed. [`crate::code_graph::CodeNode::Test`] gets the test penalty
//! directly from the graph's own real test-symbol classification --
//! believed to be a *more* accurate signal than the baseline's
//! path-substring heuristic (`test|spec|_test.`), so this module keeps
//! path-substring matching only as an ADDITIONAL vendored-path penalty
//! (no node-kind equivalent exists for "vendored"), not as a
//! replacement for the real test-symbol check. The score is never
//! returned to the caller, matching the baseline (it exists purely to
//! order [`SearchOutcome::hits`]).
//!
//! The baseline's "smallest enclosing `[start_line, end_line]` span"
//! containment (§8.2) is not reproducible here because
//! [`crate::code_graph::SymbolNode`] stores no end line (see
//! [`crate::snippet`]'s module docs for the same gap) -- this module
//! uses the nearest-preceding-symbol convention instead, a deliberate,
//! documented divergence.

use std::fs;
use std::path::Path;

use regex::Regex;

use crate::code_graph::{CodeGraph, CodeNode, SymbolNode};

/// Output verbosity, matching the baseline's compact/full/files modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// One line per hit: file, line, containing symbol name, matched
    /// text -- no surrounding context.
    Compact,
    /// Every hit's full [`SearchHit`], including context lines.
    Full,
    /// Deduplicated file list only, no per-line hits.
    Files,
}

/// One matched line, enriched with its containing symbol (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub rel_path: String,
    /// 1-based line number the match occurred on.
    pub line: usize,
    /// The full matched line's text.
    pub text: String,
    /// The nearest enclosing symbol, if the match falls inside (at or
    /// after) an indexed symbol's start line in the same file.
    pub containing_symbol: Option<String>,
    /// Structural importance rank key, per the baseline's exact score
    /// formula (module docs, "baseline parity"): 0 when there is no
    /// containing symbol. Higher sorts first. Signed because the
    /// vendored-path/test penalties can drive it below zero.
    pub structural_rank: i64,
    /// Lines immediately before the match, oldest first. Populated only
    /// when the caller requested `context_lines > 0` (empty otherwise,
    /// including in [`SearchMode::Compact`]).
    pub context_before: Vec<String>,
    /// Lines immediately after the match. Same population rule as
    /// `context_before`.
    pub context_after: Vec<String>,
}

/// A file that matched the query but could not be fully searched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableFile {
    pub rel_path: String,
    pub reason: String,
}

/// The full result of one [`search_code`] call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchOutcome {
    /// Hits actually returned (after `limit` truncation), ranked by
    /// structural importance (see module docs).
    pub hits: Vec<SearchHit>,
    /// Deduplicated, sorted list of files containing at least one match
    /// -- always populated (not just in [`SearchMode::Files`]) so a
    /// [`SearchMode::Compact`]/[`SearchMode::Full`] caller can still see
    /// the file set without re-deriving it from `hits`.
    pub files: Vec<String>,
    /// Total number of matches found before `limit` truncation --
    /// lets a caller detect truncation (`total_matches > hits.len()`
    /// in non-[`SearchMode::Files`] modes).
    pub total_matches: usize,
    /// Files that matched the query's applicability (i.e. every indexed
    /// file this search attempted to scan) but could not be read.
    /// Never silently dropped from the outcome.
    pub unreadable_files: Vec<UnreadableFile>,
}

/// Errors from [`search_code`] itself (as opposed to a per-file read
/// failure, which is reported in [`SearchOutcome::unreadable_files`]
/// rather than failing the whole call).
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("invalid regex pattern {pattern:?}: {source}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: regex::Error,
    },
}

/// This module's `Result` alias.
pub type Result<T> = std::result::Result<T, SearchError>;

/// Grouped parameters for [`search_code`] (kept as one struct rather
/// than five positional arguments -- `clippy::too_many_arguments`, and
/// the same grouping convention [`crate::code_graph`]'s
/// `NewFileParams` uses for its own multi-field constructor).
#[derive(Debug, Clone, Copy)]
pub struct SearchQuery<'a> {
    /// A regular expression; a plain literal string is itself a valid
    /// regex and matches literally.
    pub pattern: &'a str,
    pub mode: SearchMode,
    /// How many lines of context surround each hit (0 = none).
    pub context_lines: usize,
    /// Caps [`SearchOutcome::hits`] (0 = unlimited) --
    /// [`SearchOutcome::total_matches`] always reflects the untruncated
    /// count regardless of this cap.
    pub limit: usize,
}

/// Run a graph-augmented grep per `query` over every file
/// [`CodeGraph::file_nodes`] knows about, reading source bytes fresh
/// from `repo_root`.
pub fn search_code(
    graph: &CodeGraph,
    repo_root: &Path,
    query: &SearchQuery<'_>,
) -> Result<SearchOutcome> {
    let SearchQuery {
        pattern,
        mode,
        context_lines,
        limit,
    } = *query;

    let regex = Regex::new(pattern).map_err(|source| SearchError::InvalidPattern {
        pattern: pattern.to_owned(),
        source,
    })?;

    let mut hits = Vec::new();
    let mut unreadable_files = Vec::new();
    let mut files_with_hits = std::collections::BTreeSet::new();

    for file in graph.file_nodes() {
        let path = repo_root.join(&file.rel_path);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(source) => {
                // A binary file (invalid UTF-8) is reported the same way
                // as any other unreadable file -- never a silent skip.
                unreadable_files.push(UnreadableFile {
                    rel_path: file.rel_path.clone(),
                    reason: source.to_string(),
                });
                continue;
            }
        };

        let lines: Vec<&str> = content.split('\n').collect();
        let symbols = symbols_in_file_sorted(graph, &file.id);
        let is_vendored = is_vendored_path(&file.rel_path);

        for (idx, line) in lines.iter().enumerate() {
            if !regex.is_match(line) {
                continue;
            }
            let line_no = idx + 1;
            files_with_hits.insert(file.rel_path.clone());

            let containing = containing_symbol(&symbols, line_no);
            let structural_rank = containing
                .map(|s| structural_score(graph, s, is_vendored))
                .unwrap_or(0);

            let (context_before, context_after) = if context_lines > 0 {
                (
                    context_slice(&lines, idx, context_lines, true),
                    context_slice(&lines, idx, context_lines, false),
                )
            } else {
                (Vec::new(), Vec::new())
            };

            hits.push(SearchHit {
                rel_path: file.rel_path.clone(),
                line: line_no,
                text: (*line).to_string(),
                containing_symbol: containing.map(|s| s.name.clone()),
                structural_rank,
                context_before,
                context_after,
            });
        }
    }

    let total_matches = hits.len();

    // Rank: higher structural_rank first; ties broken deterministically
    // by (rel_path, line) so output is reproducible across runs.
    hits.sort_by(|a, b| {
        b.structural_rank
            .cmp(&a.structural_rank)
            .then_with(|| a.rel_path.cmp(&b.rel_path))
            .then_with(|| a.line.cmp(&b.line))
    });

    if limit > 0 && hits.len() > limit {
        hits.truncate(limit);
    }

    let files: Vec<String> = files_with_hits.into_iter().collect();

    let hits = match mode {
        SearchMode::Files => Vec::new(),
        SearchMode::Compact | SearchMode::Full => hits,
    };

    Ok(SearchOutcome {
        hits,
        files,
        total_matches,
        unreadable_files,
    })
}

fn symbols_in_file_sorted<'a>(graph: &'a CodeGraph, file_id: &str) -> Vec<&'a SymbolNode> {
    let mut symbols: Vec<&SymbolNode> = graph
        .symbol_nodes()
        .filter(|s| s.file_id == file_id)
        .collect();
    symbols.sort_by_key(|s| s.line);
    symbols
}

/// The nearest symbol at or before `line_no` (1-based) in `symbols`
/// (already sorted ascending by line), i.e. the innermost-by-convention
/// enclosing symbol. `None` if the match is before every symbol in the
/// file (or the file has none).
fn containing_symbol<'a>(symbols: &[&'a SymbolNode], line_no: usize) -> Option<&'a SymbolNode> {
    symbols.iter().rfind(|s| s.line <= line_no).copied()
}

/// How many [`crate::code_graph::CallEdge`]s anywhere in the graph name
/// `symbol_name` as their callee -- the `in_degree` term of the
/// baseline's score formula (module docs, "baseline parity"). A name
/// match, not a resolved-target match: [`crate::code_graph::CallEdge::callee`]
/// is recorded as-written in source (unresolved -- see that type's own
/// docs), so this is the same honesty-preserving approximation the rest
/// of this crate uses rather than pretending a resolution this slice
/// does not perform.
fn inbound_call_degree(graph: &CodeGraph, symbol_name: &str) -> i64 {
    graph
        .calls()
        .iter()
        .filter(|call| call.callee == symbol_name)
        .count() as i64
}

/// The baseline's exact per-hit score (module docs, "baseline parity"):
/// `in_degree + Function/Method boost + Route boost + vendored penalty +
/// test penalty`, adapted to this crate's node-kind vocabulary. `symbol`
/// is the containing symbol a hit resolved to; `is_vendored` is
/// precomputed once per file (path-substring check never changes across
/// a file's lines).
fn structural_score(graph: &CodeGraph, symbol: &SymbolNode, is_vendored: bool) -> i64 {
    const FUNCTION_BOOST: i64 = 10;
    const VENDORED_PENALTY: i64 = -50;
    const TEST_PENALTY: i64 = -5;

    let mut score = inbound_call_degree(graph, &symbol.name);

    if let Some(kind) = symbol_kind(graph, &symbol.id) {
        match kind {
            CodeNode::Function(_) => score += FUNCTION_BOOST,
            CodeNode::Test(_) => score += TEST_PENALTY,
            _ => {}
        }
    }
    if is_vendored {
        score += VENDORED_PENALTY;
    }
    score
}

/// The [`CodeNode`] wrapper for the symbol with this id, if any --
/// needed because node-kind information (Function/Type/Test) lives only
/// on the enum variant, not on [`SymbolNode`] itself.
fn symbol_kind<'a>(graph: &'a CodeGraph, symbol_id: &str) -> Option<&'a CodeNode> {
    graph.nodes().iter().find(|n| n.id() == symbol_id)
}

/// Whether `rel_path` looks vendored/third-party, matching the
/// baseline's exact substring set (module docs, "baseline parity"):
/// `vendored/`, `vendor/`, or `node_modules/`.
fn is_vendored_path(rel_path: &str) -> bool {
    rel_path.contains("vendored/")
        || rel_path.contains("vendor/")
        || rel_path.contains("node_modules/")
}

/// `count` lines of context immediately before (`before = true`) or
/// after (`before = false`) `idx` in `lines`, oldest-first, clamped at
/// the file's boundaries.
fn context_slice(lines: &[&str], idx: usize, count: usize, before: bool) -> Vec<String> {
    if before {
        let start = idx.saturating_sub(count);
        lines[start..idx].iter().map(|s| s.to_string()).collect()
    } else {
        let end = (idx + 1 + count).min(lines.len());
        lines[(idx + 1)..end]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
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

    /// Test-local shorthand for the common `SearchQuery` shape (a single
    /// pattern, given mode, no context lines, no limit) so individual
    /// tests don't repeat the struct literal.
    fn query(pattern: &str, mode: SearchMode) -> SearchQuery<'_> {
        SearchQuery {
            pattern,
            mode,
            context_lines: 0,
            limit: 0,
        }
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

    #[test]
    fn finds_matches_and_enriches_with_containing_symbol() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(
            &file_path,
            "fn helper() {\n    let needle = 1;\n}\nfn other() {\n    let needle = 2;\n}\n",
        )?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(&graph, dir.path(), &query("needle", SearchMode::Full))?;
        assert_eq!(outcome.total_matches, 2);
        assert_eq!(outcome.hits.len(), 2);
        assert!(outcome
            .hits
            .iter()
            .any(|h| h.containing_symbol.as_deref() == Some("helper")));
        assert!(outcome
            .hits
            .iter()
            .any(|h| h.containing_symbol.as_deref() == Some("other")));
        assert!(outcome.unreadable_files.is_empty());
        Ok(())
    }

    #[test]
    fn ranks_by_structural_importance_inbound_call_degree() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        // `popular` is called twice; `lonely` is never called. Both
        // contain the needle "MARK".
        fs::write(
            &file_path,
            "fn popular() {\n    // MARK\n}\nfn lonely() {\n    // MARK\n}\nfn caller_a() { popular(); }\nfn caller_b() { popular(); }\n",
        )?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(&graph, dir.path(), &query("MARK", SearchMode::Full))?;
        assert_eq!(outcome.hits.len(), 2);
        assert_eq!(
            outcome.hits[0].containing_symbol.as_deref(),
            Some("popular"),
            "the symbol called twice must rank before the never-called one"
        );
        assert!(outcome.hits[0].structural_rank > outcome.hits[1].structural_rank);
        Ok(())
    }

    #[test]
    fn modes_compact_full_files_shape_the_output_correctly() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "fn a() {\n    let x = 1;\n}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let compact = search_code(&graph, dir.path(), &query("let x", SearchMode::Compact))?;
        assert_eq!(compact.hits.len(), 1);
        assert_eq!(compact.files, vec!["lib.rs".to_string()]);

        let full = search_code(
            &graph,
            dir.path(),
            &SearchQuery {
                pattern: "let x",
                mode: SearchMode::Full,
                context_lines: 1,
                limit: 0,
            },
        )?;
        assert_eq!(full.hits.len(), 1);
        assert_eq!(full.hits[0].context_before, vec!["fn a() {".to_string()]);
        assert_eq!(full.hits[0].context_after, vec!["}".to_string()]);

        let files_mode = search_code(&graph, dir.path(), &query("let x", SearchMode::Files))?;
        assert!(
            files_mode.hits.is_empty(),
            "files mode returns no per-line hits"
        );
        assert_eq!(files_mode.files, vec!["lib.rs".to_string()]);
        assert_eq!(
            files_mode.total_matches, 1,
            "total_matches is populated even in files mode"
        );
        Ok(())
    }

    #[test]
    fn limit_truncates_hits_but_total_matches_reflects_the_untruncated_count() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "// needle\n// needle\n// needle\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(
            &graph,
            dir.path(),
            &SearchQuery {
                pattern: "needle",
                mode: SearchMode::Full,
                context_lines: 0,
                limit: 2,
            },
        )?;
        assert_eq!(outcome.hits.len(), 2);
        assert_eq!(outcome.total_matches, 3);
        Ok(())
    }

    #[test]
    fn unreadable_files_are_reported_never_silently_skipped() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "fn a() { /* needle */ }\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        // Delete the file after indexing so the graph still references
        // it but the read fails at search time.
        fs::remove_file(dir.path().join("lib.rs"))?;

        let outcome = search_code(&graph, dir.path(), &query("needle", SearchMode::Full))?;
        assert_eq!(outcome.hits.len(), 0);
        assert_eq!(outcome.unreadable_files.len(), 1);
        assert_eq!(outcome.unreadable_files[0].rel_path, "lib.rs");
        Ok(())
    }

    #[test]
    fn invalid_regex_pattern_is_a_typed_error() {
        let graph = CodeGraph::new();
        let outcome = search_code(
            &graph,
            Path::new("."),
            &query("(unterminated[", SearchMode::Full),
        );
        assert!(matches!(outcome, Err(SearchError::InvalidPattern { .. })));
    }

    #[test]
    fn regex_pattern_matches_not_just_literal_substrings() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "fn foo_1() {}\nfn foo_2() {}\nfn bar() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(&graph, dir.path(), &query(r"foo_\d", SearchMode::Full))?;
        assert_eq!(outcome.total_matches, 2);
        Ok(())
    }

    #[test]
    fn function_symbols_outrank_type_symbols_at_equal_call_degree() -> TestResult {
        // docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md
        // §8.3: label boost is +10 for Function/Method, +0 for anything
        // else -- neither symbol here is ever called, so the only
        // difference is the Function boost.
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "struct MarkerType;\nfn marker_fn() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(
            &graph,
            dir.path(),
            &query("Marker|marker", SearchMode::Full),
        )?;
        let fn_rank = outcome
            .hits
            .iter()
            .find(|h| h.containing_symbol.as_deref() == Some("marker_fn"))
            .map(|h| h.structural_rank)
            .ok_or("expected a marker_fn hit")?;
        let type_rank = outcome
            .hits
            .iter()
            .find(|h| h.containing_symbol.as_deref() == Some("MarkerType"))
            .map(|h| h.structural_rank)
            .ok_or("expected a MarkerType hit")?;
        assert!(
            fn_rank > type_rank,
            "fn_rank={fn_rank} type_rank={type_rank}"
        );
        Ok(())
    }

    #[test]
    fn test_symbols_are_penalized_below_zero() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "#[test]\nfn a_test() {\n    // marker\n}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(&graph, dir.path(), &query("marker", SearchMode::Full))?;
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(
            outcome.hits[0].structural_rank, -5,
            "an uncalled test symbol scores exactly the -5 test penalty"
        );
        Ok(())
    }

    #[test]
    fn vendored_paths_are_penalized() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let vendor_dir = dir.path().join("vendor");
        fs::create_dir_all(&vendor_dir)?;
        let file_path = vendor_dir.join("lib.rs");
        fs::write(&file_path, "fn vendored_fn() {\n    // marker\n}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let outcome = search_code(&graph, dir.path(), &query("marker", SearchMode::Full))?;
        assert_eq!(outcome.hits.len(), 1);
        // +10 Function boost, 0 in_degree, -50 vendored penalty = -40.
        assert_eq!(outcome.hits[0].structural_rank, -40);
        Ok(())
    }
}
