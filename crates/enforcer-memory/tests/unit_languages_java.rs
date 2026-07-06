//! Hard tests for the Java extractor
//! ([`enforcer_memory::languages::java`]): symbol labels (class/
//! interface/enum/method/constant/module), every edge kind Java
//! supports (IMPORTS, CALLS, INHERITS via `extends`, IMPLEMENTS via
//! `implements`, DECORATES via annotations, TYPE_REF, DEFINES),
//! `@Test`-annotation test detection, and Spring `@GetMapping`-style
//! route extraction.

use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::languages::java::parse;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_java";

#[test]
fn extracts_package_as_module_symbol() {
    let parsed = parse("package com.example.widget;\n");
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(names_kinds.contains(&("com.example.widget", SymbolKind::Module)));
}

#[test]
fn extracts_class_interface_enum_with_distinct_kinds() {
    let src = r#"
package widget;

public interface Drawable {}

public class Widget {}

public enum Color { RED, GREEN }
"#;
    let parsed = parse(src);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(
        kinds.contains(&("Drawable", SymbolKind::Interface)),
        "{kinds:?}"
    );
    assert!(kinds.contains(&("Widget", SymbolKind::Class)), "{kinds:?}");
    assert!(kinds.contains(&("Color", SymbolKind::Enum)), "{kinds:?}");
}

#[test]
fn extracts_extends_as_inherits() {
    let src = r#"
package widget;

public class Shape {}

public class Widget extends Shape {}
"#;
    let parsed = parse(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Shape")), "{inherits:?}");
}

#[test]
fn extracts_implements_as_implements_edge() {
    let src = r#"
package widget;

public interface Drawable {}
public interface Resizable {}

public class Widget implements Drawable, Resizable {}
"#;
    let parsed = parse(src);
    let implements: Vec<(&str, &str)> = parsed
        .implements
        .iter()
        .map(|i| (i.type_name.as_str(), i.trait_name.as_str()))
        .collect();
    assert!(
        implements.contains(&("Widget", "Drawable")),
        "{implements:?}"
    );
    assert!(
        implements.contains(&("Widget", "Resizable")),
        "{implements:?}"
    );
}

#[test]
fn extracts_interface_extends_as_inherits() {
    let src = r#"
package widget;

public interface Base {}
public interface Drawable extends Base {}
"#;
    let parsed = parse(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Drawable", "Base")), "{inherits:?}");
}

#[test]
fn extracts_static_final_field_as_constant() {
    let src = r#"
package widget;

public class Widget {
    public static final int MAX_WIDGETS = 10;
    private String name;
}
"#;
    let parsed = parse(src);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(
        kinds.contains(&("MAX_WIDGETS", SymbolKind::Constant)),
        "{kinds:?}"
    );
    // A non-static-final field must NOT be extracted as a Constant
    // symbol (this extractor only surfaces static-final fields).
    assert!(!kinds.iter().any(|(n, _)| *n == "name"), "{kinds:?}");

    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "MAX_WIDGETS")), "{defines:?}");
}

#[test]
fn extracts_method_as_defines_and_decorates() {
    let src = r#"
package widget;

public class Widget {
    @Override
    public String draw() { return "x"; }
}
"#;
    let parsed = parse(src);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("draw", SymbolKind::Method)), "{kinds:?}");

    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")), "{defines:?}");

    let decorates: Vec<(&str, &str)> = parsed
        .decorates
        .iter()
        .map(|d| (d.target_name.as_str(), d.decorator_name.as_str()))
        .collect();
    assert!(decorates.contains(&("draw", "Override")), "{decorates:?}");
}

#[test]
fn extracts_imports() {
    let src = r#"
package widget;

import java.util.List;
import java.util.ArrayList;
"#;
    let parsed = parse(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"java.util.List"), "{paths:?}");
    assert!(paths.contains(&"java.util.ArrayList"), "{paths:?}");
}

