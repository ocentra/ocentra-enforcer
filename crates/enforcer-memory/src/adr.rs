//! X06.3: ADR (architecture decision record) memory linked to graph
//! nodes.
//!
//! Mirrors the baseline `manage_adr` tool shape (scout digest Â§1:
//! "get/update/sections") without a persistence layer of its own --
//! this module owns the in-memory ADR record type, section
//! get/update, and the linkage to [`crate::code_graph::CodeGraph`] node
//! ids; durable storage is X06.1's concern (a caller persists
//! [`AdrStore`] however the store crate lands it -- same seam split as
//! [`crate::code_graph`]'s own module docs describe for `CodeGraph`
//! persistence).

use std::collections::BTreeMap;

use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    MemoryAdrDocumentContent, MemoryAdrId, MemoryAdrLinkedNodeId, MemoryAdrNoDocument,
    MemoryAdrSectionBody, MemoryAdrSectionName, MemoryAdrTitle,
};

/// One ADR: an id, a title, and a set of named sections (e.g.
/// "context", "decision", "consequences" -- the section names are
/// caller-defined, not a fixed enum, so this module does not need to
/// know the owner's exact ADR template to round-trip one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdrRecord {
    pub id: MemoryAdrId,
    pub title: MemoryAdrTitle,
    pub sections: BTreeMap<MemoryAdrSectionName, MemoryAdrSectionBody>,
    /// Graph node ids ([`crate::code_graph::CodeNode::id`]) this ADR is
    /// linked to -- the decision this ADR documents concerns these
    /// nodes.
    pub linked_node_ids: Vec<MemoryAdrLinkedNodeId>,
}

impl AdrRecord {
    pub fn new(id: impl Into<MemoryAdrId>, title: impl Into<MemoryAdrTitle>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            sections: BTreeMap::new(),
            linked_node_ids: Vec::new(),
        }
    }

    pub fn with_section(
        mut self,
        name: impl Into<MemoryAdrSectionName>,
        body: impl Into<MemoryAdrSectionBody>,
    ) -> Self {
        self.sections.insert(name.into(), body.into());
        self
    }

    pub fn with_linked_node(mut self, node_id: impl Into<MemoryAdrLinkedNodeId>) -> Self {
        self.linked_node_ids.push(node_id.into());
        self
    }
}

/// Error returned by [`AdrStore`] operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdrError {
    #[error("no ADR with id '{0}'")]
    NotFound(MemoryAdrId),
    #[error("ADR '{0}' already exists -- use update_section, not a second create")]
    AlreadyExists(MemoryAdrId),
}

/// Baseline-compatible whole-document response for `mode="get"`
/// (`refs/x06-baseline-tool-schemas.md` Â§14.4). `no_adr` distinguishes
/// "there genuinely is no stored document yet" from a present-but-empty
/// document string (the baseline's own `content: "" , status: "no_adr"`
/// shape when nothing has ever been stored).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdrDocument {
    pub content: MemoryAdrDocumentContent,
    pub no_adr: MemoryAdrNoDocument,
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
    records: Vec<AdrRecord>,
    /// Whole-document ADR blobs, keyed by the caller's project-ish id.
    /// This is a SEPARATE address space from `records` (the section-based
    /// extension API above): baseline `manage_adr` treats the ADR as one
    /// freeform markdown string per project (`refs/x06-baseline-tool-schemas.md`
    /// Â§14.2-Â§14.3 -- "SQLite store, one full-text field, no append/merge/
    /// diff semantics"), not a set of named `AdrRecord`s. Absence of a key
    /// here means "no ADR ever stored for this id", matching the baseline's
    /// `no_adr` status distinct from a present-but-empty document.
    documents: BTreeMap<MemoryAdrId, MemoryAdrDocumentContent>,
}

