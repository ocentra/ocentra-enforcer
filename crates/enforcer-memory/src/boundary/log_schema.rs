//! Wire shapes for the x06.1 store: append-only log records, the
//! content-addressed artifact manifest, and index manifests carrying a
//! source high-watermark. Kept separate from `crate::record`
//! (the x05 `MemoryRecord` schema this crate already ingests) because
//! these are store-internal persistence shapes, not the external
//! NDJSON memory-record contract.

use serde::{Deserialize, Serialize};

use crate::model_observations::ModelRuntimeObservationCandidate;
use crate::observations::{ProceduralRecord, RouteTrace};
use crate::traces::TraceRecord;
use enforcer_domain::memory_types::{GraphEventKind, ProceduralOutcome, RouteConfidence};

/// Current schema version for every shape in this module. Bumped only on
/// a wire-incompatible change; readers must reject an unknown version
/// rather than guess at a shape.
pub const SCHEMA_VERSION: u32 = 1;

// ROUNDTRIP-TEST: tests::legacy_and_runtime_payload_dtos_preserve_domain_values

/// One append-only observation-log entry: a single usage/incident
/// observation (mirrors `crate::ingest::Observation` but is the
/// on-disk/at-rest wire shape the store persists, independent of the
/// in-memory `Incident` type).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLogEntryDto {
    pub schema_version: u32,
    /// Monotonic sequence number assigned by the log on append.
    pub seq: u64,
    pub id: String,
    pub lesson_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault_class: Option<String>,
    pub repo_context: String,
    pub clean: bool,
    pub source_surface: String,
    pub ts: String,
    /// Id of an earlier entry this one supersedes (a correction), or
    /// `None` for a fresh observation. Append-only: superseding never
    /// deletes or edits the earlier row, it only records the relation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
    /// Optional typed payload discriminator for non-incident learning
    /// observations, for example model-runtime or route-choice records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_kind: Option<String>,
    /// Raw typed payload for replayable projection records. The
    /// top-level incident fields stay populated so older readers still
    /// have a useful observation row even when they ignore this payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// One append-only graph-event-log entry: a structural change to the
/// operational graph (node/edge add, or a supersede of an earlier
/// entry). The store's SQLite read model is rebuilt deterministically by
/// replaying this log in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEventLogEntryDto {
    pub schema_version: u32,
    pub seq: u64,
    pub id: String,
    #[serde(with = "graph_event_wire")]
    pub event: GraphEventKind,
    pub ts: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
}

/// One append-only procedural-memory log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProceduralLogEntryDto {
    pub schema_version: u32,
    pub seq: u64,
    pub id: String,
    pub lesson_id: String,
    pub outcome: ProceduralOutcome,
    pub detail: String,
    pub ts: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
}

/// One append-only route-trace log entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteTraceLogEntryDto {
    pub schema_version: u32,
    pub seq: u64,
    pub id: String,
    pub query: String,
    pub route: String,
    pub confidence: f64,
    pub ts: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
}

/// Legacy observation payload for a procedural record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProceduralRecordDto {
    pub id: String,
    pub lesson_id: String,
    pub outcome: ProceduralOutcome,
    pub detail: String,
    pub ts: String,
}

impl From<ProceduralRecordDto> for ProceduralRecord {
    fn from(value: ProceduralRecordDto) -> Self {
        Self {
            id: value.id.into(),
            lesson_id: value.lesson_id.into(),
            outcome: value.outcome,
            detail: value.detail.into(),
            ts: value.ts.into(),
        }
    }
}

/// Legacy observation payload for a route trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RouteTraceDto {
    pub id: String,
    pub query: String,
    pub route: String,
    pub confidence: f64,
    pub ts: String,
}

impl From<RouteTraceDto> for RouteTrace {
    fn from(value: RouteTraceDto) -> Self {
        Self {
            id: value.id.into(),
            query: value.query.into(),
            route: value.route.into(),
            confidence: RouteConfidence::normalized(value.confidence),
            ts: value.ts.into(),
        }
    }
}

/// Persisted runtime call-trace payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TraceRecordDto {
    pub caller: String,
    pub callee: String,
    pub count: u64,
}

impl From<&TraceRecord> for TraceRecordDto {
    fn from(value: &TraceRecord) -> Self {
        Self {
            caller: String::from(value.caller.as_str()),
            callee: String::from(value.callee.as_str()),
            count: value.count.get(),
        }
    }
}

