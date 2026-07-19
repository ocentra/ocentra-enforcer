//! a10: the native dogfood loop (domain half).
//!
//! "Eating your own dog food" for the Rust engine is native: the exact same
//! [`enforcer_scan::engine`] every `enforcer check`/`enforcer scan`
//! invocation runs is pointed at the workspace's OWN `crates/**`,
//! in-process, plus the standard Rust toolchain gates (`cargo fmt --check`,
//! `cargo clippy -D warnings`, `cargo deny check`, `cargo audit`, run by
//! [`boundary`]).
//!
//! # Why baseline-aware
//! A from-scratch self-scan of this workspace reports hundreds of
//! pre-existing error-severity findings across `crates/**` -- real
//! grandfathered debt, not noise. A gate that starts red gets bypassed
//! forever (that is exactly how the debt accumulated in the legacy engine).
//! So this module reuses d02's [`enforcer_scan::rules::baseline_ratchet`]
//! machinery UNCHANGED: a committed snapshot (`xtask/dogfood-baseline.json`)
//! records every occurrence known as of the last ratchet;
//! [`run_rust_rule_scan`] fails CLOSED only on a violation NOT in that
//! snapshot (new debt), and the snapshot can only shrink over time -- see
//! [`write_baseline_snapshot`] for the one sanctioned, explicit,
//! out-of-band operation that refreshes it (never a side effect of a
//! normal gate run, and there is no bypass flag).
//!
//! # Why in-process, not "shell out to the built `enforcer`"
//! The built `enforcer`'s `check`/`scan` subcommands have no `--baseline`
//! flag (baseline gating is not in the CLI grammar, and this workpack does
//! not own `enforcer-cli/src/**`). Shelling out would therefore always
//! observe the full grandfathered debt and exit non-zero unconditionally,
//! defeating the baseline-aware design. Calling
//! [`enforcer_scan::engine::run`] directly is not a reimplementation: it
//! is the identical engine code the `check`/`scan` dispatch calls, so the
//! two can never silently diverge. (The built artifact itself IS
//! exercised end-to-end by `crates/enforcer-cli/tests/self_enforce.rs`.)
//!
//! # Domain/boundary split
//! This file owns the typed composition: branded/domain signatures only
//! (no raw string/primitive types cross these seams). All raw-text and
//! effectful surfaces -- config-glob translation, toolchain subprocess
//! spawning -- live in [`boundary`].

use std::path::{Path, PathBuf};

use enforcer_domain::findings::Report;
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::{Outcome, ScanValidatorCount, ScopeRequest};
use enforcer_domain::xtask_types::{ToolchainMode, ToolchainOutcome, XtaskFailureDetail};
use enforcer_scan::coverage::{Coverage, TargetOutcome};
use enforcer_scan::rules::baseline_ratchet::{
    load_baseline, write_baseline, Baseline, BaselineGateOutcome, BaselineLocation,
    BaselineRatchetValidator,
};
use enforcer_scan::scope;
use enforcer_scan::{engine, walk};

pub mod boundary;

/// Any failure in the dogfood loop itself (config/decode/io) -- distinct
/// from a scan FINDING a violation, which is a normal typed outcome
/// carried in [`RustRuleScanResult`], never an `Err`.
#[derive(Debug, thiserror::Error)]
#[error("dogfood loop failed: {detail}")]
#[doc = "Typed dogfood-loop failure; see the note above."]
pub struct DogfoodError {
    // BRAND-INVARIANT: always the rendered message of the one underlying
    // config/decode/io failure this value wraps (see `from_display`);
    // display-only, never re-parsed or matched on downstream.
    detail: XtaskFailureDetail,
}

impl DogfoodError {
    /// Wrap any lower-layer failure's rendered message. Callers only ever
    /// need the rendered text, never the original error's shape.
    pub fn from_display(source: impl std::fmt::Display) -> Self {
        // ALLOC-JUSTIFICATION: the wrapped error is consumed here; one
        // owned rendering is required for this error to be 'static.
        let detail = XtaskFailureDetail::try_new(source.to_string())
            .unwrap_or_else(|_| XtaskFailureDetail::invalid_rendering());
        Self { detail }
    }
}

