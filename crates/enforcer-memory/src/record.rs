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

/// One `EnforcerMemoryRecord` wire payload, mirroring the JSON Schema exactly.
///
/// This is deliberately a transport type.  It may cross the NDJSON and signed
/// bundle boundaries, but must be converted before it enters the memory graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecordDto {
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

/// A record accepted into the local memory domain.
///
/// The JSON shape belongs to [`MemoryRecordDto`].  Keeping that payload behind
/// this domain value prevents graph, learning, recall and redaction code from
/// accidentally treating an externally supplied DTO as already-trusted domain
/// state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRecord {
    dto: MemoryRecordDto,
}

impl MemoryRecord {
    /// Convert a wire payload after the caller has crossed its input boundary.
    pub fn from_dto(dto: MemoryRecordDto) -> Self {
        Self { dto }
    }

    /// Convert this domain value to a wire payload at an explicit output
    /// boundary (NDJSON or signed bundle serialization).
    pub fn to_dto(&self) -> MemoryRecordDto {
        self.dto.clone()
    }

    pub fn id(&self) -> &str { &self.dto.id }
    pub fn kind(&self) -> RecordKind { self.dto.kind }
    pub fn domain(&self) -> RecordDomain { self.dto.domain }
    pub fn statement(&self) -> &str { &self.dto.statement }
    pub fn why(&self) -> Option<&str> { self.dto.why.as_deref() }
    pub fn how_to_apply(&self) -> Option<&str> { self.dto.how_to_apply.as_deref() }
    pub fn applies_to(&self) -> &[String] { &self.dto.applies_to }
    pub fn evidence(&self) -> Option<&Evidence> { self.dto.evidence.as_ref() }
    pub fn routes(&self) -> &[String] { &self.dto.routes }
    pub fn landed_at(&self) -> &[String] { &self.dto.landed_at }
    pub fn supersedes(&self) -> Option<&str> { self.dto.supersedes.as_deref() }
    pub fn provenance(&self) -> &Provenance { &self.dto.provenance }

    pub fn with_dto_mutated(&self, mutate: impl FnOnce(&mut MemoryRecordDto)) -> Self {
        let mut dto = self.to_dto();
        mutate(&mut dto);
        Self::from_dto(dto)
    }

    pub fn clear_landed_at(&mut self) { self.dto.landed_at.clear(); }

    /// Text this record exposes to keyword recall: statement + why +
    /// howToApply, concatenated so a single query can match any of them.
    pub fn searchable_text(&self) -> String {
        let mut parts = vec![self.statement().to_owned()];
        if let Some(why) = self.why() {
            parts.push(why.to_owned());
        }
        if let Some(how) = self.how_to_apply() {
            parts.push(how.to_owned());
        }
        parts.join(" \n ")
    }
}

impl From<MemoryRecordDto> for MemoryRecord {
    fn from(dto: MemoryRecordDto) -> Self {
        Self::from_dto(dto)
    }
}