impl From<TraceRecordDto> for TraceRecord {
    fn from(value: TraceRecordDto) -> Self {
        Self {
            caller: value.caller.into(),
            callee: value.callee.into(),
            count: value.count.into(),
        }
    }
}

/// One append-only model-runtime observation log entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelObservationLogEntryDto {
    pub schema_version: u32,
    pub seq: u64,
    pub observed_at: String,
    pub source: String,
    pub run_id: String,
    pub candidate: ModelRuntimeObservationCandidate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes_seq: Option<u64>,
}

mod graph_event_wire {
    use enforcer_domain::memory_types::GraphEventKind;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase", tag = "kind")]
    enum GraphEventRef<'a> {
        NodeAdded {
            node_id: &'a str,
            node_kind: &'a str,
        },
        EdgeAdded {
            from: &'a str,
            to: &'a str,
            label: &'a str,
        },
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", tag = "kind")]
    enum GraphEventOwned {
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

    pub fn serialize<S>(event: &GraphEventKind, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match event {
            GraphEventKind::NodeAdded { node_id, node_kind } => GraphEventRef::NodeAdded {
                node_id: node_id.as_str(),
                node_kind: node_kind.as_str(),
            }
            .serialize(serializer),
            GraphEventKind::EdgeAdded { from, to, label } => GraphEventRef::EdgeAdded {
                from: from.as_str(),
                to: to.as_str(),
                label: label.as_str(),
            }
            .serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<GraphEventKind, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match GraphEventOwned::deserialize(deserializer)? {
            GraphEventOwned::NodeAdded { node_id, node_kind } => GraphEventKind::NodeAdded {
                node_id: node_id.into(),
                node_kind: node_kind.into(),
            },
            GraphEventOwned::EdgeAdded { from, to, label } => GraphEventKind::EdgeAdded {
                from: from.into(),
                to: to.into(),
                label: label.into(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ProceduralRecordDto, RouteTraceDto, TraceRecordDto};
    use crate::observations::{ProceduralRecord, RouteTrace};
    use crate::traces::TraceRecord;
    use enforcer_domain::memory_types::ProceduralOutcome;

    #[test]
    fn legacy_and_runtime_payload_dtos_preserve_domain_values() -> Result<(), serde_json::Error> {
        let procedural_json = serde_json::json!({
            "id": "procedure-1",
            "lesson_id": "lesson-1",
            "outcome": "fix-success",
            "detail": "applied the retained fix",
            "ts": "2026-07-17T12:00:00Z"
        });
        let procedural: ProceduralRecord =
            serde_json::from_value::<ProceduralRecordDto>(procedural_json)?.into();
        assert_eq!(procedural.id.as_str(), "procedure-1");
        assert_eq!(procedural.outcome, ProceduralOutcome::FixSuccess);

        let route_json = serde_json::json!({
            "id": "route-1",
            "query": "find the owner",
            "route": "code_graph",
            "confidence": 1.25,
            "ts": "2026-07-17T12:00:01Z"
        });
        let route: RouteTrace = serde_json::from_value::<RouteTraceDto>(route_json)?.into();
        assert_eq!(route.route.as_str(), "code_graph");
        assert_eq!(route.confidence.get(), 1.0);

        let domain = TraceRecord {
            caller: "sym:caller".into(),
            callee: "sym:callee".into(),
            count: 7_u64.into(),
        };
        let encoded = serde_json::to_value(TraceRecordDto::from(&domain))?;
        let decoded: TraceRecord = serde_json::from_value::<TraceRecordDto>(encoded)?.into();
        assert_eq!(decoded, domain);
        Ok(())
    }
}

/// A content-addressed artifact manifest row: the artifact's id IS the
/// SHA-256 of its content (see `enforcer_domain::memory_types::ArtifactId`), so the
/// manifest only needs to carry metadata plus the digest for
/// verify-on-read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactManifestEntryDto {
    pub schema_version: u32,
    /// `sha256:<64 hex>` -- the artifact's content-addressed id.
    pub id: String,
    /// Repo-relative path this artifact was produced from/for, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
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
pub struct IndexManifestDto {
    pub schema_version: u32,
    /// Which log this index was built from (e.g. `"observation"`,
    /// `"graph-event"`).
    pub source_log: String,
    /// The log length (next `Seq` to be assigned) at the moment this
    /// index was built.
    pub source_high_watermark: u64,
    pub built_at: String,
}
