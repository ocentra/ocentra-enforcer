//! c07 — the pre-commit hook emitters (git-hook / husky / lefthook).
//!
//! # Charter
//!
//! `init`/`buildInitWrites` (the retired `.mjs` engine) could emit a
//! pre-commit hook in one of three flavors, selected per-adapter: a plain
//! git hook script, a husky script, or a lefthook config block. This
//! module ports that mechanically: one bundled template per flavor
//! (`adapters/{git-hooks/pre-commit.sh,husky/pre-commit,lefthook/lefthook.yml}`
//! in this repo), each written to its own fixed target path.
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

use std::path::{Path, PathBuf};

/// Which pre-commit hook mechanism to emit. Selecting one never writes
/// another flavor's file (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookFlavor {
    /// Plain git hook: `.git/hooks/pre-commit`.
    PlainGitHook,
    /// Husky: `.husky/pre-commit`.
    Husky,
    /// Lefthook: `lefthook.yml` (repo root, not under a subdirectory).
    Lefthook,
}

impl HookFlavor {
    fn target_relative_path(self) -> &'static str {
        match self {
            Self::PlainGitHook => ".git/hooks/pre-commit",
            Self::Husky => ".husky/pre-commit",
            Self::Lefthook => "lefthook.yml",
        }
    }

    fn bundled_template(self) -> &'static str {
        match self {
            Self::PlainGitHook => include_str!("../../../../adapters/git-hooks/pre-commit.sh"),
            Self::Husky => include_str!("../../../../adapters/husky/pre-commit"),
            Self::Lefthook => include_str!("../../../../adapters/lefthook/lefthook.yml"),
        }
    }

    /// Whether the target file should be marked executable once written
    /// (the two shell-script flavors; lefthook's YAML is not executed
    /// directly).
    #[must_use]
    pub fn is_executable_script(self) -> bool {
        matches!(self, Self::PlainGitHook | Self::Husky)
    }
}

/// One planned write for exactly one [`HookFlavor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedWrite {
    /// Absolute target path for the selected flavor's file only.
    pub path: PathBuf,
    /// The exact bytes that would be written.
    pub contents: &'static str,
    /// The flavor this write belongs to.
    pub flavor: HookFlavor,
}

/// Compute the planned write for `flavor` under `root`. Pure: never
/// touches disk, and returns exactly one entry — the selected flavor's
/// file, never any other flavor's.
#[must_use]
pub fn plan(root: &Path, flavor: HookFlavor) -> Vec<PlannedWrite> {
    vec![PlannedWrite {
        path: root.join(flavor.target_relative_path()),
        contents: flavor.bundled_template(),
        flavor,
    }]
}

/// Outcome of one planned write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedWrite {
    /// The write that was attempted.
    pub planned: PlannedWrite,
    /// `true` if a file was actually created/overwritten; `false` when an
    /// existing file was left alone because `force` was not set.
    pub wrote: bool,
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
pub fn apply(root: &Path, flavor: HookFlavor, force: bool) -> std::io::Result<Vec<AppliedWrite>> {
    let planned = plan(root, flavor);
    let mut applied = Vec::with_capacity(planned.len());
    for write in planned {
        if let Some(parent) = write.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let already_exists = write.path.is_file();
        let wrote = if already_exists && !force {
            false
        } else {
            std::fs::write(&write.path, write.contents)?;
            mark_executable_if_applicable(&write.path, write.flavor)?;
            true
        };
        applied.push(AppliedWrite {
            planned: write,
            wrote,
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
#[must_use]
pub fn verify(root: &Path, flavor: HookFlavor) -> Vec<crate::report::VerifyCheck> {
    plan(root, flavor)
        .into_iter()
        .map(|write| {
            let on_disk = std::fs::read_to_string(&write.path).ok();
            let passed = on_disk.as_deref() == Some(write.contents);
            crate::report::VerifyCheck {
                harness: "git-hooks-emitter".to_owned(),
                name: format!("hook-present:{}", write.path.display()),
                passed,
                detail: if passed {
                    String::new()
                } else {
                    format!("missing or drifted hook file at `{}`", write.path.display())
                },
            }
        })
        .collect()
}

#[cfg(unix)]
fn mark_executable_if_applicable(path: &Path, flavor: HookFlavor) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if flavor.is_executable_script() {
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
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/git-hooks")
            .join(name)
    }

    #[test]
    fn plain_git_hook_alone_never_writes_husky_or_lefthook(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(dir.path(), HookFlavor::PlainGitHook, false)?;

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
        apply(dir.path(), HookFlavor::Husky, false)?;

        assert!(dir.path().join(".husky/pre-commit").is_file());
        assert!(!dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!dir.path().join("lefthook.yml").exists());
        Ok(())
    }

    #[test]
    fn lefthook_alone_never_writes_plain_hook_or_husky() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(dir.path(), HookFlavor::Lefthook, false)?;

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
            apply(dir.path(), flavor, false)?;
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
        let planned = plan(dir.path(), HookFlavor::Lefthook);
        assert_eq!(planned.len(), 1);
        assert!(!dir.path().join("lefthook.yml").exists());
        Ok(())
    }

    #[test]
    fn apply_skips_an_existing_file_unless_forced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("lefthook.yml"), "hand-edited, keep me")?;

        let applied = apply(dir.path(), HookFlavor::Lefthook, false)?;
        assert!(!applied[0].wrote);
        let contents = std::fs::read_to_string(dir.path().join("lefthook.yml"))?;
        assert_eq!(contents, "hand-edited, keep me");
        Ok(())
    }

    #[test]
    fn apply_overwrites_an_existing_file_when_forced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("lefthook.yml"), "stale content")?;

        let applied = apply(dir.path(), HookFlavor::Lefthook, true)?;
        assert!(applied[0].wrote);
        let contents = std::fs::read_to_string(dir.path().join("lefthook.yml"))?;
        assert_ne!(contents, "stale content");
        Ok(())
    }

    #[test]
    fn verify_is_green_after_apply_and_red_when_the_hook_goes_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(dir.path(), HookFlavor::Husky, false)?;
        let checks = verify(dir.path(), HookFlavor::Husky);
        assert_eq!(checks.len(), 1);
        assert!(checks[0].passed);

        std::fs::remove_file(dir.path().join(".husky/pre-commit"))?;
        let checks = verify(dir.path(), HookFlavor::Husky);
        assert!(!checks[0].passed);
        Ok(())
    }
}