/// The rust-rule-scan half of the dogfood loop: the engine run over
/// `crates/**`, gated through the d02 baseline ratchet.
#[derive(Debug, Clone)]
#[doc = "Baseline-gated self-scan result; see the module docs."]
pub struct RustRuleScanResult {
    /// a09 coverage accounting for the dispatched files. The
    /// anti-silent-skip gate already ran by the time this value exists --
    /// a hollow scan is an `Err` upstream, never a zero here.
    pub coverage: Coverage,
    /// The full engine report (every finding, not just violations).
    pub report: Report,
    /// The baseline-ratchet classification of the report's violations:
    /// `errors` = NEW (unbaselined, blocking), `warnings` = grandfathered.
    pub gate: BaselineGateOutcome,
}

/// The full `xtask dogfood` outcome. `toolchain` is `None` iff the caller
/// chose [`ToolchainMode::Skip`] -- never conflated with "toolchain
/// passed".
#[derive(Debug)]
#[doc = "One dogfood run's composed outcome; see the module docs."]
pub struct DogfoodOutcome {
    /// The baseline-gated rust-rule scan result.
    pub rust_rule_scan: RustRuleScanResult,
    /// The toolchain steps, when run.
    pub toolchain: Option<ToolchainOutcome>,
}

/// Walk `repo_root` under the workspace's own config-declared ignore
/// scope and keep only `crates/**` files -- the shipped-source set the
/// self-scan gates.
fn walk_crate_files(
    repo_root: &Path,
) -> Result<Vec<enforcer_domain::paths::RelPath>, DogfoodError> {
    let ignore_rules = boundary::ignore_rules_from_config(repo_root)?;
    let all_files = walk::walk(repo_root, &ignore_rules).map_err(DogfoodError::from_display)?;
    Ok(all_files
        .into_iter()
        .filter(|entry| entry.as_str().starts_with("crates/"))
        .collect())
}

/// Run the engine over `crates/**` and fold the full [`Report`].
fn scan_crates(
    repo_root: &Path,
    crate_files: &[enforcer_domain::paths::RelPath],
) -> Result<Report, DogfoodError> {
    let repo_root_brand: RepoRoot = repo_root
        .to_string_lossy()
        .parse()
        .map_err(DogfoodError::from_display)?;
    let resolved = scope::resolve(
        &ScopeRequest::Paths(vec![PathBuf::from("crates")]),
        &repo_root_brand,
    )
    .map_err(DogfoodError::from_display)?;
    let validators = engine::build_family_validators().map_err(DogfoodError::from_display)?;
    Ok(engine::run(&resolved, crate_files, &validators))
}

/// Run the engine over `crates/**` under `repo_root` and gate the result
/// against the baseline snapshot at `baseline_store` (absent = an empty
/// baseline, which fails closed on every current violation -- the honest
/// posture for a repo that has never been baselined).
///
/// # Errors
/// Returns [`DogfoodError`] on a config/decode/io failure, on a
/// malformed or tampered baseline snapshot (its integrity hash is
/// verified on load -- an invalid record is rejected, never silently
/// treated as empty), or when the scan is hollow (zero files dispatched
/// under `crates/**` -- a09's anti-silent-skip doctrine via
/// [`Coverage::require_nonzero_ran`]).
pub fn run_rust_rule_scan(
    repo_root: &Path,
    baseline_store: &Path,
) -> Result<RustRuleScanResult, DogfoodError> {
    let crate_files = walk_crate_files(repo_root)?;

    // a09 anti-silent-skip: every dispatched file is recorded as `Ran`
    // (the engine applies at least the common+security families to every
    // file regardless of language -- see `engine::FamilyValidators::
    // applicable`). An empty `crates/**` selection is the hollow scan
    // this gate must catch, and `require_nonzero_ran` does so.
    let coverage = Coverage::from_outcomes(crate_files.iter().map(|entry| TargetOutcome {
        // CLONE-JUSTIFICATION: the coverage record owns its path key; the
        // walked list stays borrowed for the scan below.
        file: entry.clone(),
        outcome: Outcome::ran(ScanValidatorCount::try_new(std::num::NonZeroUsize::MIN)),
    }));
    coverage
        .require_nonzero_ran()
        .map_err(DogfoodError::from_display)?;

    let report = scan_crates(repo_root, &crate_files)?;

    let prior = if baseline_store.exists() {
        load_baseline(baseline_store).map_err(DogfoodError::from_display)?
    } else {
        Baseline::default()
    };
    let gate = BaselineRatchetValidator::gate(&prior, &report.violations);

    Ok(RustRuleScanResult {
        coverage,
        report,
        gate,
    })
}

