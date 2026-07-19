//! c07 — the consumer CI-workflow emitter.
//!
//! # Charter
//!
//! `init`/`buildInitWrites` (the retired `.mjs` engine) generated a fixed
//! set of `.github/workflows/*.yml` files for a consumer repo from
//! bundled templates (`GITHUB_ACTIONS_ADAPTERS` + `workflowMap`). This
//! module is that emitter, ported mechanically: five bundled templates
//! under this repo's own `adapters/github-actions/*.yml`, each written
//! byte-for-byte (via `include_str!`, so the shipped binary carries the
//! bytes — no runtime read of this repo's tree) to
//! `<root>/.github/workflows/<name>.yml`.
//!
//! # Distinct from c10
//!
//! This emitter writes the PER-CONSUMER workflow set a repo that installs
//! the enforcer gets (`codeql`, `dependency-policy`, `secret-scan`,
//! `sbom`, and the enforcer's own scan workflow, all scoped to the
//! CONSUMER's CI). It is disjoint from c10, which owns the enforcer
//! PROJECT'S OWN release pipeline and the `enforcer-scan` GitHub Action —
//! c10 does not touch this consumer-workflow set, and this module never
//! emits a release-pipeline or Action-definition file.
//!
//! # `--dry-run` / force semantics
//!
//! [`plan`] always returns the full planned-write set without touching
//! disk. [`apply`] is the only function that writes, and only when called
//! with `dry_run: false` — mirroring the reference `initWrite` contract
//! ("skip an existing file unless `force`"): an existing target is left
//! alone unless `force` is set, in which case it is overwritten with the
//! bundled bytes (never merged/partial).

//! BOUNDARY-INVARIANT: consumer CI input is decoded before typed rendering.
//! Negative invalid inputs are rejected during consumer CI decoding.
//!
use crate::error::{InstallError, InstallResult};
use enforcer_domain::install_types::{
    AppliedFileWrite, CheckStatus, CheckSubject, FileWriteOutcome, InstallReportText,
    InstallRootPath, InstallVerifyCheck, OverwriteMode, PlannedFileWrite,
};

/// One workflow this emitter can write: its bundled-template source
/// bytes and the exact target path (workpack-mandated `workflowMap`
/// target: `.github/workflows/<name>.yml`).
#[derive(Debug, Clone, Copy)]
struct WorkflowTemplate {
    /// `workflowMap` target file stem (e.g. `"codeql"`).
    // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
    name: &'static str,
    /// Bundled template bytes, embedded at compile time from this repo's
    /// own `adapters/github-actions/<name>.yml` reference file.
    // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
    bytes: &'static str,
}

/// The reference `GITHUB_ACTIONS_ADAPTERS` set: every workflow this
/// emitter can write, in a fixed, stable order.
const TEMPLATES: &[WorkflowTemplate] = &[
    WorkflowTemplate {
        name: "ocentra-enforcer",
        bytes: include_str!("../../../../adapters/github-actions/ocentra-enforcer.yml"),
    },
    WorkflowTemplate {
        name: "codeql",
        bytes: include_str!("../../../../adapters/github-actions/codeql.yml"),
    },
    WorkflowTemplate {
        name: "dependency-policy",
        bytes: include_str!("../../../../adapters/github-actions/dependency-policy.yml"),
    },
    WorkflowTemplate {
        name: "secret-scan",
        bytes: include_str!("../../../../adapters/github-actions/secret-scan.yml"),
    },
    WorkflowTemplate {
        name: "sbom",
        bytes: include_str!("../../../../adapters/github-actions/sbom.yml"),
    },
];

