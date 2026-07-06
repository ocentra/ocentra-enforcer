//! X06.8: exact artifact/snippet retrieval.
//!
//! [`crate::store::manifest::ArtifactManifest`] already gives us a
//! content-addressed blob store (`put`/`get` keyed by the artifact's own
//! SHA-256 digest). This module adds the query-surface contract the
//! workpack's "exact artifact/snippet retrieval" hard requirement
//! describes on top of it:
//!
//! - **exact match only**: [`get_exact`] either returns the artifact
//!   whose id is byte-for-byte the requested id, or a typed
//!   [`ArtifactLookupError`] -- it NEVER falls back to a
//!   similar/fuzzy/nearest artifact. A caller that asks for an id that
//!   does not exist gets [`ArtifactLookupError::NotFound`], never a
//!   "close enough" substitute; this is the fail-closed contract the
//!   hard-test list calls "wrong-id fail-closed".
//! - **traversal rejection**: any requested id or `rel_path` hint that
//!   looks like a filesystem escape attempt (`../`, backslash-form
//!   `..\`, an absolute path, or a NUL byte) is rejected with
//!   [`ArtifactLookupError::TraversalRejected`] before ever touching the
//!   manifest or the filesystem -- the manifest's own ids are always
//!   `sha256:<64 hex>` (see [`crate::ids::ArtifactId`]) so a
//!   traversal-shaped string can never legitimately be one, but the
//!   check is mechanical and runs unconditionally rather than relying on
//!   that invariant holding at every call site forever.
//!
//! Snippets are handled identically: [`get_snippet_exact`] treats a
//! snippet as an artifact like any other (the caller is expected to have
//! stored the snippet's bytes through [`crate::store::manifest::ArtifactManifest::put`]
//! the same way it stores any other artifact), so there is exactly one
//! exact-match code path for both, not two.

use crate::ids::ArtifactId;
use crate::store::manifest::ArtifactManifest;

/// Exact-artifact-lookup failures. Deliberately narrower than
/// [`crate::error::MemoryError`] (which also covers this crate's
/// store/log machinery generally) -- this is the fail-closed surface
/// callers of the retrieval API match on.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactLookupError {
    /// The requested id is well-formed but no artifact with that exact
    /// content-address exists in the manifest. NEVER substituted with a
    /// similar artifact -- the caller must treat this as "does not
    /// exist", not retry with fuzzy matching.
    #[error("no artifact with exact id {id:?} exists -- exact lookup never falls back to a similar artifact")]
    NotFound { id: String },

    /// The requested id or an accompanying path hint is shaped like a
    /// filesystem traversal attempt (`../`, `..\`, an absolute path, or
    /// embeds a NUL byte). Rejected before any manifest/filesystem
    /// access.
    #[error("rejected traversal-shaped artifact reference {raw:?}")]
    TraversalRejected { raw: String },

    /// The artifact's on-disk bytes no longer hash to the id the
    /// manifest recorded for them (propagated from
    /// [`crate::error::MemoryError::ArtifactDigestMismatch`]).
    #[error("artifact {id} failed integrity verification: {source}")]
    Corrupt {
        id: String,
        #[source]
        source: crate::error::MemoryError,
    },
}

/// Reject any raw artifact reference (an id string, or a `rel_path`
/// hint) that is shaped like a filesystem traversal attempt. This is a
/// pure string check with no filesystem access, run BEFORE the value is
/// used to build any path or manifest key.
///
/// Rejects: `..` path segments (both `/`-and `\`-delimited), a leading
/// `/` or `\` (repo-root-escaping absolute-style reference), a Windows
/// drive-letter absolute path (`C:`-shaped prefix), and any embedded NUL
/// byte.
fn reject_traversal(raw: &str) -> Result<(), ArtifactLookupError> {
    let is_traversal = raw.contains("..")
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.contains('\0')
        || raw
            .get(1..2)
            .is_some_and(|colon| colon == ":" && raw.chars().next().is_some_and(|c| c.is_ascii_alphabetic()));
    if is_traversal {
        return Err(ArtifactLookupError::TraversalRejected {
            raw: raw.to_owned(),
        });
    }
    Ok(())
}

/// Parse `raw` into an [`ArtifactId`] for an exact-match lookup. Unlike
/// content-derived ids (which are only ever produced by
/// [`ArtifactId::from_content`]), this is a caller-supplied string being
/// treated as a claimed id -- it is validated as `sha256:<64 hex>`
/// shaped, distinct from (and run after) the traversal check, so a
/// malformed id is reported as [`ArtifactLookupError::NotFound`] (it can
/// never match anything in the manifest) rather than panicking or
/// silently truncating.
fn parse_claimed_id(raw: &str) -> Result<ArtifactId, ArtifactLookupError> {
    reject_traversal(raw)?;
    let sha: enforcer_domain::hashes::Sha256 =
        raw.parse().map_err(|_| ArtifactLookupError::NotFound {
            id: raw.to_owned(),
        })?;
    Ok(ArtifactId::from_digest(sha))
}

