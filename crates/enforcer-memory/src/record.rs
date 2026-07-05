//! Wire types for one append-only memory record, mirroring
//! `memory/schema/memory-record.schema.json`.
//!
//! Deserialization is intentionally permissive on unknown-but-documented
//! optional fields and strict on the required set: a record missing
//! `schemaVersion`/`id`/`ts`/`kind`/`domain`/`statement`/`provenance` is a
//! corrupt line and ingestion must reject it, not silently drop fields.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// One `EnforcerMemoryRecord` line, mirroring the JSON Schema exactly for
/// the fields this crate consumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecord {
    pub schema_version: u32,
    pub id: String,
    pub ts: String,
    pub kind: RecordKind,
    pub domain: RecordDomain,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub how_to_apply: Option<String>,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub landed_at: Vec<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
    pub provenance: Provenance,
}

impl MemoryRecord {
    /// Text this record exposes to keyword recall: statement + why +
    /// howToApply, concatenated so a single query can match any of them.
    pub fn searchable_text(&self) -> String {
        let mut parts = vec![self.statement.clone()];
        if let Some(why) = &self.why {
            parts.push(why.clone());
        }
        if let Some(how) = &self.how_to_apply {
            parts.push(how.clone());
        }
        parts.join(" \n ")
    }
}
