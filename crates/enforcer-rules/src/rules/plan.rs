//! `PLAN-*` rule records (b02): the typed 5-way linkage
//! (`ruleId <-> validator <-> {fail,pass fixtures} <-> doc-anchor <-> tier`)
//! for every structure check `enforcer-plan::validator` implements.
//!
//! Shipped as a typed Rust constructor rather than a `rules/*.json`
//! catalog file (the pattern the baseline T1 records use) because these
//! records are this crate's only linkage into a SIBLING crate's module
//! paths (`enforcer_plan::validator::PlanCapsuleValidator`, ...); keeping
//! the `RuleId` literals as Rust `&str` constants here, re-used by
//! `enforcer-plan`'s own test module via the same literal strings, means a
//! rename shows up as a compile-checked mismatch in this crate's own
//! parity test rather than only at JSON-load time.
//!
//! Doc anchor: this plan's own workpack,
//! `docs/plans/enforcer-selfhost-plan/workpacks/b02-plan-structure-validator.md`.

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Tier;
use enforcer_domain::{
    paths::RelPath,
    rules_types::{RuleDocAnchor, RuleParameters, RuleTag, RuleTitle, RuleVersion, ValidatorPath},
};

use crate::registry::{FixtureRef, RuleRecord, ValidatorRef};

const DOC_ANCHOR: &str =
    "docs/plans/enforcer-selfhost-plan/workpacks/b02-plan-structure-validator.md";

/// Grouped constructor arguments for one [`RuleRecord`] (clippy
/// `too_many_arguments`: five-plus positional `&str`s is exactly the
/// shape that lint exists to catch).
struct RecordSpec {
    rule_id: RuleId,
    title: RuleTitle,
    validator_path: ValidatorPath,
    fail_fixture: RelPath,
    pass_fixture: RelPath,
    doc_anchor: RuleDocAnchor,
}

macro_rules! plan_spec {
    (
        rule_id: $rule_id:literal,
        title: $title:literal,
        validator_path: $validator_path:literal,
        fail_fixture: $fail_fixture:literal,
        pass_fixture: $pass_fixture:literal,
        doc_anchor_fragment: $fragment:literal,
    ) => {
        RecordSpec {
            rule_id: $rule_id
                .parse()
                .map_err(|error| crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                })?,
            title: $title
                .parse()
                .map_err(|error| crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                })?,
            validator_path: $validator_path.parse().map_err(|error| {
                crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                }
            })?,
            fail_fixture: $fail_fixture.parse().map_err(|error| {
                crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                }
            })?,
            pass_fixture: $pass_fixture.parse().map_err(|error| {
                crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                }
            })?,
            doc_anchor: format!("{DOC_ANCHOR}#{}", $fragment)
                .parse()
                .map_err(|error| crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                })?,
        }
    };
}

fn record(spec: RecordSpec) -> Result<RuleRecord, crate::RuleLoadError> {
    let RecordSpec {
        rule_id,
        title,
        validator_path,
        fail_fixture,
        pass_fixture,
        doc_anchor,
    } = spec;
    Ok(RuleRecord {
        rule_id,
        version: RuleVersion::try_new(std::num::NonZeroU32::MIN),
        title,
        tier: Tier::T1,
        validator: ValidatorRef {
            crate_name: "enforcer-plan".parse().map_err(|error| {
                crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                }
            })?,
            path: validator_path,
        },
        fixtures: FixtureRef {
            fail: fail_fixture,
            pass: pass_fixture,
        },
        doc_anchor,
        tags: vec![
            "plan"
                .parse::<RuleTag>()
                .map_err(|error| crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                })?,
            "structure"
                .parse::<RuleTag>()
                .map_err(|error| crate::RuleLoadError::Boundary {
                    reason: crate::boundary_reason(error),
                })?,
        ],
        params: RuleParameters::default(),
    })
}