impl AdrStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Baseline `mode="get"` (`refs/x06-baseline-tool-schemas.md` Â§14.4):
    /// return the whole stored markdown document for `id`, or the
    /// `no_adr` shape if nothing has ever been stored for it.
    pub fn get_document(&self, id: impl Into<MemoryAdrId>) -> AdrDocument {
        let id = id.into();
        match self.documents.get(&id) {
            Some(content) => AdrDocument {
                content: content.retained(),
                no_adr: false.into(),
            },
            None => AdrDocument {
                content: String::new().into(),
                no_adr: true.into(),
            },
        }
    }

    /// Baseline `mode="update"` (undocumented alias: `"store"`) --
    /// wholesale replace of the stored document, no merge/diff/append
    /// (`refs/x06-baseline-tool-schemas.md` Â§14.3). Returns the previous
    /// document, if any, purely for caller convenience (the baseline
    /// itself does not echo the prior content).
    pub fn update_document(
        &mut self,
        id: impl Into<MemoryAdrId>,
        content: impl Into<MemoryAdrDocumentContent>,
    ) -> Option<MemoryAdrDocumentContent> {
        self.documents.insert(id.into(), content.into())
    }

    /// Baseline `mode="sections"` (`refs/x06-baseline-tool-schemas.md`
    /// Â§14.4): the markdown heading lines of the *stored* document,
    /// verbatim (any `#`-prefixed line, any heading level, trailing `\r`
    /// trimmed) -- not derived from any caller-supplied section list, and
    /// not the section-based extension API's section names. Returns an
    /// empty list (not an error) when there is no stored document, matching
    /// the baseline's degenerate-to-`[]` behavior for a NULL-content parse.
    pub fn list_document_headings(&self, id: impl Into<MemoryAdrId>) -> Vec<MemoryAdrSectionName> {
        let id = id.into();
        let Some(content) = self.documents.get(&id) else {
            return Vec::new();
        };
        content
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .filter(|line| line.trim_start().starts_with('#'))
            .map(|line| MemoryAdrSectionName::from(line.trim_start()))
            .collect()
    }

    pub fn create(&mut self, record: AdrRecord) -> Result<(), AdrError> {
        if self.records.iter().any(|existing| existing.id == record.id) {
            return Err(AdrError::AlreadyExists(record.id));
        }
        self.records.push(record);
        Ok(())
    }

    pub fn get(&self, id: impl Into<MemoryAdrId>) -> Result<&AdrRecord, AdrError> {
        let id = id.into();
        self.records
            .iter()
            .find(|record| record.id == id)
            .ok_or(AdrError::NotFound(id))
    }

    /// Update (or add) one named section's body on an existing ADR.
    pub fn update_section(
        &mut self,
        id: impl Into<MemoryAdrId>,
        section: impl Into<MemoryAdrSectionName>,
        body: impl Into<MemoryAdrSectionBody>,
    ) -> Result<(), AdrError> {
        let id = id.into();
        let Some(record) = self.records.iter_mut().find(|record| record.id == id) else {
            return Err(AdrError::NotFound(id));
        };
        record.sections.insert(section.into(), body.into());
        Ok(())
    }

    pub fn link_node(
        &mut self,
        id: impl Into<MemoryAdrId>,
        node_id: impl Into<MemoryAdrLinkedNodeId>,
    ) -> Result<(), AdrError> {
        let id = id.into();
        let Some(record) = self.records.iter_mut().find(|record| record.id == id) else {
            return Err(AdrError::NotFound(id));
        };
        let node_id = node_id.into();
        if !record
            .linked_node_ids
            .iter()
            .any(|linked| linked == &node_id)
        {
            record.linked_node_ids.push(node_id);
        }
        Ok(())
    }

    /// Every ADR linked to `node_id` -- the graph-side of the linkage
    /// (given a code node, which decisions concern it).
    pub fn adrs_for_node(&self, node_id: impl Into<MemoryAdrLinkedNodeId>) -> Vec<&AdrRecord> {
        let node_id = node_id.into();
        self.records
            .iter()
            .filter(|r| r.linked_node_ids.iter().any(|id| id == &node_id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &AdrRecord> {
        self.records.iter()
    }
}
