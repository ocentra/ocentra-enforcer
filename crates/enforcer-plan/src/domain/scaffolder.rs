//! Raw Markdown rendering boundary for Plan scaffolding.

use std::fmt;

use enforcer_domain::plan_types::{PlanArtifactPath, PlanName, PlanOverwriteMode};

use crate::boundary::values::{artifact_path, current_state, diagnostic_detail, rel_path};
use crate::error::PlanError;
use crate::scaffolder::self_check::StructuralFinding;
use crate::scaffolder::{PlanEmission, ScopeFacts};

#[derive(Debug, Clone)]
pub(crate) struct RenderedDocument(String);

impl fmt::Display for RenderedDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy)]
enum CapsuleDoc {
    PlanState,
    ExecutionBlueprint,
    TestProofExpectations,
    WorkpackIndex,
    ResumeState,
    WorkpackStub,
}

#[derive(Debug, Clone, Copy)]
enum CapsuleDocName {
    PlanState,
    ExecutionBlueprint,
    TestProofExpectations,
    WorkpackIndex,
    ResumeState,
    WorkpackStub,
}

impl fmt::Display for CapsuleDocName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanState => formatter.write_str("Plan State"),
            Self::ExecutionBlueprint => formatter.write_str("Plan Execution Blueprint"),
            Self::TestProofExpectations => formatter.write_str("Test Proof Expectations"),
            Self::WorkpackIndex => formatter.write_str("Workpack Index"),
            Self::ResumeState => formatter.write_str("Resume State"),
            Self::WorkpackStub => formatter.write_str("Workpack Stub"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CapsuleDocKind {
    WorkpackStub,
    Index,
}

impl fmt::Display for CapsuleDocKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkpackStub => formatter
                .write_str("assigned workpack; read only when selected by hub or WORKPACK_INDEX."),
            Self::Index => formatter
                .write_str("index document; read at the start of any work in this plan."),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CapsuleDocReadWhen {
    PlanState,
    ExecutionBlueprint,
    TestProofExpectations,
    WorkpackIndex,
    ResumeState,
    WorkpackStub,
}

impl fmt::Display for CapsuleDocReadWhen {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanState => formatter.write_str("At the start of any work in this plan."),
            Self::ExecutionBlueprint => {
                formatter.write_str("Before assigning or claiming any workpack.")
            }
            Self::TestProofExpectations => {
                formatter.write_str("Before marking any workpack DONE.")
            }
            Self::WorkpackIndex => formatter.write_str("At the start of any work in this plan."),
            Self::ResumeState => {
                formatter.write_str("First, on any resume after a token-out/crash/restart.")
            }
            Self::WorkpackStub => formatter.write_str(
                "Only when this exact workpack is assigned or selected from WORKPACK_INDEX.",
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DocumentSlot {
    PlanState,
    ExecutionBlueprint,
    TestProofExpectations,
    WorkpackIndex,
    ResumeState,
    WorkpackStub,
}

struct DocumentSpec {
    slot: DocumentSlot,
    contents: RenderedDocument,
}

impl CapsuleDoc {
    fn name(self) -> CapsuleDocName {
        match self {
            Self::PlanState => CapsuleDocName::PlanState,
            Self::ExecutionBlueprint => CapsuleDocName::ExecutionBlueprint,
            Self::TestProofExpectations => CapsuleDocName::TestProofExpectations,
            Self::WorkpackIndex => CapsuleDocName::WorkpackIndex,
            Self::ResumeState => CapsuleDocName::ResumeState,
            Self::WorkpackStub => CapsuleDocName::WorkpackStub,
        }
    }

    fn kind(self) -> CapsuleDocKind {
        match self {
            Self::WorkpackStub => CapsuleDocKind::WorkpackStub,
            _ => CapsuleDocKind::Index,
        }
    }

    fn read_when(self) -> CapsuleDocReadWhen {
        match self {
            Self::PlanState => CapsuleDocReadWhen::PlanState,
            Self::ExecutionBlueprint => CapsuleDocReadWhen::ExecutionBlueprint,
            Self::TestProofExpectations => CapsuleDocReadWhen::TestProofExpectations,
            Self::WorkpackIndex => CapsuleDocReadWhen::WorkpackIndex,
            Self::ResumeState => CapsuleDocReadWhen::ResumeState,
            Self::WorkpackStub => CapsuleDocReadWhen::WorkpackStub,
        }
    }
}

pub(crate) fn empty_scope_facts() -> ScopeFacts {
    ScopeFacts {
        // ALLOC-JUSTIFICATION: plan bootstrap emits a non-empty default
        // state so the generated document is deterministic.
        where_we_are: current_state(
            "Scope not yet recorded for this plan.".to_owned(),
        ),
        requirements: Vec::new(),
    }
}

pub(crate) fn emit_plan(
    root: &PlanArtifactPath,
    plan: &PlanName,
    facts: &ScopeFacts,
    overwrite: PlanOverwriteMode,
) -> Result<PlanEmission, PlanError> {
    let plan_dir = root.as_path().join("docs").join("plans").join(plan.as_str());
    if plan_dir.exists() {
        if matches!(overwrite, PlanOverwriteMode::RefuseExisting) {
            return Err(PlanError::PlanAlreadyExists {
                // CLONE-JUSTIFICATION: path ownership is needed for deterministic recovery
                // payloads that travel through PlanError.
                path: artifact_path(plan_dir.as_path().to_path_buf()),
            });
        }
        std::fs::remove_dir_all(&plan_dir).map_err(|error| PlanError::Io {
            // CLONE-JUSTIFICATION: path ownership is needed to preserve recovery context.
            path: artifact_path(plan_dir.as_path().to_path_buf()),
            // ALLOC-JUSTIFICATION: error detail must be preserved for diagnostics
            // and downstream evidence artifacts.
            reason: diagnostic_detail(error.to_string()),
        })?;
    }

    let mut files = Vec::new();
    for spec in documents(plan, facts) {
        let relative_path = match spec.slot {
            DocumentSlot::PlanState => "PLAN_STATE.md",
            DocumentSlot::ExecutionBlueprint => "PLAN_EXECUTION_BLUEPRINT.md",
            DocumentSlot::TestProofExpectations => "TEST_PROOF_EXPECTATIONS.md",
            DocumentSlot::WorkpackIndex => "WORKPACK_INDEX.md",
            DocumentSlot::ResumeState => "RESUME_STATE.md",
            DocumentSlot::WorkpackStub => "workpacks/wp01-todo.md",
        };
        let file_path = plan_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| PlanError::Io {
                // ALLOC-JUSTIFICATION: filesystem path must be preserved for I/O diagnostics.
                path: artifact_path(parent.to_path_buf()),
                // ALLOC-JUSTIFICATION: error text must be available for reproducible
                // operational reports.
                reason: diagnostic_detail(error.to_string()),
            })?;
        }
        std::fs::write(&file_path, spec.contents.0).map_err(|error| PlanError::Io {
            // ALLOC-JUSTIFICATION: emitted plan bytes must remain addressable for recovery paths.
            path: artifact_path(file_path.to_path_buf()),
            // ALLOC-JUSTIFICATION: error text must be preserved for recovery and tracing.
            reason: diagnostic_detail(error.to_string()),
        })?;
        // ALLOC-JUSTIFICATION: this path is an emitted artifact index that must remain owned.
        files.push(rel_path(relative_path.to_owned()));
    }
    files.sort();
    Ok(PlanEmission {
        plan_dir: artifact_path(plan_dir),
        files,
    })
}

pub(crate) fn inspect_structure(plan_dir: &PlanArtifactPath) -> Vec<StructuralFinding> {
    const REQUIRED_FILES: &[&str] = &[
        "PLAN_STATE.md",
        "PLAN_EXECUTION_BLUEPRINT.md",
        "TEST_PROOF_EXPECTATIONS.md",
        "WORKPACK_INDEX.md",
        "RESUME_STATE.md",
    ];
    let mut findings = Vec::new();
    for relative in REQUIRED_FILES {
        match std::fs::read_to_string(plan_dir.as_path().join(relative)) {
            Ok(text) if !text.contains("<!-- agent-capsule -->") => {
                findings.push(StructuralFinding {
                    // ALLOC-JUSTIFICATION: output path index must be owned and replayable.
                    file: rel_path((*relative).to_owned()),
                    // ALLOC-JUSTIFICATION: finding detail must be stable, owned evidence text.
                    detail: diagnostic_detail("missing agent-capsule block".to_owned()),
                });
            }
            Err(_) => findings.push(StructuralFinding {
                // ALLOC-JUSTIFICATION: output path index must be owned and replayable.
                file: rel_path((*relative).to_owned()),
                // ALLOC-JUSTIFICATION: missing-file reason must be deterministic text.
                detail: diagnostic_detail("file missing".to_owned()),
            }),
            Ok(_) => {}
        }
    }
    if let Ok(text) = std::fs::read_to_string(plan_dir.as_path().join("RESUME_STATE.md")) {
        for section in [
            "Where We Are",
            "CHECKLIST",
            "TASKLIST",
            "PROGRESS",
            "PREV",
            "NEXT",
        ] {
            if !text.contains(section) {
                // ALLOC-JUSTIFICATION: section marker text is short and reused as evidence.
                findings.push(StructuralFinding {
                    // ALLOC-JUSTIFICATION: this file path is retained for reproducible evidence.
                    file: rel_path("RESUME_STATE.md".to_owned()),
                    detail: diagnostic_detail(format!("missing `{section}` section")),
                });
            }
        }
    }
    match std::fs::read_to_string(plan_dir.as_path().join("workpacks/wp01-todo.md")) {
        Ok(text) => {
            if !text.contains("<!-- agent-capsule -->") {
                findings.push(StructuralFinding {
                    // ALLOC-JUSTIFICATION: workpack path must be retained as owned evidence.
                    file: rel_path("workpacks/wp01-todo.md".to_owned()),
                    // ALLOC-JUSTIFICATION: this reason is short and must be fully owned.
                    detail: diagnostic_detail("missing agent-capsule block".to_owned()),
                });
            }
            for field in ["- owns:", "- deps:", "- tier:"] {
                if !text.contains(field) {
                    findings.push(StructuralFinding {
                        // ALLOC-JUSTIFICATION: workpack path must be retained as owned evidence.
                        file: rel_path("workpacks/wp01-todo.md".to_owned()),
                        detail: diagnostic_detail(format!("missing `{field}` frontmatter")),
                    });
                }
            }
        }
        Err(_) => findings.push(StructuralFinding {
            // ALLOC-JUSTIFICATION: workpack path must be retained as owned evidence.
            file: rel_path("workpacks/wp01-todo.md".to_owned()),
            // ALLOC-JUSTIFICATION: missing-file reason must be deterministic evidence text.
            detail: diagnostic_detail("file missing".to_owned()),
        }),
    }
    findings
}

fn render_capsule(plan: &PlanName, doc: CapsuleDoc) -> RenderedDocument {
    RenderedDocument(format!(
        "<!-- agent-capsule -->\n\
         > Agent Capsule\n\
         > Plan: `{plan}`\n\
         > Doc: `{}`\n\
         > Kind: {}\n\
         > Read when: {}\n\
         <!-- /agent-capsule -->\n",
        doc.name(),
        doc.kind(),
        doc.read_when(),
    ))
}

fn render_plan_state(plan: &PlanName, facts: &ScopeFacts) -> RenderedDocument {
    let capsule = render_capsule(plan, CapsuleDoc::PlanState);
    RenderedDocument(format!(
        "# {plan} — Plan State\n\n\
         {capsule}\n\
         ## Where We Are\n{}\n\n\
         ## Status\nScaffolded, no workpacks executed yet.\n",
        facts.where_we_are
    ))
}

fn render_requirement_checklist(facts: &ScopeFacts) -> RenderedDocument {
    if facts.requirements.is_empty() {
        // ALLOC-JUSTIFICATION: fallback requirement list is authored inline and must be owned for typed output.
        return RenderedDocument(
            "- [ ] (no requirements recorded yet for this plan; add scope facts before \
            the first workpack.)\n"
                .to_owned(),
        );
    }
    let mut output = String::new();
    for fact in &facts.requirements {
        output.push_str("- [ ] ");
        output.push_str(fact.statement.as_str());
        output.push('\n');
    }
    RenderedDocument(output)
}

fn render_blueprint(plan: &PlanName, facts: &ScopeFacts) -> RenderedDocument {
    let capsule = render_capsule(plan, CapsuleDoc::ExecutionBlueprint);
    RenderedDocument(format!(
        "# {plan} — Plan Execution Blueprint\n\n\
         {capsule}\n\
         ## Requirement Checklist\n{}",
        render_requirement_checklist(facts)
    ))
}

fn render_test_proof_expectations(plan: &PlanName) -> RenderedDocument {
    let capsule = render_capsule(plan, CapsuleDoc::TestProofExpectations);
    RenderedDocument(format!(
        "# {plan} — Test Proof Expectations\n\n\
         {capsule}\n\
         | Workpack | Proof tier(s) | Named test / oracle | Artifact path | Seeded-violation case | Status |\n\
         |----------|--------------|---------------------|---------------|-----------------------|--------|\n\
         | wp01 | TBD | TBD | TBD | TBD | PENDING |\n"
    ))
}

fn render_workpack_index(plan: &PlanName) -> RenderedDocument {
    let capsule = render_capsule(plan, CapsuleDoc::WorkpackIndex);
    RenderedDocument(format!(
        "# {plan} — Workpack Index\n\n\
         {capsule}\n\
         | Workpack | Owns | Deps | Tier | Status |\n\
         |----------|------|------|------|--------|\n\
         | wp01 | TBD | none | T1 | PENDING |\n"
    ))
}

fn render_resume_state(plan: &PlanName, facts: &ScopeFacts) -> RenderedDocument {
    let capsule = render_capsule(plan, CapsuleDoc::ResumeState);
    RenderedDocument(format!(
        "# {plan} — Resume State\n\n\
         {capsule}\n\
         ## Where We Are\n{}\n\n\
         ## CHECKLIST\n(none yet)\n\n\
         ## TASKLIST\n(none yet)\n\n\
         ## PROGRESS\n(none yet)\n\n\
         ## PREV\n(none — this plan has not started)\n\n\
         ## NEXT\n(none — scaffold a workpack before resuming here)\n",
        facts.where_we_are
    ))
}

fn render_workpack_stub(plan: &PlanName) -> RenderedDocument {
    let capsule = render_capsule(plan, CapsuleDoc::WorkpackStub);
    RenderedDocument(format!(
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
    ))
}

fn documents(plan: &PlanName, facts: &ScopeFacts) -> Vec<DocumentSpec> {
    let slots = [
        DocumentSlot::PlanState,
        DocumentSlot::ExecutionBlueprint,
        DocumentSlot::TestProofExpectations,
        DocumentSlot::WorkpackIndex,
        DocumentSlot::ResumeState,
        DocumentSlot::WorkpackStub,
    ];
    slots
        .into_iter()
        .zip([
            render_plan_state(plan, facts),
            render_blueprint(plan, facts),
            render_test_proof_expectations(plan),
            render_workpack_index(plan),
            render_resume_state(plan, facts),
            render_workpack_stub(plan),
        ])
        .map(|(slot, contents)| DocumentSpec {
            slot,
            contents,
        })
        .collect()
}
