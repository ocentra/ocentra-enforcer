//! b01 — the deterministic plan-directory emitter.
//!
//! # Charter (this module only)
//!
//! `enforcer plan new <name>` (the CLI/MCP surface calls into
//! [`scaffold_plan`]) writes a byte-stable `docs/plans/<name>/` skeleton:
//! `PLAN_STATE.md`, `PLAN_EXECUTION_BLUEPRINT.md`,
//! `TEST_PROOF_EXPECTATIONS.md`, `WORKPACK_INDEX.md`, `RESUME_STATE.md`, and
//! one capsule-stamped workpack stub. This module owns ONLY that emission
//! plus the golden-fixture proof under
//! `crates/enforcer-plan/tests/fixtures/scaffolder/`; it does not own the
//! PLAN-* `Validator` family (b02), the frozen template assets (b03), the
//! orchestrator binding (b04), or the `/plan` skill dispatch (b05).
//!
//! ## Sequencing deviation (documented, not silent)
//!
//! The workpack (`docs/plans/enforcer-selfhost-plan/workpacks/b01-plan-scaffolder.md`)
//! specifies emission SHOULD render from b03's `crates/enforcer-plan/templates/`
//! assets and cross-check SHOULD invoke b02's live `Validator`. Neither b02
//! nor b03 has landed on `rust-build` as of this pack's build (only the
//! arc-20 skeleton — this module doc, `error.rs`, `Cargo.toml` deps — exists
//! upstream). Rather than block on siblings this pack does not own, this
//! module:
//! - centralizes every rendered block behind ONE render function per
//!   document (no copy-pasted capsule literal across call sites) so that
//!   when b03's `templates/` assets land, swapping the render bodies to read
//!   those frozen assets is a localized, mechanical change;
//! - ships a MINIMAL structural self-check
//!   ([`self_check::structural_findings`]) standing in for b02's live
//!   `Validator` cross-check, scoped to exactly the contract this workpack's
//!   own Requirement Checklist states (capsule block present, `owns/deps/
//!   tier` frontmatter present, required sections present, resume-state
//!   present) — it does not claim to BE b02's validator, and the proof rows
//!   name it as a stand-in pending b02.
//!
//! ## L24 — checklists are DERIVED, never sibling copy-paste
//!
//! `docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md` row L24
//! records a live arc-12 defect: a workpack's Requirement Checklist
//! contradicted its own Where-We-Are because the checklist had been
//! boilerplate-copied from a sibling pack's template instead of derived from
//! this pack's own scope facts. L24 names THIS pack as the fix: "b01
//! scaffolder: generate checklist items from scope facts." Accordingly the
//! emitter never hardcodes a checklist string — [`ScopeFacts::requirements`]
//! is the caller-supplied list of capability facts for the NEW plan, and
//! [`render_requirement_checklist`] mechanically maps each fact to exactly
//! one checklist line (fact text plus an unchecked box), 1:1, in the order
//! given. There is no template checklist body anywhere in this module for a
//! caller to copy without supplying facts.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::PlanError;

/// Branded plan-directory name: lowercase kebab-case, matching the
/// `docs/plans/<name>/` directory convention every existing plan uses (e.g.
/// `enforcer-selfhost-plan`). Parsed at the boundary before any filesystem
/// I/O runs — `scaffold_plan` cannot be called with an unvalidated raw
/// string.
///
/// Crate-local rather than an `enforcer-domain` newtype: this brand's
/// validation rule (plan-directory naming) is a Track-B-only concern, not a
/// cross-crate wire shape, and `enforcer-domain` (arc-02) is not owned by
/// this pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlanName(String);

impl PlanName {
    /// View the validated inner value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for PlanName {
    type Err = PlanError;

    fn from_str(raw: &str) -> Result<Self, PlanError> {
        let ok = !raw.is_empty()
            && raw.len() <= 128
            && raw
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !raw.starts_with('-')
            && !raw.ends_with('-')
            && !raw.contains("--");
        if ok {
            Ok(Self(raw.to_owned()))
        } else {
            Err(PlanError::InvalidPlanName {
                raw: raw.to_owned(),
            })
        }
    }
}

