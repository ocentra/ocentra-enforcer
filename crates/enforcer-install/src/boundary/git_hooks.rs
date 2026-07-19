//! c07 — the pre-commit hook emitters (git-hook / husky / lefthook).
//!
//! # Charter
//!
//! `init`/`buildInitWrites` (the retired `.mjs` engine) could emit a
//! pre-commit hook in one of three flavors, selected per-adapter: a plain
//! git hook script, a husky script, or a lefthook config block. This
//! module ports that mechanically: one bundled template per flavor
//! (`adapters/{git-hooks/pre-commit.sh,husky/pre-commit,lefthook/lefthook.yml}`
//! in this repo), each written to its own target path. A linked worktree's
//! plain hook uses a worktree-local `.enforcer/hooks` directory and its
//! `config.worktree` `core.hooksPath`; it never writes the shared
//! `<common-git-dir>/hooks` directory.
//!
//! # Flavor isolation (never bleed)
//!
//! [`plan`] takes exactly one [`HookFlavor`] and returns a plan containing
//! ONLY that flavor's single target file — there is no code path in this
//! module that can touch a different flavor's file as a side effect of
//! selecting one. This is the workpack's explicit acceptance bar: choosing
//! `precommit` alone must never also write `.husky/pre-commit` or
//! `lefthook.yml`.
//!
//! # Distinct from c04/c05
//!
//! These are CONSUMER pre-commit mechanisms (a shell script / husky
//! script / lefthook YAML block a consumer repo's git commit invokes).
//! They are unrelated to c04 (`Claude PreToolUse` hook) and c05 (`Claude
//! SessionStart` hook), which are Claude-specific in-session mechanisms,
//! not consumer-repo git hooks.
//!
//! # `--dry-run` / force semantics
//!
//! Same contract as [`crate::emitters::consumer_ci`]: [`plan`] never
//! touches disk; [`apply`] is the only writer, and skips an existing
//! target unless `force` is set.

//! BOUNDARY-INVARIANT: hook payloads are decoded before hook behavior is selected.
//! Negative invalid inputs are rejected before hook rendering.
//!
use crate::error::{InstallError, InstallResult};
use enforcer_domain::install_types::{
    CheckStatus, CheckSubject, FileWriteOutcome, HookFlavor, InstallReportText, InstallRootPath,
    InstallTargetPath, InstallVerifyCheck, OverwriteMode,
};
use std::path::{Path, PathBuf};

/// The worktree-local hook directory used when `root/.git` is a linked
/// worktree indirection file. Git otherwise resolves its default hooks path
/// through the shared common git directory, which would make installing a
/// hook in one worktree silently mutate every checkout that shares it.
const WORKTREE_HOOKS_PATH: &str = ".enforcer/hooks";

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkedWorktree {
    /// The per-worktree administrative git directory named by `root/.git`.
    git_dir: PathBuf,
    /// The worktree-local config read when `extensions.worktreeConfig` is on.
    config_path: PathBuf,
}

fn linked_worktree(root: &Path) -> InstallResult<Option<LinkedWorktree>> {
    let git_entry = root.join(".git");
    let metadata = match std::fs::symlink_metadata(&git_entry) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(InstallError::Io {
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
                path: git_entry.display().to_string(),
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
                reason: error.to_string(),
            });
        }
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&git_entry).map_err(|error| InstallError::Io {
        // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
        path: git_entry.display().to_string(),
        // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
        reason: error.to_string(),
    })?;
    let Some(raw_git_dir) = contents
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(InstallError::MalformedConfig {
            // ALLOC-JUSTIFICATION: this path identifies the process-boundary git metadata.
            path: git_entry.display().to_string(),
            // ALLOC-JUSTIFICATION: this diagnostic is a fixed, validated explanation.
            reason: "linked worktree .git file must contain a non-empty `gitdir:` entry".to_owned(),
        });
    };

    let git_dir = PathBuf::from(raw_git_dir);
    let git_dir = if git_dir.is_absolute() {
        git_dir
    } else {
        git_entry.parent().unwrap_or(root).join(git_dir)
    };
    Ok(Some(LinkedWorktree {
        config_path: git_dir.join("config.worktree"),
        git_dir,
    }))
}

