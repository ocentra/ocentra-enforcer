use std::collections::HashMap;

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::ScanScope;
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::{RelPath, RepoRoot};
use enforcer_domain::plan_types::{
    ForestTier, PlanBudgetBytes, PlanCondition, PlanDocumentText, PlanName, PlanProjectName,
    PlanResumeAnchor, PlanWorkspaceName,
};
use enforcer_plan::agents_forest::{
    check_chain_resolves, declares_transitional_intent, run_resume_simulation, scaffold_forest,
    AgentsBudgetValidator, AgentsRoutingDeclaredValidator, AgentsTreeTerminatesValidator,
    ForestFacts, ForestNames, ResumeSimOutcome, TierDocument,
};
use enforcer_validator::harness::run_fixture_parity;
use enforcer_validator::validator::{ValidationInput, Validator};

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rid(s: &str) -> Result<RuleId, Box<dyn std::error::Error>> {
    Ok(s.parse()?)
}

fn repo_root() -> Result<RepoRoot, Box<dyn std::error::Error>> {
    Ok(RepoRoot::try_from(
        manifest_dir().to_string_lossy().into_owned(),
    )?)
}

fn rel(path: &str) -> Result<RelPath, Box<dyn std::error::Error>> {
    Ok(path.parse()?)
}

fn facts() -> Result<ForestFacts, Box<dyn std::error::Error>> {
    Ok(ForestFacts::with_defaults(ForestNames {
        workspace_name: PlanWorkspaceName::try_new("dev-machine".to_owned())?,
        project_name: PlanProjectName::try_new("ocentra-enforcer".to_owned())?,
        plan_name: PlanName::try_new("enforcer-selfhost-plan")?,
        project_tier_path: "AGENTS.md".parse()?,
        plan_tier_path: "docs/plans/enforcer-selfhost-plan/AGENTS.md".parse()?,
        resume_anchor: PlanResumeAnchor::try_new(
            "docs/plans/enforcer-selfhost-plan/RESUME_STATE.md".to_owned(),
        )?,
    }))
}

fn document(source: impl Into<String>) -> Result<PlanDocumentText, Box<dyn std::error::Error>> {
    Ok(PlanDocumentText::try_new(source.into())?)
}

#[test]
fn scaffold_renders_all_three_tiers_with_structured_markers(
) -> Result<(), Box<dyn std::error::Error>> {
    let forest = scaffold_forest(&facts()?)?;
    for (tier, rendered) in [
        (ForestTier::Global, &forest.global),
        (ForestTier::Project, &forest.project),
        (ForestTier::Plan, &forest.plan),
    ] {
        assert!(rendered.as_str().contains("<!-- agents-read-first -->"));
        assert!(rendered.as_str().contains("<!-- agents-next-tier -->"));
        assert!(rendered.as_str().contains("<!-- agents-decision-tree -->"));
        assert!(rendered.as_str().contains(tier.marker()));
    }
    Ok(())
}

#[test]
fn scaffold_is_deterministic_across_two_runs() -> Result<(), Box<dyn std::error::Error>> {
    let f = facts()?;
    let first = scaffold_forest(&f)?;
    let second = scaffold_forest(&f)?;
    assert_eq!(first.global, second.global);
    assert_eq!(first.project, second.project);
    assert_eq!(first.plan, second.plan);
    Ok(())
}

#[test]
fn scaffolded_forest_resolves_and_simulates_resume() -> Result<(), Box<dyn std::error::Error>> {
    let f = facts()?;
    let forest = scaffold_forest(&f)?;
    let global_doc = TierDocument {
        path: "AGENTS.md".parse()?,
        source: forest.global,
    };
    let project_doc = TierDocument {
        path: f.project_tier_path.clone(),
        source: forest.project,
    };
    let plan_doc = TierDocument {
        path: f.plan_tier_path.clone(),
        source: forest.plan,
    };
    let rule_id = rid("AGENTS-CHAIN.1")?;
    let docs = vec![global_doc.clone(), project_doc.clone(), plan_doc.clone()];
    assert!(check_chain_resolves(&rule_id, &docs).is_empty());
    let mut by_path = HashMap::new();
    by_path.insert(project_doc.path.clone(), project_doc);
    by_path.insert(plan_doc.path.clone(), plan_doc);
    assert!(matches!(
        run_resume_simulation(&global_doc, &by_path, PlanBudgetBytes::try_new(10_000)?),
        ResumeSimOutcome::Resolved { resume_anchor, .. } if resume_anchor == f.resume_anchor
    ));
    Ok(())
}

