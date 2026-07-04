//! Scan modes (f01): the caller-facing "what kind of run is this" surface
//! that selects a [`crate::scope::ScopeRequest`] and any mode-specific
//! post-processing (e.g. whether the baseline-ratchet applies).
//!
//! **SKELETON BOUNDARY**: arc-15 owns this module (`src/modes.rs`) as a
//! skeleton — the [`ScanMode`] enum and [`request_for_mode`] mapping each
//! mode to its [`crate::scope::ScopeRequest`]. Richer per-mode behavior
//! (CI-specific diff-base inference, watch-mode incremental re-scan,
//! mode-specific config layering) is owned by f01's own feature packs,
//! `deps: arc-15`.

use crate::scope::{CommitRef, ScopeRequest};

/// The scan modes this engine supports, corresponding 1:1 to the
/// tri-modal scope resolver's three [`ScopeRequest`] variants, plus the
/// caller intent each mode implies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanMode {
    /// Scan exactly the given files/directories.
    Files(Vec<std::path::PathBuf>),
    /// Scan only what changed between two commit-ish endpoints (the
    /// typical CI/PR-gate mode).
    Diff {
        /// Older endpoint of the range.
        base: CommitRef,
        /// Newer endpoint of the range.
        head: CommitRef,
    },
    /// Scan the whole tree (the typical full/nightly/self-scan mode).
    All,
}

/// Map a [`ScanMode`] to the [`ScopeRequest`] the tri-modal resolver
/// consumes. A 1:1, total, non-lossy mapping — every mode maps to exactly
/// one scope request and vice versa is recoverable from the request
/// shape.
pub fn request_for_mode(mode: ScanMode) -> ScopeRequest {
    match mode {
        ScanMode::Files(paths) => ScopeRequest::Paths(paths),
        ScanMode::Diff { base, head } => ScopeRequest::Diff { base, head },
        ScanMode::All => ScopeRequest::All,
    }
}

#[cfg(test)]
mod tests {
    use super::{request_for_mode, ScanMode};
    use crate::scope::ScopeRequest;
    use std::path::PathBuf;

    #[test]
    fn files_mode_maps_to_paths_request() {
        let mode = ScanMode::Files(vec![PathBuf::from("src/lib.rs")]);
        let request = request_for_mode(mode);
        assert!(matches!(request, ScopeRequest::Paths(paths) if paths.len() == 1));
    }

    #[test]
    fn all_mode_maps_to_all_request() {
        assert_eq!(request_for_mode(ScanMode::All), ScopeRequest::All);
    }
}
