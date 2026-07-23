//! Protected-singleton path classification.
//!
//! Ported from `src/coordination/vendor/lock-policy-singletons.js`. Any path
//! matching one of these groups is force-escalated to `globalWriteLock`
//! (cross-worktree) regardless of the declared lock kind — this is the ONE
//! intentional cross-worktree lock in the whole engine (arc-16 workpack row
//! "Protected-singleton auto-escalation").

use crate::error::Result;
use enforcer_domain::coordination_types::{ClaimGroup, ClaimPath};

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
pub fn normalize_coordination_path(value: &ClaimPath) -> Result<ClaimPath> {
    let replaced = value.as_str().trim().replace('\\', "/");
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
    let stripped = if collapsed == "./" {
        "."
    } else {
        collapsed.strip_prefix("./").unwrap_or(&collapsed)
    };
    Ok(ClaimPath::try_from(stripped.to_lowercase())?)
}

/// Return the protected-singleton group key for a path, or `None` if it is
/// an ordinary path. Ported from `lock-policy-singletons.js#protectedSingletonGroup`.
pub fn protected_singleton_group(path: &ClaimPath) -> Result<Option<ClaimGroup>> {
    let normalized = normalize_coordination_path(path)?;
    let normalized_text = normalized.as_str();
    let basename = normalized_text
        .rsplit('/')
        .next()
        .unwrap_or(normalized_text);
    if LOCKFILE_NAMES.contains(&basename) {
        return Ok(Some(ClaimGroup::try_from(format!("lockfile:{basename}"))?));
    }
    let bare = basename.strip_suffix(".md").unwrap_or(basename);
    if matches!(bare, "changelog" | "changes" | "release-notes") {
        return Ok(Some(ClaimGroup::try_from(format!("release:{basename}"))?));
    }
    if normalized_text == "version" {
        return Ok(Some(ClaimGroup::try_from(format!(
            "release:{}",
            basename.to_lowercase()
        ))?));
    }
    if normalized_text.contains("/migrations/") || normalized_text.starts_with("migrations/") {
        return Ok(Some(ClaimGroup::try_from(format!(
            "migrations:{normalized_text}"
        ))?));
    }
    let generated_path =
        normalized_text.contains("/generated/") || normalized_text.starts_with("generated/");
    let generated_contract = normalized_text.contains("generated")
        && ["schema", "contract", "dto", "bridge"]
            .iter()
            .any(|keyword| normalized_text.contains(keyword));
    if generated_path || generated_contract {
        return Ok(Some(ClaimGroup::try_from(format!(
            "generated:{normalized_text}"
        ))?));
    }
    if normalized_text.starts_with(".github/workflows/") {
        return Ok(Some(ClaimGroup::try_from(format!("ci:{normalized_text}"))?));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{normalize_coordination_path, protected_singleton_group};
    use crate::error::Result;
    use enforcer_domain::boundary::decode_error::DecodeError;
    use enforcer_domain::coordination_types::ClaimPath;
    use proptest::prelude::proptest;

    #[test]
    fn lockfiles_are_protected_singletons() -> Result<()> {
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static("Cargo.lock")?)?
                .map(|group| group.to_string()),
            Some("lockfile:cargo.lock".to_owned())
        );
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static("nested/dir/package-lock.json")?)?
                .map(|group| group.to_string()),
            Some("lockfile:package-lock.json".to_owned())
        );
        Ok(())
    }

    #[test]
    fn migrations_and_generated_and_ci_are_protected() -> Result<()> {
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static(
                "crates/foo/migrations/0001_init.sql",
            )?)?
            .map(|group| group.to_string()),
            Some("migrations:crates/foo/migrations/0001_init.sql".to_owned())
        );
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static("src/generated/schema.rs")?)?
                .map(|group| group.to_string()),
            Some("generated:src/generated/schema.rs".to_owned())
        );
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static(".github/workflows/ci.yml")?)?
                .map(|group| group.to_string()),
            Some("ci:.github/workflows/ci.yml".to_owned())
        );
        Ok(())
    }

    #[test]
    fn ordinary_source_paths_are_not_protected() -> Result<()> {
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static("crates/enforcer-core/src/lib.rs")?)?,
            None
        );
        assert_eq!(
            protected_singleton_group(&ClaimPath::from_static("README.md")?)?,
            None
        );
        Ok(())
    }

    #[test]
    fn invalid_empty_claim_path_is_rejected_before_normalization() {
        assert_eq!(
            ClaimPath::from_static(""),
            Err(DecodeError::new("claimPath", "expected a non-blank value",))
        );
    }

    #[test]
    fn malformed_repeated_separators_and_oversized_paths_normalize_deterministically() -> Result<()>
    {
        let oversized = format!("./{}\\\\leaf.rs", "segment/".repeat(64));
        let normalized = normalize_coordination_path(&ClaimPath::try_from(oversized)?)?;
        assert!(!normalized.as_str().contains("//"));
        assert!(!normalized.as_str().contains('\\'));
        assert!(normalized.as_str().ends_with("leaf.rs"));
        Ok(())
    }

    #[test]
    fn root_path_normalizes_to_a_nonblank_dot_path() -> Result<()> {
        let normalized = normalize_coordination_path(&ClaimPath::try_from(".\\".to_owned())?)?;
        assert_eq!(normalized.as_str(), ".");
        Ok(())
    }

    proptest! {
        #[test]
        fn normalization_is_idempotent(raw in "[A-Za-z0-9_./\\\\-]{1,128}") {
            let input = ClaimPath::try_from(raw)?;
            let once = normalize_coordination_path(&input)?;
            let twice = normalize_coordination_path(&once)?;
            proptest::prop_assert_eq!(once, twice);
        }

        #[test]
        fn normalization_never_retains_backslashes(raw in "[A-Za-z0-9_./\\\\-]{1,128}") {
            let input = ClaimPath::try_from(raw)?;
            let normalized = normalize_coordination_path(&input)?;
            proptest::prop_assert!(!normalized.as_str().contains('\\'));
        }
    }
}