#[test]
fn resume_simulation_fails_closed_over_tight_budget() -> Result<(), Box<dyn std::error::Error>> {
    let f = facts()?;
    let forest = scaffold_forest(&f)?;
    let global_doc = TierDocument {
        path: "AGENTS.md".parse()?,
        source: forest.global,
    };
    let project_doc = TierDocument {
        path: f.project_tier_path.clone(),
        source: forest.project,
    };
    let plan_doc = TierDocument {
        path: f.plan_tier_path,
        source: forest.plan,
    };
    let mut by_path = HashMap::new();
    by_path.insert(project_doc.path.clone(), project_doc);
    by_path.insert(plan_doc.path.clone(), plan_doc);
    assert!(matches!(
        run_resume_simulation(&global_doc, &by_path, PlanBudgetBytes::try_new(1)?),
        ResumeSimOutcome::Broken(_)
    ));
    Ok(())
}

#[test]
fn resume_simulation_fails_closed_on_a_backward_next_chain(
) -> Result<(), Box<dyn std::error::Error>> {
    let global = TierDocument {
        path: "global/AGENTS.md".parse()?,
        source: document("<!-- agents-forest-tier: global -->\n<!-- agents-next-tier -->\nNEXT: project/AGENTS.md\n<!-- /agents-next-tier -->")?,
    };
    let project = TierDocument {
        path: "project/AGENTS.md".parse()?,
        source: document("<!-- agents-forest-tier: project -->\n<!-- agents-next-tier -->\nNEXT: global/AGENTS.md\n<!-- /agents-next-tier -->")?,
    };
    let by_path = HashMap::from([
        (global.path.clone(), global.clone()),
        (project.path.clone(), project),
    ]);

    assert!(matches!(
        run_resume_simulation(&global, &by_path, PlanBudgetBytes::try_new(10_000)?),
        ResumeSimOutcome::Broken(reason)
            if reason.as_str().contains("must point to a `plan` tier")
    ));
    Ok(())
}

#[test]
fn resume_simulation_rejects_skipping_the_project_tier() -> Result<(), Box<dyn std::error::Error>> {
    let f = facts()?;
    let forest = scaffold_forest(&f)?;
    let global = TierDocument {
        path: "AGENTS.md".parse()?,
        source: document(format!(
            "<!-- agents-forest-tier: global -->\n<!-- agents-next-tier -->\nNEXT: {}\n<!-- /agents-next-tier -->",
            f.plan_tier_path
        ))?,
    };
    let plan = TierDocument {
        path: f.plan_tier_path,
        source: forest.plan,
    };
    let by_path = HashMap::from([(plan.path.clone(), plan)]);

    assert!(matches!(
        run_resume_simulation(&global, &by_path, PlanBudgetBytes::try_new(10_000)?),
        ResumeSimOutcome::Broken(reason) if reason.as_str().contains("must point to a `project` tier")
    ));
    Ok(())
}

#[test]
fn validators_hold_fixture_parity() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root()?;
    let routing_fail = rel("tests/fixtures/agents_forest/fail/missing-routing/AGENTS.md")?;
    let global_pass = rel("tests/fixtures/agents_forest/pass/global/AGENTS.md")?;
    run_fixture_parity(
        &AgentsRoutingDeclaredValidator::new(rid("AGENTS-ROUTING.1")?),
        &root,
        &routing_fail,
        &global_pass,
    )?;
    let tree_fail = rel("tests/fixtures/agents_forest/fail/dangling-leaf/AGENTS.md")?;
    let plan_pass = rel("tests/fixtures/agents_forest/pass/plan/AGENTS.md")?;
    run_fixture_parity(
        &AgentsTreeTerminatesValidator::new(rid("AGENTS-TREE.1")?),
        &root,
        &tree_fail,
        &plan_pass,
    )?;
    let budget_fail = rel("tests/fixtures/agents_forest/fail/oversized/AGENTS.md")?;
    run_fixture_parity(
        &AgentsBudgetValidator::new(rid("AGENTS-BUDGET.1")?),
        &root,
        &budget_fail,
        &global_pass,
    )?;
    Ok(())
}

