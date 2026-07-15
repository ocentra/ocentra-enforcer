//! Serde-only representation of the append-only memory-record wire format.

use serde::{Deserialize, Serialize};

use crate::record::{Evidence, Provenance, RecordDomain, RecordKind};

/// The JSON/NDJSON payload accepted at memory's transport boundary.
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

impl crate::record::MemoryRecord {
    /// Convert a payload at the NDJSON or signed-bundle boundary before it
    /// enters the local graph.
    pub fn from_dto(dto: MemoryRecordDto) -> Self {
        Self { dto }
    }

    /// Clone a DTO only when exporting through a wire boundary.
    pub fn to_dto(&self) -> MemoryRecordDto {
        self.dto.clone()
    }

    pub fn id(&self) -> &str { &self.dto.id }
    pub fn kind(&self) -> RecordKind { self.dto.kind }
    pub fn domain(&self) -> RecordDomain { self.dto.domain }
    pub fn statement(&self) -> &str { &self.dto.statement }
    pub fn why(&self) -> Option<&str> { self.dto.why.as_deref() }
    pub fn how_to_apply(&self) -> Option<&str> { self.dto.how_to_apply.as_deref() }
    pub fn evidence(&self) -> Option<&Evidence> { self.dto.evidence.as_ref() }
    pub fn landed_at(&self) -> &[String] { &self.dto.landed_at }
    pub fn supersedes(&self) -> Option<&str> { self.dto.supersedes.as_deref() }
    pub fn provenance(&self) -> &Provenance { &self.dto.provenance }

    pub fn clear_landed_at(&mut self) { self.dto.landed_at.clear(); }

    pub fn searchable_text(&self) -> String {
        [Some(self.statement()), self.why(), self.how_to_apply()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" \n ")
    }
}

impl From<MemoryRecordDto> for crate::record::MemoryRecord {
    fn from(dto: MemoryRecordDto) -> Self {
        Self::from_dto(dto)
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryRecordDto;

    #[test]
    fn record_dto_roundtrip_preserves_the_external_wire_shape(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"{"schemaVersion":1,"id":"mem-1","ts":"2026-07-14T00:00:00Z","kind":"lesson","domain":"harness","statement":"Use the local gate.","provenance":{"writer":"primary"}}"#;
        let dto: MemoryRecordDto = serde_json::from_str(source)?;
        let serialized = serde_json::to_string(&dto)?;
        let reparsed: MemoryRecordDto = serde_json::from_str(&serialized)?;
        assert_eq!(reparsed, dto);
        Ok(())
    }

    #[test]
    /// Negative wire input must fail before it can become a domain record.
    fn negative_record_dto_rejects_missing_required_wire_fields() {
        let missing_statement =
            r#"{"schemaVersion":1,"id":"mem-1","ts":"2026-07-14T00:00:00Z","kind":"lesson","domain":"harness","provenance":{"writer":"primary"}}"#;
        assert!(serde_json::from_str::<MemoryRecordDto>(missing_statement).is_err());
    }
}
