//! Domain values for records accepted into the local memory graph.

use serde::{Deserialize, Serialize};

/// `kind` enum from the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordKind {
    UserPref,
    Lesson,
    Decision,
    Observation,
    Incident,
}

/// `domain` enum from the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordDomain {
    Harness,
    Code,
    User,
}

/// `evidence` object from the schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
}

/// `provenance` object from the schema.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub writer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// A record accepted into the local memory domain.
///
/// The JSON shape belongs to [`crate::boundary::record::MemoryRecordDto`]. Keeping that payload behind
/// this domain value prevents graph, learning, recall and redaction code from
/// accidentally treating an externally supplied DTO as already-trusted domain
/// state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    pub(crate) dto: crate::boundary::record::MemoryRecordDto,
}
