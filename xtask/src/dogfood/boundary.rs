//! a10 boundary: the raw-text and effectful surfaces of the dogfood loop.
//!
//! Two concerns live here, both boundary-shaped by nature:
//! - translating `ocentra-enforcer.config.json`'s `ignoreFileGlobs` (raw
//!   glob strings) into the [`walk::IgnoreRules`] representation the scan
//!   walker matches on;
//! - spawning the standard Rust toolchain subprocesses (`cargo fmt`/
//!   `clippy`/`deny`/`audit`) and folding their raw exit status + output
//!   into typed [`StepOutcome`]s.
//!
//! The domain half ([`crate::dogfood`]) consumes only the typed results.

use std::path::Path;
use std::process::Command;

use enforcer_scan::walk;

use crate::dogfood::DogfoodError;

/// Translate a directory-shaped ignore glob (`**/<segment>/**`,
/// `<segment>/**`) into the bare directory segment
/// [`walk::IgnoreRules::ignore_dirs`] matches on. Returns `None` for any
/// glob that is not a single-segment directory shape (those stay in
/// `ignore_file_globs`, where `walk`'s deliberately-minimal matcher
/// applies its own fails-closed semantics: a glob it cannot express never
/// matches, i.e. over-scans).
fn dir_segment_of(glob: &str) -> Option<String> {
    let inner = glob.strip_prefix("**/").unwrap_or(glob);
    let segment = inner.strip_suffix("/**")?;
    if segment.is_empty() || segment.contains('/') || segment.contains('*') {
        return None;
    }
    Some(String::from(segment))
}

/// Build the [`walk::IgnoreRules`] the self-scan walks with: the
/// workspace's own `ocentra-enforcer.config.json` `ignoreFileGlobs`, so
/// the scan stays scoped to product source (not fixtures/docs/vendor).
///
/// `walk`'s glob matcher only supports one leading OR trailing `*`, so
/// the config's directory-shaped `**/<segment>/**` entries (the dominant
/// shape in the committed config: `**/fixtures/**`, `**/vendor/**`, ...)
/// are translated into `ignore_dirs` segments -- the representation
/// `walk` matches exactly -- rather than being handed to the glob
/// matcher, where they would silently never match and the self-scan would
/// flood with intentional fixture findings.
///
/// # Errors
/// Returns [`DogfoodError`] when the project config fails to load or
/// validate (an invalid config is rejected, never defaulted around).
pub fn ignore_rules_from_config(repo_root: &Path) -> Result<walk::IgnoreRules, DogfoodError> {
    let config_file = repo_root.join("ocentra-enforcer.config.json");
    let effective =
        enforcer_config::load_project_config(&config_file).map_err(DogfoodError::from_display)?;
    let mut ignore_dirs = Vec::new();
    let mut ignore_file_globs = Vec::new();
    for glob in &effective.ignore_file_globs {
        if let Some(segment) = dir_segment_of(glob.as_str()) {
            ignore_dirs.push(segment);
        } else {
            ignore_file_globs.push(String::from(glob.as_str()));
        }
    }
    Ok(walk::IgnoreRules {
        ignore_dirs,
        ignore_file_globs,
    })
}

/// One toolchain step's outcome. `Skipped` is only ever produced for a
/// NON-required step (see [`run_toolchain_checks`]) -- it is never used
/// to silently absorb a required step's failure, and it always carries
/// the raw evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "One toolchain step's typed outcome; see the note above."]
pub enum StepOutcome {
    /// The step ran and exited zero.
    Passed,
    /// The step did not block the gate: its tool was unavailable or it
    /// exited non-zero, but this step is not required (per the project
    /// config's `requireCargoDeny`/`requireCargoAudit`), so it is
    /// reported, visibly, rather than silently dropped.
    Skipped {
        /// Why -- the tool's own output, or the spawn failure.
        reason: String,
    },
    /// The step ran (or tried to) and blocks the gate.
    Failed {
        /// The tool's own output, or the spawn failure.
        detail: String,
    },
}

