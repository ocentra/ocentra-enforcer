use enforcer_domain::plan_types::{
    OrchestratorTickCount, PlanBudgetBytes, PlanBudgetLines, PlanCondition, PlanDocumentText,
    PlanImportCount, PlanWorkspaceName, PlanWriteOutcome,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn canonical_plan_text_rejects_empty_and_control_text() -> TestResult {
    let name: PlanWorkspaceName = "workspace".parse()?;
    assert_eq!(name.as_str(), "workspace");
    assert_eq!(
        "".parse::<PlanWorkspaceName>()
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("plan.workspaceName")
    );
    assert_eq!(
        "bad\nname"
            .parse::<PlanWorkspaceName>()
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("plan.workspaceName")
    );
    Ok(())
}

#[test]
fn canonical_plan_counts_are_non_zero() -> TestResult {
    assert_eq!(PlanBudgetLines::try_new(40)?.get(), 40);
    assert_eq!(PlanBudgetBytes::try_new(2048)?.get(), 2048);
    assert_eq!(
        usize::from(OrchestratorTickCount::from_count(PlanImportCount::from(3)).get()),
        3
    );
    assert_eq!(
        PlanBudgetLines::try_new(0)
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("plan.budgetLines")
    );
    Ok(())
}

#[test]
fn plan_document_and_closed_outcomes_preserve_domain_meaning() -> TestResult {
    let document: PlanDocumentText = "# Plan\n\n- first\n\t- nested".parse()?;
    assert_eq!(document.as_str(), "# Plan\n\n- first\n\t- nested");
    assert_eq!(
        PlanDocumentText::try_new("# Plan\0hidden".to_owned())
            .as_ref()
            .err()
            .map(|error| error.path.as_str()),
        Some("plan.documentText")
    );
    assert_ne!(PlanCondition::Satisfied, PlanCondition::Unsatisfied);
    assert_ne!(PlanWriteOutcome::Written, PlanWriteOutcome::DryRun);
    Ok(())
}
