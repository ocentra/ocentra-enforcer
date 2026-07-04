//! The tri-modal scope resolver: `<paths...>` | `--base <sha> --head <sha>`
//! | `--all` → a canonical [`ResolvedScope`] wrapping
//! `enforcer_domain::findings::ScanScope`.
//!
//! Exactly one mode is ever active — there is deliberately NO override
//! flag that lets a caller combine modes or force a fourth path. The three
//! [`ScopeRequest`] variants are the entire input surface; [`resolve`]
//! turns whichever one the caller built into the concrete file set the
//! walker ([`crate::walk`]) enumerates.
//!
//! Windows-first: every path is normalized through
//! `enforcer_core::platform::normalize_separators` (backslash → forward
//! slash) before it is compared, sorted, or stored, so argv-quoting
//! differences between `cmd.exe`, PowerShell, and POSIX shells never
//! produce a different resolved scope for the same logical input.

use std::path::PathBuf;

use enforcer_core::error::DecodeError;
use enforcer_core::platform::normalize_separators;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::{RelPath, RepoRoot};

/// One git commit-ish endpoint of a `--base <sha> --head <sha>` diff range.
/// Deliberately a plain validated string (not a `Sha256` brand — a git
/// "commit-ish" may be an abbreviated sha, a branch name, or `HEAD~1`,
/// none of which are full SHA-256 hex digests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRef(String);

impl CommitRef {
    /// View the raw commit-ish string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for CommitRef {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DecodeError::new("scope.commitRef", "must not be empty"));
        }
        Ok(Self(trimmed.to_owned()))
    }
}

/// The caller's requested scope, before resolution against the repo tree.
/// Exactly one variant is ever constructed for a given invocation — there
/// is no combinator that merges two of these into a fourth mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeRequest {
    /// An explicit list of paths (files or directories), as given on the
    /// command line, not yet normalized or walked.
    Paths(Vec<PathBuf>),
    /// A git-diff range between two commit-ish endpoints.
    Diff {
        /// The range's older endpoint.
        base: CommitRef,
        /// The range's newer endpoint.
        head: CommitRef,
    },
    /// The whole tree rooted at the resolved [`RepoRoot`].
    All,
}

/// The canonical, resolved scope: the [`ScanScope`] discriminant
/// (`enforcer-domain`'s wire-facing enum) plus the concrete root/paths/diff
/// endpoints the walker needs to actually enumerate files. Two
/// [`ScopeRequest`]s that mean the same thing (e.g. the same path spelled
/// with `\` vs `/`) resolve to an equal `ResolvedScope` — this equality is
/// what the idempotency guard in [`crate::walk`] relies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    /// The wire-facing scope discriminant this resolution maps to.
    pub kind: ScanScope,
    /// Repository root every path in this scope is relative to.
    pub repo_root: RepoRoot,
    /// For [`ScopeRequest::Paths`]: the normalized, de-duplicated,
    /// lexicographically sorted repo-relative paths. Empty for the other
    /// two modes (the walker resolves their file sets structurally).
    pub explicit_paths: Vec<RelPath>,
    /// For [`ScopeRequest::Diff`]: the resolved base/head pair. `None` for
    /// the other two modes.
    pub diff_range: Option<(CommitRef, CommitRef)>,
}

/// Resolve a [`ScopeRequest`] against a repository root into a canonical
/// [`ResolvedScope`]. Pure and deterministic: the same `(request,
/// repo_root)` pair always resolves to an equal `ResolvedScope`, which is
/// the contract [`crate::walk`]'s idempotency guard is built on.
///
/// # Errors
/// Returns [`DecodeError`] if an explicit path does not normalize to a
/// valid repo-relative [`RelPath`] (absolute-outside-root, `..` escape,
/// empty).
pub fn resolve(request: &ScopeRequest, repo_root: &RepoRoot) -> Result<ResolvedScope, DecodeError> {
    match request {
        ScopeRequest::Paths(paths) => {
            let mut explicit_paths = Vec::with_capacity(paths.len());
            for path in paths {
                let normalized = normalize_separators(&path.to_string_lossy());
                let rel = to_repo_relative(&normalized, repo_root)?;
                explicit_paths.push(rel);
            }
            explicit_paths.sort();
            explicit_paths.dedup();
            Ok(ResolvedScope {
                kind: ScanScope::Files,
                repo_root: repo_root.clone(),
                explicit_paths,
                diff_range: None,
            })
        }
        ScopeRequest::Diff { base, head } => Ok(ResolvedScope {
            kind: ScanScope::Diff,
            repo_root: repo_root.clone(),
            explicit_paths: Vec::new(),
            diff_range: Some((base.clone(), head.clone())),
        }),
        ScopeRequest::All => Ok(ResolvedScope {
            kind: ScanScope::Workspace,
            repo_root: repo_root.clone(),
            explicit_paths: Vec::new(),
            diff_range: None,
        }),
    }
}

