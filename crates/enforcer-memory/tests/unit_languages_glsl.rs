//! Hard tests for GLSL, onboarded by REUSING C's own generic
//! spec-table engine machinery verbatim
//! ([`enforcer_syntax::languages::generic::parse_glsl`]) -- the baseline's
//! own `lang_specs.c` row reuses C's node-type arrays byte-for-byte for
//! this language (see
//! [`enforcer_syntax::languages::spec::LangSpec::glsl`]'s own doc
//! comment), so these tests assert the same C-family shapes
//! `tests/unit_languages_c.rs` already covers, plus the GLSL-specific
//! finding that shader storage qualifiers (`uniform`/`in`/`out`/
//! `layout(...)`) on GLOBAL variable declarations produce locally
//! contained parse-error nodes that never affect function/struct/call
//! extraction (this crate's entire GLSL extraction scope).

use enforcer_syntax::languages::generic::parse_glsl;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_glsl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition() {
    let src = r#"
vec3 helper(vec3 x) {
    return x;
}
"#;
    let parsed = parse_glsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_and_field_defines() {
    let src = r#"
struct Light {
    vec3 position;
    vec3 color;
};
"#;
    let parsed = parse_glsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Light"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Light", "position")));
    assert!(defines.contains(&("Light", "color")));
}

#[test]
fn extracts_call_expression() -> TestResult {
    let src = r#"
void main() {
    helper();
}
"#;
    let parsed = parse_glsl(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
void main() {
    helper();
}
"#;
    let parsed = parse_glsl(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_include_as_imports_edge() {
    let src = r#"
#include "common.glsl"
"#;
    let parsed = parse_glsl(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"common.glsl"));
}

#[test]
fn version_directive_is_ignored_gracefully() {
    // `#version 330 core` is not `preproc_include` -- it must not crash
    // and must not produce a spurious import.
    let src = r#"
#version 330 core

void main() {
}
"#;
    let parsed = parse_glsl(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
    assert!(
        symbol_kind(&parsed.symbols, "main").is_some(),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn function_extraction_is_unaffected_by_qualifier_parse_errors_elsewhere() {
    // `uniform`/`out`/`layout(...) in` storage-qualified GLOBAL
    // declarations produce locally contained parse-error nodes against
    // plain C grammar (see `LangSpec::glsl`'s own doc comment) -- this
    // must never affect `main()`'s own extraction, including a call
    // inside its body referencing one of the malformed globals by name.
    let src = r#"
layout (location = 0) in vec3 aPos;
out vec3 vColor;
uniform mat4 model;

vec3 transform(vec3 p) {
    return p;
}

void main() {
    vec3 result = transform(aPos);
    vColor = result;
}
"#;
    let parsed = parse_glsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "transform"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"transform"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_glsl("void main( { this is not valid glsl @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.glsl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_glsl(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "main"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "common.glsl"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
