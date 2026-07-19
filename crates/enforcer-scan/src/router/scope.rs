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
use enforcer_domain::scan_types::RouteScope;

/// Narrow a walked, repo-relative path list down to only the paths
/// [`RouteScope::admits`]. Pure and total: an empty `paths` or a scope that
/// admits nothing both yield an empty result, never a fallback to "admit
/// everything" — an honest empty narrowing, not a false whole-repo route.
pub fn narrow<'a>(paths: &'a [RelPath], scope: &RouteScope) -> Vec<&'a RelPath> {
    paths
        .iter()
        .filter(|path| scope_admits(scope, path))
        .collect()
}

fn scope_admits(scope: &RouteScope, path: &RelPath) -> bool {
    let Some(root) = scope.root() else {
        return true;
    };
    path == root
        || path
            .as_str()
            .strip_prefix(root.as_str())
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::narrow;
    use enforcer_domain::scan_types::RouteScope;
    use std::str::FromStr;

    fn rel(literal: &str) -> Result<enforcer_domain::paths::RelPath, Box<dyn std::error::Error>> {
        Ok(enforcer_domain::paths::RelPath::from_str(literal)?)
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
        let scope = RouteScope::Crate("crates/enforcer-scan".parse()?);
        let narrowed = narrow(&paths, &scope);
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].as_str(), "crates/enforcer-scan/src/lib.rs");
        Ok(())
    }

    #[test]
    fn folder_scope_excludes_siblings_with_shared_prefix() -> Result<(), Box<dyn std::error::Error>>
    {
        let paths = vec![rel("apps/web/index.ts")?, rel("apps/web2/index.ts")?];
        let scope = RouteScope::Folder("apps/web".parse()?);
        let narrowed = narrow(&paths, &scope);
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].as_str(), "apps/web/index.ts");
        Ok(())
    }

    #[test]
    fn scope_admitting_nothing_yields_honest_empty_result() -> Result<(), Box<dyn std::error::Error>>
    {
        let paths = vec![rel("crates/enforcer-scan/src/lib.rs")?];
        let scope = RouteScope::Crate("crates/enforcer-other".parse()?);
        assert!(narrow(&paths, &scope).is_empty());
        Ok(())
    }

    #[test]
    fn default_scope_is_repo() {
        assert_eq!(RouteScope::default(), RouteScope::Repo);
    }
}