#[test]
fn extracts_call_edges() {
    let src = r#"
package widget;

public class Widget {
    public void f() {
        helper();
        this.name.trim();
    }
}
"#;
    let parsed = parse(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
    assert!(callees.iter().any(|c| c.ends_with(".trim")), "{callees:?}");
}

#[test]
fn extracts_signature_type_refs() {
    let src = r#"
package widget;

public class Widget {
    public boolean combine(int a, String b) { return true; }
}
"#;
    let parsed = parse(src);
    let types: Vec<&str> = parsed
        .type_refs
        .iter()
        .map(|t| t.type_name.as_str())
        .collect();
    assert!(types.contains(&"int"), "{types:?}");
    assert!(types.contains(&"String"), "{types:?}");
    assert!(types.contains(&"boolean"), "{types:?}");
}

#[test]
fn test_annotation_detects_test_method() {
    let src = r#"
package widget;

import org.junit.Test;

public class WidgetTest {
    @Test
    public void testDraw() {}

    public void helperNotATest() {}
}
"#;
    let parsed = parse(src);
    let names_kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(
        names_kinds.contains(&("testDraw", SymbolKind::Test)),
        "{names_kinds:?}"
    );
    assert!(
        names_kinds.contains(&("helperNotATest", SymbolKind::Method)),
        "{names_kinds:?}"
    );
}

#[test]
fn extracts_spring_get_mapping_route() {
    let src = r#"
package widget;

import org.springframework.web.bind.annotation.GetMapping;

public class WidgetController {
    @GetMapping("/widgets")
    public String listWidgets() { return "[]"; }
}
"#;
    let parsed = parse(src);
    let routes: Vec<(&str, &str)> = parsed
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(routes.contains(&("GET", "/widgets")), "{routes:?}");
}

#[test]
fn extracts_spring_post_mapping_named_argument_route() {
    let src = r#"
package widget;

import org.springframework.web.bind.annotation.PostMapping;

public class WidgetController {
    @PostMapping(path = "/widgets")
    public String createWidget() { return "{}"; }
}
"#;
    let parsed = parse(src);
    let routes: Vec<(&str, &str)> = parsed
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(routes.contains(&("POST", "/widgets")), "{routes:?}");
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse("class ( { this is not valid java @@@");
    let _ = parsed;
}

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn init_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

fn copy_fixtures(dest: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join(FIXTURE_DIR);
    let mut copied = Vec::new();
    for entry in fs::read_dir(&fixture_root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest_path = dest.join(entry.file_name());
            fs::copy(entry.path(), &dest_path)?;
            copied.push(dest_path);
        }
    }
    Ok(copied)
}

/// Incremental reindex over a real Java fixture repo (same tempdir-git
/// pattern as `tests/code_graph_indexer.rs`): unchanged files skip
/// re-parse, a changed Java file gets a fresh symbol set, and
/// CodeGraph never panics end-to-end on the Java extractor's output.
#[test]
fn java_fixture_repo_reindexes_incrementally() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let files = copy_fixtures(dir.path())?;
    commit_all(dir.path(), "initial java fixture import")?;

    let mut graph1 = CodeGraph::new();
    let (manifest_v1, report_v1) =
        graph1.index_repository(dir.path(), &files, &Manifest::default())?;
    assert_eq!(report_v1.added.len(), files.len());

    let symbol_names: Vec<&str> = graph1.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"Widget"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"Drawable"), "{symbol_names:?}");
    assert!(symbol_names.contains(&"testDraw"), "{symbol_names:?}");

    let routes: Vec<(&str, &str)> = graph1
        .routes()
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(routes.contains(&("GET", "/widgets")), "{routes:?}");

    // Second run, nothing changed: every file is skipped.
    let mut graph2 = CodeGraph::new();
    let (manifest_v2, report_v2) = graph2.index_repository(dir.path(), &files, &manifest_v1)?;
    assert_eq!(report_v2.unchanged.len(), files.len());
    assert!(report_v2.changed.is_empty());

    // Mutate Widget.java (adds a new method) and reindex again.
    let widget_java = dir.path().join("Widget.java");
    let mut contents = fs::read_to_string(&widget_java)?;
    let insertion_point = contents.rfind('}').ok_or("missing closing brace")?;
    contents.insert_str(insertion_point, "public void brandNewMethod() {}\n");
    fs::write(&widget_java, contents)?;
    commit_all(dir.path(), "change Widget.java")?;

    let mut graph3 = CodeGraph::new();
    let (_manifest_v3, report_v3) = graph3.index_repository(dir.path(), &files, &manifest_v2)?;
    assert_eq!(report_v3.changed, vec!["Widget.java".to_string()]);

    let symbol_names_v3: Vec<&str> = graph3.symbol_nodes().map(|s| s.name.as_str()).collect();
    assert!(
        symbol_names_v3.contains(&"brandNewMethod"),
        "{symbol_names_v3:?}"
    );
    Ok(())
}