#[test]
fn chain_resolves_fires_on_broken_next_pointer() -> Result<(), Box<dyn std::error::Error>> {
    let root = manifest_dir();
    let docs = ["global-AGENTS.md", "project-AGENTS.md"]
        .into_iter()
        .map(|name| -> Result<TierDocument, Box<dyn std::error::Error>> {
            let source = std::fs::read_to_string(root.join(format!(
                "tests/fixtures/agents_forest/fail/broken-chain/{name}"
            )))?;
            Ok(TierDocument {
                path: name.parse()?,
                source: document(source)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let findings = check_chain_resolves(&rid("AGENTS-CHAIN.1")?, &docs);
    assert_eq!(findings.len(), 2);
    assert!(findings
        .iter()
        .all(|finding| finding.rule_id.as_str() == "AGENTS-CHAIN.1"));
    Ok(())
}

#[test]
fn chain_resolves_rejects_a_next_pointer_that_skips_a_tier(
) -> Result<(), Box<dyn std::error::Error>> {
    let docs = vec![
        TierDocument {
            path: "global/AGENTS.md".parse()?,
            source: document("<!-- agents-forest-tier: global -->\n<!-- agents-next-tier -->\nNEXT: plan/AGENTS.md\n<!-- /agents-next-tier -->")?,
        },
        TierDocument {
            path: "plan/AGENTS.md".parse()?,
            source: document("<!-- agents-forest-tier: plan -->\n<!-- agents-next-tier -->\nNEXT: RESUME_STATE.md\n<!-- /agents-next-tier -->")?,
        },
    ];

    let findings = check_chain_resolves(&rid("AGENTS-CHAIN.1")?, &docs);
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].title.as_str(),
        "NEXT pointer targets wrong tier"
    );
    Ok(())
}

#[test]
fn chain_resolves_silent_on_pass_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let root = manifest_dir();
    let docs = ["global", "project", "plan"]
        .into_iter()
        .map(|tier| -> Result<TierDocument, Box<dyn std::error::Error>> {
            let path = format!("pass/{tier}/AGENTS.md");
            let source =
                std::fs::read_to_string(root.join("tests/fixtures/agents_forest").join(&path))?;
            Ok(TierDocument {
                path: path.parse()?,
                source: document(source)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert!(check_chain_resolves(&rid("AGENTS-CHAIN.1")?, &docs).is_empty());
    Ok(())
}

#[test]
fn transition_intent_and_file_scope_contract_hold() -> Result<(), Box<dyn std::error::Error>> {
    let module_source = std::fs::read_to_string(manifest_dir().join("src/agents_forest.rs"))?;
    assert_eq!(
        declares_transitional_intent(&document(module_source)?),
        PlanCondition::Satisfied
    );
    let forest = scaffold_forest(&facts()?)?;
    for tier in [&forest.global, &forest.project, &forest.plan] {
        assert_eq!(declares_transitional_intent(tier), PlanCondition::Satisfied);
    }
    let file: RelPath = "AGENTS.md".parse()?;
    let input = ValidationInput {
        file: &file,
        source: ValidationSource::from_text("no managed blocks"),
        scope: ScanScope::Files,
    };
    assert_eq!(
        AgentsRoutingDeclaredValidator::new(rid("AGENTS-ROUTING.1")?)
            .validate(input)
            .len(),
        1
    );
    assert_eq!(
        AgentsTreeTerminatesValidator::new(rid("AGENTS-TREE.1")?)
            .validate(input)
            .len(),
        1
    );
    Ok(())
}
