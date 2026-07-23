//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Wire/request shapes emitted by the filesystem watcher.

use enforcer_domain::memory_types::MemoryWatchPath;

// ROUNDTRIP-TEST: tests/unit_watch.rs::reindex_request_maps_to_canonical_paths
// NEGATIVE-TEST: tests/unit_watch.rs::reindex_request_maps_to_canonical_paths covers an empty batch.

/// One debounced batch of changed repo-relative-or-absolute paths.
///
/// The watcher owns the batching semantics; this boundary type keeps the
/// emitted request shape explicit without making the watcher module a domain
/// DTO container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReindexRequest {
    pub paths: Vec<MemoryWatchPath>,
}

impl From<ReindexRequest> for Vec<MemoryWatchPath> {
    fn from(value: ReindexRequest) -> Self {
        value.paths
    }
}
