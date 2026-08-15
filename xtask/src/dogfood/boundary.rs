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

use enforcer_domain::config_types::{Glob, RuleEnabled};
use enforcer_domain::scan_types::{IgnoreDirectorySegment, ScanTargetCount};
use enforcer_domain::telemetry_types::{FindingCount, RecordSchemaVersion};
use enforcer_domain::xtask_types::{
    DogfoodExecution, DogfoodGateVerdict, ToolchainOutcome, ToolchainStepOutcome,
    XtaskFailureDetail,
};
use enforcer_scan::walk;

use crate::dogfood::{DogfoodError, DogfoodOutcome};

/// Machine-readable native-only evidence emitted by `xtask dogfood`.
/// This is output data rather than a persisted proof artifact, so a normal
/// scan gate remains free of output-file side effects.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDogfoodManifestDto {
    schema_version: RecordSchemaVersion,
    execution: DogfoodExecution,
    ran_count: ScanTargetCount,
    finding_count: FindingCount,
    new_violation_count: FindingCount,
    baselined_violation_count: FindingCount,
    toolchain_included: bool,
    verdict: DogfoodGateVerdict,
}

impl NativeDogfoodManifestDto {
    /// Encode one native dogfood result. No legacy or external scanner
    /// identity is representable in this closed record.
    pub fn from_outcome(outcome: &DogfoodOutcome) -> Self {
        let scan = &outcome.rust_rule_scan;
        let toolchain_green = outcome
            .toolchain
            .as_ref()
            .is_none_or(|toolchain| matches!(toolchain.verdict(), DogfoodGateVerdict::Pass));
        let verdict = if matches!(
            scan.gate.passes(),
            enforcer_domain::findings::ReportOutcome::Clean
        ) && toolchain_green
        {
            DogfoodGateVerdict::Pass
        } else {
            DogfoodGateVerdict::Fail
        };
        Self {
            schema_version: RecordSchemaVersion::V1,
            execution: DogfoodExecution::NativeRust,
            ran_count: scan.coverage.ran_count(),
            finding_count: finding_count(scan.report.findings.len()),
            new_violation_count: finding_count(scan.gate.errors.len()),
            baselined_violation_count: finding_count(scan.gate.warnings.len()),
            toolchain_included: outcome.toolchain.is_some(),
            verdict,
        }
    }

    /// Serialize the exact JSON record the process boundary emits.
    pub fn to_json(&self) -> Result<String, DogfoodError> {
        serde_json::to_string(self).map_err(DogfoodError::from_display)
    }
}

/// Widen an in-memory collection size for the stable manifest wire count.
fn finding_count(length: usize) -> FindingCount {
    // CAST-JUSTIFICATION: `usize` is at most 64 bits on every supported
    // target, so this widening to the manifest's u64 count cannot truncate.
    FindingCount::new(length as u64)
}

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
            ignore_dirs.push(
                IgnoreDirectorySegment::try_new(segment).map_err(DogfoodError::from_display)?,
            );
        } else {
            ignore_file_globs.push(
                Glob::try_new(String::from(glob.as_str())).map_err(DogfoodError::from_display)?,
            );
        }
    }
    Ok(walk::IgnoreRules::new(ignore_dirs, ignore_file_globs))
}

