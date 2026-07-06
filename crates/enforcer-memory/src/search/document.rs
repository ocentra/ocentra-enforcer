//! The shared indexable document shape [`fulltext`](crate::fulltext),
//! [`vector`](crate::vector), and [`ranking`](crate::ranking) all
//! operate over. Deliberately NOT a new graph/node model (D-02/the
//! borrow-policy "no second graph model" rule) -- this is a thin,
//! read-only *projection* callers build from existing node types
//! ([`crate::graph::MemoryNode`], [`crate::code_graph::CodeNode`])
//! without those modules needing to know retrieval exists.

use serde::{Deserialize, Serialize};

/// Structural kind a document was produced from. Drives the label boost
/// in [`crate::fulltext`] (D-07: "Functions > Routes > Classes",
/// scout-documented baseline behavior) and the memory-tier routing in
/// [`crate::ranking`] soft signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentKind {
    Function,
    Route,
    Type,
    Test,
    File,
    Lesson,
    Artifact,
    Summary,
    Other,
}

impl DocumentKind {
    /// Structural label boost applied by [`crate::fulltext`] BM25
    /// scoring: matches the scout-documented baseline ordering
    /// (Function > Route > Class/Type), extended with the memory-side
    /// kinds this crate also indexes.
    pub fn label_boost(&self) -> f64 {
        match self {
            DocumentKind::Function => 10.0,
            DocumentKind::Route => 8.0,
            DocumentKind::Type => 5.0,
            DocumentKind::Test => 4.0,
            DocumentKind::Lesson => 6.0,
            DocumentKind::Summary => 3.0,
            DocumentKind::Artifact => 2.0,
            DocumentKind::File | DocumentKind::Other => 1.0,
        }
    }
}

/// One document eligible for full-text/vector retrieval. `id` is a
/// stable, caller-assigned key (e.g. a `CodeNode::id()` or
/// `MemoryNode::id()` value) that the fusion/rerank stages carry through
/// unchanged so results can be joined back to the source graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub kind: DocumentKind,
    /// The text actually tokenized/embedded. For code chunks this is
    /// the symbol/function body (or file text for `TextOnly`/File
    /// documents); for memory documents this is `searchable_text()`.
    pub text: String,
    /// Short human-readable snippet returned in the final context pack
    /// (may equal `text` if already short).
    pub snippet: String,
    /// Repo-relative path or logical source, for trace/provenance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

impl SearchDocument {
    pub fn new(id: impl Into<String>, kind: DocumentKind, text: impl Into<String>) -> Self {
        let text = text.into();
        let snippet = snippet_of(&text);
        Self {
            id: id.into(),
            kind,
            text,
            snippet,
            source_path: None,
        }
    }

    pub fn with_source_path(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }
}

fn snippet_of(text: &str) -> String {
    const MAX: usize = 240;
    if text.len() <= MAX {
        text.to_owned()
    } else {
        let mut end = MAX;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &text[..end])
    }
}
