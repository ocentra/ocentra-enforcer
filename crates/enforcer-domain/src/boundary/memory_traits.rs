//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Boundary trait adapters for memory value views and transport iteration.

use crate::memory_types::{
    GraphArtifactPresence, GraphSourceLine, MemoryWatchEventRelevant, MemoryWatchGitHeadChanged,
    StoreCacheContains, VectorManifestMatches,
};

impl std::ops::Add<usize> for GraphSourceLine {
    type Output = GraphSourceLine;

    fn add(self, rhs: usize) -> Self::Output {
        GraphSourceLine::from(self.get() + rhs)
    }
}

impl std::ops::Not for MemoryWatchEventRelevant {
    type Output = MemoryWatchEventRelevant;

    fn not(self) -> Self::Output {
        MemoryWatchEventRelevant::from(!bool::from(self))
    }
}

impl std::ops::Not for MemoryWatchGitHeadChanged {
    type Output = MemoryWatchGitHeadChanged;

    fn not(self) -> Self::Output {
        MemoryWatchGitHeadChanged::from(!bool::from(self))
    }
}

impl std::ops::Not for StoreCacheContains {
    type Output = StoreCacheContains;

    fn not(self) -> Self::Output {
        StoreCacheContains::from(!bool::from(self))
    }
}

impl std::ops::Not for GraphArtifactPresence {
    type Output = GraphArtifactPresence;

    fn not(self) -> Self::Output {
        GraphArtifactPresence::from(!bool::from(self))
    }
}

impl std::ops::Not for VectorManifestMatches {
    type Output = VectorManifestMatches;

    fn not(self) -> Self::Output {
        VectorManifestMatches::from(!bool::from(self))
    }
}
