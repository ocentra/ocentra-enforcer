//! X06.3: ADR (architecture decision record) memory linked to graph
//! nodes.
//!
//! Mirrors the baseline `manage_adr` tool shape (scout digest §1:
//! "get/update/sections") without a persistence layer of its own --
//! this module owns the in-memory ADR record type, section
//! get/update, and the linkage to [`crate::code_graph::CodeGraph`] node
//! ids; durable storage is X06.1's concern (a caller persists
//! [`AdrStore`] however the store crate lands it -- same seam split as
//! [`crate::code_graph`]'s own module docs describe for `CodeGraph`
//! persistence).

use std::collections::BTreeMap;

/// One ADR: an id, a title, and a set of named sections (e.g.
/// "context", "decision", "consequences" -- the section names are
/// caller-defined, not a fixed enum, so this module does not need to
/// know the owner's exact ADR template to round-trip one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdrRecord {
    pub id: String,
    pub title: String,
    pub sections: BTreeMap<String, String>,
    /// Graph node ids ([`crate::code_graph::CodeNode::id`]) this ADR is
    /// linked to -- the decision this ADR documents concerns these
    /// nodes.
    pub linked_node_ids: Vec<String>,
}

impl AdrRecord {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            sections: BTreeMap::new(),
            linked_node_ids: Vec::new(),
        }
    }

    pub fn with_section(mut self, name: impl Into<String>, body: impl Into<String>) -> Self {
        self.sections.insert(name.into(), body.into());
        self
    }

    pub fn with_linked_node(mut self, node_id: impl Into<String>) -> Self {
        self.linked_node_ids.push(node_id.into());
        self
    }
}

/// Error returned by [`AdrStore`] operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdrError {
    #[error("no ADR with id '{0}'")]
    NotFound(String),
    #[error("ADR '{0}' already exists -- use update_section, not a second create")]
    AlreadyExists(String),
}

/// An in-memory ADR store, keyed by ADR id. Append-only for the ADR
/// list itself (an ADR is never deleted, only superseded by
/// convention -- callers wanting a "superseded_by" relationship model
/// it as a section or a graph edge, this module does not special-case
/// it); section bodies ARE mutable in place via
/// [`AdrStore::update_section`] (an ADR's *decision* section, say,
/// legitimately gets amended text over time, unlike x05's lesson ledger
/// append-only discipline).
#[derive(Debug, Clone, Default)]
pub struct AdrStore {
    records: BTreeMap<String, AdrRecord>,
}

impl AdrStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, record: AdrRecord) -> Result<(), AdrError> {
        if self.records.contains_key(&record.id) {
            return Err(AdrError::AlreadyExists(record.id));
        }
        self.records.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<&AdrRecord, AdrError> {
        self.records
            .get(id)
            .ok_or_else(|| AdrError::NotFound(id.to_string()))
    }

    /// Update (or add) one named section's body on an existing ADR.
    pub fn update_section(
        &mut self,
        id: &str,
        section: &str,
        body: impl Into<String>,
    ) -> Result<(), AdrError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| AdrError::NotFound(id.to_string()))?;
        record.sections.insert(section.to_string(), body.into());
        Ok(())
    }

    pub fn link_node(&mut self, id: &str, node_id: impl Into<String>) -> Result<(), AdrError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| AdrError::NotFound(id.to_string()))?;
        let node_id = node_id.into();
        if !record.linked_node_ids.contains(&node_id) {
            record.linked_node_ids.push(node_id);
        }
        Ok(())
    }

    /// Every ADR linked to `node_id` -- the graph-side of the linkage
    /// (given a code node, which decisions concern it).
    pub fn adrs_for_node(&self, node_id: &str) -> Vec<&AdrRecord> {
        self.records
            .values()
            .filter(|r| r.linked_node_ids.iter().any(|id| id == node_id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &AdrRecord> {
        self.records.values()
    }
}