impl StepOutcome {
    /// True when this step blocks the overall gate.
    pub fn blocks(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

/// The standard Rust toolchain half of the dogfood loop. `fmt`/`clippy`
/// are always required (the workspace's own `[workspace.lints]`
/// deny-wall); `deny`/`audit` are required only when the project config
/// says so -- an unmet-but-not-required step is an honest, visible skip,
/// never a silent pass.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "The four toolchain steps' outcomes; see the note above."]
pub struct ToolchainOutcome {
    /// `cargo fmt --all --check`.
    pub fmt: StepOutcome,
    /// `cargo clippy --all-targets -- -D warnings`.
    pub clippy: StepOutcome,
    /// `cargo deny check`.
    pub deny: StepOutcome,
    /// `cargo audit`.
    pub audit: StepOutcome,
}

impl ToolchainOutcome {
    /// True when no step blocks the gate.
    pub fn passes(&self) -> bool {
        !self.fmt.blocks() && !self.clippy.blocks() && !self.deny.blocks() && !self.audit.blocks()
    }
}

/// Spawn one `cargo` step and fold its status into a [`StepOutcome`],
/// honoring the step's requiredness.
fn run_step(repo_root: &Path, step_args: &[&str], required: bool) -> StepOutcome {
    match Command::new("cargo")
        .args(step_args)
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => StepOutcome::Passed,
        Ok(output) => {
            let detail = format!(
                "`cargo {}` exited {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
                step_args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if required {
                StepOutcome::Failed { detail }
            } else {
                StepOutcome::Skipped { reason: detail }
            }
        }
        Err(err) => {
            let reason = format!(
                "`cargo {}` could not be spawned: {err}",
                step_args.join(" ")
            );
            if required {
                StepOutcome::Failed { detail: reason }
            } else {
                StepOutcome::Skipped { reason }
            }
        }
    }
}

/// Run the four toolchain steps against `repo_root`, reading the
/// `requireCargoDeny`/`requireCargoAudit` posture from the project
/// config (both `false` in the committed config today, since `cargo-deny`
/// is not installed in every environment this repo runs in).
///
/// # Errors
/// Returns [`DogfoodError`] when the project config fails to load -- an
/// individual step failing is a typed [`StepOutcome`], never an `Err`.
pub fn run_toolchain_checks(repo_root: &Path) -> Result<ToolchainOutcome, DogfoodError> {
    let config_file = repo_root.join("ocentra-enforcer.config.json");
    let effective =
        enforcer_config::load_project_config(&config_file).map_err(DogfoodError::from_display)?;
    let policy = &effective.cargo_dependency_policy;
    Ok(ToolchainOutcome {
        fmt: run_step(repo_root, &["fmt", "--all", "--check"], true),
        clippy: run_step(
            repo_root,
            &["clippy", "--all-targets", "--", "-D", "warnings"],
            true,
        ),
        deny: run_step(repo_root, &["deny", "check"], policy.require_cargo_deny),
        audit: run_step(repo_root, &["audit"], policy.require_cargo_audit),
    })
}

#[cfg(test)]
mod tests {
    use super::{dir_segment_of, ignore_rules_from_config, StepOutcome};
    use crate::boundary::testkit::seed_config;

    #[test]
    fn directory_shaped_globs_translate_to_segments() {
        assert_eq!(
            dir_segment_of("**/fixtures/**"),
            Some(String::from("fixtures"))
        );
        assert_eq!(dir_segment_of("vendor/**"), Some(String::from("vendor")));
    }

    #[test]
    fn non_directory_or_malformed_globs_stay_file_globs() {
        // Invalid/malformed directory shapes must NOT become segments:
        // an empty segment, a nested path, an embedded wildcard, and a
        // bare file glob all fall through to the fails-closed matcher.
        assert_eq!(dir_segment_of("**//**"), None);
        assert_eq!(dir_segment_of("**/tests/fixtures/**"), None);
        assert_eq!(dir_segment_of("**/fix*res/**"), None);
        assert_eq!(dir_segment_of("**/README.md"), None);
        assert_eq!(dir_segment_of(""), None);
    }

    #[test]
    fn config_globs_partition_into_dirs_and_file_globs() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        let rules = ignore_rules_from_config(temp.path()).map_err(std::io::Error::other)?;
        assert!(rules
            .ignore_dirs
            .iter()
            .any(|segment| segment == "fixtures"));
        Ok(())
    }

    #[test]
    fn missing_config_falls_back_to_default_profile() -> Result<(), std::io::Error> {
        // `enforcer-config`'s own documented contract: no project config
        // file means the `default` profile, not an error.
        let temp = tempfile::tempdir()?;
        let rules = ignore_rules_from_config(temp.path()).map_err(std::io::Error::other)?;
        assert!(rules.ignore_file_globs.is_empty());
        Ok(())
    }

    #[test]
    fn only_failed_steps_block() {
        assert!(!StepOutcome::Passed.blocks());
        let skipped = StepOutcome::Skipped {
            reason: String::from("tool not installed"),
        };
        assert!(!skipped.blocks());
        let failed = StepOutcome::Failed {
            detail: String::from("exit status 1"),
        };
        assert!(failed.blocks());
    }
}
