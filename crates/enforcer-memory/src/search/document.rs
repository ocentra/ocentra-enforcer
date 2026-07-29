//! The shared indexable document shape [`fulltext`](crate::fulltext),
//! [`vector`](crate::vector), and [`ranking`](crate::ranking) all
//! operate over. Deliberately NOT a new graph/node model (D-02/the
//! borrow-policy "no second graph model" rule) -- this is a thin,
//! read-only *projection* callers build from existing node types
//! ([`crate::graph::MemoryNode`], [`crate::code_graph::CodeNode`])
//! without those modules needing to know retrieval exists.

use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    DocumentKind, MemorySearchDocumentId, MemorySearchDocumentSnippet,
    MemorySearchDocumentSourcePath, MemorySearchDocumentText, ParserSourceText,
};
/// One document eligible for full-text/vector retrieval. `id` is a
/// stable, caller-assigned key (e.g. a `CodeNode::id()` or
/// `MemoryNode::id()` value) that the fusion/rerank stages carry through
/// unchanged so results can be joined back to the source graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    pub id: MemorySearchDocumentId,
    pub kind: DocumentKind,
    /// The text actually tokenized/embedded. For code chunks this is
    /// the symbol/function body (or file text for `TextOnly`/File
    /// documents); for memory documents this is `searchable_text()`.
    pub text: MemorySearchDocumentText,
    /// Short human-readable snippet returned in the final context pack
    /// (may equal `text` if already short).
    pub snippet: MemorySearchDocumentSnippet,
    /// Repo-relative path or logical source, for trace/provenance only.
    pub source_path: Option<MemorySearchDocumentSourcePath>,
}

impl SearchDocument {
    pub fn new(
        id: impl Into<MemorySearchDocumentId>,
        kind: DocumentKind,
        text: impl Into<MemorySearchDocumentText>,
    ) -> Self {
        let text = text.into();
        let snippet = snippet_of(ParserSourceText::from(text.as_str()));
        Self {
            id: id.into(),
            kind,
            text,
            snippet,
            source_path: None,
        }
    }

    pub fn with_source_path(mut self, path: impl Into<MemorySearchDocumentSourcePath>) -> Self {
        self.source_path = Some(path.into());
        self
    }
}

fn snippet_of(text: ParserSourceText<'_>) -> MemorySearchDocumentSnippet {
    const MAX: usize = 240;
    if text.as_str().len() <= MAX {
        text.as_str().retained().into()
    } else {
        let mut end = MAX;
        while end > 0 && !text.as_str().is_char_boundary(end) {
            end -= 1;
        }
        match text.as_str().get(..end) {
            Some(prefix) => format!("{prefix}...").into(),
            None => text.as_str().retained().into(),
        }
    }
}