/// One planned write this emitter would make (or, for `--dry-run`, would
/// have made).
/// Compute the full planned-write set for `root` (a consumer repo root).
/// Pure: never touches disk. This is the function both a real run's
/// planning phase AND `--dry-run` call — a dry run is this rendered, never
/// a separate code path.
pub fn plan(root: &InstallRootPath) -> InstallResult<Vec<PlannedFileWrite>> {
    TEMPLATES
        .iter()
        .map(|template| -> InstallResult<PlannedFileWrite> {
            Ok(PlannedFileWrite {
                path: root.join_target(
                    std::path::Path::new(".github")
                        .join("workflows")
                        .join(format!("{}.yml", template.name)),
                ),
                // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                content: InstallReportText::try_from(template.bytes.to_owned())?,
            })
        })
        .collect()
}

/// Execute [`plan`]'s output against `root`'s real filesystem. Never
/// called for `--dry-run`.
///
/// - `force: false` (default): an existing target file is left byte-for-byte
///   alone (skip-existing, matching the reference `initWrite`).
/// - `force: true`: every target is (re)written with the bundled bytes,
///   regardless of what (if anything) was there before.
///
/// # Errors
/// Returns a [`std::io::Error`] the moment any individual write fails
/// (creating the `.github/workflows` directory, or writing a file).
pub fn apply(
    root: &InstallRootPath,
    overwrite: OverwriteMode,
) -> InstallResult<Vec<AppliedFileWrite>> {
    let planned = plan(root)?;
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
            std::fs::write(write.path.as_path(), write.content.as_str()).map_err(|error| {
                InstallError::Io {
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    path: write.path.as_path().display().to_string(),
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    reason: error.to_string(),
                }
            })?;
            FileWriteOutcome::Written
        };
        applied.push(AppliedFileWrite {
            planned: write,
            outcome,
        });
    }
    Ok(applied)
}

