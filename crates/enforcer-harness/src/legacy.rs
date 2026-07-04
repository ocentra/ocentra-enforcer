//! Half B — legacy dual-read migration [G3].
//!
//! Reads BOTH `.enforce` (authoritative) and the legacy `.ocentra-enforcer`
//! storage root, deduping by `runId` so pre-migration run history is never
//! lost. `.enforce` is authoritative for writes; the legacy root is
//! read-only here. Coordinated with arc-23 install/migration (arc-18 owns
//! the read/dedupe logic; arc-23 owns install-time copy/move). Mirrors
//! `candidateStorageRoots` in `src/harness.mjs`.

use std::path::{Path, PathBuf};

use crate::config::HarnessConfig;
use enforcer_core::error::Result;

/// Normalize an absolute path to a `/`-separated path relative to `root`.
/// Returns `.` for the root itself.
pub fn normalize_rel(root: &Path, target: &Path) -> String {
    let relative = target.strip_prefix(root).unwrap_or(target);
    if relative.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Every storage root to read from, authoritative-first: `.enforce`
/// (or the configured storage dir) then the legacy `.ocentra-enforcer`
/// root, deduped by path.
pub fn candidate_storage_roots(repo_root: &Path, config: &HarnessConfig) -> Result<Vec<PathBuf>> {
    let authoritative = config.storage_root(repo_root)?;
    let legacy = crate::config::legacy_storage_root(repo_root);
    let mut roots = vec![authoritative];
    if !roots.contains(&legacy) {
        roots.push(legacy);
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::candidate_storage_roots;
    use crate::config::HarnessConfig;
    use enforcer_core::error::Result;

    #[test]
    fn candidate_roots_lists_authoritative_before_legacy() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let roots = candidate_storage_roots(dir.path(), &HarnessConfig::default())?;
        assert_eq!(roots.len(), 2);
        assert!(roots[0].ends_with(".enforce"));
        assert!(roots[1].ends_with(".ocentra-enforcer"));
        Ok(())
    }
}
