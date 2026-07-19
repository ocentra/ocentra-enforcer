//! Hard tests for ISPC, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_ispc`])
//! -- there is no bespoke `languages::ispc` extractor to prove
//! zero-regression against (ISPC has never had one in this crate). This
//! grammar reuses [`enforcer_memory::languages::spec::LangSpec::c`]'s own
//! arrays and declarator-unwrapping quirk verbatim (see that row's own
//! doc comment: this grammar is node-kind-and-field identical to plain C
//! for every construct this crate's extraction scope reads), VENDORED
//! via `vendor/tree-sitter-ispc-local/`.

use enforcer_memory::languages::generic::parse_ispc;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_ispc";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_via_declarator_unwrap() {
    let src = "float add(float a, float b) {\n    return a + b;\n}\n";
    let parsed = parse_ispc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_function_with_from_symbol_scope() -> TestResult {
    let src = "float main() {\n    return add(1, 2);\n}\n";
    let parsed = parse_ispc(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"));
    Ok(())
}

#[test]
fn extracts_struct_specifier_with_field_defines() {
    let src = "struct Particle {\n    float x;\n    float y;\n};\n";
    let parsed = parse_ispc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Particle"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_preproc_include_as_import() {
    let src = "#include \"common.isph\"\n";
    let parsed = parse_ispc(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"common.isph"), "{paths:?}");
}

#[test]
fn extracts_export_prefixed_function_with_foreach_body_without_panicking() {
    let src = r#"
export void updateAll(uniform float pos[], uniform int n) {
    foreach (i = 0 ... n) {
        pos[i] = pos[i] + 1.0f;
    }
}
"#;
    let parsed = parse_ispc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "updateAll"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.ispc");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_ispc(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "updateAll"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Particle"),
        Some(&SymbolKind::Struct)
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "float main() {\n    return add(1, 2);\n}\n";
    let first = parse_ispc(src);
    let second = parse_ispc(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_ispc("float ( { this is not valid ispc @@@");
    let _ = parsed;
}