/// Mechanical, read-only health check: does `root` currently have every
/// bundled workflow file, with bytes matching the bundled template. Feeds
/// the shared [`crate::doctor`] aggregation the same way a
/// [`crate::core::HarnessAdapter::verify`] check does — re-reads disk at
/// call time, never trusts a previous [`plan`]/[`apply`] outcome.
pub fn verify(root: &InstallRootPath) -> InstallResult<Vec<InstallVerifyCheck>> {
    plan(root)?
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
            let passed = on_disk.as_deref() == Some(write.content.as_str());
            Ok(InstallVerifyCheck {
                subject: CheckSubject::SkillAsset(InstallReportText::try_from(
                    // ALLOC-JUSTIFICATION: ownership is required to construct the typed report or diagnostic value that crosses this boundary.
                    write.path.as_path().display().to_string(),
                )?),
                name: InstallReportText::try_from(format!(
                    "workflow-present:{}",
                    write.path.as_path().display()
                ))?,
                status: if passed {
                    CheckStatus::Passed
                } else {
                    CheckStatus::Failed
                },
                detail: InstallReportText::try_from(if passed {
                    String::new()
                } else {
                    format!(
                        "missing or drifted workflow file at `{}`",
                        write.path.as_path().display()
                    )
                })?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{apply, plan, verify};
    use enforcer_domain::install_types::{InstallRootPath, OverwriteMode};
    use std::path::PathBuf;

    const EXPECTED_NAMES: &[&str] = &[
        "ocentra-enforcer",
        "codeql",
        "dependency-policy",
        "secret-scan",
        "sbom",
    ];

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/consumer-ci")
            .join(name)
    }

    fn root(path: &std::path::Path) -> Result<InstallRootPath, Box<dyn std::error::Error>> {
        Ok(InstallRootPath::try_from(path.to_path_buf())?)
    }

    #[test]
    fn plan_names_exactly_the_five_reference_workflows() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::TempDir::new()?;
        let planned = plan(&root(dir.path())?)?;
        assert_eq!(planned.len(), 5);
        let names: Vec<String> = planned
            .iter()
            .map(|w| -> Result<String, &'static str> {
                w.path
                    .as_path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("planned workflow path must have a UTF-8 file stem")
                    .map(str::to_owned)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for expected in EXPECTED_NAMES {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
        Ok(())
    }

    #[test]
    fn plan_never_touches_disk() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let before: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(before.is_empty());
        let _planned = plan(&root(dir.path())?)?;
        let after: Vec<_> = std::fs::read_dir(dir.path())?.collect();
        assert!(after.is_empty(), "plan() must not write anything");
        Ok(())
    }

    #[test]
    fn apply_writes_exactly_the_five_workflow_files_with_golden_bytes(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(&root(dir.path())?, OverwriteMode::PreserveExisting)?;

        let workflows_dir = dir.path().join(".github/workflows");
        let mut written: Vec<String> = std::fs::read_dir(&workflows_dir)?
            .map(|entry| entry.map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_, _>>()?;
        written.sort();
        let mut expected: Vec<String> = EXPECTED_NAMES.iter().map(|n| format!("{n}.yml")).collect();
        expected.sort();
        assert_eq!(written, expected);

        for name in EXPECTED_NAMES {
            let golden = std::fs::read_to_string(fixture_root(name).with_extension("yml"))?;
            let actual = std::fs::read_to_string(workflows_dir.join(format!("{name}.yml")))?;
            assert_eq!(actual, golden, "byte mismatch for {name}.yml");
        }
        Ok(())
    }

    #[test]
    fn apply_skips_an_existing_file_unless_forced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let workflows_dir = dir.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows_dir)?;
        std::fs::write(workflows_dir.join("codeql.yml"), "hand-edited, keep me")?;

        let applied = apply(&root(dir.path())?, OverwriteMode::PreserveExisting)?;
        let codeql = applied
            .iter()
            .find(|apply| apply.planned.path.as_path().ends_with("codeql.yml"))
            .ok_or("expected a codeql.yml planned write")?;
        assert!(
            !matches!(
                codeql.outcome,
                enforcer_domain::install_types::FileWriteOutcome::Written
            ),
            "existing file must be skipped without force"
        );
        let contents = std::fs::read_to_string(workflows_dir.join("codeql.yml"))?;
        assert_eq!(contents, "hand-edited, keep me");
        Ok(())
    }

    #[test]
    fn apply_overwrites_an_existing_file_when_forced() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let workflows_dir = dir.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows_dir)?;
        std::fs::write(workflows_dir.join("codeql.yml"), "stale content")?;

        let applied = apply(&root(dir.path())?, OverwriteMode::Force)?;
        let codeql = applied
            .iter()
            .find(|apply| apply.planned.path.as_path().ends_with("codeql.yml"))
            .ok_or("expected a codeql.yml planned write")?;
        assert!(
            matches!(
                codeql.outcome,
                enforcer_domain::install_types::FileWriteOutcome::Written
            ),
            "force must overwrite an existing file"
        );
        let contents = std::fs::read_to_string(workflows_dir.join("codeql.yml"))?;
        assert_ne!(contents, "stale content");
        Ok(())
    }

    #[test]
    fn verify_is_green_after_apply_and_red_when_a_workflow_goes_missing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        apply(&root(dir.path())?, OverwriteMode::PreserveExisting)?;
        let checks = verify(&root(dir.path())?)?;
        assert_eq!(checks.len(), 5);
        assert!(checks.iter().all(|check| matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        )));

        std::fs::remove_file(dir.path().join(".github/workflows/codeql.yml"))?;
        let checks = verify(&root(dir.path())?)?;
        assert!(checks.iter().any(|check| !matches!(
            check.status,
            enforcer_domain::install_types::CheckStatus::Passed
        ) && check.name.as_str().contains("codeql.yml")));
        Ok(())
    }

    #[test]
    fn dry_run_returns_the_planned_write_set_with_zero_files_created(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        // The dry-run contract IS `plan()` -- there is no separate
        // "--dry-run apply" code path to drift from it.
        let planned = plan(&root(dir.path())?)?;
        assert_eq!(planned.len(), 5);
        assert!(!dir.path().join(".github").exists());
        Ok(())
    }
}
