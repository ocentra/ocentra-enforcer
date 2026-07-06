//! Parses QA-001..QA-250 into typed [`QaRow`]s from the two binding
//! docs at repo-relative
//! `docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_PROOF_GATE.md`
//! (QA-001..QA-100 canonical row text + proof expectation -- the BINDING
//! gate per `MEMORY_RETRIEVAL_QA_BENCHMARKS.md` §1: "Where this file and
//! the gate differ, the gate wins") and
//! `docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_BENCHMARKS.md`
//! (QA-101..QA-250, §2.5, which carries an explicit `Category` column).
//!
//! Docs are READ-ONLY (worker file claims: never edit the QA docs) --
//! this module only parses them at test time; it never writes to them.
//!
//! QA-001..QA-100 carry no category column in either source doc.
//! BENCHMARKS §2.5 gives the per-ID grouping-key classification as prose
//! bullets (not a machine-parseable table) alongside a checkable
//! arithmetic table verifying the classification sums to the §2 minimums.
//! Reproducing that exact classification as a literal, hand-transcribed
//! table here (see [`TRANCHE_1_CATEGORIES`]) is the honest choice: a
//! prose-bullet parser would be fragile heuristics pretending to be a
//! mechanical parse, and the doc is read-only so this table cannot drift
//! silently out of sync without a test noticing (see
//! `category_counts_match_the_docs_own_arithmetic_table` below).

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

/// One parsed QA benchmark row: identity + query + expectation, ready
/// for a [`crate::feature_parity::runners::RowRunner`] to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaRow {
    /// `QA-001`..`QA-250`, zero-padded exactly as printed in the docs.
    pub id: String,
    /// Grouping key from BENCHMARKS §2.5:
    /// `Symbol, CodeGraph, Architecture, Repository, GitHistory, Lessons,
    /// Experience, Retrieval, Reranking, TokenReduction, Learning,
    /// Performance, Federation, MCP, CLI`.
    pub category: String,
    /// The user-style query / task text.
    pub query: String,
    /// The required retrieval behavior / proof expectation text.
    pub expectation: String,
}

/// Numeric suffix of a `QA-###` id, for ordering/range assertions.
fn qa_number(id: &str) -> Option<u32> {
    id.strip_prefix("QA-")?.parse().ok()
}

#[derive(Debug)]
pub enum QaParseError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A row id appeared more than once within a single source doc.
    DuplicateInDoc { path: PathBuf, id: String },
    /// A markdown table row did not have the expected column count.
    MalformedRow { path: PathBuf, line: String },
}

impl fmt::Display for QaParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QaParseError::Io { path, source } => {
                write!(f, "{}: {source}", path.display())
            }
            QaParseError::DuplicateInDoc { path, id } => {
                write!(f, "{}: duplicate row id {id}", path.display())
            }
            QaParseError::MalformedRow { path, line } => {
                write!(f, "{}: malformed table row: {line}", path.display())
            }
        }
    }
}

impl std::error::Error for QaParseError {}

/// Split a markdown table row (`| a | b | c |`) into trimmed cells,
/// dropping the leading/trailing empty cells produced by the outer
/// pipes. Returns `None` for lines that are not table rows at all
/// (doesn't start with `|`) or are the `|---|---|` separator row.
fn split_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
    if inner.chars().all(|c| matches!(c, '-' | ':' | '|' | ' ')) {
        return None; // header separator row
    }
    Some(
        inner
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect(),
    )
}

/// Strip a single layer of backtick-code-span markers from a cell, e.g.
/// `` `TS-1.1` `` -> `TS-1.1`. QA row text frequently wraps identifiers
/// in backticks; row text itself is kept verbatim (backticks preserved)
/// since it is meant for humans reading the query, but ids are cleaned.
fn strip_backticks(cell: &str) -> String {
    cell.trim_matches('`').to_string()
}

