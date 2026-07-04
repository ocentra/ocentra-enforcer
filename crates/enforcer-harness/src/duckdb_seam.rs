//! The `ndjson-duckdb` store seam [G2].
//!
//! **Posture: DEFERRED.** NDJSON is authoritative; DuckDB ingestion is an
//! OPTIONAL seam this crate stamps but does not implement. Mirrors
//! `writeDuckDbStatus` in `src/harness.mjs`.

use std::path::Path;

use enforcer_core::error::Result;

/// The `.enforce/db/duckdb-status.json` contract.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DuckDbStatus {
    pub mode: String,
    pub available: bool,
    pub database: String,
    pub detail: String,
}

/// Stamp `.enforce/db/duckdb-status.json` under `storage_root` (relative to
/// `repo_root` for the `database` path field), returning the written
/// status. `available` is always `false` — DuckDB ingestion is deferred;
/// NDJSON stays authoritative regardless of whether duckdb is installed.
pub fn write_duckdb_status(repo_root: &Path, storage_root: &Path) -> Result<DuckDbStatus> {
    let storage_rel = crate::legacy::normalize_rel(repo_root, storage_root);
    let status = DuckDbStatus {
        mode: "optional".to_owned(),
        available: false,
        database: format!("{storage_rel}/db/harness.duckdb"),
        detail: "DuckDB ingestion is reserved (deferred); NDJSON is authoritative when duckdb is not installed."
            .to_owned(),
    };
    let db_dir = storage_root.join("db");
    std::fs::create_dir_all(&db_dir)?;
    let body = format!("{}\n", serde_json::to_string_pretty(&status)?);
    std::fs::write(db_dir.join("duckdb-status.json"), body)?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::write_duckdb_status;
    use enforcer_core::error::Result;

    #[test]
    fn stamps_available_false_and_ndjson_authoritative_detail() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let repo_root = dir.path();
        let storage_root = repo_root.join(".enforce");
        std::fs::create_dir_all(&storage_root)?;
        let status = write_duckdb_status(repo_root, &storage_root)?;
        assert_eq!(status.mode, "optional");
        assert!(!status.available);
        assert!(status.database.ends_with("db/harness.duckdb"));

        let path = storage_root.join("db").join("duckdb-status.json");
        assert!(path.exists());
        let read_back: super::DuckDbStatus = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        assert_eq!(read_back, status);
        Ok(())
    }
}
