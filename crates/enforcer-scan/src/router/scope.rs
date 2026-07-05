//! Route-plan scope resolution (f05, stage 3): narrows a detected language
//! set down to the [`RouteScope`] the emitted [`super::plan::RoutePlan`]
//! actually applies to.
//!
//! Distinct from [`crate::scope`] (the tri-modal `<paths...>` | `--base
//! <sha> --head <sha>` | `--all` CLI-facing resolver that feeds
//! [`crate::walk`]): this module is the ROUTER's own narrower vocabulary —
//! `repo|workspace|crate|package|folder|domain|diff` — matching
//! `enforcer-domain`'s [`ScanScope`] where the two vocabularies overlap,
//! with the extra `Package`/`Folder`/`Domain` narrowing kinds a route plan
//! needs but a raw scan-scope resolution does not. The default is always
//! [`RouteScope::Repo`]: an explicit narrower request is required to shrink
//! the plan, and narrowing NEVER widens what gets routed — it only removes
//! paths (and therefore languages) outside the requested scope.

use enforcer_domain::paths::RelPath;

/// The scope a route plan applies to. Default is [`RouteScope::Repo`]
/// (whole-repo, honest default); every other variant is an explicit
/// narrowing request from the caller (an AI, a CLI flag, or the f03 tie
/// config).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RouteScope {
    /// The whole repository. Default — no narrowing was requested.
    #[default]
    Repo,
    /// A Cargo workspace as a unit (all member crates together).
    Workspace,
    /// One Cargo crate, identified by its manifest directory (repo-relative
    /// path to the crate root, e.g. `crates/enforcer-scan`).
    Crate(String),
    /// One non-Cargo package (e.g. a `package.json` package), identified by
    /// its manifest directory.
    Package(String),
    /// An arbitrary repo-relative folder, not tied to a manifest.
    Folder(String),
    /// A named domain/monorepo sub-project grouping (e.g. `apps/web`,
    /// `services/billing`) — a folder-shaped narrowing with a semantic
    /// label distinct from a bare [`RouteScope::Folder`].
    Domain(String),
    /// A git diff range — the route plan applies only to files touched
    /// between two commit-ish endpoints. Carries no endpoints itself (those
    /// live in [`crate::scope::ResolvedScope`]); this variant just marks
    /// that the plan was narrowed by diff rather than by a static path.
    Diff,
}

impl RouteScope {
    /// The repo-relative root this scope narrows to, if any. `None` for
    /// [`RouteScope::Repo`], [`RouteScope::Workspace`], and
    /// [`RouteScope::Diff`] (none of which are a single static path root).
    pub fn root(&self) -> Option<&str> {
        match self {
            RouteScope::Repo | RouteScope::Workspace | RouteScope::Diff => None,
            RouteScope::Crate(root)
            | RouteScope::Package(root)
            | RouteScope::Folder(root)
            | RouteScope::Domain(root) => Some(root.as_str()),
        }
    }

    /// Does `path` fall under this scope? [`RouteScope::Repo`] and
    /// [`RouteScope::Workspace`] admit every path (no narrowing).
    /// [`RouteScope::Diff`] is resolved by the caller's pre-filtered path
    /// list, not by this predicate, so it also admits every path it is
    /// asked about here. The path-rooted variants admit only paths under
    /// their `root`.
    pub fn admits(&self, path: &RelPath) -> bool {
        match self.root() {
            Some(root) => {
                let candidate = path.as_str();
                candidate == root || candidate.starts_with(&format!("{root}/"))
            }
            None => true,
        }
    }
}

/// Narrow a walked, repo-relative path list down to only the paths
/// [`RouteScope::admits`]. Pure and total: an empty `paths` or a scope that
/// admits nothing both yield an empty result, never a fallback to "admit
/// everything" — an honest empty narrowing, not a false whole-repo route.
pub fn narrow<'a>(paths: &'a [RelPath], scope: &RouteScope) -> Vec<&'a RelPath> {
    paths.iter().filter(|p| scope.admits(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::{narrow, RouteScope};
    use std::str::FromStr;

    fn rel(path: &str) -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(enforcer_domain::paths::RelPath::from_str(path)?)
    }

    #[test]
    fn repo_scope_admits_every_path() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![rel("a/one.rs")?, rel("b/two.ts")?];
        let narrowed = narrow(&paths, &RouteScope::Repo);
        assert_eq!(narrowed.len(), 2);
        Ok(())
    }

    #[test]
    fn crate_scope_narrows_to_its_root_only() -> Result<(), Box<dyn std::error::Error>> {
        let paths = vec![
            rel("crates/enforcer-scan/src/lib.rs")?,
            rel("crates/enforcer-core/src/lib.rs")?,
        ];
        let scope = RouteScope::Crate("crates/enforcer-scan".to_owned());
        let narrowed = narrow(&paths, &scope);
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].as_str(), "crates/enforcer-scan/src/lib.rs");
        Ok(())
    }

    #[test]
    fn folder_scope_excludes_siblings_with_shared_prefix() -> Result<(), Box<dyn std::error::Error>>
    {
        let paths = vec![rel("apps/web/index.ts")?, rel("apps/web2/index.ts")?];
        let scope = RouteScope::Folder("apps/web".to_owned());
        let narrowed = narrow(&paths, &scope);
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].as_str(), "apps/web/index.ts");
        Ok(())
    }

    #[test]
    fn scope_admitting_nothing_yields_honest_empty_result() -> Result<(), Box<dyn std::error::Error>>
    {
        let paths = vec![rel("crates/enforcer-scan/src/lib.rs")?];
        let scope = RouteScope::Crate("crates/enforcer-other".to_owned());
        assert!(narrow(&paths, &scope).is_empty());
        Ok(())
    }

    #[test]
    fn default_scope_is_repo() {
        assert_eq!(RouteScope::default(), RouteScope::Repo);
    }
}
