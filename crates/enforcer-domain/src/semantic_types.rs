//! Typed contracts for bounded, read-only semantic graph observations.
//!
//! This module owns only the cross-crate shape. It does not open a graph
//! store, traverse a graph, invoke a process, or decide a rule outcome.

use crate::hashes::Sha256;
use crate::memory_types::{
    CommitId, GraphArtifactSchemaVersion, GraphEdgeCount, GraphNodeCount, ImpactSignal,
};
use crate::paths::RelPath;

/// Closed freshness/result state for one graph predicate observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphFactState {
    /// The provider returned a complete observation for the requested scope.
    Complete,
    /// The provider returned only a bounded subset of the requested graph.
    Partial,
    /// The provider data was built from a different source commit.
    Stale,
    /// The provider detected a cycle it could not safely resolve.
    Cyclic,
    /// No trustworthy provider result was available.
    Unavailable,
}

/// The single provider kind implemented by the UL13 packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphFactProviderKind {
    /// The repository-owned in-process `CodeGraph` read model.
    CodeGraph,
}

/// One bounded input for a route-coverage graph predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPredicateInput {
    /// Repository-relative path whose upstream graph impact is inspected.
    pub changed_path: RelPath,
    /// Source commit the caller expects the provider snapshot to represent.
    pub expected_commit: CommitId,
}

impl GraphPredicateInput {
    /// Construct an input from already validated path and commit values.
    pub const fn new(changed_path: RelPath, expected_commit: CommitId) -> Self {
        Self {
            changed_path,
            expected_commit,
        }
    }
}

/// Provenance-bearing result returned by one graph predicate provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPredicateEvidence {
    /// Version of the serialized graph fact contract used by the provider.
    pub schema_version: GraphArtifactSchemaVersion,
    /// Provider that produced this observation.
    pub provider: GraphFactProviderKind,
    /// Commit represented by the provider snapshot.
    pub source_commit: CommitId,
    /// Digest of the graph snapshot or immutable graph artifact.
    pub graph_digest: Sha256,
    /// Trust/freshness state for this observation.
    pub state: GraphFactState,
    /// Number of route files in the bounded upstream impact set.
    pub route_file_count: GraphNodeCount,
    /// Number of test nodes in the bounded upstream impact set.
    pub covering_test_count: GraphNodeCount,
    /// Number of graph edges considered by the provider snapshot.
    pub edge_count: GraphEdgeCount,
    /// Whether the provider found at least one test covering the impact set.
    pub has_test_coverage: ImpactSignal,
}

/// Read-only provider seam consumed by semantic predicates.
pub trait GraphFactProvider: Send + Sync {
    /// Evaluate the bounded route-coverage input without mutating storage.
    fn route_coverage(&self, input: &GraphPredicateInput) -> GraphPredicateEvidence;
}
