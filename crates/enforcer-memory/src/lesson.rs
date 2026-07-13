//! Parser for the orchestration-lessons ledger
//! (`docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md`):
//! a pipe-delimited markdown table with columns
//! `id | date | observed | lesson | landed-at | ships-via`.
//!
//! This is a transitional reader per the ledger's own capsule ("this file
//! is the TRANSITIONAL, human-readable seed of the x06 memory graph"):
//! every row becomes a lesson node. The parser is intentionally narrow —
//! it only understands the documented table shape and skips everything
//! else (headings, prose, the capsule block, separator rows).

use serde::{Deserialize, Serialize};

/// One parsed row of the orchestration-lessons ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonRow {
    pub id: String,
    pub date: String,
    pub observed: String,
    pub lesson: String,
    pub landed_at: String,
    pub ships_via: String,
}

impl LessonRow {
    /// Text this row exposes to keyword recall.
    pub fn searchable_text(&self) -> String {
        format!("{} \n {} \n {}", self.lesson, self.observed, self.landed_at)
    }
}

/// Split a markdown table row `| a | b | c |` into trimmed cells.
/// Returns `None` if the line is not a `|`-delimited row.
fn split_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
    Some(
        inner
            .split('|')
            // ALLOC-JUSTIFICATION: Parsed cells must outlive the borrowed ledger input
            // because they become owned fields of the returned LessonRow values.
            .map(|cell| cell.trim().to_string())
            .collect(),
    )
}

/// A markdown table separator row looks like `|---|---|---|` (dashes and
/// colons only per cell, once trimmed).
fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'))
}

/// Parse the ledger's `| id | date | observed | lesson | landed-at |
/// ships-via |` table into rows. Non-table lines (headings, prose, the
/// capsule HTML comment block) are skipped. The header row (`| id |
/// date | ...`) is recognized by its literal `id` first cell and skipped.
pub fn parse_ledger(text: &str) -> Vec<LessonRow> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let Some(cells) = split_row(line) else {
            continue;
        };
        if cells.len() < 6 {
            continue;
        }
        if is_separator_row(&cells) {
            continue;
        }
        if cells
            .first()
            .is_some_and(|first_cell| first_cell.eq_ignore_ascii_case("id"))
        {
            continue;
        }
        if cells.first().is_some_and(String::is_empty) {
            continue;
        }
        let mut fields = cells.into_iter();
        let (Some(id), Some(date), Some(observed), Some(lesson), Some(landed_at), Some(ships_via)) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        ) else {
            continue;
        };
        rows.push(LessonRow {
            id,
            date,
            observed,
            lesson,
            landed_at,
            ships_via,
        });
    }
    rows
}
