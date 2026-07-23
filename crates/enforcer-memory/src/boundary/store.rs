//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Serialized store marker DTOs.
//!
//! BOUNDARY-INVARIANT: persisted marker text is contained in these DTOs and
//! converted to canonical project identities before it influences store ownership.

use serde::{Deserialize, Serialize};

use enforcer_domain::memory_types::{
    MemoryProjectId, MemoryProjectInitializedAt, MemoryProjectRepoRoot, MemoryStoreMarkerProjectId,
};

// ROUNDTRIP-TEST: unit_store_sqlite::store_marker_survives_reopen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoreMarkerDto {
    pub(crate) schema_version: u32,
    pub(crate) project_id: MemoryStoreMarkerProjectId,
    pub(crate) repo_root: String,
    pub(crate) initialized_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProjectStoreMarkerDto {
    pub(crate) project_id: MemoryProjectId,
    pub(crate) repo_root: MemoryProjectRepoRoot,
    pub(crate) initialized_at: MemoryProjectInitializedAt,
}