impl fmt::Display for PlanName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One capability/requirement fact about the plan being scaffolded, as
/// stated by the caller (the human or agent invoking `plan new`) about the
/// new plan's OWN scope. This is the L24 anti-copy-paste seam: the emitter
/// turns each fact into exactly one Requirement Checklist line and nothing
/// else populates that section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementFact {
    /// The capability statement, written as a checklist item body (no
    /// leading `- [ ]`; the renderer adds that).
    pub statement: String,
}

impl RequirementFact {
    /// Build a requirement fact from caller-supplied text.
    pub fn new(statement: impl Into<String>) -> Self {
        Self {
            statement: statement.into(),
        }
    }
}

/// The scope facts that seed a new plan's skeleton documents.
///
/// Every field here is a fact ABOUT THE NEW PLAN supplied by the caller,
/// never a value copied from an existing sibling plan. `requirements` in
/// particular MUST be empty-or-caller-supplied — see the module doc's L24
/// section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeFacts {
    /// One-paragraph statement of the new plan's current state (renders
    /// into `PLAN_STATE.md`'s `Where We Are`).
    pub where_we_are: String,
    /// Requirement facts this plan's own scope states, each mapped 1:1 to a
    /// checklist line. May be empty for a freshly-scaffolded plan with no
    /// requirements decided yet — an empty list renders an explicit
    /// "no requirements recorded yet" placeholder line, never a borrowed
    /// sibling item.
    pub requirements: Vec<RequirementFact>,
}

impl ScopeFacts {
    /// An empty-but-well-formed fact set: a freshly scaffolded plan with no
    /// decided scope yet, still rendering valid (non-hollow) documents.
    pub fn empty_but_well_formed() -> Self {
        Self {
            where_we_are: "Scope not yet recorded for this plan.".to_owned(),
            requirements: Vec::new(),
        }
    }
}

/// Paths written by one [`scaffold_plan`] call, relative to the plan
/// directory root (`docs/plans/<name>/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEmission {
    /// Absolute path of the plan directory that was created.
    pub plan_dir: PathBuf,
    /// Plan-relative paths of every file written, sorted for determinism.
    pub files: Vec<String>,
}

const CAPSULE_KIND_INDEX: &str = "plan index; read at the start of any work in this plan.";
const CAPSULE_KIND_WORKPACK: &str =
    "assigned workpack; read only when selected by hub or WORKPACK_INDEX.";

/// Render the standard agent-capsule block shared by every emitted
/// document. ONE render function, called from every document renderer
/// below, so there is exactly one place the capsule literal lives in this
/// module (the workpack's "no inline duplicate of the capsule literal"
/// requirement, pending b03's frozen template asset taking over this
/// responsibility).
fn render_capsule(plan: &PlanName, doc: &str, kind: &str, read_when: &str) -> String {
    format!(
        "<!-- agent-capsule -->\n\
         > Agent Capsule\n\
         > Plan: `{plan}`\n\
         > Doc: `{doc}`\n\
         > Kind: {kind}\n\
         > Read when: {read_when}\n\
         <!-- /agent-capsule -->\n"
    )
}

/// Render `PLAN_STATE.md`'s `Where We Are` from the caller's scope facts —
/// never a hardcoded/sibling-borrowed paragraph.
fn render_plan_state(plan: &PlanName, facts: &ScopeFacts) -> String {
    let capsule = render_capsule(
        plan,
        "Plan State",
        CAPSULE_KIND_INDEX,
        "At the start of any work in this plan.",
    );
    format!(
        "# {plan} — Plan State\n\n\
         {capsule}\n\
         ## Where We Are\n{where_we_are}\n\n\
         ## Status\nScaffolded, no workpacks executed yet.\n",
        where_we_are = facts.where_we_are,
    )
}

/// Render the Requirement Checklist section: exactly one line per
/// [`RequirementFact`], in the order given, no hardcoded items. This is the
/// L24 seam (see module doc) made concrete.
fn render_requirement_checklist(facts: &ScopeFacts) -> String {
    if facts.requirements.is_empty() {
        return "- [ ] (no requirements recorded yet for this plan; add scope facts before \
                the first workpack.)\n"
            .to_owned();
    }
    let mut out = String::new();
    for fact in &facts.requirements {
        out.push_str("- [ ] ");
        out.push_str(&fact.statement);
        out.push('\n');
    }
    out
}

