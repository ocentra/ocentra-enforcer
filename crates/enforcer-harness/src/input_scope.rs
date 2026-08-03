//! BOUNDARY-INVARIANT: enumerate only one explicit, bounded Cargo input scope.
//!
//! The scope is exactly `Cargo.toml`, `Cargo.lock`, and recursive `src/**`
//! below the reviewed disposable cwd. The target directory is an exclusion
//! derived from the already validated command argument.

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use enforcer_core::error::{Error, Result};
use enforcer_domain::boundary::hash::validate;
use enforcer_domain::harness_types::{HarnessCommandArgument, HarnessInputLimits};
use enforcer_domain::hashes::Sha256;

use crate::execution::ExecuteRequest;

const SCOPE_POLICY: &str = "cargo-input-scope-v1";

/// Computed evidence for one complete reviewed input scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputedInputTree {
    digest: Sha256,
    file_count: u32,
    total_bytes: u64,
    excluded_target: String,
}

impl ComputedInputTree {
    /// Digest of policy, exclusion, normalized paths, lengths, and bytes.
    #[must_use]
    pub const fn digest(&self) -> &Sha256 {
        &self.digest
    }

    /// Number of regular files included in the complete scope.
    #[must_use]
    pub const fn file_count(&self) -> u32 {
        self.file_count
    }

    /// Total bytes included in the complete scope.
    #[must_use]
    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Normalized target directory excluded from the scope digest.
    #[must_use]
    pub fn excluded_target(&self) -> &str {
        &self.excluded_target
    }
}

/// Compute the closed Cargo input scope from the reviewed request and target.
pub fn compute_input_tree(
    request: &ExecuteRequest,
    target_dir: &HarnessCommandArgument,
    limits: HarnessInputLimits,
) -> Result<ComputedInputTree> {
    let root = canonical_root(request)?;
    let cwd = reviewed_cwd(request, &root)?;
    let target = target_dir.as_str();
    let target_relative = validate_target(&root, &cwd, target)?;

    let mut collector = Collector {
        cwd: &cwd,
        limits,
        files: Vec::new(),
        seen: HashSet::new(),
        total_bytes: 0,
    };
    collector.collect_file("Cargo.toml", 0)?;
    collector.collect_file("Cargo.lock", 0)?;
    collector.collect_directory("src", 1)?;
    if collector.files.is_empty() {
        return Err(Error::InvalidConfig(
            "Cargo input scope must contain at least one regular file".to_owned(),
        ));
    }
    collector.files.sort_by(|left, right| left.0.cmp(&right.0));
    let file_count = u32::try_from(collector.files.len()).map_err(|_| {
        Error::InvalidConfig("Cargo input scope file count exceeds typed bounds".to_owned())
    })?;
    let digest = digest_scope(&collector.files, target_relative.as_str());
    Ok(ComputedInputTree {
        digest,
        file_count,
        total_bytes: collector.total_bytes,
        excluded_target: target_relative,
    })
}

struct Collector<'a> {
    cwd: &'a Path,
    limits: HarnessInputLimits,
    files: Vec<(String, Vec<u8>)>,
    seen: HashSet<String>,
    total_bytes: u64,
}

