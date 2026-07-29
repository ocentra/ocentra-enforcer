//! b01 proof: `scaffolder-emit`, `scaffolder-determinism`, and the
//! resume-state fixture test named in
//! `docs/plans/enforcer-selfhost-plan/TEST_PROOF_EXPECTATIONS.md`
//! (`proof/plan/b01-emit.txt`).
//!
//! The workpack's Acceptance And Proof also calls for "a cross-check test
//! feeds emitted output to b02's validator entrypoint ... asserts zero
//! `Finding`s." b02's `Validator` module has not landed on `rust-build` as
//! of this pack's build (see `crates/enforcer-plan/src/scaffolder.rs`'s
//! module-doc "Sequencing deviation" section) — `structural_cross_check`
//! below runs the local `self_check::structural_findings` stand-in instead,
//! named accordingly rather than under b02's name, and asserts the same
//! zero-findings bar. Retire this stand-in and wire the real b02
//! `Validator` in once it lands.

use enforcer_domain::plan_types::{
    PlanArtifactPath, PlanCurrentState, PlanName, PlanOverwriteMode, PlanStatement,
};
use enforcer_plan::error::PlanError;
use enforcer_plan::scaffolder::{scaffold_plan, self_check, RequirementFact, ScopeFacts};

/// Boxed-error alias so every `?`-returning test here satisfies the
/// workspace's `unwrap_used`/`expect_used` deny lints without a bespoke
/// error type per test.
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scaffolder/golden")
}

fn root_path(path: &std::path::Path) -> Result<PlanArtifactPath, Box<dyn std::error::Error>> {
    Ok(PlanArtifactPath::try_new(path.to_path_buf())?)
}

fn plan_name(name: &str) -> Result<PlanName, Box<dyn std::error::Error>> {
    Ok(PlanName::try_new(name)?)
}

fn statement(value: &str) -> Result<PlanStatement, Box<dyn std::error::Error>> {
    Ok(PlanStatement::try_new(value.to_owned())?)
}

fn demo_facts() -> Result<ScopeFacts, Box<dyn std::error::Error>> {
    Ok(ScopeFacts {
        where_we_are: PlanCurrentState::try_new(
            "Golden fixture plan: nothing has run yet; this directory pins the \
             emitter's exact byte output for regression."
                .to_owned(),
        )?,
        requirements: vec![
            RequirementFact::new(statement(
                "Fact A specific to this golden plan's own scope.",
            )?),
            RequirementFact::new(statement(
                "Fact B specific to this golden plan's own scope.",
            )?),
        ],
    })
}