/// Parse QA-001..QA-100 from `MEMORY_RETRIEVAL_QA_PROOF_GATE.md`'s
/// single table (`| ID | Query / task | Proof expectation |`). This is
/// the BINDING row text per BENCHMARKS §1.
fn parse_proof_gate(
    text: &str,
    path: &Path,
) -> Result<Vec<(String, String, String)>, QaParseError> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let Some(cells) = split_row(line) else {
            continue;
        };
        if cells.len() != 3 {
            continue;
        }
        let id = strip_backticks(&cells[0]);
        if !id.starts_with("QA-") {
            continue; // header row ("ID | Query / task | ...")
        }
        if !seen.insert(id.clone()) {
            return Err(QaParseError::DuplicateInDoc {
                path: path.to_path_buf(),
                id,
            });
        }
        rows.push((id, cells[1].clone(), cells[2].clone()));
    }
    Ok(rows)
}

/// Parse QA-101..QA-250 from `MEMORY_RETRIEVAL_QA_BENCHMARKS.md` §2.5's
/// table (`| ID | Category | User-style query | Required retrieval
/// behavior / proof expectation |`).
fn parse_benchmarks_tranche2(
    text: &str,
    path: &Path,
) -> Result<Vec<(String, String, String, String)>, QaParseError> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let Some(cells) = split_row(line) else {
            continue;
        };
        if cells.len() != 4 {
            continue;
        }
        let id = strip_backticks(&cells[0]);
        if !id.starts_with("QA-") {
            continue; // header row or the arithmetic-table rows above it
        }
        let Some(number) = qa_number(&id) else {
            return Err(QaParseError::MalformedRow {
                path: path.to_path_buf(),
                line: line.to_string(),
            });
        };
        if !(101..=250).contains(&number) {
            continue; // this 4-column shape only appears for tranche 2
        }
        if !seen.insert(id.clone()) {
            return Err(QaParseError::DuplicateInDoc {
                path: path.to_path_buf(),
                id,
            });
        }
        rows.push((id, cells[1].clone(), cells[2].clone(), cells[3].clone()));
    }
    Ok(rows)
}

/// QA-001..QA-100 grouping-key classification, hand-transcribed from
/// BENCHMARKS §2.5's explicit per-ID bullet list (verified against that
/// same section's arithmetic table by
/// `category_counts_match_the_docs_own_arithmetic_table`). Order matches
/// the doc's own bullet order; entries are `(category, &[ids])`.
const TRANCHE_1_CATEGORIES: &[(&str, &[u32])] = &[
    ("Symbol", &[1, 2, 13, 15, 16, 20, 41, 42, 43, 55, 56, 59]),
    ("CodeGraph", &[3, 5, 14, 26, 27, 28, 78, 79, 80]),
    (
        "Architecture",
        &[6, 7, 33, 34, 39, 57, 58, 62, 90, 91, 92, 93, 96],
    ),
    ("Repository", &[4, 9, 10, 18, 19, 37, 60, 61]),
    ("GitHistory", &[8, 46, 94, 95]),
    ("Lessons", &[12, 21, 22, 23, 48]),
    ("Experience", &[49, 50, 51, 70, 87, 88, 89]),
    (
        "Retrieval",
        &[
            11, 25, 29, 32, 38, 40, 52, 53, 54, 64, 66, 71, 72, 73, 74, 86, 100,
        ],
    ),
    ("Reranking", &[31, 65, 67]),
    ("TokenReduction", &[30, 69, 99]),
    ("Learning", &[68, 97, 98]),
    ("Performance", &[44, 45, 47, 75, 76, 77, 81, 82]),
    ("Federation", &[24, 63, 83, 84, 85]),
    ("MCP", &[17, 36]),
    ("CLI", &[35]),
];

fn tranche_1_category_for(number: u32) -> Option<&'static str> {
    TRANCHE_1_CATEGORIES
        .iter()
        .find(|(_, ids)| ids.contains(&number))
        .map(|(category, _)| *category)
}

