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

use enforcer_domain::config_types::Glob;
use enforcer_domain::paths::RelPath;
use enforcer_domain::scan_types::IgnoreDirectorySegment;

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

fn is_always_ignored_dir_segment(segment: &str) -> bool {
    ALWAYS_IGNORED_DIR_SEGMENTS.contains(&segment)
        || segment
            .strip_prefix("target-")
            .is_some_and(|suffix| !suffix.is_empty())
}

/// A minimal glob matcher for the `enforcer-config` ignore-file-glob shape:
/// supports a single trailing/leading `*` wildcard (the shapes
/// `ignoreFileGlobs` actually uses, e.g. `*.snap`, `generated/*`).
/// Anything more exotic falls back to an exact-match comparison rather
/// than silently matching everything — a glob this matcher cannot express
/// simply never matches, which fails closed (over-scans, never
/// under-scans).
fn glob_matches(glob: &str, candidate: &str) -> bool {
    fn wildcard_closure(states: &[bool]) -> Vec<bool> {
        let mut reached = false;
        states
            .iter()
            .copied()
            .map(|state| {
                reached |= state;
                reached
            })
            .collect()
    }

    let text = candidate.as_bytes();
    let mut states = vec![false; text.len() + 1];
    if let Some(initial) = states.first_mut() {
        *initial = true;
    }
    let mut pattern = glob.as_bytes().iter().copied().peekable();

    while let Some(token) = pattern.next() {
        if token == b'*' {
            if pattern.peek() == Some(&b'*') {
                pattern.next();
                // `**/` consumes the slash as part of the wildcard so the
                // directory prefix can be empty. A bare `**` is equivalent
                // to the existing path-spanning `*` contract.
                if pattern.peek() == Some(&b'/') {
                    pattern.next();
                }
            }
            states = wildcard_closure(&states);
            continue;
        }

        let mut next = Vec::with_capacity(states.len());
        next.push(false);
        for (state, candidate_byte) in states.iter().copied().zip(text.iter().copied()) {
            next.push(state && candidate_byte == token);
        }
        states = next;
    }

    matches!(states.last(), Some(true))
}

/// Ignore rules the walk honors, threaded in from `enforcer-config`'s
/// resolved `EffectiveConfig` (this module does not read config itself —
/// arc-15 depends on arc-03 only for these two fields, not for the whole
/// config-load boundary).
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    /// Extra directory segment names to skip, beyond
    /// [`ALWAYS_IGNORED_DIR_SEGMENTS`].
    ignore_dirs: Vec<IgnoreDirectorySegment>,
    /// File-path glob patterns to skip (matched against the repo-relative
    /// path).
    ignore_file_globs: Vec<Glob>,
}

impl IgnoreRules {
    /// Builds ignore rules from validated directory segments and file globs.
    pub fn new(ignore_dirs: Vec<IgnoreDirectorySegment>, ignore_file_globs: Vec<Glob>) -> Self {
        Self {
            ignore_dirs,
            ignore_file_globs,
        }
    }
    /// Does this repo-relative path fall under an ignored directory
    /// segment (built-in or config-declared)?
    fn is_under_ignored_dir(&self, rel: &str) -> bool {
        rel.split('/').any(|segment| {
            is_always_ignored_dir_segment(segment)
                || self.ignore_dirs.iter().any(|d| d.as_str() == segment)
        })
    }

    /// Does this repo-relative path match an ignored file glob?
    fn matches_ignored_file_glob(&self, rel: &str) -> bool {
        self.ignore_file_globs
            .iter()
            .any(|glob| glob_matches(glob.as_str(), rel))
    }

    /// True if this path should be excluded from the walk result.
    pub fn is_ignored(&self, rel: &RelPath) -> bool {
        self.is_ignored_raw(rel.as_str())
    }

    fn is_ignored_raw(&self, rel: &str) -> bool {
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
        if rules.is_ignored_raw(&rel) {
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
        .filter(|p| !rules.is_ignored(p))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::{filter_explicit, glob_matches, walk, IgnoreRules};
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
        write_file(temp.path(), "target-nextest/debug/build.log", "noise")?;
        write_file(temp.path(), "target-ci/release/build.log", "noise")?;
        write_file(temp.path(), "targeting/source.rs", "fn retained() {}")?;
        write_file(temp.path(), ".git/HEAD", "ref: refs/heads/main")?;
        let found = walk(temp.path(), &IgnoreRules::default())?;
        let rendered: Vec<&str> = found.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["src/lib.rs", "targeting/source.rs"]);
        Ok(())
    }

    #[test]
    fn walk_skips_config_declared_ignore_dir_and_glob() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(temp.path(), "src/lib.rs", "fn main() {}")?;
        write_file(temp.path(), "vendor/thing.rs", "// vendored")?;
        write_file(temp.path(), "src/lib.snap", "snapshot")?;
        let rules = IgnoreRules::new(
            vec![
                enforcer_domain::scan_types::IgnoreDirectorySegment::try_new("vendor".to_owned())?,
            ],
            vec![enforcer_domain::config_types::Glob::new(
                "*.snap".to_owned(),
            )?],
        );
        let found = walk(temp.path(), &rules)?;
        let rendered: Vec<&str> = found.iter().map(|p| p.as_str()).collect();
        assert_eq!(rendered, vec!["src/lib.rs"]);
        Ok(())
    }

    #[test]
    fn walk_matches_multi_segment_double_star_globs() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        write_file(
            temp.path(),
            "crates/enforcer-ui/frontend/src/bindings/Finding.ts",
            "export type Finding = string;",
        )?;
        write_file(
            temp.path(),
            "crates/enforcer-ui/frontend/src/bindings/nested/Generated.ts",
            "export type Generated = string;",
        )?;
        write_file(
            temp.path(),
            "crates/enforcer-ui/frontend/src/components/App.tsx",
            "export function App() {}",
        )?;
        write_file(temp.path(), "README.md", "root readme")?;
        write_file(temp.path(), "crates/enforcer-ui/README.md", "crate readme")?;
        let rules = IgnoreRules::new(
            Vec::new(),
            vec![
                enforcer_domain::config_types::Glob::new(
                    "crates/enforcer-ui/frontend/src/bindings/**".to_owned(),
                )?,
                enforcer_domain::config_types::Glob::new("**/README.md".to_owned())?,
            ],
        );
        let found = walk(temp.path(), &rules)?;
        let rendered: Vec<&str> = found.iter().map(|path| path.as_str()).collect();
        assert_eq!(
            rendered,
            vec!["crates/enforcer-ui/frontend/src/components/App.tsx"]
        );
        Ok(())
    }

    #[test]
    fn glob_matcher_preserves_empty_and_nested_double_star_prefixes() {
        assert!(glob_matches("**/README.md", "README.md"));
        assert!(glob_matches("**/README.md", "docs/README.md"));
        assert!(glob_matches("a/**/b", "a/b"));
        assert!(glob_matches("a/**/b", "a/deep/b"));
        assert!(!glob_matches("a/**/b", "a/deep/c"));
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
            "target-nextest/generated.rs".parse()?,
            "targeting/source.rs".parse()?,
            "a/file.rs".parse()?,
        ];
        let filtered = filter_explicit(&paths, &IgnoreRules::default());
        let rendered: Vec<&str> = filtered.iter().map(|p| p.as_str()).collect();
        assert_eq!(
            rendered,
            vec!["a/file.rs", "b/file.rs", "targeting/source.rs"]
        );
        Ok(())
    }
}
