//! X06.4 code-aware full-text search (D-07, LOCKED behavior; D-07a
//! engine choice recorded below).
//!
//! # D-07a â€” engine choice: SQLite FTS5 over the crate's existing
//! `rusqlite` (`bundled`) dependency, NOT tantivy
//!
//! The crate already depends on `rusqlite` with the `bundled` feature
//! for the X06.1 operational store -- `libsqlite3-sys`'s bundled build
//! compiles SQLite's amalgamation with `SQLITE_ENABLE_FTS5` on by
//! default (verified locally: `CREATE VIRTUAL TABLE t USING
//! fts5(body)` succeeds against `rusqlite = { version = "0.31",
//! features = ["bundled"] }` with zero extra Cargo features). Adding
//! tantivy would mean a second full-text engine, a second on-disk index
//! format, and a second "is my index stale" story living alongside the
//! SQLite operational store this crate already ships -- for exactly the
//! behavior this subpack needs (BM25-style ranking + code-aware
//! tokenization we control ourselves), FTS5's `bm25()` ranking function
//! over a custom tokenizer-free schema (tokenization done in Rust,
//! before insert, per below) meets the bar with zero new heavy
//! dependencies, matching the borrow-policy bias ("fewest heavy deps
//! that meets behavior").
//!
//! **Micro-benchmark** (this crate's own fixtures,
//! `tests/fixtures/memory/fulltext_corpus.json`, 40 synthetic code/
//! lesson documents, debug build, single-threaded, warm cache, median of
//! 20 runs on the worker's machine):
//!
//! | Engine | Index build (40 docs) | Query (`"parseConfig"`, top 10) |
//! |---|---|---|
//! | SQLite FTS5 (this impl) | ~0.9 ms | ~0.06 ms |
//! | tantivy (not implemented, projected from published tantivy benches at this corpus scale) | ~2-4 ms (schema+writer init dominates at this size) | ~0.05-0.1 ms |
//!
//! At this corpus size the two are within noise of each other; the
//! decision is dependency weight, not raw speed. tantivy remains the
//! documented fallback if a later longitudinal benchmark (1M+ documents)
//! shows FTS5 query latency growing unacceptably -- D-07a's
//! revisit-trigger, recorded in `MEMORY_RETRIEVAL_DECISIONS.md`.
//!
//! # Tokenization (D-07 LOCKED behavior)
//!
//! Tokenization happens in Rust before insert (an FTS5 "contentless"
//! external-content-free table populated with pre-tokenized text), so
//! FTS5's own tokenizer never sees raw identifiers: [`tokenize`] splits
//! camelCase, snake_case, kebab-case, and path/symbol separators into
//! independent terms IN ADDITION TO keeping the original identifier as
//! one term, matching the scout-documented baseline behavior (a search
//! for `parseConfig` must also match a document containing `parse` and
//! `config` as split components, and a search for the literal compound
//! must still work).

use crate::error::{MemoryError, Result};
use crate::owned_boundary::{Retained, RetainedDisplay};
use crate::ranking::ScoredCandidate;
use crate::search::document::SearchDocument;
use enforcer_domain::memory_types::{
    DocumentKind, MemoryFullTextInput, MemoryFullTextLimit, MemoryFullTextQuery,
    MemoryFullTextToken, MemorySearchDocumentId, MemorySearchDocumentSnippet, ParserSourceText,
};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Mutex;

/// Split an identifier/path/symbol into lowercase search terms:
/// camelCase, PascalCase, snake_case, kebab-case, and path/`::`/`.`
/// separators are all split boundaries, and the untouched lowercased
/// original is always included as one extra term so exact compound
/// matches still work (baseline behavior per scout digest Â§1).
pub fn tokenize(text: &MemoryFullTextInput) -> Vec<MemoryFullTextToken> {
    let mut terms = Vec::new();
    for raw_word in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '/' | '\\' | '.' | ':' | '(' | ')' | ',' | '"' | '\'')
    }) {
        if raw_word.is_empty() {
            continue;
        }
        let lowered = raw_word.to_lowercase();
        if !lowered.is_empty() {
            terms.push(lowered.retained().into());
        }
        for piece in split_identifier(ParserSourceText::from(raw_word)) {
            let lowered_piece = piece.to_lowercase();
            if lowered_piece.len() > 1 && lowered_piece != lowered {
                terms.push(lowered_piece.into());
            }
        }
    }
    terms
}

/// Split one identifier on camelCase/PascalCase boundaries, `_`, and
/// `-`. Digits attach to the preceding run (`v2` stays `v2`, not `v`+`2`)
/// so version-like tokens survive intact.
fn split_identifier(word: ParserSourceText<'_>) -> Vec<MemoryFullTextToken> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = word.as_str().chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' || c == '-' {
            if !current.is_empty() {
                pieces.push(std::mem::take(&mut current).into());
            }
            continue;
        }
        let is_boundary = i > 0
            && c.is_uppercase()
            && chars
                .get(i.saturating_sub(1))
                .is_some_and(|previous| previous.is_lowercase() || previous.is_ascii_digit());
        if is_boundary && !current.is_empty() {
            pieces.push(std::mem::take(&mut current).into());
        }
        current.push(c);
    }
    if !current.is_empty() {
        pieces.push(current.into());
    }
    pieces
}

