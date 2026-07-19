//! Memory path decoding at the filesystem boundary.

use crate::error::{MemoryError, Result};
use enforcer_domain::memory_types::MemoryPathInput;

/// Validate + normalize a raw path string into a repo-relative
/// [`enforcer_domain::paths::RelPath`], translating a decode failure into
/// this crate's [`MemoryError::InvalidPath`] rather than leaking the
/// `enforcer_core` decode error type across the crate boundary.
pub fn rel_path(raw: &MemoryPathInput) -> Result<enforcer_domain::paths::RelPath> {
    raw.as_str()
        .parse()
        .map_err(|source| MemoryError::InvalidPath {
            path: std::path::Path::new(raw.as_str()).into(),
            source,
        })
}

/// Validate + normalize a raw path string into an
/// [`enforcer_domain::paths::RepoRoot`].
pub fn repo_root(raw: &MemoryPathInput) -> Result<enforcer_domain::paths::RepoRoot> {
    raw.as_str()
        .parse()
        .map_err(|source| MemoryError::InvalidPath {
            path: std::path::Path::new(raw.as_str()).into(),
            source,
        })
}