impl Collector<'_> {
    fn collect_directory(&mut self, relative: &str, depth: u32) -> Result<()> {
        if depth > self.limits.max_depth() {
            return Err(Error::InvalidConfig(
                "Cargo input scope recursion depth exceeded".to_owned(),
            ));
        }
        let directory = self.cwd.join(relative);
        let metadata = regular_directory_metadata(&directory, relative)?;
        if !metadata.is_dir() {
            return Err(Error::InvalidConfig(format!(
                "Cargo input scope entry is not a directory: {relative}"
            )));
        }
        let entries = fs::read_dir(&directory).map_err(|error| {
            Error::InvalidConfig(format!(
                "Cargo input scope directory could not be enumerated: {relative}: {error}"
            ))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                Error::InvalidConfig(format!(
                    "Cargo input scope directory changed during enumeration: {relative}: {error}"
                ))
            })?;
            let name = entry.file_name().into_string().map_err(|_| {
                Error::InvalidConfig("Cargo input scope contains a non-UTF path".to_owned())
            })?;
            let child = normalize_relative(&format!("{relative}/{name}"))?;
            let child_metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
                Error::InvalidConfig(format!(
                    "Cargo input scope metadata could not be read: {child}: {error}"
                ))
            })?;
            if child_metadata.file_type().is_symlink() || has_reparse_point(&child_metadata) {
                return Err(Error::InvalidConfig(format!(
                    "Cargo input scope rejects symlink or reparse entry: {child}"
                )));
            }
            if child_metadata.is_dir() {
                self.collect_directory(&child, depth + 1)?;
            } else if child_metadata.is_file() {
                self.collect_file(&child, depth)?;
            } else {
                return Err(Error::InvalidConfig(format!(
                    "Cargo input scope rejects non-regular entry: {child}"
                )));
            }
        }
        Ok(())
    }

    fn collect_file(&mut self, relative: &str, depth: u32) -> Result<()> {
        if depth > self.limits.max_depth() {
            return Err(Error::InvalidConfig(
                "Cargo input scope recursion depth exceeded".to_owned(),
            ));
        }
        let normalized = normalize_relative(relative)?;
        let collision_key = if cfg!(windows) {
            normalized.to_ascii_lowercase()
        } else {
            normalized.clone()
        };
        if !self.seen.insert(collision_key) {
            return Err(Error::InvalidConfig(format!(
                "Cargo input scope contains a normalized duplicate: {normalized}"
            )));
        }
        if self.files.len() >= usize::try_from(self.limits.max_files()).unwrap_or(usize::MAX) {
            return Err(Error::InvalidConfig(
                "Cargo input scope file-count bound exceeded".to_owned(),
            ));
        }
        let path = self.cwd.join(relative);
        let snapshot = regular_file_snapshot(&path, &normalized)?;
        let length = snapshot.length;
        if length > self.limits.max_file_bytes() {
            return Err(Error::InvalidConfig(format!(
                "Cargo input scope per-file byte bound exceeded: {normalized}"
            )));
        }
        let next_total = self.total_bytes.checked_add(length).ok_or_else(|| {
            Error::InvalidConfig("Cargo input scope total byte count overflowed".to_owned())
        })?;
        if next_total > self.limits.max_total_bytes() {
            return Err(Error::InvalidConfig(
                "Cargo input scope total-byte bound exceeded".to_owned(),
            ));
        }
        let read_limit = self.limits.max_file_bytes().min(
            self.limits
                .max_total_bytes()
                .saturating_sub(self.total_bytes),
        );
        let mut file = fs::File::open(&path).map_err(|error| {
            Error::InvalidConfig(format!(
                "Cargo input scope file could not be read: {normalized}: {error}"
            ))
        })?;
        let opened_before = file_snapshot_from_metadata(
            &file.metadata().map_err(|error| {
                Error::InvalidConfig(format!(
                "Cargo input scope opened-file metadata could not be read: {normalized}: {error}"
            ))
            })?,
            &normalized,
        )?;
        if opened_before != snapshot {
            return Err(Error::InvalidConfig(format!(
                "Cargo input scope file changed before read: {normalized}"
            )));
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(read_limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| {
                Error::InvalidConfig(format!(
                    "Cargo input scope file could not be read: {normalized}: {error}"
                ))
            })?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > read_limit {
            return Err(Error::InvalidConfig(format!(
                "Cargo input scope byte bound exceeded while reading: {normalized}"
            )));
        }
        let opened_after = file_snapshot_from_metadata(
            &file.metadata().map_err(|error| {
                Error::InvalidConfig(format!(
                "Cargo input scope opened-file metadata could not be read: {normalized}: {error}"
            ))
            })?,
            &normalized,
        )?;
        let after_read = regular_file_snapshot(&path, &normalized)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != length
            || opened_after != snapshot
            || after_read != snapshot
        {
            return Err(Error::InvalidConfig(format!(
                "Cargo input scope file changed during read: {normalized}"
            )));
        }
        self.total_bytes = next_total;
        self.files.push((normalized, bytes));
        Ok(())
    }
}