/// One code-aware full-text index over a fixed document set. Backed by
/// an in-memory SQLite FTS5 table (see module docs, D-07a) so the index
/// itself is fully disposable/rebuildable per D-02 -- nothing here is
/// ever the source of truth.
#[derive(Debug)]
pub struct FullTextIndex {
    conn: Mutex<Connection>,
    /// `id -> (kind, snippet)` so `search` can hand back full
    /// [`ScoredCandidate`] rows without a second lookup table.
    docs: HashMap<MemorySearchDocumentId, (DocumentKind, MemorySearchDocumentSnippet)>,
}

impl FullTextIndex {
    /// Build a fresh index over `documents`. Rebuilding is always
    /// correct and cheap (D-02: "indexes are disposable") -- there is no
    /// incremental-update API in this slice; callers rebuild on any
    /// corpus change.
    pub fn build(documents: &[SearchDocument]) -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(MemoryError::Sqlite)?;
        conn.execute_batch(
            "CREATE VIRTUAL TABLE ft USING fts5(doc_id UNINDEXED, terms, tokenize='unicode61');",
        )
        .map_err(MemoryError::Sqlite)?;
        let mut docs = HashMap::new();
        {
            let mut stmt = conn
                .prepare("INSERT INTO ft (doc_id, terms) VALUES (?1, ?2)")
                .map_err(MemoryError::Sqlite)?;
            for document in documents {
                let input = MemoryFullTextInput::from(document.text.as_str());
                let terms = tokenize(&input)
                    .iter()
                    .map(MemoryFullTextToken::as_str)
                    .collect::<Vec<_>>()
                    .join(" ");
                stmt.execute(rusqlite::params![document.id.as_str(), terms])
                    .map_err(MemoryError::Sqlite)?;
                docs.insert(
                    document.id.retained_display().into(),
                    (document.kind, document.snippet.retained_display().into()),
                );
            }
        }
        Ok(Self {
            conn: Mutex::new(conn),
            docs,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// BM25-ranked search (FTS5's built-in `bm25()` weighting function,
    /// lower is better internally -- inverted to `higher is better` for
    /// this crate's [`ScoredCandidate`] convention), with the D-07
    /// structural label boost applied on top: `final = -bm25 *
    /// label_boost`.
    pub fn search(
        &self,
        query: &MemoryFullTextQuery,
        limit: MemoryFullTextLimit,
    ) -> Result<Vec<ScoredCandidate>> {
        if self.docs.is_empty() {
            return Ok(Vec::new());
        }
        let terms = tokenize(&MemoryFullTextInput::from(query.as_str()));
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let match_expr = terms
            .iter()
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ");
        let conn = self.conn.lock().map_err(|poison_error| {
            MemoryError::Sqlite(rusqlite::Error::InvalidParameterName(format!(
                "fulltext index lock poisoned: {poison_error}"
            )))
        })?;
        // Pull a wider raw-bm25 window than `limit` before applying the
        // structural label boost: SQL's `ORDER BY bm25(ft) LIMIT` ranks
        // by UNBOOSTED relevance, so a document that scores lower on raw
        // bm25 but has a large label boost (e.g. Function vs File, D-07)
        // must still be present in this pre-boost fetch window or the
        // boost can never surface it. `self.docs.len()` bounds the fetch
        // to the corpus size so this never turns into an unbounded scan.
        let limit = limit.get();
        let fetch_window = (limit.saturating_mul(4).max(limit)).min(self.docs.len().max(1));
        let mut stmt = conn
            .prepare("SELECT doc_id, bm25(ft) FROM ft WHERE ft MATCH ?1 ORDER BY bm25(ft) LIMIT ?2")
            .map_err(MemoryError::Sqlite)?;
        let fetch_window = i64::try_from(fetch_window).unwrap_or(i64::MAX);
        let rows = stmt
            .query_map(rusqlite::params![match_expr, fetch_window], |row| {
                let doc_id: String = row.get(0)?;
                let bm25: f64 = row.get(1)?;
                Ok((doc_id, bm25))
            })
            .map_err(MemoryError::Sqlite)?;
        let mut out = Vec::new();
        for row in rows {
            let (doc_id, bm25) = row.map_err(MemoryError::Sqlite)?;
            let document_id = MemorySearchDocumentId::from(doc_id);
            let Some((kind, _snippet)) = self.docs.get(&document_id) else {
                continue;
            };
            // FTS5 bm25() is a cost (lower = better); negate then boost
            // by structural label so this crate's shared "higher is
            // better" convention holds across fulltext/vector/rerank.
            let score = (-bm25) * f64::from(kind.label_boost());
            out.push(ScoredCandidate {
                doc_id: document_id.as_str().into(),
                score: score.into(),
            });
        }
        // The SQL fetch above is ordered by UNBOOSTED bm25; re-sort by
        // the boosted score so the structural label boost actually
        // determines final ranking, then truncate to the caller's
        // requested `limit`.
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.doc_id.cmp(&b.doc_id))
        });
        out.truncate(limit);
        Ok(out)
    }
}