fn target_path(root: &InstallRootPath, flavor: HookFlavor) -> InstallResult<InstallTargetPath> {
    let relative = match flavor {
        HookFlavor::PlainGitHook if linked_worktree(root.as_path())?.is_some() => {
            Path::new(WORKTREE_HOOKS_PATH).join("pre-commit")
        }
        HookFlavor::PlainGitHook => PathBuf::from(".git/hooks/pre-commit"),
        HookFlavor::Husky => PathBuf::from(".husky/pre-commit"),
        HookFlavor::Lefthook => PathBuf::from("lefthook.yml"),
    };
    Ok(root.join_target(relative))
}

fn config_has_worktree_extensions(config_path: &Path) -> InstallResult<bool> {
    let contents = match std::fs::read_to_string(config_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(InstallError::Io {
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
                path: config_path.display().to_string(),
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
                reason: error.to_string(),
            });
        }
    };
    Ok(config_value(&contents, "extensions", "worktreeConfig")
        .is_some_and(|value| value.eq_ignore_ascii_case("true")))
}

fn config_value<'a>(contents: &'a str, section_name: &str, key_name: &str) -> Option<&'a str> {
    let mut section = String::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(section_text) = trimmed
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
        {
            section = section_text.trim().to_ascii_lowercase();
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if section == section_name.to_ascii_lowercase() && key.trim().eq_ignore_ascii_case(key_name)
        {
            return Some(value.trim());
        }
    }
    None
}

fn set_worktree_hooks_path(worktree: &LinkedWorktree) -> InstallResult<()> {
    let common_dir_path = worktree.git_dir.join("commondir");
    let common_dir = match std::fs::read_to_string(&common_dir_path) {
        Ok(value) => {
            let value = PathBuf::from(value.trim());
            if value.is_absolute() {
                value
            } else {
                common_dir_path
                    .parent()
                    .unwrap_or(&worktree.git_dir)
                    .join(value)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A linked worktree without `commondir` is not a valid Git layout.
            return Err(InstallError::MalformedConfig {
                // ALLOC-JUSTIFICATION: this path identifies the process-boundary git metadata.
                path: common_dir_path.display().to_string(),
                // ALLOC-JUSTIFICATION: this diagnostic is a fixed, validated explanation.
                reason: "linked worktree is missing its `commondir` metadata".to_owned(),
            });
        }
        Err(error) => {
            return Err(InstallError::Io {
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
                path: common_dir_path.display().to_string(),
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
                reason: error.to_string(),
            });
        }
    };
    let common_config = common_dir.join("config");
    if !config_has_worktree_extensions(&common_config)? {
        return Err(InstallError::MalformedConfig {
            // ALLOC-JUSTIFICATION: this path identifies the shared config we refuse to mutate.
            path: common_config.display().to_string(),
            // ALLOC-JUSTIFICATION: this diagnostic explicitly explains the safety boundary.
            reason: "linked worktree requires `extensions.worktreeConfig = true`; refusing to mutate the shared config".to_owned(),
        });
    }

    let existing = match std::fs::read_to_string(&worktree.config_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(InstallError::Io {
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
                path: worktree.config_path.display().to_string(),
                // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
                reason: error.to_string(),
            });
        }
    };
    let updated =
        replace_or_append_config_value(&existing, "core", "hooksPath", WORKTREE_HOOKS_PATH);
    if updated != existing {
        std::fs::write(&worktree.config_path, updated).map_err(|error| InstallError::Io {
            // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
            path: worktree.config_path.display().to_string(),
            // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
            reason: error.to_string(),
        })?;
    }
    Ok(())
}