/// Parse every QA-001..QA-250 row from the two binding docs, in id
/// order. `repo_root` is the workspace root (the directory containing
/// `docs/`) -- callers in `crates/enforcer-memory/tests/**` derive it
/// via `CARGO_MANIFEST_DIR/../..`.
pub fn parse_all(repo_root: &Path) -> Result<Vec<QaRow>, QaParseError> {
    let proof_gate_path =
        repo_root.join("docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_PROOF_GATE.md");
    let benchmarks_path =
        repo_root.join("docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_QA_BENCHMARKS.md");

    let proof_gate_text =
        std::fs::read_to_string(&proof_gate_path).map_err(|source| QaParseError::Io {
            path: proof_gate_path.clone(),
            source,
        })?;
    let benchmarks_text =
        std::fs::read_to_string(&benchmarks_path).map_err(|source| QaParseError::Io {
            path: benchmarks_path.clone(),
            source,
        })?;

    let tranche_1 = parse_proof_gate(&proof_gate_text, &proof_gate_path)?;
    let tranche_2 = parse_benchmarks_tranche2(&benchmarks_text, &benchmarks_path)?;

    let mut rows = Vec::with_capacity(tranche_1.len() + tranche_2.len());
    for (id, query, expectation) in tranche_1 {
        let Some(number) = qa_number(&id) else {
            return Err(QaParseError::MalformedRow {
                path: proof_gate_path,
                line: id,
            });
        };
        let category = tranche_1_category_for(number)
            .unwrap_or("Uncategorized")
            .to_string();
        rows.push(QaRow {
            id,
            category,
            query,
            expectation,
        });
    }
    for (id, category, query, expectation) in tranche_2 {
        rows.push(QaRow {
            id,
            category,
            query,
            expectation,
        });
    }

    rows.sort_by_key(|row| qa_number(&row.id).unwrap_or(u32::MAX));
    Ok(rows)
}

/// Repo root as seen from this test binary: `enforcer-memory`'s own
/// Cargo manifest dir is `crates/enforcer-memory`, two levels below the
/// workspace root.
pub fn workspace_root() -> PathBuf {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    strip_extended_length_prefix(&canonical)
}

