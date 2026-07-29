//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Raw AGENTS forest template and markdown parsing boundary.
//!
//! NEGATIVE-TEST: unresolved template tokens are rejected with a typed error.

use std::collections::HashMap;

use enforcer_domain::plan_types::{PlanBudgetBytes, PlanBudgetLines};

use crate::agents_forest::{ForestFacts, RenderedForest};
use crate::boundary::values::{artifact_path, diagnostic_detail, document_text};
use crate::error::PlanError;

const GLOBAL_TEMPLATE: &str = include_str!("../../templates/agents-global.tpl");
const PROJECT_TEMPLATE: &str = include_str!("../../templates/agents-project.tpl");
const PLAN_TEMPLATE: &str = include_str!("../../templates/agents-plan.tpl");

pub(crate) fn render_forest(facts: &ForestFacts) -> Result<RenderedForest, PlanError> {
    let budget_lines = facts.budget_lines.get().to_string();
    let budget_bytes = facts.budget_bytes.get().to_string();

    let global = render(
        GLOBAL_TEMPLATE,
        &HashMap::from([
            (
                "workspace_name".to_owned(),
                facts.workspace_name.as_str().to_owned(),
            ),
            (
                "next_tier_path".to_owned(),
                facts.project_tier_path.as_str().to_owned(),
            ),
            (
                "resume_anchor".to_owned(),
                facts.resume_anchor.as_str().to_owned(),
            ),
            ("budget_lines".to_owned(), budget_lines.clone()),
            ("budget_bytes".to_owned(), budget_bytes.clone()),
        ]),
    )?;
    let project = render(
        PROJECT_TEMPLATE,
        &HashMap::from([
            (
                "project_name".to_owned(),
                facts.project_name.as_str().to_owned(),
            ),
            (
                "next_tier_path".to_owned(),
                facts.plan_tier_path.as_str().to_owned(),
            ),
            (
                "resume_anchor".to_owned(),
                facts.resume_anchor.as_str().to_owned(),
            ),
            ("budget_lines".to_owned(), budget_lines.clone()),
            ("budget_bytes".to_owned(), budget_bytes.clone()),
        ]),
    )?;
    let plan = render(
        PLAN_TEMPLATE,
        &HashMap::from([
            ("plan_name".to_owned(), facts.plan_name.as_str().to_owned()),
            (
                "next_tier_path".to_owned(),
                facts.resume_anchor.as_str().to_owned(),
            ),
            (
                "resume_anchor".to_owned(),
                facts.resume_anchor.as_str().to_owned(),
            ),
            ("budget_lines".to_owned(), budget_lines),
            ("budget_bytes".to_owned(), budget_bytes),
        ]),
    )?;

    Ok(RenderedForest {
        global: document_text(global),
        project: document_text(project),
        plan: document_text(plan),
    })
}

pub(crate) fn render(
    template: &str,
    bindings: &HashMap<String, String>,
) -> Result<String, PlanError> {
    let mut result = template.to_owned();
    for (name, value) in bindings {
        let placeholder = format!("{{{{{name}}}}}");
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    if let Some(position) = result.find("{{") {
        let unresolved = result.get(position..).unwrap_or_default();
        if let Some(end) = unresolved.find("}}") {
            let placeholder_length = end.checked_add(2).unwrap_or(end);
            let placeholder = unresolved.get(..placeholder_length).unwrap_or(unresolved);
            return Err(PlanError::Io {
                path: artifact_path("agents-forest template".into()),
                reason: diagnostic_detail(format!("missing placeholder: {placeholder}")),
            });
        }
    }
    Ok(result)
}

pub(crate) fn managed_block<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<!-- {name} -->");
    let close = format!("<!-- /{name} -->");
    let (_, rest) = source.split_once(open.as_str())?;
    let (block, _) = rest.split_once(close.as_str())?;
    Some(block.trim())
}

pub(crate) fn tier_marker(source: &str) -> Option<&str> {
    let (_, rest) = source.split_once("<!-- agents-forest-tier:")?;
    let (marker, _) = rest.split_once("-->")?;
    Some(marker.trim())
}

pub(crate) fn extract_keyed_value(block_text: &str, key: &str) -> Option<String> {
    block_text.lines().find_map(|line| {
        line.trim()
            .trim_start_matches('>')
            .trim()
            .strip_prefix(key)
            .map(|rest| rest.trim().to_owned())
    })
}

pub(crate) fn extract_leaf_pointers(block_text: &str) -> Vec<String> {
    block_text
        .lines()
        .filter_map(|line| {
            line.trim()
                .trim_start_matches('>')
                .trim()
                .strip_prefix("LEAF ->")
                .map(|rest| rest.trim().to_owned())
        })
        .collect()
}

pub(crate) fn parse_declared_budget(source: &str) -> (PlanBudgetLines, PlanBudgetBytes) {
    let Some(text) = managed_block(source, "agents-read-first") else {
        return (PlanBudgetLines::DEFAULT, PlanBudgetBytes::DEFAULT);
    };
    let Some(marker) = text.find("Budget: stay under") else {
        return (PlanBudgetLines::DEFAULT, PlanBudgetBytes::DEFAULT);
    };
    let rest = text.get(marker..).unwrap_or_default();
    let lines = rest
        .split_whitespace()
        .find_map(|token| token.parse::<usize>().ok())
        .and_then(|value| PlanBudgetLines::try_new(value).ok())
        .unwrap_or(PlanBudgetLines::DEFAULT);
    let bytes = rest
        .split('/')
        .nth(1)
        .and_then(|segment| {
            segment
                .split_whitespace()
                .find_map(|token| token.parse::<usize>().ok())
        })
        .and_then(|value| PlanBudgetBytes::try_new(value).ok())
        .unwrap_or(PlanBudgetBytes::DEFAULT);
    (lines, bytes)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::render;
    use crate::error::PlanError;

    #[test]
    fn render_rejects_an_unresolved_template_token() {
        let result = render("{{missing}}", &HashMap::new());
        assert!(matches!(
            result,
            Err(PlanError::Io { reason, .. })
                if reason.as_str() == "missing placeholder: {{missing}}"
        ));
    }
}
