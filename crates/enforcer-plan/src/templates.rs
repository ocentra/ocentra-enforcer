//! Template loading and rendering for plan documents.
//!
//! This module provides deterministic string substitution over frozen
//! template files (`capsule.tpl`, `workpack-index.tpl`, `plan-readme.tpl`).
//! Templates are embedded at compile time via `include_str!`. Rendering
//! performs named-placeholder substitution with strict error handling —
//! missing placeholders return a typed error, never a panic.
//!
//! # Template Format
//!
//! Templates use `{{name}}` as the placeholder syntax. Each render call
//! supplies a map of `name -> value` bindings. A template with a missing
//! binding returns [`TemplateError::MissingPlaceholder`].

use std::collections::HashMap;

/// The frozen capsule template, embedded at compile time.
const CAPSULE_TEMPLATE: &str = include_str!("../templates/capsule.tpl");

/// The frozen workpack-index template, embedded at compile time.
const WORKPACK_INDEX_TEMPLATE: &str = include_str!("../templates/workpack-index.tpl");

/// The frozen plan-readme template, embedded at compile time.
const PLAN_README_TEMPLATE: &str = include_str!("../templates/plan-readme.tpl");

/// Errors encountered during template rendering.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("missing placeholder: {0}")]
    MissingPlaceholder(String),
}

/// Render the capsule template with the given bindings.
///
/// # Arguments
///
/// * `bindings` - Map of placeholder names to replacement strings (e.g., `doc` -> "Workpack Name")
///
/// # Errors
///
/// Returns [`TemplateError::MissingPlaceholder`] if a placeholder in the template
/// is not present in `bindings`.
pub fn render_capsule(bindings: &HashMap<String, String>) -> Result<String, TemplateError> {
    render_template(CAPSULE_TEMPLATE, bindings)
}

/// Render the workpack-index template with the given bindings.
///
/// # Arguments
///
/// * `bindings` - Map of placeholder names to replacement strings (e.g., `plan` -> "enforcer-selfhost-plan", `rows` -> table rows)
///
/// # Errors
///
/// Returns [`TemplateError::MissingPlaceholder`] if a placeholder in the template
/// is not present in `bindings`.
pub fn render_workpack_index(bindings: &HashMap<String, String>) -> Result<String, TemplateError> {
    render_template(WORKPACK_INDEX_TEMPLATE, bindings)
}

/// Render the plan-readme template with the given bindings.
///
/// # Arguments
///
/// * `bindings` - Map of placeholder names to replacement strings (e.g., `plan`, `description`, `no_read_list`)
///
/// # Errors
///
/// Returns [`TemplateError::MissingPlaceholder`] if a placeholder in the template
/// is not present in `bindings`.
pub fn render_plan_readme(bindings: &HashMap<String, String>) -> Result<String, TemplateError> {
    render_template(PLAN_README_TEMPLATE, bindings)
}

/// Perform deterministic string substitution over a template.
///
/// Replaces all occurrences of `{{name}}` with corresponding values from `bindings`.
/// Returns an error if any placeholder has no binding.
fn render_template(
    template: &str,
    bindings: &HashMap<String, String>,
) -> Result<String, TemplateError> {
    let mut result = template.to_string();

    // Find all placeholders in the form {{name}}
    for (name, value) in bindings {
        let placeholder = format!("{{{{{}}}}}", name);
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }

    // Check if any placeholders remain (indicates missing bindings)
    if let Some(pos) = result.find("{{") {
        if let Some(end) = result[pos..].find("}}") {
            let placeholder = result[pos..pos + end + 2].to_string();
            return Err(TemplateError::MissingPlaceholder(placeholder));
        }
    }

    Ok(result)
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_render_capsule_with_doc_binding() {
        let mut bindings = HashMap::new();
        bindings.insert("doc".to_string(), "Test Workpack".to_string());
        let result = render_capsule(&bindings).expect("render should succeed");
        assert!(result.contains("> Doc: `Test Workpack`"));
        assert!(result.contains("<!-- agent-capsule -->"));
        assert!(result.contains("<!-- /agent-capsule -->"));
    }

    #[test]
    fn test_render_workpack_index_with_bindings() {
        let mut bindings = HashMap::new();
        bindings.insert("plan".to_string(), "test-plan".to_string());
        bindings.insert(
            "rows".to_string(),
            "| a01 | Setup | Done | user | Notes |\n".to_string(),
        );
        let result = render_workpack_index(&bindings).expect("render should succeed");
        assert!(result.contains("# test-plan Workpack Index"));
        assert!(result.contains("| a01 | Setup | Done | user | Notes |"));
    }

    #[test]
    fn test_missing_placeholder() {
        let bindings = HashMap::new();
        let result = render_capsule(&bindings);
        assert!(result.is_err());
        match result {
            Err(TemplateError::MissingPlaceholder(p)) => assert_eq!(p, "{{doc}}"),
            _ => panic!("expected MissingPlaceholder error"),
        }
    }

    #[test]
    fn test_render_plan_readme() {
        let mut bindings = HashMap::new();
        bindings.insert("plan".to_string(), "sample-plan".to_string());
        bindings.insert("description".to_string(), "a sample project".to_string());
        bindings.insert(
            "no_read_list".to_string(),
            "- workpack a01\n- workpack a02\n".to_string(),
        );
        let result = render_plan_readme(&bindings).expect("render should succeed");
        assert!(result.contains("# sample-plan"));
        assert!(result.contains("a sample project"));
    }
}
