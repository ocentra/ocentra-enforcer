//! Branded path newtypes. Paths are stored normalized to forward slashes
//! (the workspace-canonical representation from
//! a local separator normalizer); validation happens at
//! construction so no illegal path value can exist downstream.
//!
//! [`RepoRoot::resolve`] and [`RepoRoot::relativize`] are the only sanctioned
//! way to combine the two brands: a root joins with a `RelPath` (never two
//! `RelPath`s, never two `RepoRoot`s), and an absolute string relativizes
//! against a root back into a validated `RelPath`.

use crate::boundary::decode_error::DecodeError;

fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

/// Branded absolute repository root.
///
/// Accepts Windows drive-letter roots (`C:/...` or `C:\...`), UNC roots
/// (`//server/share`), and POSIX absolute roots (`/...`); stores the
/// normalized (forward-slash) form.
/// BRAND-INVARIANT: constructed only by validated conversions; the inner text
/// is a normalized, absolute repository root (POSIX, UNC, or drive-letter).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(try_from = "String", into = "String")]
#[ts(type = "string")]
#[doc = "BRAND-INVARIANT: validated normalized absolute repository root."]
pub struct RepoRoot(String);

/// Branded repo-relative path.
///
/// Always relative (no leading separator or drive letter), normalized to
/// forward slashes, and confined: no `..` segment may escape the root.
/// BRAND-INVARIANT: constructed only by validated conversions; the inner text
/// is normalized, relative, and cannot escape its repository root.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(try_from = "String", into = "String")]
#[ts(type = "string")]
#[doc = "BRAND-INVARIANT: validated normalized relative path confined to its root."]
pub struct RelPath(String);

impl RepoRoot {
    /// View the normalized inner value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Join this root with a repo-relative path, producing the normalized
    /// absolute string. Typed so the base is always a `RepoRoot` and the
    /// joined segment is always a validated `RelPath`; there is no method
    /// that joins two `RelPath`s or two `RepoRoot`s.
    pub fn resolve(&self, rel: &RelPath) -> String {
        format!("{}/{}", self.0, rel.0)
    }

    /// Strip this root's prefix from an absolute path, producing a
    /// validated `RelPath`. Fails closed when `abs` does not fall under
    /// this root, or when the remainder is empty or escapes (defense in
    /// depth: a path already under the root cannot contain a `..` escape,
    /// but the check is re-run via [`RelPath`]'s own constructor).
    pub fn relativize(&self, abs: &str) -> Result<RelPath, DecodeError> {
        let normalized = normalize_separators(abs);
        let stripped = normalized
            .strip_prefix(&self.0)
            .and_then(|rest| rest.strip_prefix('/'))
            .ok_or_else(|| {
                DecodeError::new(
                    "relPath",
                    format!("path does not fall under repo root `{}`", self.0),
                )
            })?;
        // ALLOC-JUSTIFICATION: `RelPath` owns its validated normalized value;
        // the stripped slice borrows the short-lived normalized input.
        RelPath::try_from(stripped.to_owned())
    }
}

impl RelPath {
    /// View the normalized inner value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn windows_drive_root_marker(normalized: &str) -> Option<()> {
    match normalized.as_bytes() {
        [drive, b':', b'/', ..] if drive.is_ascii_alphabetic() => Some(()),
        _ => None,
    }
}

impl TryFrom<String> for RepoRoot {
    type Error = DecodeError;

    fn try_from(raw: String) -> Result<Self, DecodeError> {
        if raw.trim().is_empty() {
            return Err(DecodeError::new("repoRoot", "must not be empty"));
        }
        let normalized = normalize_separators(&raw);
        let unc = normalized.starts_with("//") && normalized.len() > 2;
        let posix = normalized.starts_with('/') && !normalized.starts_with("//");
        let windows = windows_drive_root_marker(&normalized).is_some();
        if !(unc || posix || windows) {
            return Err(DecodeError::new(
                "repoRoot",
                "must be absolute (drive-letter, UNC, or POSIX root)",
            ));
        }
        Ok(Self(normalized))
    }
}

impl TryFrom<String> for RelPath {
    type Error = DecodeError;

    fn try_from(raw: String) -> Result<Self, DecodeError> {
        if raw.trim().is_empty() {
            return Err(DecodeError::new("relPath", "must not be empty"));
        }
        let normalized = normalize_separators(&raw);
        if normalized.starts_with('/') || windows_drive_root_marker(&normalized).is_some() {
            return Err(DecodeError::new(
                "relPath",
                "must be relative (no leading separator or drive letter)",
            ));
        }
        let mut depth: i32 = 0;
        for segment in normalized.split('/') {
            match segment {
                ".." => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(DecodeError::new(
                            "relPath",
                            "`..` segment escapes the repository root",
                        ));
                    }
                }
                "" | "." => {}
                _ => depth += 1,
            }
        }
        Ok(Self(normalized))
    }
}

impl std::str::FromStr for RepoRoot {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: `RepoRoot` owns its normalized validated value;
        // the `FromStr` input is borrowed from the caller.
        Self::try_from(raw.to_owned())
    }
}