/// Render `PLAN_EXECUTION_BLUEPRINT.md`.
fn render_blueprint(plan: &PlanName, facts: &ScopeFacts) -> String {
    let capsule = render_capsule(
        plan,
        "Plan Execution Blueprint",
        CAPSULE_KIND_INDEX,
        "Before assigning or claiming any workpack.",
    );
    format!(
        "# {plan} — Plan Execution Blueprint\n\n\
         {capsule}\n\
         ## Requirement Checklist\n{checklist}",
        checklist = render_requirement_checklist(facts),
    )
}

/// Render `TEST_PROOF_EXPECTATIONS.md`.
fn render_test_proof_expectations(plan: &PlanName) -> String {
    let capsule = render_capsule(
        plan,
        "Test Proof Expectations",
        CAPSULE_KIND_INDEX,
        "Before marking any workpack DONE.",
    );
    format!(
        "# {plan} — Test Proof Expectations\n\n\
         {capsule}\n\
         | Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |\n\
         |----------|--------------|---------------------|---------------|-----------------------|--------|\n\
         | wp01 | TBD | TBD | TBD | TBD | PENDING |\n"
    )
}

/// Render `WORKPACK_INDEX.md`.
fn render_workpack_index(plan: &PlanName) -> String {
    let capsule = render_capsule(
        plan,
        "Workpack Index",
        CAPSULE_KIND_INDEX,
        "At the start of any work in this plan.",
    );
    format!(
        "# {plan} — Workpack Index\n\n\
         {capsule}\n\
         | Workpack | Owns | Deps | Tier | Status |\n\
         |----------|------|------|------|--------|\n\
         | wp01 | TBD | none | T1 | PENDING |\n"
    )
}

/// Render `RESUME_STATE.md` — required by the owner req 2026-07-04
/// (AUDIT_FINDINGS WAVE 5): every new plan skeleton carries a dedicated
/// resume-state doc so a token-out/crash resumes cheaply without
/// re-deriving state. Seeded empty-but-well-formed: the lists exist and are
/// structurally valid, with no items yet (nothing has run against a
/// freshly-scaffolded plan).
fn render_resume_state(plan: &PlanName, facts: &ScopeFacts) -> String {
    let capsule = render_capsule(
        plan,
        "Resume State",
        CAPSULE_KIND_INDEX,
        "First, on any resume after a token-out/crash/restart.",
    );
    format!(
        "# {plan} — Resume State\n\n\
         {capsule}\n\
         ## Where We Are\n{where_we_are}\n\n\
         ## CHECKLIST\n(none yet)\n\n\
         ## TASKLIST\n(none yet)\n\n\
         ## PROGRESS\n(none yet)\n\n\
         ## PREV\n(none — this plan has not started)\n\n\
         ## NEXT\n(none — scaffold a workpack before resuming here)\n",
        where_we_are = facts.where_we_are,
    )
}

/// Render the single seeded workpack stub (`wp01-todo.md`), capsule-stamped
/// with `owns/deps/tier` frontmatter so b02's PLAN-* validator sees a
/// well-formed pack even before any real workpack content is authored.
fn render_workpack_stub(plan: &PlanName) -> String {
    let capsule = render_capsule(
        plan,
        "Workpack Stub",
        CAPSULE_KIND_WORKPACK,
        "Only when this exact workpack is assigned or selected from WORKPACK_INDEX.",
    );
    format!(
        "# wp01 — TODO: name this workpack\n\n\
         {capsule}\n\
         - owns: `TBD`\n\
         - deps: `none`\n\
         - tier: `TBD`\n\n\
         ## Where We Are\nTBD — fill in this workpack's own scope before deriving its \
         Requirement Checklist (see `enforcer-plan`'s L24 doctrine: never copy a sibling \
         pack's checklist).\n\n\
         ## Where We Want To Be\nTBD.\n\n\
         ## Requirement Checklist\n- [ ] (derive from this workpack's own Where We Are)\n\n\
         ## Acceptance And Proof\nTBD.\n\n\
         ## Parallel Ownership Notes\nTBD.\n"
    )
}

