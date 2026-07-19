//! Raw Markdown rendering boundary for Plan scaffolding.

use enforcer_domain::plan_types::{PlanArtifactPath, PlanName, PlanOverwriteMode};

use crate::boundary::values::{artifact_path, current_state, diagnostic_detail, rel_path};
use crate::error::PlanError;
use crate::scaffolder::self_check::StructuralFinding;
use crate::scaffolder::{PlanEmission, ScopeFacts};

pub(crate) fn empty_scope_facts() -> ScopeFacts {
    ScopeFacts {
        where_we_are: current_state("Scope not yet recorded for this plan.".to_owned()),
        requirements: Vec::new(),
    }
}

pub(crate) fn emit_plan(
    root: &PlanArtifactPath,
    plan: &PlanName,
    facts: &ScopeFacts,
    overwrite: PlanOverwriteMode,
) -> Result<PlanEmission, PlanError> {
    let plan_dir = root
        .as_path()
        .join("docs")
        .join("plans")
        .join(plan.as_str());
    if plan_dir.exists() {
        if matches!(overwrite, PlanOverwriteMode::RefuseExisting) {
            return Err(PlanError::PlanAlreadyExists {
                path: artifact_path(plan_dir),
            });
        }
        std::fs::remove_dir_all(&plan_dir).map_err(|error| PlanError::Io {
            path: artifact_path(plan_dir.clone()),
            reason: diagnostic_detail(error.to_string()),
        })?;
    }

    let mut files = Vec::new();
    for (relative, contents) in documents(plan, facts) {
        let file_path = plan_dir.join(relative);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| PlanError::Io {
                path: artifact_path(parent.to_path_buf()),
                reason: diagnostic_detail(error.to_string()),
            })?;
        }
        std::fs::write(&file_path, contents).map_err(|error| PlanError::Io {
            path: artifact_path(file_path.clone()),
            reason: diagnostic_detail(error.to_string()),
        })?;
        files.push(rel_path(relative.to_owned()));
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
                    file: rel_path((*relative).to_owned()),
                    detail: diagnostic_detail("missing agent-capsule block".to_owned()),
                });
            }
            Err(_) => findings.push(StructuralFinding {
                file: rel_path((*relative).to_owned()),
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
                findings.push(StructuralFinding {
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
                    file: rel_path("workpacks/wp01-todo.md".to_owned()),
                    detail: diagnostic_detail("missing agent-capsule block".to_owned()),
                });
            }
            for field in ["- owns:", "- deps:", "- tier:"] {
                if !text.contains(field) {
                    findings.push(StructuralFinding {
                        file: rel_path("workpacks/wp01-todo.md".to_owned()),
                        detail: diagnostic_detail(format!("missing `{field}` frontmatter")),
                    });
                }
            }
        }
        Err(_) => findings.push(StructuralFinding {
            file: rel_path("workpacks/wp01-todo.md".to_owned()),
            detail: diagnostic_detail("file missing".to_owned()),
        }),
    }
    findings
}

const CAPSULE_KIND_INDEX: &str = "plan index; read at the start of any work in this plan.";
const CAPSULE_KIND_WORKPACK: &str =
    "assigned workpack; read only when selected by hub or WORKPACK_INDEX.";

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
         ## Where We Are\n{}\n\n\
         ## Status\nScaffolded, no workpacks executed yet.\n",
        facts.where_we_are
    )
}

fn render_requirement_checklist(facts: &ScopeFacts) -> String {
    if facts.requirements.is_empty() {
        return "- [ ] (no requirements recorded yet for this plan; add scope facts before \
                the first workpack.)\n"
            .to_owned();
    }
    let mut output = String::new();
    for fact in &facts.requirements {
        output.push_str("- [ ] ");
        output.push_str(fact.statement.as_str());
        output.push('\n');
    }
    output
}

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
         ## Requirement Checklist\n{}",
        render_requirement_checklist(facts)
    )
}

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
         ## Where We Are\n{}\n\n\
         ## CHECKLIST\n(none yet)\n\n\
         ## TASKLIST\n(none yet)\n\n\
         ## PROGRESS\n(none yet)\n\n\
         ## PREV\n(none — this plan has not started)\n\n\
         ## NEXT\n(none — scaffold a workpack before resuming here)\n",
        facts.where_we_are
    )
}

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

pub(crate) fn documents(plan: &PlanName, facts: &ScopeFacts) -> Vec<(&'static str, String)> {
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