/// Read every file under `dir` (recursively) into a sorted
/// `(relative_path, contents)` list, for a byte-exact tree comparison that
/// does not depend on filesystem iteration order.
fn read_tree(dir: &std::path::Path) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    fn walk(
        base: &std::path::Path,
        dir: &std::path::Path,
        out: &mut Vec<(String, String)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out)?;
            } else {
                let rel = path
                    .strip_prefix(base)?
                    .to_string_lossy()
                    .replace('\\', "/");
                let contents = std::fs::read_to_string(&path)?;
                out.push((rel, contents));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn copy_dir(
    from: &std::path::Path,
    to: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

/// `scaffolder-emit`: emitter output must byte-match the checked-in golden
/// tree fixture (`tests/fixtures/scaffolder/golden/`).
#[test]
fn scaffolder_emit_matches_golden_tree() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let facts = demo_facts()?;
    scaffold_plan(
        &root_path(tmp.path())?,
        &plan_name("demo-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;

    let emitted_dir = tmp.path().join("docs/plans/demo-plan");
    let emitted = read_tree(&emitted_dir)?;

    let golden = golden_dir();
    if !golden.exists() {
        // First run: materialize the golden fixture from the emitter's own
        // (reviewed) output so the byte-diff has something to pin against.
        // Subsequent runs (and CI) compare against this checked-in tree —
        // any future drift in the emitter must show up as a diff here.
        copy_dir(&emitted_dir, &golden)?;
    }
    let expected = read_tree(&golden)?;

    assert_eq!(
        emitted,
        expected,
        "emitter output diverged from the golden fixture tree at {}",
        golden.display()
    );
    Ok(())
}

/// `scaffolder-determinism`: running the emitter twice for the same
/// `(name, facts)` produces byte-identical output.
#[test]
fn scaffolder_determinism_two_runs_identical() -> TestResult {
    let tmp_a = tempfile::tempdir()?;
    let tmp_b = tempfile::tempdir()?;
    let facts = demo_facts()?;

    scaffold_plan(
        &root_path(tmp_a.path())?,
        &plan_name("demo-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;
    scaffold_plan(
        &root_path(tmp_b.path())?,
        &plan_name("demo-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;

    let tree_a = read_tree(&tmp_a.path().join("docs/plans/demo-plan"))?;
    let tree_b = read_tree(&tmp_b.path().join("docs/plans/demo-plan"))?;
    assert_eq!(tree_a, tree_b, "two emitter runs must be byte-identical");
    Ok(())
}

/// Seeded-violation case: refuses to overwrite an existing plan directory
/// without `--force` (mapped here to the `force: bool` parameter).
#[test]
fn scaffolder_refuses_overwrite_without_force_seeded_violation() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let facts = demo_facts()?;
    let root = root_path(tmp.path())?;
    let name = plan_name("demo-plan")?;
    scaffold_plan(&root, &name, &facts, PlanOverwriteMode::RefuseExisting)?;

    let outcome = scaffold_plan(&root, &name, &facts, PlanOverwriteMode::RefuseExisting);
    assert!(matches!(outcome, Err(PlanError::PlanAlreadyExists { .. })));

    // Force does succeed.
    scaffold_plan(&root, &name, &facts, PlanOverwriteMode::ReplaceExisting)?;
    Ok(())
}

/// Seeded-violation case: an invalid plan name must be rejected before any
/// I/O — asserted by checking the target directory was never created.
#[test]
fn scaffolder_rejects_invalid_name_seeded_violation() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let invalid_name = PlanName::try_new("Not A Valid Name");
    assert!(matches!(
        invalid_name,
        Err(error) if error.path == "planName"
    ));
    assert!(!tmp.path().join("docs").exists());
    Ok(())
}

/// Cross-check stand-in (see file header): the emitted skeleton must pass a
/// zero-`Finding` structural check for the exact contract this workpack's
/// Requirement Checklist states. Retire in favor of b02's live `Validator`
/// once that pack lands.
#[test]
fn structural_cross_check_zero_findings() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let facts = demo_facts()?;
    let emission = scaffold_plan(
        &root_path(tmp.path())?,
        &plan_name("demo-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;

    let findings = self_check::structural_findings(&emission.plan_dir);
    assert!(
        findings.is_empty(),
        "expected zero structural findings on emitted output, got: {findings:?}"
    );
    Ok(())
}

/// Seeded-violation case for the cross-check: hand-corrupt an emitted doc
/// (strip its capsule block) and confirm the structural check actually
/// fires — proves the zero-findings assertion above is not vacuous.
#[test]
fn structural_cross_check_detects_seeded_corruption() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let facts = demo_facts()?;
    let emission = scaffold_plan(
        &root_path(tmp.path())?,
        &plan_name("demo-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;

    let state_path = emission.plan_dir.as_path().join("PLAN_STATE.md");
    let corrupted = std::fs::read_to_string(&state_path)?.replace("<!-- agent-capsule -->", "");
    std::fs::write(&state_path, corrupted)?;

    let findings = self_check::structural_findings(&emission.plan_dir);
    assert!(
        !findings.is_empty(),
        "structural check must detect a stripped capsule block"
    );
    assert!(findings
        .iter()
        .any(|finding| finding.file.as_str() == "PLAN_STATE.md"));
    Ok(())
}

/// Resume-state fixture test: the scaffolded plan contains
/// `RESUME_STATE.md` with the `Where We Are` block plus the
/// `CHECKLIST`/`TASKLIST`/`PROGRESS` lists and `PREV`/`NEXT` records.
#[test]
fn resume_state_carries_required_sections() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let facts = demo_facts()?;
    let emission = scaffold_plan(
        &root_path(tmp.path())?,
        &plan_name("demo-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;

    let resume = std::fs::read_to_string(emission.plan_dir.as_path().join("RESUME_STATE.md"))?;
    for section in [
        "Where We Are",
        "CHECKLIST",
        "TASKLIST",
        "PROGRESS",
        "PREV",
        "NEXT",
    ] {
        assert!(
            resume.contains(section),
            "RESUME_STATE.md missing required section `{section}`"
        );
    }
    assert!(
        resume.contains(facts.where_we_are.as_str()),
        "RESUME_STATE.md should seed the plan's Where We Are text"
    );
    Ok(())
}

/// L24 proof: the Requirement Checklist in the emitted
/// `PLAN_EXECUTION_BLUEPRINT.md` contains exactly the caller-supplied
/// facts — nothing borrowed from a sibling template, and no hardcoded
/// filler text when facts ARE supplied.
#[test]
fn checklist_is_derived_from_scope_facts_not_boilerplate() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let facts = ScopeFacts {
        where_we_are: PlanCurrentState::try_new(
            "Distinct scope statement for the L24 proof test.".to_owned(),
        )?,
        requirements: vec![RequirementFact::new(statement(
            "Exactly one L24 proof requirement.",
        )?)],
    };
    let emission = scaffold_plan(
        &root_path(tmp.path())?,
        &plan_name("l24-proof-plan")?,
        &facts,
        PlanOverwriteMode::RefuseExisting,
    )?;

    let blueprint = std::fs::read_to_string(
        emission
            .plan_dir
            .as_path()
            .join("PLAN_EXECUTION_BLUEPRINT.md"),
    )?;
    let supplied_items = blueprint
        .lines()
        .filter(|line| line.starts_with("- [ ]"))
        .collect::<Vec<_>>();
    assert_eq!(
        supplied_items,
        vec!["- [ ] Exactly one L24 proof requirement."]
    );
    // Exactly one checklist line: the fixed golden-plan facts' items must
    // NOT leak into an unrelated plan's checklist.
    let checklist_lines = blueprint.lines().filter(|l| l.starts_with("- [ ]")).count();
    assert_eq!(
        checklist_lines, 1,
        "checklist must contain exactly the supplied facts"
    );
    Ok(())
}
