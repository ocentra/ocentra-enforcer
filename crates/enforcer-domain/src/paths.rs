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

/// Branded absolute repository root.
///
/// Accepts Windows drive-letter roots (`C:/...` or `C:\...`), UNC roots
/// (`//server/share`), and POSIX absolute roots (`/...`); stores the
/// normalized (forward-slash) form.
/// BRAND-INVARIANT: constructed only by validated conversions; the inner text
/// is a normalized, absolute repository root (POSIX, UNC, or drive-letter).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "string")]
#[doc = "BRAND-INVARIANT: validated normalized absolute repository root."]
pub struct RepoRoot(String);

/// Branded repo-relative path.
///
/// Always relative (no leading separator or drive letter), normalized to
/// forward slashes, and confined: no `..` segment may escape the root.
/// BRAND-INVARIANT: constructed only by validated conversions; the inner text
/// is normalized, relative, and cannot escape its repository root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ts_rs::TS)]
#[ts(type = "string")]
#[doc = "BRAND-INVARIANT: validated normalized relative path confined to its root."]
pub struct RelPath(String);

impl RepoRoot {
    /// Try to validate an absolute repository root from its wire form.
    pub fn try_new(raw: &str) -> Result<Self, DecodeError> {
        <Self as std::str::FromStr>::from_str(raw)
    }

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
        // ALLOC-JUSTIFICATION: normalization owns the boundary path while validation runs.
        let normalized = abs.replace('\\', "/");
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
    /// Try to validate a confined repository-relative path from its wire form.
    pub fn try_new(raw: &str) -> Result<Self, DecodeError> {
        <Self as std::str::FromStr>::from_str(raw)
    }

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
            return Err(DecodeError::new(
                "repoRoot",
                "invalid repository root: must not be empty",
            ));
        }
        let normalized = raw.replace('\\', "/");
        let unc = normalized.starts_with("//") && normalized.len() > 2;
        let posix = normalized.starts_with('/') && !normalized.starts_with("//");
        let windows = windows_drive_root_marker(&normalized).is_some();
        if !(unc || posix || windows) {
            return Err(DecodeError::new(
                "repoRoot",
                "invalid repository root: must be absolute (drive-letter, UNC, or POSIX root)",
            ));
        }
        Ok(Self(normalized))
    }
}

impl TryFrom<&std::path::Path> for RepoRoot {
    type Error = DecodeError;

    fn try_from(path: &std::path::Path) -> Result<Self, Self::Error> {
        Self::try_from(path.to_string_lossy().into_owned())
    }
}

impl TryFrom<String> for RelPath {
    type Error = DecodeError;

    fn try_from(raw: String) -> Result<Self, DecodeError> {
        if raw.trim().is_empty() {
            return Err(DecodeError::new(
                "relPath",
                "invalid relative path: must not be empty",
            ));
        }
        let normalized = raw.replace('\\', "/");
        if normalized.starts_with('/') || windows_drive_root_marker(&normalized).is_some() {
            return Err(DecodeError::new(
                "relPath",
                "invalid relative path: must be relative (no leading separator or drive letter)",
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
                            "invalid relative path: `..` segment escapes the repository root",
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

macro_rules! validated_path_wire {
    ($value:ty) => {
        impl serde::Serialize for $value {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $value {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::try_from(raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_path_wire!(RepoRoot);
validated_path_wire!(RelPath);