/// Exact-match artifact retrieval by content-addressed id. Returns the
/// artifact's raw bytes on an exact hit; every other outcome (unknown
/// id, malformed id, traversal-shaped id, corrupted blob) is a distinct
/// typed error -- never a substituted "similar" artifact.
pub fn get_exact(manifest: &ArtifactManifest, raw_id: &str) -> Result<Vec<u8>, ArtifactLookupError> {
    let id = parse_claimed_id(raw_id)?;
    if manifest.entry(&id).is_none() {
        return Err(ArtifactLookupError::NotFound {
            id: raw_id.to_owned(),
        });
    }
    manifest.get(&id).map_err(|source| match source {
        crate::error::MemoryError::Io { .. } => ArtifactLookupError::NotFound {
            id: raw_id.to_owned(),
        },
        other => ArtifactLookupError::Corrupt {
            id: raw_id.to_owned(),
            source: other,
        },
    })
}

/// Exact-match snippet retrieval. A snippet is an artifact like any
/// other in this manifest -- same content-address, same fail-closed
/// exact-match contract, same traversal rejection. Kept as a separate
/// named entry point (rather than requiring every caller to know
/// snippets and artifacts share a code path) because the workpack names
/// "artifact/snippet retrieval" as one hard requirement with two nouns.
pub fn get_snippet_exact(
    manifest: &ArtifactManifest,
    raw_id: &str,
) -> Result<Vec<u8>, ArtifactLookupError> {
    get_exact(manifest, raw_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "enforcer-memory-artifacts-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn exact_id_returns_exact_content() {
        let root = temp_dir("exact-hit");
        let mut manifest = ArtifactManifest::open(&root).expect("open manifest");
        let id = manifest
            .put(b"hello artifact", Some("a.txt"), "2026-07-05T00:00:00Z")
            .expect("put");

        let content = get_exact(&manifest, id.as_str()).expect("exact lookup");
        assert_eq!(content, b"hello artifact");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wrong_id_is_fail_closed_not_similar() {
        let root = temp_dir("wrong-id");
        let mut manifest = ArtifactManifest::open(&root).expect("open manifest");
        // Store TWO artifacts so a "similar" fallback would have
        // something plausible to (wrongly) return if it existed.
        manifest
            .put(b"artifact one", Some("a.txt"), "2026-07-05T00:00:00Z")
            .expect("put a");
        manifest
            .put(b"artifact two", Some("b.txt"), "2026-07-05T00:00:01Z")
            .expect("put b");

        // A well-formed but unknown id.
        let unknown = format!("sha256:{}", "ab".repeat(32));
        let outcome = get_exact(&manifest, &unknown);
        assert!(
            matches!(outcome, Err(ArtifactLookupError::NotFound { .. })),
            "unknown exact id must fail closed, never substitute a similar artifact"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn malformed_id_is_not_found_not_panic() {
        let root = temp_dir("malformed-id");
        let manifest = ArtifactManifest::open(&root).expect("open manifest");
        let outcome = get_exact(&manifest, "not-a-real-id");
        assert!(matches!(outcome, Err(ArtifactLookupError::NotFound { .. })));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn traversal_shaped_ids_are_rejected() {
        let root = temp_dir("traversal");
        let manifest = ArtifactManifest::open(&root).expect("open manifest");

        let cases = [
            "../../etc/passwd",
            "..\\..\\windows\\system32",
            "/etc/passwd",
            "\\windows\\system32",
            "C:\\Windows\\System32\\config",
            "sha256:..\u{0}deadbeef",
        ];
        for raw in cases {
            let outcome = get_exact(&manifest, raw);
            assert!(
                matches!(outcome, Err(ArtifactLookupError::TraversalRejected { .. })),
                "expected traversal rejection for {raw:?}, got {outcome:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn snippet_exact_shares_the_same_fail_closed_contract() {
        let root = temp_dir("snippet");
        let mut manifest = ArtifactManifest::open(&root).expect("open manifest");
        let id = manifest
            .put(b"fn snippet() {}", Some("snip.rs"), "2026-07-05T00:00:00Z")
            .expect("put");
        let content = get_snippet_exact(&manifest, id.as_str()).expect("exact snippet lookup");
        assert_eq!(content, b"fn snippet() {}");

        let outcome = get_snippet_exact(&manifest, "../escape");
        assert!(matches!(
            outcome,
            Err(ArtifactLookupError::TraversalRejected { .. })
        ));
        let _ = std::fs::remove_dir_all(&root);
    }
}
