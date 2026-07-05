//! Wire shapes for the x06.1 store: append-only log records, the
//! content-addressed artifact manifest, and index manifests carrying a
//! source high-watermark. Kept separate from `crate::record`
//! (the x05 `MemoryRecord` schema this crate already ingests) because
//! these are store-internal persistence shapes, not the external
//! NDJSON memory-record contract.

use serde::{Deserialize, Serialize};

/// Current schema version for every shape in this module. Bumped only on
/// a wire-incompatible change; readers must reject an unknown version
/// rather than guess at a shape.
pub const SCHEMA_VERSION: u32 = 1;

/// One append-only observation-log entry: a single usage/incident
/// observation (mirrors `crate::ingest::Observation` but is the
/// on-disk/at-rest wire shape the store persists, independent of the
/// in-memory `Incident` type).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLogEntry {
    pub schema_version: u32,
    /// Monotonic sequence number assigned by the log on append.
    pub seq: u64,
    pub id: String,
    pub lesson_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault_class: Option<String>,
    pub repo_context: String,
    pub clean: bool,
    pub source_surface: String,
    pub ts: String,
    /// Id of an earlier entry this one supersedes (a correction), or
    /// `None` for a fresh observation. Append-only: superseding never
    /// deletes or edits the earlier row, it only records the relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
}

/// One append-only graph-event-log entry: a structural change to the
/// operational graph (node/edge add, or a supersede of an earlier
/// entry). The store's SQLite read model is rebuilt deterministically by
/// replaying this log in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEventLogEntry {
    pub schema_version: u32,
    pub seq: u64,
    pub id: String,
    pub event: GraphEventKind,
    pub ts: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
}

/// Structural graph mutation kinds this log records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum GraphEventKind {
    NodeAdded {
        node_id: String,
        node_kind: String,
    },
    EdgeAdded {
        from: String,
        to: String,
        label: String,
    },
}

/// A content-addressed artifact manifest row: the artifact's id IS the
/// SHA-256 of its content (see `crate::ids::ArtifactId`), so the
/// manifest only needs to carry metadata plus the digest for
/// verify-on-read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactManifestEntry {
    pub schema_version: u32,
    /// `sha256:<64 hex>` -- the artifact's content-addressed id.
    pub id: String,
    /// Repo-relative path this artifact was produced from/for, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel_path: Option<String>,
    /// Byte length of the stored content, recorded independently of the
    /// content itself so a truncated read is detectable without
    /// rehashing.
    pub byte_len: u64,
    pub ts: String,
}

/// An index manifest: records the append-only log's length ("source
/// high-watermark") the index was built against. A read of the index
/// with a watermark behind the log's current length means the index is
/// stale and must be rebuilt before being trusted (`MemoryError::StaleIndex`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexManifest {
    pub schema_version: u32,
    /// Which log this index was built from (e.g. `"observation"`,
    /// `"graph-event"`).
    pub source_log: String,
    /// The log length (next `Seq` to be assigned) at the moment this
    /// index was built.
    pub source_high_watermark: u64,
    pub built_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_entry_round_trips() -> Result<(), serde_json::Error> {
        let entry = ObservationLogEntry {
            schema_version: SCHEMA_VERSION,
            seq: 0,
            id: "obs-scan-0000".to_owned(),
            lesson_id: "L1".to_owned(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/enforcer-memory".to_owned(),
            clean: true,
            source_surface: "scan".to_owned(),
            ts: "2026-07-04T00:00:00Z".to_owned(),
            supersedes_seq: None,
        };
        let json = serde_json::to_string(&entry)?;
        let back: ObservationLogEntry = serde_json::from_str(&json)?;
        assert_eq!(entry, back);
        Ok(())
    }

    #[test]
    fn graph_event_kind_tags_on_wire() -> Result<(), serde_json::Error> {
        let node = GraphEventKind::NodeAdded {
            node_id: "n1".to_owned(),
            node_kind: "file".to_owned(),
        };
        let json = serde_json::to_string(&node)?;
        assert!(json.contains("\"kind\":\"nodeAdded\""));
        Ok(())
    }

    #[test]
    fn index_manifest_round_trips() -> Result<(), serde_json::Error> {
        let manifest = IndexManifest {
            schema_version: SCHEMA_VERSION,
            source_log: "observation".to_owned(),
            source_high_watermark: 42,
            built_at: "2026-07-04T00:00:00Z".to_owned(),
        };
        let json = serde_json::to_string(&manifest)?;
        let back: IndexManifest = serde_json::from_str(&json)?;
        assert_eq!(manifest, back);
        Ok(())
    }
}
