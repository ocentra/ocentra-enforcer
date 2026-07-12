//! z01: the terminal composing proof gate (domain half).
//!
//! COMPOSES the self-validation entrypoints other packs already ship --
//! it does not reimplement any of them:
//! - a10's [`crate::dogfood::run_dogfood`] (the baseline-gated rust-rule
//!   scan over `crates/**`, plus the optional toolchain steps);
//! - e01's `enforcer-literal-scan` floor (the same entry point `enforcer
//!   advise literals` calls), gated against a committed T2 ceiling;
//! - b02's `enforcer-plan` PLAN-* structure validators, run read-only
//!   over the live plan's workpacks (reported, not gated -- see below).
//!
//! The effectful composition -- running the scans, persisting
//! `proof/dogfood-manifest.json`, appending the tamper-evident
//! `enforcer-proof` journal record -- lives in [`boundary`]. This file
//! owns the POLICY: the closed [`Verdict`]/[`FloorCheck`] domain enums,
//! the [`judge`] decision function, and the ruleset fingerprint.
//!
//! # Scope of the PASS/FAIL verdict (a documented judgment call)
//! b02's own test suite (`crates/enforcer-plan/src/validator.rs`,
//! `self_host_full_plan_reports_findings_readonly`) establishes the
//! precedent the PLAN-* composition follows: a REPORT-only sweep over the
//! full, still-in-flight plan (111+ sibling workpacks this gate does not
//! own the compliance of), never a hard gate on that count -- b02 itself
//! documents that gating the whole plan's PLAN-* compliance from one pack
//! would make that pack responsible for fixing docs it does not own. The
//! manifest reports the PLAN-* count honestly; the terminal verdict is
//! computed from the parts THIS gate owns closing the loop on: the
//! baseline-gated rust-rule scan (new violations only; existing debt
//! stays grandfathered by design), the toolchain steps when run, and the
//! e01 floor against its committed T2 ceiling. This keeps z01 honestly
//! GREEN today rather than permanently red on debt this workpack does not
//! own, while still surfacing every family's real count for a human/CI
//! reader.

use std::path::Path;

use enforcer_domain::hashes::Sha256;
use enforcer_scan::rules::baseline_ratchet::BaselineGateOutcome;

use crate::dogfood::boundary::ToolchainOutcome;

pub mod boundary;

/// Any failure in the gate's own composition/io -- never a self-violation
/// FINDING, which is a normal typed outcome carried through [`Verdict`].
#[derive(Debug, thiserror::Error)]
#[error("dogfood gate failed: {detail}")]
#[doc = "Typed gate failure; see the note above."]
pub struct GateError {
    // BRAND-INVARIANT: always the rendered message of the one underlying
    // composition/io failure this value wraps (see `from_display`);
    // display-only, never re-parsed or matched on downstream.
    detail: String,
}

impl GateError {
    /// Wrap any lower-layer failure's rendered message.
    pub fn from_display(source: impl std::fmt::Display) -> Self {
        // ALLOC-JUSTIFICATION: the wrapped error is consumed here; one
        // owned rendering is required for this error to be 'static.
        let detail = source.to_string();
        Self { detail }
    }
}

impl From<crate::dogfood::DogfoodError> for GateError {
    fn from(source: crate::dogfood::DogfoodError) -> Self {
        Self::from_display(source)
    }
}

/// Terminal PASS/FAIL verdict. Closed two-variant domain enum; rendered
/// (`Display`) as its lowercase manifest token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Closed terminal verdict; see the module docs."]
pub enum Verdict {
    /// Zero new self-violations across every gated family.
    Pass,
    /// At least one blocking condition fired.
    Fail,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => formatter.write_str("pass"),
            Self::Fail => formatter.write_str("fail"),
        }
    }
}

/// The e01 literal-scan floor's standing against its committed T2
/// ceiling. Computed by [`boundary`] (which owns the counts), consumed by
/// [`judge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Closed T2-floor standing; see the module docs."]
pub enum FloorCheck {
    /// The current hard-finding count is at or below the committed
    /// ceiling (existing debt, grandfathered by the same
    /// start-green-not-bypassed doctrine as the a10 baseline).
    WithinCeiling,
    /// The count GREW past the committed ceiling -- new T2 debt.
    ExceedsCeiling,
}

