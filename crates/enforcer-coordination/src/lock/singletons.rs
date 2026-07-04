//! Protected-singleton path classification.
//!
//! Ported from `src/coordination/vendor/lock-policy-singletons.js`. Any path
//! matching one of these groups is force-escalated to `globalWriteLock`
//! (cross-worktree) regardless of the declared lock kind — this is the ONE
//! intentional cross-worktree lock in the whole engine (arc-16 workpack row
//! "Protected-singleton auto-escalation").

const LOCKFILE_NAMES: &[&str] = &[
    "cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "uv.lock",
    "poetry.lock",
];

/// Normalize a coordination path the same way `lock-policy-singletons.js`
/// does locally: backslash→slash, collapse repeated slashes, strip leading
/// `./`, lowercase.
pub fn normalize_coordination_path(value: &str) -> String {
    let replaced = value.trim().replace('\\', "/");
    let mut collapsed = String::with_capacity(replaced.len());
    let mut last_was_slash = false;
    for c in replaced.chars() {
        if c == '/' {
            if last_was_slash {
                continue;
            }
            last_was_slash = true;
        } else {
            last_was_slash = false;
        }
        collapsed.push(c);
    }
    let stripped = collapsed.strip_prefix("./").unwrap_or(&collapsed);
    stripped.to_lowercase()
}

/// Return the protected-singleton group key for a path, or `None` if it is
/// an ordinary path. Ported from `lock-policy-singletons.js#protectedSingletonGroup`.
pub fn protected_singleton_group(path: &str) -> Option<String> {
    let normalized = normalize_coordination_path(path);
    let basename = normalized.rsplit('/').next().unwrap_or(&normalized);
    lockfile_group(basename)
        .or_else(|| release_group(&normalized, basename))
        .or_else(|| migration_group(&normalized))
        .or_else(|| generated_group(&normalized))
        .or_else(|| ci_group(&normalized))
}

fn lockfile_group(basename: &str) -> Option<String> {
    LOCKFILE_NAMES
        .contains(&basename)
        .then(|| format!("lockfile:{basename}"))
}

fn release_group(normalized: &str, basename: &str) -> Option<String> {
    let bare = basename.strip_suffix(".md").unwrap_or(basename);
    if matches!(bare, "changelog" | "changes" | "release-notes") {
        return Some(format!("release:{basename}"));
    }
    if normalized == "version" || normalized == "VERSION" {
        return Some(format!("release:{}", basename.to_lowercase()));
    }
    None
}

fn migration_group(normalized: &str) -> Option<String> {
    (normalized.contains("/migrations/") || normalized.starts_with("migrations/"))
        .then(|| format!("migrations:{normalized}"))
}

fn generated_group(normalized: &str) -> Option<String> {
    let generated_path = normalized.contains("/generated/") || normalized.starts_with("generated/");
    let generated_contract = normalized.contains("generated")
        && ["schema", "contract", "dto", "bridge"]
            .iter()
            .any(|kw| normalized.contains(kw));
    (generated_path || generated_contract).then(|| format!("generated:{normalized}"))
}

fn ci_group(normalized: &str) -> Option<String> {
    normalized
        .starts_with(".github/workflows/")
        .then(|| format!("ci:{normalized}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfiles_are_protected_singletons() {
        assert_eq!(
            protected_singleton_group("Cargo.lock"),
            Some("lockfile:cargo.lock".to_owned())
        );
        assert_eq!(
            protected_singleton_group("nested/dir/package-lock.json"),
            Some("lockfile:package-lock.json".to_owned())
        );
    }

    #[test]
    fn migrations_and_generated_and_ci_are_protected() {
        assert!(protected_singleton_group("crates/foo/migrations/0001_init.sql").is_some());
        assert!(protected_singleton_group("src/generated/schema.rs").is_some());
        assert!(protected_singleton_group(".github/workflows/ci.yml").is_some());
    }

    #[test]
    fn ordinary_source_paths_are_not_protected() {
        assert_eq!(
            protected_singleton_group("crates/enforcer-core/src/lib.rs"),
            None
        );
        assert_eq!(protected_singleton_group("README.md"), None);
    }
}