fn replace_or_append_config_value(
    contents: &str,
    section_name: &str,
    key_name: &str,
    value: &str,
) -> String {
    let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
    let mut section_start = None;
    let mut section_end = lines.len();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(section) = trimmed
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
        {
            let section = section.trim();
            if section.eq_ignore_ascii_case(section_name) {
                section_start = Some(index);
                section_end = lines.len();
            } else if section_start.is_some() && section_end == lines.len() {
                section_end = index;
            }
        }
    }
    if let Some(start) = section_start {
        for line in lines.iter_mut().take(section_end).skip(start + 1) {
            let trimmed = line.trim_start();
            if let Some((key, _)) = trimmed.split_once('=') {
                if key.trim().eq_ignore_ascii_case(key_name) {
                    *line = format!("\thooksPath = {value}");
                    return lines.join("\n") + if contents.ends_with('\n') { "\n" } else { "" };
                }
            }
        }
        lines.insert(section_end, format!("\thooksPath = {value}"));
    } else {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("[{section_name}]"));
        lines.push(format!("\thooksPath = {value}"));
    }
    lines.join("\n")
        + if contents.ends_with('\n') || contents.is_empty() {
            "\n"
        } else {
            ""
        }
}

/// Which pre-commit hook mechanism to emit. Selecting one never writes
/// another flavor's file (see module docs).
fn bundled_template(flavor: HookFlavor) -> &'static str {
    match flavor {
        HookFlavor::PlainGitHook => include_str!("../../../../adapters/git-hooks/pre-commit.sh"),
        HookFlavor::Husky => include_str!("../../../../adapters/husky/pre-commit"),
        HookFlavor::Lefthook => include_str!("../../../../adapters/lefthook/lefthook.yml"),
    }
}

#[cfg(unix)]
fn is_executable_script(flavor: HookFlavor) -> bool {
    matches!(flavor, HookFlavor::PlainGitHook | HookFlavor::Husky)
}

/// One planned write for exactly one [`HookFlavor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedWrite {
    /// Absolute target path for the selected flavor's file only.
    pub path: InstallTargetPath,
    /// The exact bytes that would be written.
    pub contents: InstallReportText,
    /// The flavor this write belongs to.
    pub flavor: HookFlavor,
}

/// Compute the planned write for `flavor` under `root`. Pure: never
/// touches disk, and returns exactly one entry — the selected flavor's
/// file, never any other flavor's.
pub fn plan(root: &InstallRootPath, flavor: HookFlavor) -> InstallResult<Vec<PlannedWrite>> {
    Ok(vec![PlannedWrite {
        path: target_path(root, flavor)?,
        // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
        contents: InstallReportText::try_from(bundled_template(flavor).to_owned())?,
        flavor,
    }])
}

/// Outcome of one planned write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedWrite {
    /// The write that was attempted.
    pub planned: PlannedWrite,
    /// `true` if a file was actually created/overwritten; `false` when an
    /// existing file was left alone because `force` was not set.
    pub outcome: FileWriteOutcome,
}

/// Execute [`plan`]'s single-flavor output against `root`'s real
/// filesystem. Never called for `--dry-run`. Skip-existing-unless-`force`
/// semantics match [`crate::emitters::consumer_ci::apply`].
///
/// # Errors
/// Returns a [`std::io::Error`] the moment the write (or its parent
/// directory creation) fails. On Unix, the script flavors are marked
/// executable (`0o755`) after writing; this step is a no-op on
/// non-Unix targets.
pub fn apply(
    root: &InstallRootPath,
    flavor: HookFlavor,
    overwrite: OverwriteMode,
) -> InstallResult<Vec<AppliedWrite>> {
    if flavor == HookFlavor::PlainGitHook {
        if let Some(worktree) = linked_worktree(root.as_path())? {
            set_worktree_hooks_path(&worktree)?;
        }
    }
    let planned = plan(root, flavor)?;
    let mut applied = Vec::with_capacity(planned.len());
    for write in planned {
        if let Some(parent) = write.path.as_path().parent() {
            std::fs::create_dir_all(parent).map_err(|error| InstallError::Io {
                // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                path: parent.display().to_string(),
                // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                reason: error.to_string(),
            })?;
        }
        let already_exists = write.path.as_path().is_file();
        let outcome = if already_exists && overwrite == OverwriteMode::PreserveExisting {
            FileWriteOutcome::PreservedExisting
        } else {
            std::fs::write(write.path.as_path(), write.contents.as_str()).map_err(|error| {
                InstallError::Io {
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    path: write.path.as_path().display().to_string(),
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    reason: error.to_string(),
                }
            })?;
            mark_executable_if_applicable(write.path.as_path(), write.flavor).map_err(|error| {
                InstallError::Io {
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    path: write.path.as_path().display().to_string(),
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    reason: error.to_string(),
                }
            })?;
            FileWriteOutcome::Written
        };
        applied.push(AppliedWrite {
            planned: write,
            outcome,
        });
    }
    Ok(applied)
}

