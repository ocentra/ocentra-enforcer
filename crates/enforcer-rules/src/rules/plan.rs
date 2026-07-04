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

use enforcer_domain::severity::Tier;

use crate::registry::{FixtureRef, RuleRecord, ValidatorRef};

const DOC_ANCHOR: &str =
    "docs/plans/enforcer-selfhost-plan/workpacks/b02-plan-structure-validator.md";

/// Grouped constructor arguments for one [`RuleRecord`] (clippy
/// `too_many_arguments`: five-plus positional `&str`s is exactly the
/// shape that lint exists to catch).
#[derive(Clone, Copy)]
struct RecordSpec<'a> {
    rule_id: &'a str,
    title: &'a str,
    validator_path: &'a str,
    fail_fixture: &'a str,
    pass_fixture: &'a str,
    doc_anchor_fragment: &'a str,
}

fn record(spec: RecordSpec<'_>) -> Result<RuleRecord, crate::RuleLoadError> {
    let RecordSpec {
        rule_id,
        title,
        validator_path,
        fail_fixture,
        pass_fixture,
        doc_anchor_fragment,
    } = spec;
    Ok(RuleRecord {
        rule_id: rule_id
            .parse()
            .map_err(|e: enforcer_core::error::DecodeError| {
                crate::RuleLoadError::MalformedRecord {
                    rule_id: rule_id.to_owned(),
                    reason: e.to_string(),
                }
            })?,
        version: 1,
        title: title.to_owned(),
        tier: Tier::T1,
        validator: ValidatorRef {
            crate_name: "enforcer-plan".to_owned(),
            path: validator_path.to_owned(),
        },
        fixtures: FixtureRef {
            fail: fail_fixture.to_owned(),
            pass: pass_fixture.to_owned(),
        },
        doc_anchor: format!("{DOC_ANCHOR}#{doc_anchor_fragment}"),
        tags: vec!["plan".to_owned(), "structure".to_owned()],
        params: serde_json::Value::Null,
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
        record(RecordSpec {
            rule_id: "PLAN-CAPSULE.1",
            title: "Workpack carries the exact agent-capsule marker block",
            validator_path: "validator::PlanCapsuleValidator",
            fail_fixture: "tests/fixtures/plan-validator/capsule/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/capsule/pass/workpack.md",
            doc_anchor_fragment: "PLAN-CAPSULE",
        })?,
        record(RecordSpec {
            rule_id: "PLAN-SKELETON.1",
            title: "Workpack carries the required section headings, in order",
            validator_path: "validator::PlanSkeletonValidator",
            fail_fixture: "tests/fixtures/plan-validator/skeleton/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/skeleton/pass/workpack.md",
            doc_anchor_fragment: "PLAN-SKELETON",
        })?,
        record(RecordSpec {
            rule_id: "PLAN-FRONTMATTER.1",
            title: "Workpack owns/deps/tier frontmatter is present and well-formed",
            validator_path: "validator::PlanFrontmatterValidator",
            fail_fixture: "tests/fixtures/plan-validator/frontmatter/fail/workpack.md",
            pass_fixture: "tests/fixtures/plan-validator/frontmatter/pass/workpack.md",
            doc_anchor_fragment: "PLAN-FRONTMATTER",
        })?,
        record(RecordSpec {
            rule_id: "PLAN-PARALLEL.1",
            title: "No-dep-edge workpacks declare disjoint owns globs",
            validator_path: "validator::check_parallel_safety",
            fail_fixture: "tests/fixtures/plan-validator/parallel-safety/overlap-a.md",
            pass_fixture: "tests/fixtures/plan-validator/parallel-safety/disjoint-a.md",
            doc_anchor_fragment: "PLAN-PARALLEL-SAFETY",
        })?,
        record(RecordSpec {
            rule_id: "PLAN-RESUME.1",
            title: "Plan carries live resume-state (Where-We-Are + checklist/progress + prev/next)",
            validator_path: "validator::PlanResumeStateValidator",
            fail_fixture: "tests/fixtures/plan-validator/resume-state/fail/RESUME_STATE.md",
            pass_fixture: "tests/fixtures/plan-validator/resume-state/pass/RESUME_STATE.md",
            doc_anchor_fragment: "PLAN-RESUME-STATE",
        })?,
        record(RecordSpec {
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
            assert!(!record.validator.crate_name.is_empty());
            assert!(!record.validator.path.is_empty());
            assert!(!record.fixtures.fail.is_empty());
            assert!(!record.fixtures.pass.is_empty());
            assert!(!record.doc_anchor.is_empty());
            assert_eq!(record.validator.crate_name, "enforcer-plan");
        }
        Ok(())
    }

    #[test]
    fn plan_records_load_into_a_registry_without_collision(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let records = plan_rule_records()?;
        let registry = crate::registry::RuleRegistry::from_records(records)?;
        assert_eq!(registry.len(), 6);
        for id in [
            "PLAN-CAPSULE.1",
            "PLAN-SKELETON.1",
            "PLAN-FRONTMATTER.1",
            "PLAN-PARALLEL.1",
            "PLAN-RESUME.1",
            "PLAN-DRIFT.1",
        ] {
            let rule_id = id.parse()?;
            assert!(registry.get(&rule_id).is_some(), "expected {id} to load");
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
                .join(&record.fixtures.fail);
            let pass_path = workspace_root
                .join("crates/enforcer-plan")
                .join(&record.fixtures.pass);
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