/// Every file this emitter writes, as (plan-relative path, contents) pairs,
/// in a FIXED order so callers (and the determinism/golden-fixture tests)
/// see a stable file list.
fn documents(plan: &PlanName, facts: &ScopeFacts) -> Vec<(&'static str, String)> {
    vec![
        ("PLAN_STATE.md", render_plan_state(plan, facts)),
        ("PLAN_EXECUTION_BLUEPRINT.md", render_blueprint(plan, facts)),
        (
            "TEST_PROOF_EXPECTATIONS.md",
            render_test_proof_expectations(plan),
        ),
        ("WORKPACK_INDEX.md", render_workpack_index(plan)),
        ("RESUME_STATE.md", render_resume_state(plan, facts)),
        ("workpacks/wp01-todo.md", render_workpack_stub(plan)),
    ]
}

/// Emit a complete plan-directory skeleton under `<root>/docs/plans/<name>/`.
///
/// Fail-closed: refuses to write anything if the target directory already
/// exists, unless `force` is `true` (in which case it is removed and
/// rewritten). The plan name is validated ([`PlanName::from_str`]) before
/// any I/O — an invalid name never reaches the filesystem.
///
/// Deterministic: for a fixed `(name, facts)` pair, every emitted file's
/// bytes are identical across calls (no timestamps, random ids, or
/// environment-dependent content anywhere in the rendered text).
pub fn scaffold_plan(
    root: &Path,
    name: &str,
    facts: &ScopeFacts,
    force: bool,
) -> Result<PlanEmission, PlanError> {
    let plan: PlanName = name.parse()?;
    let plan_dir = root.join("docs").join("plans").join(plan.as_str());

    if plan_dir.exists() {
        if !force {
            return Err(PlanError::PlanAlreadyExists {
                path: plan_dir.display().to_string(),
            });
        }
        std::fs::remove_dir_all(&plan_dir).map_err(|e| PlanError::Io {
            path: plan_dir.display().to_string(),
            reason: e.to_string(),
        })?;
    }

    let mut files = Vec::new();
    for (rel, contents) in documents(&plan, facts) {
        let file_path = plan_dir.join(rel);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PlanError::Io {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }
        std::fs::write(&file_path, contents).map_err(|e| PlanError::Io {
            path: file_path.display().to_string(),
            reason: e.to_string(),
        })?;
        files.push(rel.to_owned());
    }
    files.sort();

    Ok(PlanEmission { plan_dir, files })
}

/// Minimal structural self-check standing in for b02's live PLAN-*
/// `Validator` cross-check (see module doc's "Sequencing deviation"
/// section). Scoped to exactly what this workpack's own Requirement
/// Checklist states must hold; NOT a claim to implement b02's rule family.
pub mod self_check {
    use std::path::Path;

