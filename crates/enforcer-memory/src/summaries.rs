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

use enforcer_domain::memory_types::{
    MemorySummaryEntityLinked, MemorySummaryNodeId, MemorySummaryRelativePath, MemorySummaryStale,
    MemorySummaryText,
};
use std::collections::{HashMap, HashSet};

/// One file's cached summary and whether it is still trustworthy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryEntry {
    pub text: MemorySummaryText,
    pub stale: MemorySummaryStale,
}

/// The summary cache + entity/associative link table the enrichment
/// workers read and write. Keyed by repo-relative path for summaries,
/// by node id for links -- same key vocabulary as
/// [`crate::code_graph::FileNode::rel_path`] /
/// [`crate::code_graph::CodeNode::id`], so callers do not need a
/// translation layer once this is wired to the real graph.
#[derive(Debug, Clone, Default)]
pub struct SummaryStore {
    summaries: HashMap<MemorySummaryRelativePath, SummaryEntry>,
    /// node_id -> set of node ids it is associatively linked to.
    /// Symmetric by construction: [`SummaryStore::link_entity`] only
    /// ever links a node to itself: entity linking in this slice
    /// records *that a node was seen and is link-eligible*; the actual
    /// 2-3 hop associative computation is graph traversal (X06.3's
    /// `crate::graph`/`crate::code_graph` surface), consumed read-only
    /// once wired at integration. See [`SummaryStore::linked_entities`].
    linked_entities: HashSet<MemorySummaryNodeId>,
    /// rel_path -> node ids whose link entry should be dropped when
    /// that file is deleted (so `unlink_entities_for_path` has
    /// something to act on without depending on the real graph).
    entities_by_path: HashMap<MemorySummaryRelativePath, HashSet<MemorySummaryNodeId>>,
}

impl SummaryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a freshly computed summary for `rel_path`. Not stale.
    pub fn set_summary(
        &mut self,
        rel_path: impl Into<MemorySummaryRelativePath>,
        text: impl Into<MemorySummaryText>,
    ) {
        self.summaries.insert(
            rel_path.into(),
            SummaryEntry {
                text: text.into(),
                stale: false.into(),
            },
        );
    }

    /// Mark `rel_path`'s cached summary stale without deleting it (a
    /// stale-but-present summary is still useful context while a fresh
    /// one is computed -- never silently blank).
    pub fn invalidate(&mut self, rel_path: impl Into<MemorySummaryRelativePath>) {
        let rel_path = rel_path.into();
        match self.summaries.get_mut(&rel_path) {
            Some(entry) => entry.stale = true.into(),
            None => {
                self.summaries.insert(
                    rel_path,
                    SummaryEntry {
                        text: String::new().into(),
                        stale: true.into(),
                    },
                );
            }
        }
    }

    /// Remove a file's summary entirely (used on file deletion, unlike
    /// [`SummaryStore::invalidate`] which keeps the stale text around).
    pub fn remove(&mut self, rel_path: impl Into<MemorySummaryRelativePath>) {
        self.summaries.remove(&rel_path.into());
    }

    pub fn is_stale(&self, rel_path: impl Into<MemorySummaryRelativePath>) -> MemorySummaryStale {
        let rel_path = rel_path.into();
        self.summaries
            .get(&rel_path)
            .map_or_else(|| true.into(), |entry| entry.stale)
    }

    pub fn get(&self, rel_path: impl Into<MemorySummaryRelativePath>) -> Option<&SummaryEntry> {
        self.summaries.get(&rel_path.into())
    }

    /// Record that `node_id` is link-eligible (entity/symbol linker
    /// and associative linker both call through here -- see module
    /// docs on why this slice does not compute real hop-distance
    /// links).
    pub fn link_entity(&mut self, node_id: impl Into<MemorySummaryNodeId>) {
        self.linked_entities.insert(node_id.into());
    }

    /// Associate `node_id` with the file it lives in, so a later file
    /// deletion can clean up the entity link table without depending
    /// on the real graph being available.
    pub fn associate_entity_with_path(
        &mut self,
        node_id: impl Into<MemorySummaryNodeId>,
        rel_path: impl Into<MemorySummaryRelativePath>,
    ) {
        self.entities_by_path
            .entry(rel_path.into())
            .or_default()
            .insert(node_id.into());
    }

    pub fn is_entity_linked(
        &self,
        node_id: impl Into<MemorySummaryNodeId>,
    ) -> MemorySummaryEntityLinked {
        self.linked_entities.contains(&node_id.into()).into()
    }

    /// Drop every entity link recorded for `rel_path` (invoked on file
    /// deletion).
    pub fn unlink_entities_for_path(&mut self, rel_path: impl Into<MemorySummaryRelativePath>) {
        if let Some(node_ids) = self.entities_by_path.remove(&rel_path.into()) {
            for node_id in node_ids {
                self.linked_entities.remove(&node_id);
            }
        }
    }
}