/// Mechanical, read-only health check for one flavor: does `root`
/// currently have that flavor's target file, with bytes matching the
/// bundled template. Feeds the shared [`crate::doctor`] aggregation the
/// same way a [`crate::core::HarnessAdapter::verify`] check does —
/// re-reads disk at call time, never trusts a previous [`plan`]/[`apply`]
/// outcome.
pub fn verify(
    root: &InstallRootPath,
    flavor: HookFlavor,
) -> InstallResult<Vec<InstallVerifyCheck>> {
    let linked = if flavor == HookFlavor::PlainGitHook {
        linked_worktree(root.as_path())?
    } else {
        None
    };
    plan(root, flavor)?
        .into_iter()
        .map(|write| -> InstallResult<InstallVerifyCheck> {
            let on_disk = match std::fs::read_to_string(write.path.as_path()) {
                Ok(contents) => Some(contents),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(InstallError::Io {
                        // ALLOC-JUSTIFICATION: the typed I/O diagnostic must retain the failing path.
                        path: write.path.as_path().display().to_string(),
                        // ALLOC-JUSTIFICATION: the typed I/O diagnostic must own the source error text.
                        reason: error.to_string(),
                    });
                }
            };
            let hooks_configured = linked
                .as_ref()
                .map(|worktree| {
                    let config = std::fs::read_to_string(&worktree.config_path).ok();
                    config.as_deref().is_some_and(|contents| {
                        config_value(contents, "core", "hooksPath") == Some(WORKTREE_HOOKS_PATH)
                    })
                })
                .unwrap_or(true);
            let passed = on_disk.as_deref() == Some(write.contents.as_str()) && hooks_configured;
            Ok(InstallVerifyCheck {
                subject: CheckSubject::SkillAsset(InstallReportText::try_from(
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    write.path.as_path().display().to_string(),
                )?),
                name: InstallReportText::try_from(format!(
                    "hook-present:{}",
                    write.path.as_path().display()
                ))?,
                status: if passed {
                    CheckStatus::Passed
                } else {
                    CheckStatus::Failed
                },
                detail: InstallReportText::try_from(if passed {
                    String::new()
                } else if hooks_configured {
                    format!(
                        "missing or drifted hook file at `{}`",
                        write.path.as_path().display()
                    )
                } else {
                    format!(
                        "missing or drifted worktree `core.hooksPath` at `{}`",
                        linked
                            .as_ref()
                            .map(|worktree| worktree.config_path.display().to_string())
                            .unwrap_or_default()
                    )
                })?,
            })
        })
        .collect()
}

