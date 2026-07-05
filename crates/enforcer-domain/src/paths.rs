//! Branded path newtypes. Paths are stored normalized to forward slashes
//! (the workspace-canonical representation from
//! `enforcer_core::platform::normalize_separators`); validation happens at
//! construction so no illegal path value can exist downstream.
//!
//! [`RepoRoot::resolve`] and [`RepoRoot::relativize`] are the only sanctioned
//! way to combine the two brands: a root joins with a `RelPath` (never two
//! `RelPath`s, never two `RepoRoot`s), and an absolute string relativizes
//! against a root back into a validated `RelPath`.

use enforcer_core::error::DecodeError;

/// Branded absolute repository root.
///
/// Accepts Windows drive-letter roots (`C:/...` or `C:\...`), UNC roots
/// (`//server/share`), and POSIX absolute roots (`/...`); stores the
/// normalized (forward-slash) form.
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
pub struct RepoRoot(String);

/// Branded repo-relative path.
///
/// Always relative (no leading separator or drive letter), normalized to
/// forward slashes, and confined: no `..` segment may escape the root.
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
        let normalized = enforcer_core::platform::normalize_separators(abs);
        let stripped = normalized
            .strip_prefix(&self.0)
            .and_then(|rest| rest.strip_prefix('/'))
            .ok_or_else(|| {
                DecodeError::new(
                    "relPath",
                    format!("path does not fall under repo root `{}`", self.0),
                )
            })?;
        RelPath::try_from(stripped.to_owned())
    }
}

impl RelPath {
    /// View the normalized inner value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_windows_drive_root(normalized: &str) -> bool {
    let bytes = normalized.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

impl TryFrom<String> for RepoRoot {
    type Error = DecodeError;

    fn try_from(raw: String) -> Result<Self, DecodeError> {
        if raw.trim().is_empty() {
            return Err(DecodeError::new("repoRoot", "must not be empty"));
        }
        let normalized = enforcer_core::platform::normalize_separators(&raw);
        let unc = normalized.starts_with("//") && normalized.len() > 2;
        let posix = normalized.starts_with('/') && !normalized.starts_with("//");
        let windows = is_windows_drive_root(&normalized);
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
        let normalized = enforcer_core::platform::normalize_separators(&raw);
        if normalized.starts_with('/') || is_windows_drive_root(&normalized) {
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
        Self::try_from(raw.to_owned())
    }
}

impl std::str::FromStr for RelPath {
    type Err = DecodeError;

    fn from_str(raw: &str) -> Result<Self, DecodeError> {
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
    use enforcer_core::error::DecodeError;

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
    fn serde_boundary_enforces_path_rules() -> Result<(), serde_json::Error> {
        let ok: RelPath = serde_json::from_str("\"src/lib.rs\"")?;
        assert_eq!(ok.as_str(), "src/lib.rs");
        assert!(serde_json::from_str::<RelPath>("\"/abs\"").is_err());
        assert!(serde_json::from_str::<RepoRoot>("\"not-absolute\"").is_err());
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