    /// One structural problem found in an emitted plan directory.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct StructuralFinding {
        /// Plan-relative path the finding is about.
        pub file: String,
        /// Human-readable description of what is missing/wrong.
        pub detail: String,
    }

    const REQUIRED_FILES: &[&str] = &[
        "PLAN_STATE.md",
        "PLAN_EXECUTION_BLUEPRINT.md",
        "TEST_PROOF_EXPECTATIONS.md",
        "WORKPACK_INDEX.md",
        "RESUME_STATE.md",
    ];

    /// Check an emitted plan directory for the structural contract this
    /// workpack's Requirement Checklist states: capsule block present in
    /// every required doc, `owns/deps/tier` frontmatter present in the
    /// workpack stub, and `RESUME_STATE.md` carrying its required sections.
    /// Returns an empty vec when the directory is fully compliant.
    pub fn structural_findings(plan_dir: &Path) -> Vec<StructuralFinding> {
        let mut findings = Vec::new();

        for rel in REQUIRED_FILES {
            let path = plan_dir.join(rel);
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    if !text.contains("<!-- agent-capsule -->") {
                        findings.push(StructuralFinding {
                            file: rel.to_string(),
                            detail: "missing agent-capsule block".to_owned(),
                        });
                    }
                }
                Err(_) => findings.push(StructuralFinding {
                    file: rel.to_string(),
                    detail: "file missing".to_owned(),
                }),
            }
        }

        let resume_path = plan_dir.join("RESUME_STATE.md");
        if let Ok(text) = std::fs::read_to_string(&resume_path) {
            for section in [
                "Where We Are",
                "CHECKLIST",
                "TASKLIST",
                "PROGRESS",
                "PREV",
                "NEXT",
            ] {
                if !text.contains(section) {
                    findings.push(StructuralFinding {
                        file: "RESUME_STATE.md".to_owned(),
                        detail: format!("missing `{section}` section"),
                    });
                }
            }
        }

        let stub_path = plan_dir.join("workpacks/wp01-todo.md");
        match std::fs::read_to_string(&stub_path) {
            Ok(text) => {
                if !text.contains("<!-- agent-capsule -->") {
                    findings.push(StructuralFinding {
                        file: "workpacks/wp01-todo.md".to_owned(),
                        detail: "missing agent-capsule block".to_owned(),
                    });
                }
                for field in ["- owns:", "- deps:", "- tier:"] {
                    if !text.contains(field) {
                        findings.push(StructuralFinding {
                            file: "workpacks/wp01-todo.md".to_owned(),
                            detail: format!("missing `{field}` frontmatter"),
                        });
                    }
                }
            }
            Err(_) => findings.push(StructuralFinding {
                file: "workpacks/wp01-todo.md".to_owned(),
                detail: "file missing".to_owned(),
            }),
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::{scaffold_plan, PlanName, RequirementFact, ScopeFacts};
    use crate::error::PlanError;

    /// Boxed-error alias so `?`-returning tests satisfy the workspace's
    /// `unwrap_used`/`expect_used` deny lints without a bespoke error type.
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn plan_name_accepts_kebab_case_and_rejects_malformed() {
        for good in ["demo-plan", "enforcer-selfhost-plan", "a", "a1-b2"] {
            assert!(good.parse::<PlanName>().is_ok(), "should accept {good:?}");
        }
        for bad in [
            "",
            "Demo-Plan",
            "has space",
            "-lead",
            "trail-",
            "a--b",
            "under_score",
        ] {
            assert!(bad.parse::<PlanName>().is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn scaffold_refuses_invalid_name_before_any_io() -> TestResult {
        let dir = tempfile::tempdir()?;
        let facts = ScopeFacts::empty_but_well_formed();
        let outcome = scaffold_plan(dir.path(), "Not Valid", &facts, false);
        assert!(matches!(outcome, Err(PlanError::InvalidPlanName { .. })));
        assert!(
            !dir.path().join("docs").exists(),
            "must not touch disk on invalid name"
        );
        Ok(())
    }

    #[test]
    fn scaffold_refuses_overwrite_without_force() -> TestResult {
        let dir = tempfile::tempdir()?;
        let facts = ScopeFacts::empty_but_well_formed();
        scaffold_plan(dir.path(), "demo-plan", &facts, false)?;
        let outcome = scaffold_plan(dir.path(), "demo-plan", &facts, false);
        assert!(matches!(outcome, Err(PlanError::PlanAlreadyExists { .. })));
        Ok(())
    }

    #[test]
    fn scaffold_force_overwrites() -> TestResult {
        let dir = tempfile::tempdir()?;
        let facts = ScopeFacts::empty_but_well_formed();
        scaffold_plan(dir.path(), "demo-plan", &facts, false)?;
        scaffold_plan(dir.path(), "demo-plan", &facts, true)?;
        Ok(())
    }

    #[test]
    fn checklist_derives_from_supplied_facts_not_hardcoded() -> TestResult {
        let dir = tempfile::tempdir()?;
        let facts = ScopeFacts {
            where_we_are: "Custom scope statement unique to this test.".to_owned(),
            requirements: vec![
                RequirementFact::new("Do the specific thing A."),
                RequirementFact::new("Do the specific thing B."),
            ],
        };
        let emission = scaffold_plan(dir.path(), "demo-plan", &facts, false)?;
        let blueprint =
            std::fs::read_to_string(emission.plan_dir.join("PLAN_EXECUTION_BLUEPRINT.md"))?;
        assert!(blueprint.contains("Do the specific thing A."));
        assert!(blueprint.contains("Do the specific thing B."));

        let state = std::fs::read_to_string(emission.plan_dir.join("PLAN_STATE.md"))?;
        assert!(state.contains("Custom scope statement unique to this test."));
        Ok(())
    }
}
