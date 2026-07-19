//! The `ndjson-duckdb` store seam [G2].
//!
//! **Posture: DEFERRED.** NDJSON is authoritative; DuckDB ingestion is an
//! OPTIONAL seam this crate stamps but does not implement. Mirrors
//! `writeDuckDbStatus` in `src/harness.mjs`.

use std::path::Path;

use enforcer_core::error::Result;
use enforcer_domain::harness_types::{
    HarnessDiagnosticMessage, HarnessDiagnosticPath, HarnessDuckDbAvailability, HarnessDuckDbMode,
};

/// The `.enforce/db/duckdb-status.json` contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuckDbStatus {
    pub mode: HarnessDuckDbMode,
    pub availability: HarnessDuckDbAvailability,
    pub database: HarnessDiagnosticPath,
    pub detail: HarnessDiagnosticMessage,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DuckDbStatusDto {
    mode: String,
    available: bool,
    database: String,
    detail: String,
}

impl From<&DuckDbStatus> for DuckDbStatusDto {
    fn from(status: &DuckDbStatus) -> Self {
        Self {
            mode: status.mode.as_str().to_owned(),
            available: status.availability.as_bool(),
            database: status.database.as_str().to_owned(),
            detail: status.detail.as_str().to_owned(),
        }
    }
}

impl From<DuckDbStatusDto> for DuckDbStatus {
    fn from(status: DuckDbStatusDto) -> Self {
        Self {
            mode: HarnessDuckDbMode::Optional,
            availability: if status.available {
                HarnessDuckDbAvailability::Available
            } else {
                HarnessDuckDbAvailability::Deferred
            },
            database: HarnessDiagnosticPath::from_adapter(&status.database),
            detail: HarnessDiagnosticMessage::from_adapter(&status.detail),
        }
    }
}

/// Stamp `.enforce/db/duckdb-status.json` under `storage_root` (relative to
/// `repo_root` for the `database` path field), returning the written
/// status. `available` is always `false` — DuckDB ingestion is deferred;
/// NDJSON stays authoritative regardless of whether duckdb is installed.
pub fn write_duckdb_status(repo_root: &Path, storage_root: &Path) -> Result<DuckDbStatus> {
    let storage_rel = crate::legacy::normalize_rel(repo_root, storage_root);
    let status = DuckDbStatus {
        mode: HarnessDuckDbMode::Optional,
        availability: HarnessDuckDbAvailability::Deferred,
        database: HarnessDiagnosticPath::from_adapter(&format!(
            "{storage_rel}/db/harness.duckdb"
        )),
        detail: HarnessDiagnosticMessage::from_adapter(
            "DuckDB ingestion is reserved (deferred); NDJSON is authoritative when duckdb is not installed.",
        ),
    };
    let db_dir = storage_root.join("db");
    std::fs::create_dir_all(&db_dir)?;
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(&DuckDbStatusDto::from(&status))?
    );
    std::fs::write(db_dir.join("duckdb-status.json"), body)?;
    Ok(status)
}

pub(crate) fn status_wire_value(status: &DuckDbStatus) -> Result<serde_json::Value> {
    Ok(serde_json::to_value(DuckDbStatusDto::from(status))?)
}

#[cfg(test)]
mod tests {
    use super::write_duckdb_status;
    use enforcer_core::error::Result;
    use enforcer_domain::harness_types::{HarnessDuckDbAvailability, HarnessDuckDbMode};

    #[test]
    fn duckdb_status_wire_roundtrip_preserves_domain_status() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let repo_root = dir.path();
        let storage_root = repo_root.join(".enforce");
        std::fs::create_dir_all(&storage_root)?;
        let status = write_duckdb_status(repo_root, &storage_root)?;
        assert_eq!(status.mode, HarnessDuckDbMode::Optional);
        assert_eq!(status.availability, HarnessDuckDbAvailability::Deferred);
        assert!(status.database.as_str().ends_with("db/harness.duckdb"));

        let path = storage_root.join("db").join("duckdb-status.json");
        assert!(path.exists());
        let read_back: super::DuckDbStatusDto =
            serde_json::from_str(&std::fs::read_to_string(path)?)?;
        assert_eq!(super::DuckDbStatus::from(read_back), status);
        Ok(())
    }

    #[test]
    fn duckdb_status_dto_rejects_incomplete_wire_shape() {
        let invalid = r#"{"mode":"optional","available":false}"#;
        let decoded = serde_json::from_str::<super::DuckDbStatusDto>(invalid);
        assert_eq!(
            decoded.err().map(|error| error.classify()),
            Some(serde_json::error::Category::Data)
        );
    }
}