impl std::str::FromStr for RelPath {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
        // ALLOC-JUSTIFICATION: `RelPath` owns its normalized validated value;
        // the `FromStr` input is borrowed from the caller.
        Self::try_from(raw.to_owned())
    }
}

impl From<RepoRoot> for String {
    fn from(value: RepoRoot) -> String {
        value.0
    }
}

impl From<RelPath> for String {
    fn from(value: RelPath) -> String {
        value.0
    }
}

impl std::fmt::Display for RepoRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for RelPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{RelPath, RepoRoot};
    use crate::boundary::decode_error::DecodeError;

    #[test]
    fn repo_root_accepts_absolute_forms_and_normalizes() -> Result<(), DecodeError> {
        let win: RepoRoot = r"C:\Projects\enforcer".parse()?;
        assert_eq!(win.as_str(), "C:/Projects/enforcer");
        let posix: RepoRoot = "/home/user/repo".parse()?;
        assert_eq!(posix.as_str(), "/home/user/repo");
        let unc: RepoRoot = r"\\server\share\repo".parse()?;
        assert_eq!(unc.as_str(), "//server/share/repo");
        Ok(())
    }

    #[test]
    fn repo_root_rejects_relative_and_empty() {
        for bad in ["", "  ", "relative/path", "./here", "C:"] {
            let outcome: Result<RepoRoot, _> = bad.parse();
            assert!(outcome.is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn rel_path_accepts_relative_and_normalizes_backslashes() -> Result<(), DecodeError> {
        let p: RelPath = r"crates\enforcer-domain\src\lib.rs".parse()?;
        assert_eq!(p.as_str(), "crates/enforcer-domain/src/lib.rs");
        let dotted: RelPath = "a/./b".parse()?;
        assert_eq!(dotted.as_str(), "a/./b");
        let contained: RelPath = "a/b/../c".parse()?;
        assert_eq!(contained.as_str(), "a/b/../c");
        Ok(())
    }

    #[test]
    fn rel_path_rejects_absolute_and_escaping() {
        for bad in ["", "/abs/path", r"C:\abs", "../escape", "a/../../escape"] {
            let outcome: Result<RelPath, _> = bad.parse();
            assert!(outcome.is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn conversion_boundary_enforces_path_rules() -> Result<(), DecodeError> {
        let ok = RelPath::try_from(String::from("src/lib.rs"))?;
        assert_eq!(ok.as_str(), "src/lib.rs");
        assert!(RelPath::try_from(String::from("/abs")).is_err());
        assert!(RepoRoot::try_from(String::from("not-absolute")).is_err());
        Ok(())
    }

    #[test]
    fn resolve_joins_root_and_rel_typed_so_only_relpath_is_accepted() -> Result<(), DecodeError> {
        let root: RepoRoot = r"C:\Projects\enforcer".parse()?;
        let rel: RelPath = r"crates\enforcer-domain\src\lib.rs".parse()?;
        assert_eq!(
            root.resolve(&rel),
            "C:/Projects/enforcer/crates/enforcer-domain/src/lib.rs"
        );
        // NOTE: `RepoRoot::resolve` takes `&RelPath` by type, not `&str`;
        // there is no overload accepting a second `RepoRoot`, so
        // `root.resolve(&other_root)` is a compile error, not a runtime
        // check. See `tests/compile_reject` fixtures for the enforced case.
        Ok(())
    }

    #[test]
    fn relativize_strips_root_and_validates_the_remainder() -> Result<(), DecodeError> {
        let root: RepoRoot = "/home/user/repo".parse()?;
        let rel = root.relativize("/home/user/repo/crates/enforcer-domain/src/lib.rs")?;
        assert_eq!(rel.as_str(), "crates/enforcer-domain/src/lib.rs");

        let win_root: RepoRoot = r"C:\Projects\enforcer".parse()?;
        let win_rel = win_root.relativize(r"C:\Projects\enforcer\crates\x\src\lib.rs")?;
        assert_eq!(win_rel.as_str(), "crates/x/src/lib.rs");
        Ok(())
    }

    #[test]
    fn relativize_rejects_paths_outside_the_root_or_equal_to_it() -> Result<(), DecodeError> {
        let root: RepoRoot = "/home/user/repo".parse()?;
        assert!(root.relativize("/home/user/other/file.rs").is_err());
        assert!(root.relativize("/completely/different").is_err());
        // Equal to the root itself: no `/` remainder to strip -> rejected,
        // not silently accepted as an empty RelPath.
        assert!(root.relativize("/home/user/repo").is_err());
        Ok(())
    }

    #[test]
    fn resolve_and_relativize_round_trip() -> Result<(), DecodeError> {
        let root: RepoRoot = "/home/user/repo".parse()?;
        let rel: RelPath = "crates/enforcer-domain/src/paths.rs".parse()?;
        let abs = root.resolve(&rel);
        let round_tripped = root.relativize(&abs)?;
        assert_eq!(round_tripped, rel);
        Ok(())
    }
}