/// One toolchain step's outcome. `Skipped` is only ever produced for a
/// NON-required step (see [`run_toolchain_checks`]) -- it is never used
/// to silently absorb a required step's failure, and it always carries
/// the raw evidence.
/// Spawn one `cargo` step and fold its status into a [`StepOutcome`],
/// honoring the step's requiredness.
fn run_step(repo_root: &Path, step_args: &[&str], required: bool) -> ToolchainStepOutcome {
    match Command::new("cargo")
        .args(step_args)
        .current_dir(repo_root)
        .output()
    {
        Ok(output) if output.status.success() => ToolchainStepOutcome::Passed,
        Ok(output) => {
            let detail = format!(
                "`cargo {}` exited {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
                step_args.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let detail = XtaskFailureDetail::try_new(detail)
                .unwrap_or_else(|_| XtaskFailureDetail::invalid_rendering());
            if required {
                ToolchainStepOutcome::Failed { detail }
            } else {
                ToolchainStepOutcome::Skipped { reason: detail }
            }
        }
        Err(err) => {
            let reason = format!(
                "`cargo {}` could not be spawned: {err}",
                step_args.join(" ")
            );
            let reason = XtaskFailureDetail::try_new(reason)
                .unwrap_or_else(|_| XtaskFailureDetail::invalid_rendering());
            if required {
                ToolchainStepOutcome::Failed { detail: reason }
            } else {
                ToolchainStepOutcome::Skipped { reason }
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
        deny: run_step(
            repo_root,
            &["deny", "check"],
            matches!(policy.require_cargo_deny, RuleEnabled::Enabled),
        ),
        audit: run_step(
            repo_root,
            &["audit"],
            matches!(policy.require_cargo_audit, RuleEnabled::Enabled),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{dir_segment_of, ignore_rules_from_config, NativeDogfoodManifestDto};
    use crate::boundary::testkit::{clean_body, seed, seed_config};
    use crate::dogfood::run_dogfood;
    use enforcer_domain::xtask_types::{
        ToolchainOutcome, ToolchainStepOutcome, XtaskFailureDetail,
    };

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
        let ignored = "crates/sample/tests/fixtures/fail.rs"
            .parse()
            .map_err(std::io::Error::other)?;
        let included = "crates/sample/src/lib.rs"
            .parse()
            .map_err(std::io::Error::other)?;
        assert!(rules.is_ignored(&ignored));
        assert!(!rules.is_ignored(&included));
        Ok(())
    }

    #[test]
    fn missing_config_falls_back_to_default_profile() -> Result<(), std::io::Error> {
        // `enforcer-config`'s own documented contract: no project config
        // file means the `default` profile, not an error.
        let temp = tempfile::tempdir()?;
        let rules = ignore_rules_from_config(temp.path()).map_err(std::io::Error::other)?;
        let included = "crates/sample/src/lib.rs"
            .parse()
            .map_err(std::io::Error::other)?;
        assert!(!rules.is_ignored(&included));
        Ok(())
    }

    #[test]
    fn only_failed_steps_block() {
        let skipped = ToolchainStepOutcome::Skipped {
            reason: XtaskFailureDetail::invalid_rendering(),
        };
        let green = ToolchainOutcome {
            fmt: ToolchainStepOutcome::Passed,
            clippy: ToolchainStepOutcome::Passed,
            deny: skipped,
            audit: ToolchainStepOutcome::Passed,
        };
        assert_eq!(
            green.verdict(),
            enforcer_domain::xtask_types::DogfoodGateVerdict::Pass
        );
        let failed = ToolchainStepOutcome::Failed {
            detail: XtaskFailureDetail::invalid_rendering(),
        };
        let red = ToolchainOutcome {
            fmt: failed,
            clippy: ToolchainStepOutcome::Passed,
            deny: ToolchainStepOutcome::Passed,
            audit: ToolchainStepOutcome::Passed,
        };
        assert_eq!(
            red.verdict(),
            enforcer_domain::xtask_types::DogfoodGateVerdict::Fail
        );
    }

    #[test]
    fn native_dogfood_manifest_dto_round_trip_names_only_the_rust_execution_path(
    ) -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        seed(temp.path(), "crates/sample/src/lib.rs", &clean_body())?;
        seed(temp.path(), "xtask/src/main.rs", &clean_body())?;
        let outcome = run_dogfood(
            temp.path(),
            &temp.path().join("baseline.json"),
            enforcer_domain::xtask_types::ToolchainMode::Skip,
        )
        .map_err(std::io::Error::other)?;
        let manifest = NativeDogfoodManifestDto::from_outcome(&outcome);
        let payload = serde_json::to_string(&manifest)?;
        let decoded: NativeDogfoodManifestDto = serde_json::from_str(&payload)?;
        assert_eq!(
            decoded, manifest,
            "native manifest wire round-trip diverged"
        );
        assert!(payload.contains("\"execution\":\"native-rust\""));
        assert!(!payload.to_ascii_lowercase().contains("mjs"));
        assert!(!payload.to_ascii_lowercase().contains("node"));
        Ok(())
    }
}