fn digest_scope(files: &[(String, Vec<u8>)], excluded_target: &str) -> Sha256 {
    let mut framed = Vec::new();
    append_frame(&mut framed, SCOPE_POLICY.as_bytes());
    append_frame(&mut framed, excluded_target.as_bytes());
    for (path, bytes) in files {
        append_frame(&mut framed, path.as_bytes());
        append_frame(&mut framed, bytes.len().to_string().as_bytes());
        append_frame(&mut framed, bytes);
    }
    validate(&framed)
}

fn append_frame(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(format!("{:016x}", value.len()).as_bytes());
    output.extend_from_slice(value);
}

fn canonical_root(request: &ExecuteRequest) -> Result<PathBuf> {
    fs::canonicalize(request.repo_root.as_str()).map_err(|error| {
        Error::InvalidConfig(format!("Cargo input repository root is invalid: {error}"))
    })
}

fn reviewed_cwd(request: &ExecuteRequest, root: &Path) -> Result<PathBuf> {
    let relative = request.cwd.as_deref().unwrap_or("");
    if Path::new(relative).components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(Error::InvalidConfig(
            "Cargo input cwd must be repository-relative".to_owned(),
        ));
    }
    let candidate = Path::new(request.repo_root.as_str()).join(relative);
    reject_link_components(&candidate, root, "Cargo input cwd")?;
    let cwd = fs::canonicalize(candidate)
        .map_err(|error| Error::InvalidConfig(format!("Cargo input cwd is invalid: {error}")))?;
    if !cwd.starts_with(root) {
        return Err(Error::InvalidConfig(
            "Cargo input cwd must remain below the repository root".to_owned(),
        ));
    }
    Ok(cwd)
}

fn validate_target(root: &Path, cwd: &Path, target: &str) -> Result<String> {
    let target_path = Path::new(target);
    if target.trim().is_empty()
        || target_path.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        return Err(Error::InvalidConfig(
            "Cargo input target must be repository-relative".to_owned(),
        ));
    }
    let normalized = normalize_relative(target)?;
    if normalized.is_empty() {
        return Err(Error::InvalidConfig(
            "Cargo input target must not be the current directory".to_owned(),
        ));
    }
    let overlap_key = comparison_key(&normalized);
    if overlap_key == "cargo.toml"
        || overlap_key == "cargo.lock"
        || overlap_key == "src"
        || overlap_key.starts_with("src/")
    {
        return Err(Error::InvalidConfig(
            "Cargo target directory overlaps the reviewed input scope".to_owned(),
        ));
    }
    let candidate = cwd.join(target_path);
    reject_link_components(&candidate, root, "Cargo input target")?;
    if candidate.exists() {
        let metadata = fs::symlink_metadata(&candidate).map_err(|error| {
            Error::InvalidConfig(format!(
                "Cargo input target metadata could not be read: {error}"
            ))
        })?;
        if !metadata.is_dir() {
            return Err(Error::InvalidConfig(
                "Cargo input target must be a directory".to_owned(),
            ));
        }
    }
    let existing = nearest_existing_path(&candidate).ok_or_else(|| {
        Error::InvalidConfig("Cargo input target has no existing contained ancestor".to_owned())
    })?;
    let existing = fs::canonicalize(existing).map_err(|error| {
        Error::InvalidConfig(format!("Cargo input target containment failed: {error}"))
    })?;
    if candidate == root || !existing.starts_with(root) {
        return Err(Error::InvalidConfig(
            "Cargo input target must remain below the repository root".to_owned(),
        ));
    }
    Ok(normalized)
}

