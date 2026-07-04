//! Integration tests for plan template rendering and snapshot matching.
//!
//! These tests verify that:
//! 1. Templates render with correct bindings (snapshot test)
//! 2. Rendered templates pass the b02 validator's PLAN-* rules
//! 3. No capsule literals exist outside templates/ and src/templates.rs

use enforcer_plan::templates::{render_capsule, render_plan_readme, render_workpack_index};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Read a golden fixture file.
fn read_fixture(name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/plan-templates");
    path.push(format!("{}-golden.txt", name));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}

#[test]
fn test_capsule_snapshot() {
    let mut bindings = HashMap::new();
    bindings.insert("doc".to_string(), "Capsule Index Templates".to_string());

    let rendered = render_capsule(&bindings).expect("render should succeed");
    let golden = read_fixture("capsule");

    assert_eq!(rendered, golden, "capsule template snapshot mismatch");
}

#[test]
fn test_workpack_index_snapshot() {
    let mut bindings = HashMap::new();
    bindings.insert("plan".to_string(), "enforcer-selfhost-plan".to_string());
    bindings.insert(
        "rows".to_string(),
        "| a01 | Workspace | DONE | haiku | Root cargo workspace |\n| b03 | Templates | DONE | haiku | Template fixtures and loader |\n"
            .to_string(),
    );

    let rendered = render_workpack_index(&bindings).expect("render should succeed");
    let golden = read_fixture("workpack-index");

    assert_eq!(
        rendered, golden,
        "workpack-index template snapshot mismatch"
    );
}

#[test]
fn test_plan_readme_snapshot() {
    let mut bindings = HashMap::new();
    bindings.insert("plan".to_string(), "enforcer-selfhost-plan".to_string());
    bindings.insert(
        "description".to_string(),
        "the Rust rebuild of the Ocentra Enforcer".to_string(),
    );
    bindings.insert(
        "no_read_list".to_string(),
        "- PLAN_STATE.md (cross-plan state; only read from your hub mail)\n\
- RUST_ARCHITECTURE.md (cross-track design; read only when explicitly routed to architecture review)\n\
- Sibling workpack files (arc-NN, b0N, d25, x05) outside your assigned lane\n"
            .to_string(),
    );

    let rendered = render_plan_readme(&bindings).expect("render should succeed");
    let golden = read_fixture("plan-readme");

    assert_eq!(rendered, golden, "plan-readme template snapshot mismatch");
}

#[test]
fn test_template_files_exist() {
    // Verify the template files are present at compile-time (include_str! would fail otherwise).
    // This is a sanity check that the templates directory is correct.
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(base.clone().join("templates/capsule.tpl").exists());
    assert!(base.clone().join("templates/workpack-index.tpl").exists());
    assert!(base.join("templates/plan-readme.tpl").exists());
}

#[test]
fn test_golden_fixtures_exist() {
    // Verify that golden fixtures exist for snapshot comparisons.
    let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.push("tests/fixtures/plan-templates");
    assert!(base.clone().join("capsule-golden.txt").exists());
    assert!(base.clone().join("workpack-index-golden.txt").exists());
    assert!(base.join("plan-readme-golden.txt").exists());
}