/// The one sanctioned, explicit, out-of-band operation that refreshes the
/// committed baseline snapshot: re-scan and persist EXACTLY the current
/// violation set (ratcheting away fixed occurrences, recording new ones
/// as now-known). This is NOT a bypass flag on the gate itself --
/// [`run_rust_rule_scan`] never calls this implicitly, and a normal
/// `xtask dogfood` invocation never mutates the snapshot. Mirrors d02's
/// own documented `--baseline write` mode.
///
/// # Errors
/// Returns [`DogfoodError`] on the same conditions as
/// [`run_rust_rule_scan`], or if the snapshot cannot be written.
pub fn write_baseline_snapshot(
    repo_root: &Path,
    baseline_store: &Path,
) -> Result<Baseline, DogfoodError> {
    let crate_files = walk_crate_files(repo_root)?;
    let report = scan_crates(repo_root, &crate_files)?;
    let baseline = Baseline::from_known(
        report
            .violations
            .iter()
            .map(BaselineLocation::for_violation),
    );
    write_baseline(baseline_store, &baseline).map_err(DogfoodError::from_display)?;
    Ok(baseline)
}

/// Run the full `xtask dogfood` loop: the baseline-gated rust-rule scan,
/// plus -- under [`ToolchainMode::Include`] -- the `cargo fmt`/`clippy`/
/// `deny`/`audit` steps (see [`boundary::run_toolchain_checks`] for the
/// required-vs-honest-skip contract).
///
/// # Errors
/// Returns [`DogfoodError`] under the same conditions as
/// [`run_rust_rule_scan`], or if a toolchain step cannot even be
/// dispatched.
pub fn run_dogfood(
    repo_root: &Path,
    baseline_store: &Path,
    mode: ToolchainMode,
) -> Result<DogfoodOutcome, DogfoodError> {
    let rust_rule_scan = run_rust_rule_scan(repo_root, baseline_store)?;
    let toolchain = match mode {
        ToolchainMode::Include => Some(boundary::run_toolchain_checks(repo_root)?),
        ToolchainMode::Skip => None,
    };
    Ok(DogfoodOutcome {
        rust_rule_scan,
        toolchain,
    })
}

#[cfg(test)]
mod tests {
    use super::{run_rust_rule_scan, write_baseline_snapshot};
    use crate::boundary::testkit::{
        clean_body, second_violating_body, seed, seed_config, violating_body,
    };
    use enforcer_domain::findings::ReportOutcome;