/// Normalize a caller-given path (absolute or relative, either separator
/// style) down to a repo-relative [`RelPath`]. An absolute path is
/// stripped of the `repo_root` prefix; anything already relative is taken
/// as-is (post-normalization).
fn to_repo_relative(normalized: &str, repo_root: &RepoRoot) -> Result<RelPath, DecodeError> {
    let root = repo_root.as_str();
    let stripped = normalized
        .strip_prefix(root)
        .map(|rest| rest.trim_start_matches('/'))
        .unwrap_or(normalized);
    if stripped.is_empty() {
        return Err(DecodeError::new(
            "scope.path",
            "path resolves to the repo root itself, not a file or subdirectory",
        ));
    }
    stripped.parse()
}

#[cfg(test)]
mod tests {
    use super::{resolve, CommitRef, ScopeRequest};
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RepoRoot;
    use std::path::PathBuf;

    fn repo_root() -> Result<RepoRoot, Box<dyn std::error::Error>> {
        Ok(r"C:\Projects\enforcer".parse()?)
    }

    #[test]
    fn paths_mode_resolves_to_files_scope() -> Result<(), Box<dyn std::error::Error>> {
        let root = repo_root()?;
        let request = ScopeRequest::Paths(vec![PathBuf::from(r"crates\enforcer-scan\src\lib.rs")]);
        let resolved = resolve(&request, &root)?;
        assert_eq!(resolved.kind, ScanScope::Files);
        assert_eq!(
            resolved.explicit_paths[0].as_str(),
            "crates/enforcer-scan/src/lib.rs"
        );
        Ok(())
    }

    #[test]
    fn paths_mode_strips_absolute_prefix_and_normalizes_backslashes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = repo_root()?;
        let request = ScopeRequest::Paths(vec![PathBuf::from(
            r"C:\Projects\enforcer\crates\enforcer-scan\src\lib.rs",
        )]);
        let resolved = resolve(&request, &root)?;
        assert_eq!(resolved.explicit_paths.len(), 1);
        assert_eq!(
            resolved.explicit_paths[0].as_str(),
            "crates/enforcer-scan/src/lib.rs"
        );
        Ok(())
    }

    #[test]
    fn paths_mode_dedupes_and_sorts() -> Result<(), Box<dyn std::error::Error>> {
        let root = repo_root()?;
        let request = ScopeRequest::Paths(vec![
            PathBuf::from(r"b\file.rs"),
            PathBuf::from(r"a\file.rs"),
            PathBuf::from(r"b\file.rs"),
        ]);
        let resolved = resolve(&request, &root)?;
        let rendered: Vec<&str> = resolved.explicit_paths.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["a/file.rs", "b/file.rs"]);
        Ok(())
    }

    #[test]
    fn diff_mode_resolves_to_diff_scope_and_carries_range() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = repo_root()?;
        let request = ScopeRequest::Diff {
            base: CommitRef::from_str_helper("main")?,
            head: CommitRef::from_str_helper("HEAD")?,
        };
        let resolved = resolve(&request, &root)?;
        assert_eq!(resolved.kind, ScanScope::Diff);
        let (base, head) = resolved
            .diff_range
            .ok_or("resolved diff scope must carry a diff range")?;
        assert_eq!(base.as_str(), "main");
        assert_eq!(head.as_str(), "HEAD");
        Ok(())
    }

    #[test]
    fn all_mode_resolves_to_workspace_scope() -> Result<(), Box<dyn std::error::Error>> {
        let root = repo_root()?;
        let resolved = resolve(&ScopeRequest::All, &root)?;
        assert_eq!(resolved.kind, ScanScope::Workspace);
        assert!(resolved.explicit_paths.is_empty());
        assert!(resolved.diff_range.is_none());
        Ok(())
    }

    #[test]
    fn empty_commit_ref_is_rejected() {
        assert!(CommitRef::from_str_helper("").is_err());
        assert!(CommitRef::from_str_helper("   ").is_err());
    }

    #[test]
    fn same_logical_scope_resolves_equal_regardless_of_separator_style(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = repo_root()?;
        let forward = ScopeRequest::Paths(vec![PathBuf::from("crates/enforcer-scan/src/lib.rs")]);
        let backward = ScopeRequest::Paths(vec![PathBuf::from(r"crates\enforcer-scan\src\lib.rs")]);
        assert_eq!(resolve(&forward, &root)?, resolve(&backward, &root)?);
        Ok(())
    }

    impl CommitRef {
        /// Test helper: the real API is `FromStr`, but spelling
        /// `"main".parse::<CommitRef>()` at every call site reads worse
        /// than a named helper in test code specifically.
        fn from_str_helper(raw: &str) -> Result<Self, enforcer_core::error::DecodeError> {
            raw.parse()
        }
    }
}
