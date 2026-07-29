//! b01 — the deterministic plan-directory emitter.
//!
//! # Charter (this module only)
//!
//! `enforcer plan new <name>` (the CLI/MCP surface calls into
//! [`scaffold_plan`]) writes a byte-stable `docs/plans/<name>/` skeleton:
//! `PLAN_STATE.md`, `PLAN_EXECUTION_BLUEPRINT.md`,
//! `TEST_PROOF_EXPECTATIONS.md`, `WORKPACK_INDEX.md`, `RESUME_STATE.md`, and
//! one capsule-stamped initial workpack file. This module owns ONLY that emission
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
//!   document (no duplicated capsule literal across call sites) so that
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
//! inherited from a sibling pack's template instead of derived from
//! this pack's own scope facts. L24 names THIS pack as the fix: "b01
//! scaffolder: generate checklist items from scope facts." Accordingly the
//! emitter never hardcodes a checklist string — [`ScopeFacts::requirements`]
//! is the caller-supplied list of capability facts for the NEW plan, and
//! [`render_requirement_checklist`] mechanically maps each fact to exactly
//! one checklist line (fact text plus an unchecked box), 1:1, in the order
//! given. There is no template checklist body anywhere in this module for a
//! caller to duplicate without supplying facts.

use enforcer_domain::paths::RelPath;
use enforcer_domain::plan_types::{
    PlanArtifactPath, PlanCurrentState, PlanName, PlanOverwriteMode, PlanStatement,
};

use crate::boundary::scaffolder::{emit_plan, empty_scope_facts};
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
/// One capability/requirement fact about the plan being scaffolded, as
/// stated by the caller (the human or agent invoking `plan new`) about the
/// new plan's OWN scope. This is the L24 anti-copy-paste seam: the emitter
/// turns each fact into exactly one Requirement Checklist line and nothing
/// else populates that section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementFact {
    /// The capability statement, written as a checklist item body (no
    /// leading `- [ ]`; the renderer adds that).
    pub statement: PlanStatement,
}

impl RequirementFact {
    /// Build a requirement fact from caller-supplied text.
    pub fn new(statement: PlanStatement) -> Self {
        Self { statement }
    }
}

/// The scope facts that seed a new plan's skeleton documents.
///
/// Every field here is a fact ABOUT THE NEW PLAN supplied by the caller,
/// never a value inherited from an existing sibling plan. `requirements` in
/// particular MUST be empty-or-caller-supplied — see the module doc's L24
/// section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeFacts {
    /// One-paragraph statement of the new plan's current state (renders
    /// into `PLAN_STATE.md`'s `Where We Are`).
    pub where_we_are: PlanCurrentState,
    /// Requirement facts this plan's own scope states, each mapped 1:1 to a
    /// checklist line. May be empty for a freshly-scaffolded plan with no
    /// requirements decided yet — an empty list renders an explicit
    /// "no requirements recorded yet" undecided marker, never a borrowed
    /// sibling item.
    pub requirements: Vec<RequirementFact>,
}

impl ScopeFacts {
    /// An empty-but-well-formed fact set: a freshly scaffolded plan with no
    /// decided scope yet, still rendering valid (non-hollow) documents.
    pub fn empty_but_well_formed() -> Self {
        empty_scope_facts()
    }
}

/// Paths written by one [`scaffold_plan`] call, relative to the plan
/// directory root (`docs/plans/<name>/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEmission {
    /// Absolute path of the plan directory that was created.
    pub plan_dir: PlanArtifactPath,
    /// Plan-relative paths of every file written, sorted for determinism.
    pub files: Vec<RelPath>,
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
    root: &PlanArtifactPath,
    plan: &PlanName,
    facts: &ScopeFacts,
    overwrite: PlanOverwriteMode,
) -> Result<PlanEmission, PlanError> {
    emit_plan(root, plan, facts, overwrite)
}

/// Minimal structural self-check standing in for b02's live PLAN-*
/// `Validator` cross-check (see module doc's "Sequencing deviation"
/// section). Scoped to exactly what this workpack's own Requirement
/// Checklist states must hold; NOT a claim to implement b02's rule family.
pub mod self_check {
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::plan_types::{PlanArtifactPath, PlanDiagnosticDetail};

    use crate::boundary::scaffolder::inspect_structure;

    /// One structural problem found in an emitted plan directory.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct StructuralFinding {
        /// Plan-relative path the finding is about.
        pub file: RelPath,
        /// Human-readable description of what is missing/wrong.
        pub detail: PlanDiagnosticDetail,
    }

    /// Check an emitted plan directory for the structural contract this
    /// workpack's Requirement Checklist states: capsule block present in
    /// every required doc, `owns/deps/tier` frontmatter present in the
    /// initial workpack file, and `RESUME_STATE.md` carrying its required sections.
    /// Returns an empty vec when the directory is fully compliant.
    pub fn structural_findings(plan_dir: &PlanArtifactPath) -> Vec<StructuralFinding> {
        inspect_structure(plan_dir)
    }
}