/// Every `PLAN-*` rule record this crate ships. Fails closed (returns
/// `Err`) if any literal below regresses to a malformed `RuleId` — the
/// same fail-closed contract [`crate::registry::RuleRegistry::from_records`]
/// enforces at load time, checked here too so a typo in one of these
/// constants is caught by this function's own callers (this module's
/// `cargo test`), not only by a consumer that happens to load the catalog.
pub fn plan_rule_records() -> Result<Vec<RuleRecord>, crate::RuleLoadError> {
    Ok(vec![
        record(plan_spec! {
            rule_id: "PLAN-CAPSULE.1",
            title: "Workpack carries the exact agent-capsule marker block",
            validator_path: "validator::PlanCapsuleValidator",
            fail_fixture: "tests/fixtures/plan-validator/capsule/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/capsule/pass/workpack.md",
            doc_anchor_fragment: "PLAN-CAPSULE",
        })?,
        record(plan_spec! {
            rule_id: "PLAN-SKELETON.1",
            title: "Workpack carries the required section headings, in order",
            validator_path: "validator::PlanSkeletonValidator",
            fail_fixture: "tests/fixtures/plan-validator/skeleton/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/skeleton/pass/workpack.md",
            doc_anchor_fragment: "PLAN-SKELETON",
        })?,
        record(plan_spec! {
            rule_id: "PLAN-FRONTMATTER.1",
            title: "Workpack owns/deps/tier frontmatter is present and well-formed",
            validator_path: "validator::PlanFrontmatterValidator",
            fail_fixture: "tests/fixtures/plan-validator/frontmatter/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/frontmatter/pass/workpack.md",
            doc_anchor_fragment: "PLAN-FRONTMATTER",
        })?,
        record(plan_spec! {
            rule_id: "PLAN-PARALLEL.1",
            title: "No-dep-edge workpacks declare disjoint owns globs",
            validator_path: "validator::check_parallel_safety",
            fail_fixture: "tests/fixtures/plan-validator/parallel-safety/overlap-a.md",
            pass_fixture: "tests/fixtures/plan-validator/parallel-safety/disjoint-a.md",
            doc_anchor_fragment: "PLAN-PARALLEL-SAFETY",
        })?,
        record(plan_spec! {
            rule_id: "PLAN-RESUME.1",
            title: "Plan carries live resume-state (Where-We-Are + checklist/progress + prev/next)",
            validator_path: "validator::PlanResumeStateValidator",
            fail_fixture: "tests/fixtures/plan-validator/resume-state/fail/RESUME_STATE.md",
            pass_fixture: "tests/fixtures/plan-validator/resume-state/pass/RESUME_STATE.md",
            doc_anchor_fragment: "PLAN-RESUME-STATE",
        })?,
        record(plan_spec! {
            rule_id: "PLAN-DRIFT.1",
            title:
                "Requirement Checklist does not contradict this workpack's own Where-We-Are (L24)",
            validator_path: "validator::check_checklist_drift",
            fail_fixture: "tests/fixtures/plan-validator/checklist-drift/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/checklist-drift/pass/workpack.md",
            doc_anchor_fragment: "PLAN-CHECKLIST-DRIFT-L24",
        })?,
    ])
}

#[cfg(test)]
mod tests {
    use super::plan_rule_records;

    #[test]
    fn every_plan_record_has_full_5_way_linkage() -> Result<(), Box<dyn std::error::Error>> {
        let records = plan_rule_records()?;
        assert_eq!(records.len(), 6, "expected six PLAN-* records");
        for record in &records {
            assert_eq!(record.validator.crate_name.as_str(), "enforcer-plan");
        }
        Ok(())
    }

    #[test]
    fn plan_records_load_into_a_registry_without_collision(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let records = plan_rule_records()?;
        let registry = crate::registry::RuleRegistry::from_records(records)?;
        assert_eq!(registry.iter().count(), 6);
        for id in [
            "PLAN-CAPSULE.1",
            "PLAN-SKELETON.1",
            "PLAN-FRONTMATTER.1",
            "PLAN-PARALLEL.1",
            "PLAN-RESUME.1",
            "PLAN-DRIFT.1",
        ] {
            let rule_id = id.parse()?;
            assert_eq!(
                registry.get(&rule_id).map(|record| &record.rule_id),
                Some(&rule_id),
                "expected {id} to load"
            );
        }
        Ok(())
    }

    #[test]
    fn every_plan_record_fixture_path_exists_on_disk() -> Result<(), Box<dyn std::error::Error>> {
        // Parity check: a rule record whose fixture path does not resolve
        // on disk is a silently-broken parity claim. Fixtures live in the
        // sibling `enforcer-plan` crate; walk up to the workspace root.
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .ok_or("expected a workspace root two levels up from CARGO_MANIFEST_DIR")?
            .to_path_buf();
        let records = plan_rule_records()?;
        for record in &records {
            let fail_path = workspace_root
                .join("crates/enforcer-plan")
                .join(record.fixtures.fail.as_str());
            let pass_path = workspace_root
                .join("crates/enforcer-plan")
                .join(record.fixtures.pass.as_str());
            assert!(
                fail_path.is_file(),
                "{}: fail fixture missing at {}",
                record.rule_id,
                fail_path.display()
            );
            assert!(
                pass_path.is_file(),
                "{}: pass fixture missing at {}",
                record.rule_id,
                pass_path.display()
            );
        }
        Ok(())
    }
}