#[cfg(unix)]
fn mark_executable_if_applicable(path: &Path, flavor: HookFlavor) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if is_executable_script(flavor) {
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn mark_executable_if_applicable(_path: &Path, _flavor: HookFlavor) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{apply, plan, verify, HookFlavor};
    use enforcer_domain::install_types::{InstallRootPath, OverwriteMode};
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/git-hooks")
            .join(name)
    }

    fn root(path: &std::path::Path) -> Result<InstallRootPath, Box<dyn std::error::Error>> {
        Ok(InstallRootPath::try_from(path.to_path_buf())?)
    }

    #[test]
    fn plain_git_hook_alone_never_writes_husky_or_lefthook(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(
            &root(dir.path())?,
            HookFlavor::PlainGitHook,
            OverwriteMode::PreserveExisting,
        )?;

        assert!(dir.path().join(".git/hooks/pre-commit").is_file());
        assert!(
            !dir.path().join(".husky/pre-commit").exists(),
            "selecting plain git-hook must never write husky"
        );
        assert!(
            !dir.path().join("lefthook.yml").exists(),
            "selecting plain git-hook must never write lefthook"
        );
        Ok(())
    }

    #[test]
    fn husky_alone_never_writes_plain_hook_or_lefthook() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(
            &root(dir.path())?,
            HookFlavor::Husky,
            OverwriteMode::PreserveExisting,
        )?;

        assert!(dir.path().join(".husky/pre-commit").is_file());
        assert!(!dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!dir.path().join("lefthook.yml").exists());
        Ok(())
    }

    #[test]
    fn lefthook_alone_never_writes_plain_hook_or_husky() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(
            &root(dir.path())?,
            HookFlavor::Lefthook,
            OverwriteMode::PreserveExisting,
        )?;

        assert!(dir.path().join("lefthook.yml").is_file());
        assert!(!dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!dir.path().join(".husky/pre-commit").exists());
        Ok(())
    }

    #[test]
    fn each_flavor_writes_golden_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            (
                HookFlavor::PlainGitHook,
                ".git/hooks/pre-commit",
                "pre-commit.sh",
            ),
            (HookFlavor::Husky, ".husky/pre-commit", "husky-pre-commit"),
            (HookFlavor::Lefthook, "lefthook.yml", "lefthook.yml"),
        ];
        for (flavor, target_rel, golden_name) in cases {
            let dir = tempfile::tempdir()?;
            apply(&root(dir.path())?, flavor, OverwriteMode::PreserveExisting)?;
            let actual = std::fs::read_to_string(dir.path().join(target_rel))?;
            let golden = std::fs::read_to_string(fixture_root(golden_name))?;
            assert_eq!(actual, golden, "byte mismatch for {target_rel}");
        }
        Ok(())
    }

    #[test]
    fn dry_run_returns_the_single_planned_write_with_zero_files_created(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let planned = plan(&root(dir.path())?, HookFlavor::Lefthook)?;
        assert_eq!(planned.len(), 1);
        assert!(!dir.path().join("lefthook.yml").exists());
        Ok(())
    }

    #[test]
    fn apply_skips_an_existing_file_unless_forced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("lefthook.yml"), "hand-edited, keep me")?;

        let applied = apply(
            &root(dir.path())?,
            HookFlavor::Lefthook,
            OverwriteMode::PreserveExisting,
        )?;
        assert!(!matches!(
            applied[0].outcome,
            enforcer_domain::install_types::FileWriteOutcome::Written
        ));
        let contents = std::fs::read_to_string(dir.path().join("lefthook.yml"))?;
        assert_eq!(contents, "hand-edited, keep me");
        Ok(())
    }

    #[test]
    fn apply_overwrites_an_existing_file_when_forced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("lefthook.yml"), "stale content")?;

        let applied = apply(
            &root(dir.path())?,
            HookFlavor::Lefthook,
            OverwriteMode::Force,
        )?;
        assert!(matches!(
            applied[0].outcome,
            enforcer_domain::install_types::FileWriteOutcome::Written
        ));
        let contents = std::fs::read_to_string(dir.path().join("lefthook.yml"))?;
        assert_ne!(contents, "stale content");
        Ok(())
    }

    #[test]
    fn verify_is_green_after_apply_and_red_when_the_hook_goes_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(
            &root(dir.path())?,
            HookFlavor::Husky,
            OverwriteMode::PreserveExisting,
        )?;
        let checks = verify(&root(dir.path())?, HookFlavor::Husky)?;
        assert_eq!(checks.len(), 1);
        assert!(matches!(
            checks[0].status,
            enforcer_domain::install_types::CheckStatus::Passed
        ));

        std::fs::remove_file(dir.path().join(".husky/pre-commit"))?;
        let checks = verify(&root(dir.path())?, HookFlavor::Husky)?;
        assert!(!matches!(
            checks[0].status,
            enforcer_domain::install_types::CheckStatus::Passed
        ));
        Ok(())
    }
}