impl std::fmt::Display for FloorCheck {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WithinCeiling => formatter.write_str("within ceiling"),
            Self::ExceedsCeiling => formatter.write_str("exceeds ceiling"),
        }
    }
}

/// The gate's one decision point: PASS iff the baseline-gated rust-rule
/// scan found no NEW violations, every required toolchain step passed
/// (when the toolchain ran at all), and the e01 floor stayed within its
/// committed ceiling. Pure policy -- no io, fully covered by the
/// truth-table test below.
pub fn judge(
    scan_gate: &BaselineGateOutcome,
    toolchain: Option<&ToolchainOutcome>,
    floor: FloorCheck,
) -> Verdict {
    let scan_green = scan_gate.passes();
    let toolchain_green = toolchain.is_none_or(ToolchainOutcome::passes);
    let floor_green = matches!(floor, FloorCheck::WithinCeiling);
    if scan_green && toolchain_green && floor_green {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

/// Compute the ruleset fingerprint: load every catalog under `rules_dir`
/// (`crates/enforcer-rules/rules/*.json`), sort every loaded record's
/// `(ruleId, version)` pair, and digest the canonical payload into a
/// branded [`Sha256`]. Changes iff the shipped ruleset itself changes.
///
/// # Errors
/// Returns [`GateError`] when the directory is unreadable or any catalog
/// is malformed/invalid -- the registry loader rejects a bad record, so a
/// partial set is never silently fingerprinted.
pub fn ruleset_fingerprint(rules_dir: &Path) -> Result<Sha256, GateError> {
    let mut catalog_files: Vec<std::path::PathBuf> = std::fs::read_dir(rules_dir)
        .map_err(GateError::from_display)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| candidate.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    catalog_files.sort();
    let catalog_refs: Vec<&Path> = catalog_files
        .iter()
        .map(std::path::PathBuf::as_path)
        .collect();
    let registry = enforcer_rules::loader::load_registry_from_files(&catalog_refs)
        .map_err(GateError::from_display)?;

    let mut pairs: Vec<(String, u32)> = registry
        .iter()
        // ALLOC-JUSTIFICATION: the digest preimage owns its sorted
        // `(ruleId, version)` rows; the registry stays borrowed.
        .map(|record| (record.rule_id.to_string(), record.version))
        .collect();
    pairs.sort();
    let preimage = serde_json::to_vec(&pairs).map_err(GateError::from_display)?;
    enforcer_core::hash_chain::link_digest(None, &preimage)
        .parse::<Sha256>()
        .map_err(GateError::from_display)
}

#[cfg(test)]
mod tests {
    use super::{judge, ruleset_fingerprint, FloorCheck, Verdict};
    use crate::boundary::testkit::{seed, seed_rules_catalog};
    use crate::dogfood::boundary::{StepOutcome, ToolchainOutcome};
    use enforcer_scan::rules::baseline_ratchet::{Baseline, BaselineRatchetValidator};

    fn green_toolchain() -> ToolchainOutcome {
        let deny_reason = String::from("not required by config");
        let audit_reason = String::from("not required by config");
        ToolchainOutcome {
            fmt: StepOutcome::Passed,
            clippy: StepOutcome::Passed,
            deny: StepOutcome::Skipped {
                reason: deny_reason,
            },
            audit: StepOutcome::Skipped {
                reason: audit_reason,
            },
        }
    }

    fn red_toolchain() -> ToolchainOutcome {
        let fmt_detail = String::from("exit status 1");
        ToolchainOutcome {
            fmt: StepOutcome::Failed { detail: fmt_detail },
            clippy: StepOutcome::Passed,
            deny: StepOutcome::Passed,
            audit: StepOutcome::Passed,
        }
    }

    fn clean_scan_gate() -> enforcer_scan::rules::baseline_ratchet::BaselineGateOutcome {
        BaselineRatchetValidator::gate(&Baseline::default(), &[])
    }

    fn dirty_scan_gate(
    ) -> Result<enforcer_scan::rules::baseline_ratchet::BaselineGateOutcome, std::io::Error> {
        let seeded_title = String::from("seeded");
        let seeded_detail = String::from("seeded detail");
        let finding = enforcer_domain::findings::Finding {
            rule_id: "RR-6.1".parse().map_err(std::io::Error::other)?,
            severity: enforcer_domain::severity::Severity::Error,
            title: seeded_title,
            detail: seeded_detail,
            file: "crates/sample/src/lib.rs"
                .parse()
                .map_err(std::io::Error::other)?,
            line: 1,
            snippet: None,
        };
        let violation = enforcer_domain::findings::Violation::try_from(finding)
            .map_err(std::io::Error::other)?;
        Ok(BaselineRatchetValidator::gate(
            &Baseline::default(),
            &[violation],
        ))
    }

    /// PROPERTY-TEST: [`judge`] is a pure conjunction over a closed input
    /// domain -- exhaustively enumerating every (scan, toolchain, floor)
    /// combination covers the whole truth table: PASS iff all three
    /// inputs are green, FAIL otherwise (including the invalid/rejecting
    /// states an unclean input produces).
    #[test]
    fn judge_truth_table_is_exhaustive() -> Result<(), std::io::Error> {
        let floors = [FloorCheck::WithinCeiling, FloorCheck::ExceedsCeiling];
        let scan_states = [false, true];
        let toolchain_states = [None, Some(false), Some(true)];
        for scan_dirty in scan_states {
            for toolchain_state in toolchain_states {
                for floor in floors {
                    let scan_gate = if scan_dirty {
                        dirty_scan_gate()?
                    } else {
                        clean_scan_gate()
                    };
                    let toolchain = match toolchain_state {
                        None => None,
                        Some(true) => Some(green_toolchain()),
                        Some(false) => Some(red_toolchain()),
                    };
                    let verdict = judge(&scan_gate, toolchain.as_ref(), floor);
                    let all_green = !scan_dirty
                        && toolchain_state != Some(false)
                        && floor == FloorCheck::WithinCeiling;
                    let expected = if all_green {
                        Verdict::Pass
                    } else {
                        Verdict::Fail
                    };
                    assert_eq!(
                        verdict, expected,
                        "judge({scan_dirty}, {toolchain_state:?}, {floor:?}) diverged"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn verdict_and_floor_render_their_manifest_tokens() {
        assert_eq!(format!("{}", Verdict::Pass), "pass");
        assert_eq!(format!("{}", Verdict::Fail), "fail");
        assert_eq!(format!("{}", FloorCheck::WithinCeiling), "within ceiling");
        assert_eq!(format!("{}", FloorCheck::ExceedsCeiling), "exceeds ceiling");
    }

    #[test]
    fn fingerprint_is_stable_across_repeated_loads() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_rules_catalog(temp.path())?;
        let rules_dir = temp.path().join("crates/enforcer-rules/rules");
        let first = ruleset_fingerprint(&rules_dir).map_err(std::io::Error::other)?;
        let second = ruleset_fingerprint(&rules_dir).map_err(std::io::Error::other)?;
        assert_eq!(
            first, second,
            "one unchanged catalog must fingerprint identically"
        );
        Ok(())
    }

    #[test]
    fn fingerprint_changes_when_the_ruleset_changes() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_rules_catalog(temp.path())?;
        let rules_dir = temp.path().join("crates/enforcer-rules/rules");
        let before = ruleset_fingerprint(&rules_dir).map_err(std::io::Error::other)?;
        seed(
            temp.path(),
            "crates/enforcer-rules/rules/second.json",
            r#"[{
                "ruleId": "RR-2.1",
                "version": 1,
                "title": "Second",
                "tier": "T1",
                "validator": { "crateName": "c", "path": "p" },
                "fixtures": { "fail": "f", "pass": "p" },
                "docAnchor": "d"
            }]"#,
        )?;
        let after = ruleset_fingerprint(&rules_dir).map_err(std::io::Error::other)?;
        assert_ne!(
            before, after,
            "a changed ruleset must change the fingerprint"
        );
        Ok(())
    }

    #[test]
    fn malformed_catalog_is_rejected_never_partially_fingerprinted() -> Result<(), std::io::Error> {
        let temp = tempfile::tempdir()?;
        seed_rules_catalog(temp.path())?;
        // An invalid/malformed catalog alongside the good one must fail
        // the whole load (empty required fields are rejected), never be
        // silently dropped from the fingerprint preimage.
        seed(
            temp.path(),
            "crates/enforcer-rules/rules/broken.json",
            "[ { this is malformed json ]",
        )?;
        let rules_dir = temp.path().join("crates/enforcer-rules/rules");
        assert!(
            ruleset_fingerprint(&rules_dir).is_err(),
            "a malformed catalog must be rejected, not partially fingerprinted"
        );
        Ok(())
    }
}
