//! X06.5: the weaver's summary cache and entity-link table.
//!
//! A plain in-process store (mirrors [`crate::code_graph::CodeGraph`]'s
//! "flat node store, no in-place mutation" shape): summaries are
//! never edited in place, only marked stale and later replaced by the
//! summarizer worker producing a fresh one; entity links are append/
//! remove, not mutate.
//!
//! This module owns cache *shape* only -- persistence (writing summaries
//! into X06.1's SQLite store) is out of this subpack's file claims and
//! is wired at integration.

use std::collections::{HashMap, HashSet};

/// One file's cached summary and whether it is still trustworthy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryEntry {
    pub text: String,
    pub stale: bool,
}

/// The summary cache + entity/associative link table the enrichment
/// workers read and write. Keyed by repo-relative path for summaries,
/// by node id for links -- same key vocabulary as
/// [`crate::code_graph::FileNode::rel_path`] /
/// [`crate::code_graph::CodeNode::id`], so callers do not need a
/// translation layer once this is wired to the real graph.
#[derive(Debug, Clone, Default)]
pub struct SummaryStore {
    summaries: HashMap<String, SummaryEntry>,
    /// node_id -> set of node ids it is associatively linked to.
    /// Symmetric by construction: [`SummaryStore::link_entity`] only
    /// ever links a node to itself: entity linking in this slice
    /// records *that a node was seen and is link-eligible*; the actual
    /// 2-3 hop associative computation is graph traversal (X06.3's
    /// `crate::graph`/`crate::code_graph` surface), consumed read-only
    /// once wired at integration. See [`SummaryStore::linked_entities`].
    linked_entities: HashSet<String>,
    /// rel_path -> node ids whose link entry should be dropped when
    /// that file is deleted (so `unlink_entities_for_path` has
    /// something to act on without depending on the real graph).
    entities_by_path: HashMap<String, HashSet<String>>,
}

impl SummaryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a freshly computed summary for `rel_path`. Not stale.
    pub fn set_summary(&mut self, rel_path: &str, text: impl Into<String>) {
        self.summaries.insert(
            rel_path.to_owned(),
            SummaryEntry {
                text: text.into(),
                stale: false,
            },
        );
    }

    /// Mark `rel_path`'s cached summary stale without deleting it (a
    /// stale-but-present summary is still useful context while a fresh
    /// one is computed -- never silently blank).
    pub fn invalidate(&mut self, rel_path: &str) {
        match self.summaries.get_mut(rel_path) {
            Some(entry) => entry.stale = true,
            None => {
                self.summaries.insert(
                    rel_path.to_owned(),
                    SummaryEntry {
                        text: String::new(),
                        stale: true,
                    },
                );
            }
        }
    }

    /// Remove a file's summary entirely (used on file deletion, unlike
    /// [`SummaryStore::invalidate`] which keeps the stale text around).
    pub fn remove(&mut self, rel_path: &str) {
        self.summaries.remove(rel_path);
    }

    pub fn is_stale(&self, rel_path: &str) -> bool {
        self.summaries.get(rel_path).map(|e| e.stale).unwrap_or(true)
    }

    pub fn get(&self, rel_path: &str) -> Option<&SummaryEntry> {
        self.summaries.get(rel_path)
    }

    /// Record that `node_id` is link-eligible (entity/symbol linker
    /// and associative linker both call through here -- see module
    /// docs on why this slice does not compute real hop-distance
    /// links).
    pub fn link_entity(&mut self, node_id: &str) {
        self.linked_entities.insert(node_id.to_owned());
    }

    /// Associate `node_id` with the file it lives in, so a later file
    /// deletion can clean up the entity link table without depending
    /// on the real graph being available.
    pub fn associate_entity_with_path(&mut self, node_id: &str, rel_path: &str) {
        self.entities_by_path
            .entry(rel_path.to_owned())
            .or_default()
            .insert(node_id.to_owned());
    }

    pub fn is_entity_linked(&self, node_id: &str) -> bool {
        self.linked_entities.contains(node_id)
    }

    /// Drop every entity link recorded for `rel_path` (invoked on file
    /// deletion).
    pub fn unlink_entities_for_path(&mut self, rel_path: &str) {
        if let Some(node_ids) = self.entities_by_path.remove(rel_path) {
            for node_id in node_ids {
                self.linked_entities.remove(&node_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidate_marks_existing_summary_stale_without_deleting_it() {
        let mut store = SummaryStore::new();
        store.set_summary("src/lib.rs", "a summary");
        assert!(!store.is_stale("src/lib.rs"));

        store.invalidate("src/lib.rs");

        assert!(store.is_stale("src/lib.rs"));
        assert_eq!(store.get("src/lib.rs").map(|e| e.text.as_str()), Some("a summary"));
    }

    #[test]
    fn missing_summary_is_considered_stale() {
        let store = SummaryStore::new();
        assert!(store.is_stale("never/seen.rs"));
    }

    #[test]
    fn remove_deletes_the_entry_entirely() {
        let mut store = SummaryStore::new();
        store.set_summary("src/lib.rs", "a summary");
        store.remove("src/lib.rs");
        assert!(store.get("src/lib.rs").is_none());
    }

    #[test]
    fn deleting_a_file_unlinks_its_entities() {
        let mut store = SummaryStore::new();
        store.link_entity("sym:src/lib.rs:1:foo");
        store.associate_entity_with_path("sym:src/lib.rs:1:foo", "src/lib.rs");
        assert!(store.is_entity_linked("sym:src/lib.rs:1:foo"));

        store.unlink_entities_for_path("src/lib.rs");

        assert!(!store.is_entity_linked("sym:src/lib.rs:1:foo"));
    }
}