    #[test]
    fn hollow_scan_with_no_crates_dir_fails_closed() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        let baseline_store = temp.path().join("baseline.json");
        let outcome = run_rust_rule_scan(temp.path(), &baseline_store);
        assert!(
            outcome.is_err(),
            "a repo with zero crates/** files must fail closed (hollow self-scan), not pass"
        );
        Ok(())
    }

    #[test]
    fn unbaselined_violation_fails_closed_without_a_snapshot() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        seed(temp.path(), "crates/sample/src/lib.rs", &violating_body())?;
        let baseline_store = temp.path().join("baseline.json");
        let result =
            run_rust_rule_scan(temp.path(), &baseline_store).map_err(std::io::Error::other)?;
        assert!(
            matches!(result.gate.passes(), ReportOutcome::Violations),
            "an unbaselined violation must fail closed against an absent baseline"
        );
        assert!(!result.coverage.ran_count().is_zero());
        Ok(())
    }

    #[test]
    fn baselined_violation_passes_and_new_debt_still_fails() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        seed(temp.path(), "crates/sample/src/lib.rs", &violating_body())?;
        let baseline_store = temp.path().join("baseline.json");
        let recorded =
            write_baseline_snapshot(temp.path(), &baseline_store).map_err(std::io::Error::other)?;
        assert!(
            !recorded.is_empty(),
            "expected the seeded unwrap() to be baselined"
        );

        let covered =
            run_rust_rule_scan(temp.path(), &baseline_store).map_err(std::io::Error::other)?;
        assert!(
            matches!(covered.gate.passes(), ReportOutcome::Clean),
            "a violation already recorded in the baseline must not re-trip the gate"
        );

        // Grow the debt: a second, distinct violation must still fail
        // closed even though the file set already carries baselined debt.
        seed(
            temp.path(),
            "crates/sample/src/other.rs",
            &second_violating_body(),
        )?;
        let grown =
            run_rust_rule_scan(temp.path(), &baseline_store).map_err(std::io::Error::other)?;
        assert!(
            matches!(grown.gate.passes(), ReportOutcome::Violations),
            "a NEW violation beyond the committed baseline must fail closed"
        );
        Ok(())
    }

    #[test]
    fn baseline_ratchets_down_when_debt_is_fixed() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        seed(temp.path(), "crates/sample/src/lib.rs", &violating_body())?;
        let baseline_store = temp.path().join("baseline.json");
        write_baseline_snapshot(temp.path(), &baseline_store).map_err(std::io::Error::other)?;

        // Fix the violation, then refresh the snapshot: it must shrink to
        // empty, never silently retain a now-absent entry.
        seed(temp.path(), "crates/sample/src/lib.rs", &clean_body())?;
        let refreshed =
            write_baseline_snapshot(temp.path(), &baseline_store).map_err(std::io::Error::other)?;
        assert!(
            refreshed.is_empty(),
            "a fixed violation must ratchet the baseline down, not persist"
        );
        Ok(())
    }

    #[test]
    fn malformed_baseline_snapshot_is_rejected_not_treated_as_empty() -> Result<(), std::io::Error>
    {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        seed(temp.path(), "crates/sample/src/lib.rs", &clean_body())?;
        // Invalid/corrupt snapshot content: not a baseline record at all;
        // the loader must reject this malformed input, never coerce it.
        let baseline_store = temp.path().join("baseline.json");
        std::fs::write(&baseline_store, b"{ this is corrupt junk, reject it }")?;
        let outcome = run_rust_rule_scan(temp.path(), &baseline_store);
        assert!(
            outcome.is_err(),
            "a malformed baseline snapshot must be a hard error, never an empty baseline"
        );
        Ok(())
    }

    #[test]
    fn config_ignore_globs_scope_the_scan_to_product_source() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_config(temp.path())?;
        // Under an ignored `fixtures/` path -- must never surface as a
        // violation, baselined or not.
        seed(
            temp.path(),
            "crates/sample/tests/fixtures/bad/fail.rs",
            &violating_body(),
        )?;
        seed(temp.path(), "crates/sample/src/lib.rs", &clean_body())?;
        let baseline_store = temp.path().join("baseline.json");
        let result =
            run_rust_rule_scan(temp.path(), &baseline_store).map_err(std::io::Error::other)?;
        assert!(
            matches!(result.gate.passes(), ReportOutcome::Clean),
            "an ignored fixtures/ path must never contribute a violation"
        );
        Ok(())
    }

    /// PROPERTY-TEST: over a generated grid of fixture states (every
    /// pairing of a clean file and a seeded-violation file), two
    /// consecutive scans of the same state produce identical gate
    /// outcomes -- the determinism property the committed-baseline
    /// contract leans on.
    #[test]
    fn repeated_scans_of_the_same_state_agree() -> Result<(), std::io::Error> {
        let bodies = [clean_body(), violating_body()];
        for first_body in &bodies {
            for second_body in &bodies {
                let temp = tempfile::tempdir()?;
                seed_config(temp.path())?;
                seed(temp.path(), "crates/sample/src/a.rs", first_body)?;
                seed(temp.path(), "crates/sample/src/b.rs", second_body)?;
                let baseline_store = temp.path().join("baseline.json");
                let first = run_rust_rule_scan(temp.path(), &baseline_store)
                    .map_err(std::io::Error::other)?;
                let second = run_rust_rule_scan(temp.path(), &baseline_store)
                    .map_err(std::io::Error::other)?;
                assert_eq!(
                    first.gate, second.gate,
                    "two scans over one unchanged state must agree exactly"
                );
            }
        }
        Ok(())
    }
}
