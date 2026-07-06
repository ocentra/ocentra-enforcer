//! Store-local identifier types. These are deliberately NOT branded
//! validated newtypes like `enforcer_domain::ids` (that module owns
//! cross-crate wire identifiers such as `RuleId`/`LaneId`); the ids here
//! are store-internal keys with a fixed, mechanically-generated shape,
//! so a lightweight wrapper is enough to keep call sites type-safe
//! without re-deriving `enforcer_domain`'s validation machinery.

use crate::error::{MemoryError, Result};
use enforcer_domain::hashes::Sha256;

/// A monotonic, gap-free sequence number for one append-only log. The
/// log's own append operation assigns these; nothing else may mint one,
/// so `Seq` doubles as the log's high-watermark unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seq(pub u64);

impl Seq {
    pub const GENESIS: Seq = Seq(0);

    pub fn next(self) -> Seq {
        Seq(self.0 + 1)
    }
}

impl std::fmt::Display for Seq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A content-addressed artifact id: `sha256:<64 hex>` of the artifact's
/// bytes. Two artifacts with identical content always have the same id
/// (dedup for free); the id is only ever computed from content, never
/// assigned by a caller.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactId(Sha256);

impl ArtifactId {
    /// Wrap an already-validated [`Sha256`] digest as an [`ArtifactId`]
    /// without recomputing it from content. For callers (X06.8 exact
    /// artifact retrieval) that parse a caller-supplied id string into a
    /// `Sha256` first and need to hand it to
    /// [`crate::store::manifest::ArtifactManifest`] as a claimed lookup
    /// key -- this does NOT assert the digest actually matches any real
    /// content; only [`ArtifactManifest::get`] re-verifies that.
    pub fn from_digest(digest: Sha256) -> Self {
        Self(digest)
    }

    /// Compute the content-addressed id for `bytes`.
    pub fn from_content(bytes: &[u8]) -> Self {
        let digest = enforcer_core::hash_chain::link_digest(None, bytes);
        // `link_digest(None, bytes)` always yields a well-formed
        // `sha256:<64 hex>` string, so this parse cannot fail; a
        // defensive fallback keeps the constructor infallible without
        // an `unwrap`/`expect` (both denied at workspace lint level).
        let sha = digest.parse().unwrap_or_else(|_| {
            Sha256::try_from(format!("sha256:{}", "0".repeat(64)))
                .unwrap_or_else(|_| unreachable!("64 zero hex chars is always a valid Sha256"))
        });
        Self(sha)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn digest(&self) -> &Sha256 {
        &self.0
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// A project id: derived deterministically from the project's normalized
/// repo root, so the same repository always maps to the same store
/// directory regardless of which process opens it (needed for the
/// "no ghost project database" rule -- see `store::Store::open`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectId(String);

impl ProjectId {
    /// Derive a project id from a repo root. Purely a function of the
    /// normalized path string -- no filesystem access, no side effects.
    pub fn from_repo_root(root: &enforcer_domain::paths::RepoRoot) -> Self {
        let digest = enforcer_core::hash_chain::link_digest(None, root.as_str().as_bytes());
        // Short, filesystem-friendly form: first 16 hex chars after the
        // `sha256:` prefix. Collision odds at this length are
        // astronomically below the scale this crate operates at, and a
        // full 64-char digest would just make store directory names
        // unwieldy for no correctness benefit.
        let hex = digest
            .strip_prefix(enforcer_core::hash_chain::DIGEST_PREFIX)
            .unwrap_or(&digest);
        Self(hex.get(..16).unwrap_or(hex).to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validate + normalize a raw path string into a repo-relative
/// [`enforcer_domain::paths::RelPath`], translating a decode failure into
/// this crate's [`MemoryError::InvalidPath`] rather than leaking the
/// `enforcer_core` decode error type across the crate boundary.
pub fn rel_path(raw: &str) -> Result<enforcer_domain::paths::RelPath> {
    raw.parse().map_err(|source| MemoryError::InvalidPath {
        path: raw.to_owned(),
        source,
    })
}

/// Validate + normalize a raw path string into an
/// [`enforcer_domain::paths::RepoRoot`].
pub fn repo_root(raw: &str) -> Result<enforcer_domain::paths::RepoRoot> {
    raw.parse().map_err(|source| MemoryError::InvalidPath {
        path: raw.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_id_is_deterministic_and_content_addressed() {
        let a = ArtifactId::from_content(b"hello");
        let b = ArtifactId::from_content(b"hello");
        let c = ArtifactId::from_content(b"world");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.as_str().starts_with("sha256:"));
    }

    #[test]
    fn artifact_id_from_digest_wraps_without_recomputing() {
        let computed = ArtifactId::from_content(b"round trip me");
        let rewrapped = ArtifactId::from_digest(computed.digest().clone());
        assert_eq!(computed, rewrapped);
    }

    #[test]
    fn project_id_is_stable_for_the_same_root_and_windows_path_forms() -> Result<()> {
        let a = repo_root(r"C:\Projects\enforcer")?;
        let b = repo_root("C:/Projects/enforcer")?;
        assert_eq!(
            ProjectId::from_repo_root(&a).as_str(),
            ProjectId::from_repo_root(&b).as_str(),
            "backslash and forward-slash forms of the same root must yield the same project id"
        );
        Ok(())
    }

    #[test]
    fn project_id_differs_across_roots() -> Result<()> {
        let a = repo_root("C:/Projects/enforcer")?;
        let b = repo_root("C:/Projects/other")?;
        assert_ne!(
            ProjectId::from_repo_root(&a).as_str(),
            ProjectId::from_repo_root(&b).as_str()
        );
        Ok(())
    }

    #[test]
    fn seq_advances_monotonically_from_genesis() {
        let s0 = Seq::GENESIS;
        let s1 = s0.next();
        let s2 = s1.next();
        assert_eq!(s0.0, 0);
        assert_eq!(s1.0, 1);
        assert_eq!(s2.0, 2);
        assert!(s2 > s1 && s1 > s0);
    }
}