fn normalize_relative(value: &str) -> Result<String> {
    let path = Path::new(value);
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(Error::InvalidConfig(
            "Cargo input path must be normalized and relative".to_owned(),
        ));
    }
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => components.push(value.to_str().ok_or_else(|| {
                Error::InvalidConfig("Cargo input scope contains a non-UTF path".to_owned())
            })?),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(Error::InvalidConfig(
                    "Cargo input path must be normalized and relative".to_owned(),
                ));
            }
        }
    }
    let normalized = components.join("/");
    if cfg!(windows) && !normalized.is_ascii() {
        return Err(Error::InvalidConfig(
            "Cargo input scope rejects non-ASCII Windows paths".to_owned(),
        ));
    }
    Ok(normalized)
}

fn comparison_key(value: &str) -> String {
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value.to_owned()
    }
}

fn reject_link_components(path: &Path, root: &Path, label: &str) -> Result<()> {
    let mut current = path;
    loop {
        if current.exists() {
            let metadata = fs::symlink_metadata(current).map_err(|error| {
                Error::InvalidConfig(format!("{label} metadata could not be read: {error}"))
            })?;
            if metadata.file_type().is_symlink() || has_reparse_point(&metadata) {
                return Err(Error::InvalidConfig(format!(
                    "{label} rejects symlink or reparse containment"
                )));
            }
        }
        if current == root || !current.starts_with(root) {
            break;
        }
        current = current.parent().ok_or_else(|| {
            Error::InvalidConfig(format!("{label} containment could not be evaluated"))
        })?;
    }
    Ok(())
}

fn regular_file_metadata(path: &Path, label: &str) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        Error::InvalidConfig(format!("Cargo input file is unavailable: {label}: {error}"))
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || has_reparse_point(&metadata) {
        return Err(Error::InvalidConfig(format!(
            "Cargo input file must be a regular non-link file: {label}"
        )));
    }
    Ok(metadata)
}

/// Stable metadata continuity for one bounded file read.
///
/// Unix records the stable device/inode identity. Windows records the stable
/// creation-time metadata marker because this toolchain does not expose the
/// volume/file-index APIs; that marker is not a unique OS file identifier.
/// Other platforms use the portable length/mtime fallback. In every case the
/// path snapshot and opened-handle snapshot must agree before and after
/// reading, which detects ordinary replacement, growth, shrinkage, and
/// timestamp changes. A replacement that reproduces every available metadata
/// value is outside this proof and remains part of the final TOCTOU limitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSnapshot {
    identity: FileIdentity,
    length: u64,
    modified: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileIdentity {
    #[cfg(unix)]
    Unix { device: u64, inode: u64 },
    #[cfg(windows)]
    WindowsCreationTime(u64),
    #[cfg(not(any(unix, windows)))]
    Portable,
}

fn regular_file_snapshot(path: &Path, label: &str) -> Result<FileSnapshot> {
    let metadata = regular_file_metadata(path, label)?;
    file_snapshot_from_metadata(&metadata, label)
}

fn file_snapshot_from_metadata(metadata: &fs::Metadata, label: &str) -> Result<FileSnapshot> {
    let modified = metadata.modified().map_err(|error| {
        Error::InvalidConfig(format!(
            "Cargo input file modification time could not be read: {label}: {error}"
        ))
    })?;
    Ok(FileSnapshot {
        identity: file_identity(metadata),
        length: metadata.len(),
        modified,
    })
}

fn file_identity(metadata: &fs::Metadata) -> FileIdentity {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        return FileIdentity::Unix {
            device: metadata.dev(),
            inode: metadata.ino(),
        };
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        return FileIdentity::WindowsCreationTime(metadata.creation_time());
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        FileIdentity::Portable
    }
}

fn regular_directory_metadata(path: &Path, label: &str) -> Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        Error::InvalidConfig(format!(
            "Cargo input directory is unavailable: {label}: {error}"
        ))
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || has_reparse_point(&metadata) {
        return Err(Error::InvalidConfig(format!(
            "Cargo input directory must be a regular non-link directory: {label}"
        )));
    }
    Ok(metadata)
}

fn nearest_existing_path(path: &Path) -> Option<&Path> {
    let mut current = path;
    loop {
        if current.exists() {
            return Some(current);
        }
        current = current.parent()?;
    }
}

#[cfg(windows)]
fn has_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
const fn has_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}
