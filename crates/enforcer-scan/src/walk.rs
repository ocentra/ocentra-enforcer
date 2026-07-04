//! The ignored-segments walk + the idempotency guard.
//!
//! [`walk`] enumerates the concrete file set for a [`ResolvedScope`],
//! skipping `target/`, `.git/`, and other vendored/generated directory
//! segments, plus any path matching an `enforcer-config`
//! `ignoreDirs`/`ignoreFileGlobs` entry. The result is always returned in
//! deterministic (lexicographically sorted, de-duplicated) order — that
//! determinism is the idempotency guard: re-walking the same scope with
//! the same ignore rules always yields the same file list, in the same
//! order, whether the caller then fans out serially or in parallel
//! ([`crate::engine`]).

use std::path::Path;

use enforcer_domain::paths::RelPath;

/// Directory segments always skipped, regardless of `enforcer-config`.
/// These are workspace/VCS mechanics, never a legitimate scan target.
const ALWAYS_IGNORED_DIR_SEGMENTS: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    ".enforce",
    "dist",
    "build",
    "coverage",
];

/// A minimal glob matcher for the `enforcer-config` ignore-file-glob shape:
/// supports a single trailing/leading `*` wildcard (the shapes
/// `ignoreFileGlobs` actually uses, e.g. `*.snap`, `generated/*`).
/// Anything more exotic falls back to an exact-match comparison rather
/// than silently matching everything — a glob this matcher cannot express
/// simply never matches, which fails closed (over-scans, never
/// under-scans).
fn glob_matches(glob: &str, candidate: &str) -> bool {
    if let Some(prefix) = glob.strip_suffix('*') {
        return candidate.starts_with(prefix);
    }
    if let Some(suffix) = glob.strip_prefix('*') {
        return candidate.ends_with(suffix);
    }
    glob == candidate
}

/// Ignore rules the walk honors, threaded in from `enforcer-config`'s
/// resolved `EffectiveConfig` (this module does not read config itself —
/// arc-15 depends on arc-03 only for these two fields, not for the whole
/// config-load boundary).
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    /// Extra directory segment names to skip, beyond
    /// [`ALWAYS_IGNORED_DIR_SEGMENTS`].
    pub ignore_dirs: Vec<String>,
    /// File-path glob patterns to skip (matched against the repo-relative
    /// path).
    pub ignore_file_globs: Vec<String>,
}

impl IgnoreRules {
    /// Does this repo-relative path fall under an ignored directory
    /// segment (built-in or config-declared)?
    fn is_under_ignored_dir(&self, rel: &str) -> bool {
        rel.split('/').any(|segment| {
            ALWAYS_IGNORED_DIR_SEGMENTS.contains(&segment)
                || self.ignore_dirs.iter().any(|d| d == segment)
        })
    }

    /// Does this repo-relative path match an ignored file glob?
    fn matches_ignored_file_glob(&self, rel: &str) -> bool {
        self.ignore_file_globs
            .iter()
            .any(|glob| glob_matches(glob, rel))
    }

    /// True if this path should be excluded from the walk result.
    pub fn is_ignored(&self, rel: &str) -> bool {
        self.is_under_ignored_dir(rel) || self.matches_ignored_file_glob(rel)
    }
}

/// Walk every file under `root`, filtering through `rules`, and return the
/// surviving repo-relative paths in deterministic (sorted) order.
///
/// # Errors
/// Returns an [`std::io::Error`] if directory traversal fails (permission
/// denied, root does not exist, etc).
pub fn walk(root: &Path, rules: &IgnoreRules) -> std::io::Result<Vec<RelPath>> {
    let mut out = Vec::new();
    walk_into(root, root, rules, &mut out)?;
    out.sort();
    out.dedup();
    Ok(out)
}

fn walk_into(
    root: &Path,
    dir: &Path,
    rules: &IgnoreRules,
    out: &mut Vec<RelPath>,
) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        let rel = to_rel(root, &path);
        if rules.is_ignored(&rel) {
            continue;
        }
        if file_type.is_dir() {
            walk_into(root, &path, rules, out)?;
        } else if file_type.is_file() {
            if let Ok(rel_path) = rel.parse() {
                out.push(rel_path);
            }
        }
    }
    Ok(())
}

/// Render `path` relative to `root`, normalized to forward slashes.
fn to_rel(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    enforcer_core::platform::normalize_separators(&rel.to_string_lossy())
}

/// Restrict an already-walked file list down to an explicit set of
/// [`RelPath`]s (the `ScopeRequest::Paths` mode) or a diff-derived set —
/// callers that already know their exact file set (explicit paths, a git
/// diff) skip [`walk`] entirely and call this instead, still honoring
/// `rules` so an explicitly-named but ignored path is dropped rather than
/// silently scanned.
pub fn filter_explicit(paths: &[RelPath], rules: &IgnoreRules) -> Vec<RelPath> {
    let mut out: Vec<RelPath> = paths
        .iter()
        .filter(|p| !rules.is_ignored(p.as_str()))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::{filter_explicit, walk, IgnoreRules};
    use std::fs;

    fn write_file(root: &std::path::Path, rel: &str, contents: &str) -> std::io::Result<()> {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }

    #[test]
    fn walk_skips_always_ignored_dirs() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(temp.path(), "src/lib.rs", "fn main() {}")?;
        write_file(temp.path(), "target/debug/build.log", "noise")?;
        write_file(temp.path(), ".git/HEAD", "ref: refs/heads/main")?;
        let found = walk(temp.path(), &IgnoreRules::default())?;
        let rendered: Vec<&str> = found.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["src/lib.rs"]);
        Ok(())
    }

    #[test]
    fn walk_skips_config_declared_ignore_dir_and_glob() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(temp.path(), "src/lib.rs", "fn main() {}")?;
        write_file(temp.path(), "vendor/thing.rs", "// vendored")?;
        write_file(temp.path(), "src/lib.snap", "snapshot")?;
        let rules = IgnoreRules {
            ignore_dirs: vec!["vendor".to_owned()],
            ignore_file_globs: vec!["*.snap".to_owned()],
        };
        let found = walk(temp.path(), &rules)?;
        let rendered: Vec<&str> = found.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["src/lib.rs"]);
        Ok(())
    }

    #[test]
    fn walk_is_idempotent_across_repeated_runs() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(temp.path(), "b/two.rs", "")?;
        write_file(temp.path(), "a/one.rs", "")?;
        write_file(temp.path(), "a/three.rs", "")?;
        let rules = IgnoreRules::default();
        let first = walk(temp.path(), &rules)?;
        let second = walk(temp.path(), &rules)?;
        assert_eq!(
            first, second,
            "two runs over the same scope must agree byte-for-byte"
        );
        let rendered: Vec<&str> = first.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["a/one.rs", "a/three.rs", "b/two.rs"]);
        Ok(())
    }

    #[test]
    fn filter_explicit_drops_ignored_and_sorts() -> Result<(), Box<dyn std::error::Error>> {
        let paths: Vec<enforcer_domain::paths::RelPath> = vec![
            "b/file.rs".parse()?,
            "target/generated.rs".parse()?,
            "a/file.rs".parse()?,
        ];
        let filtered = filter_explicit(&paths, &IgnoreRules::default());
        let rendered: Vec<&str> = filtered.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["a/file.rs", "b/file.rs"]);
        Ok(())
    }
}
