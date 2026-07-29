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

use enforcer_core::platform::normalize_separators;
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::scan_types::{ResolvedScope, ScopeRequest};

/// One git commit-ish endpoint of a `--base <sha> --head <sha>` diff range.
/// Deliberately a plain validated string (not a `Sha256` brand — a git
/// "commit-ish" may be an abbreviated sha, a branch name, or `HEAD~1`,
/// none of which are full SHA-256 hex digests).
/// BRAND-INVARIANT: the inner value is trimmed non-empty at parse time and
/// represents exactly one git revision expression supplied by the caller.
/// The caller's requested scope, before resolution against the repo tree.
/// Exactly one variant is ever constructed for a given invocation — there
/// is no combinator that merges two of these into a fourth mode.
/// The canonical, resolved scope: the [`ScanScope`] discriminant
/// (`enforcer-domain`'s wire-facing enum) plus the concrete root/paths/diff
/// endpoints the walker needs to actually enumerate files. Two
/// [`ScopeRequest`]s that mean the same thing (e.g. the same path spelled
/// with `\` vs `/`) resolve to an equal `ResolvedScope` — this equality is
/// what the idempotency guard in [`crate::walk`] relies on.
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
                // CLONE-JUSTIFICATION: the resolved scope owns the root for
                // use after the caller's request borrow ends.
                repo_root: repo_root.clone(),
                explicit_paths,
                diff_range: None,
            })
        }
        ScopeRequest::Diff { base, head } => Ok(ResolvedScope {
            kind: ScanScope::Diff,
            // CLONE-JUSTIFICATION: a resolved scope owns its root and commit
            // range independently of the borrowed request.
            repo_root: repo_root.clone(),
            explicit_paths: Vec::new(),
            // CLONE-JUSTIFICATION: both branded commit endpoints are stored
            // in the returned resolved scope.
            diff_range: Some((base.clone(), head.clone())),
        }),
        ScopeRequest::All => Ok(ResolvedScope {
            kind: ScanScope::Workspace,
            // CLONE-JUSTIFICATION: the resolved scope owns the root beyond
            // the request call.
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
    use super::resolve;
    use enforcer_domain::findings::ScanScope;
    use enforcer_domain::paths::RepoRoot;
    use enforcer_domain::scan_types::{CommitRef, ScopeRequest};
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
            base: "main".parse()?,
            head: "HEAD".parse()?,
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
        for raw in ["", "   "] {
            assert_eq!(
                raw.parse::<CommitRef>()
                    .err()
                    .map(|error| error.to_string()),
                Some("decode/validation failed at `scope.commitRef`: must not be empty".to_owned())
            );
        }
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
}
