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

use enforcer_domain::memory_types::{
    MemoryLedgerLessonId, MemoryLessonDate, MemoryLessonLandedAt, MemoryLessonLedgerCell,
    MemoryLessonLedgerDocument, MemoryLessonLedgerLine, MemoryLessonObserved,
    MemoryLessonSearchText, MemoryLessonSeparatorRow, MemoryLessonShipsVia, MemoryLessonText,
};
/// One parsed row of the orchestration-lessons ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LessonRow {
    pub id: MemoryLedgerLessonId,
    pub date: MemoryLessonDate,
    pub observed: MemoryLessonObserved,
    pub lesson: MemoryLessonText,
    pub landed_at: MemoryLessonLandedAt,
    pub ships_via: MemoryLessonShipsVia,
}

impl LessonRow {
    /// Text this row exposes to keyword recall.
    pub fn searchable_text(&self) -> MemoryLessonSearchText {
        format!("{} \n {} \n {}", self.lesson, self.observed, self.landed_at).into()
    }
}

/// Split a markdown table row `| a | b | c |` into trimmed cells.
/// Returns `None` if the line is not a `|`-delimited row.
fn split_row(line: &MemoryLessonLedgerLine) -> Option<Vec<MemoryLessonLedgerCell>> {
    let trimmed = line.as_str().trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
    Some(inner
            .split('|')
            // ALLOC-JUSTIFICATION: Parsed cells must outlive the borrowed ledger input
            // because they become owned fields of the returned LessonRow values.
            .map(|cell| cell.trim().into())
            .collect())
}

/// A markdown table separator row looks like `|---|---|---|` (dashes and
/// colons only per cell, once trimmed).
fn is_separator_row(cells: &[MemoryLessonLedgerCell]) -> MemoryLessonSeparatorRow {
    (!cells.is_empty()
        && cells.iter().all(|cell| {
            !cell.is_empty()
                && cell
                    .as_str()
                    .chars()
                    .all(|character| character == '-' || character == ':')
        }))
    .into()
}

/// Parse the ledger's `| id | date | observed | lesson | landed-at |
/// ships-via |` table into rows. Non-table lines (headings, prose, the
/// capsule HTML comment block) are skipped. The header row (`| id |
/// date | ...`) is recognized by its literal `id` first cell and skipped.
pub fn parse_ledger(text: impl Into<MemoryLessonLedgerDocument>) -> Vec<LessonRow> {
    let text = text.into();
    let mut rows = Vec::new();
    for line in text.as_str().lines() {
        let Some(cells) = split_row(&line.into()) else {
            continue;
        };
        if cells.len() < 6 {
            continue;
        }
        if is_separator_row(&cells).is_separator() {
            continue;
        }
        if cells
            .first()
            .is_some_and(|first_cell| first_cell.as_str().eq_ignore_ascii_case("id"))
        {
            continue;
        }
        if cells.first().is_some_and(MemoryLessonLedgerCell::is_empty) {
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
            id: id.as_str().into(),
            date: date.as_str().into(),
            observed: observed.as_str().into(),
            lesson: lesson.as_str().into(),
            landed_at: landed_at.as_str().into(),
            ships_via: ships_via.as_str().into(),
        });
    }
    rows
}