/// Strip the Windows extended-length path prefix (`\\?\`, and the UNC
/// variant `\\?\UNC\`) that [`Path::canonicalize`] adds on Windows.
/// Downstream consumers of this path -- notably
/// [`enforcer_memory::git::GitMetadata::open`] via
/// [`super::runners::GitHistoryRunner`] -- resolve real on-disk repo
/// paths and file paths built from this root; a `\\?\`-prefixed root
/// is otherwise a no-op on non-git-touching code but breaks
/// `git2::Repository::discover`. Kept a no-op on non-Windows, where
/// `canonicalize` never emits this prefix.
fn strip_extended_length_prefix(path: &Path) -> PathBuf {
    const UNC_PREFIX: &str = r"\\?\UNC\";
    const VERBATIM_PREFIX: &str = r"\\?\";

    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(rest) = text.strip_prefix(UNC_PREFIX) {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(VERBATIM_PREFIX) {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn parses_exactly_250_unique_rows() -> TestResult {
        let rows = parse_all(&workspace_root())?;
        assert_eq!(rows.len(), 250, "expected exactly 250 QA rows");

        let ids: HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids.len(), 250, "expected 250 unique QA row ids");

        for n in 1..=250u32 {
            let expected = format!("QA-{n:03}");
            assert!(
                ids.contains(expected.as_str()),
                "missing expected row {expected}"
            );
        }
        Ok(())
    }

    #[test]
    fn every_row_has_nonempty_query_and_category() -> TestResult {
        let rows = parse_all(&workspace_root())?;
        for row in &rows {
            assert!(!row.query.trim().is_empty(), "{}: empty query", row.id);
            assert!(
                !row.category.trim().is_empty(),
                "{}: empty category",
                row.id
            );
            assert_ne!(
                row.category, "Uncategorized",
                "{}: row fell through classification",
                row.id
            );
        }
        Ok(())
    }

    /// Recount BENCHMARKS §2.5's own arithmetic table mechanically from
    /// [`TRANCHE_1_CATEGORIES`] + the parsed tranche-2 category column,
    /// rather than trusting the doc's printed totals (L41 doctrine:
    /// mechanical recount, never the doc's/worker's claimed number).
    #[test]
    fn category_counts_match_the_docs_own_arithmetic_table() -> TestResult {
        let rows = parse_all(&workspace_root())?;

        // §2 minimums, keyed by the doc's own grouping-key rollups.
        let symbol_traversal_min = 30; // Symbol + CodeGraph
        let architecture_min = 30;
        let repository_min = 30;
        let git_history_min = 20;
        let learning_memory_min = 30; // Lessons + Experience
        let retrieval_quality_min = 30; // Retrieval + Reranking
        let token_reduction_min = 10;
        let learning_min = 10;
        let performance_min = 10;
        let federation_min = 10;
        let mcp_min = 10;
        let cli_min = 10;

        let count = |category: &str| rows.iter().filter(|r| r.category == category).count();

        assert_eq!(
            count("Symbol") + count("CodeGraph"),
            39,
            "Symbol traversal combined count drifted"
        );
        assert!(count("Symbol") + count("CodeGraph") >= symbol_traversal_min);
        assert_eq!(count("Architecture"), 30);
        assert!(count("Architecture") >= architecture_min);
        assert_eq!(count("Repository"), 30);
        assert!(count("Repository") >= repository_min);
        assert_eq!(count("GitHistory"), 20);
        assert!(count("GitHistory") >= git_history_min);
        assert_eq!(count("Lessons") + count("Experience"), 30);
        assert!(count("Lessons") + count("Experience") >= learning_memory_min);
        assert_eq!(count("Retrieval") + count("Reranking"), 41);
        assert!(count("Retrieval") + count("Reranking") >= retrieval_quality_min);
        assert_eq!(count("TokenReduction"), 10);
        assert!(count("TokenReduction") >= token_reduction_min);
        assert_eq!(count("Learning"), 10);
        assert!(count("Learning") >= learning_min);
        assert_eq!(count("Performance"), 10);
        assert!(count("Performance") >= performance_min);
        assert_eq!(count("Federation"), 10);
        assert!(count("Federation") >= federation_min);
        assert_eq!(count("MCP"), 10);
        assert!(count("MCP") >= mcp_min);
        assert_eq!(count("CLI"), 10);
        assert!(count("CLI") >= cli_min);

        let total: usize = [
            "Symbol",
            "CodeGraph",
            "Architecture",
            "Repository",
            "GitHistory",
            "Lessons",
            "Experience",
            "Retrieval",
            "Reranking",
            "TokenReduction",
            "Learning",
            "Performance",
            "Federation",
            "MCP",
            "CLI",
        ]
        .iter()
        .map(|c| count(c))
        .sum();
        assert_eq!(
            total, 250,
            "category counts must partition all 250 rows exactly once"
        );
        Ok(())
    }

    #[test]
    fn qa_gate_rows_win_over_benchmarks_prose_for_shared_ids() -> TestResult {
        // BENCHMARKS §1 explicitly says the PROOF_GATE table wins for
        // QA-001..QA-100 where the two differ; this harness only ever
        // parses tranche 1 FROM the proof gate doc, so there is no
        // possibility of silently preferring the non-binding text.
        let rows = parse_all(&workspace_root())?;
        let qa001 = rows
            .iter()
            .find(|r| r.id == "QA-001")
            .ok_or("QA-001 missing from parsed rows")?;
        assert!(qa001.query.contains("tests directly connected"));
        Ok(())
    }
}
