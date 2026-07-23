//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.

use enforcer_domain::plan_types::{PlanArtifactPath, PlanName, PlanOverwriteMode};

use crate::domain::scaffolder;
use crate::error::PlanError;
use crate::scaffolder::self_check::StructuralFinding;
use crate::scaffolder::{PlanEmission, ScopeFacts};

pub(crate) fn empty_scope_facts() -> ScopeFacts {
    scaffolder::empty_scope_facts()
}

pub(crate) fn emit_plan(
    root: &PlanArtifactPath,
    plan: &PlanName,
    facts: &ScopeFacts,
    overwrite: PlanOverwriteMode,
) -> Result<PlanEmission, PlanError> {
    scaffolder::emit_plan(root, plan, facts, overwrite)
}

pub(crate) fn inspect_structure(plan_dir: &PlanArtifactPath) -> Vec<StructuralFinding> {
    scaffolder::inspect_structure(plan_dir)
}
