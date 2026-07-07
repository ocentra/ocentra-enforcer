//! Hard tests for HLSL, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_hlsl`])
//! -- there is no bespoke `languages::hlsl` extractor to prove
//! zero-regression against (HLSL has never had one in this crate). This
//! grammar reuses [`enforcer_memory::languages::spec::LangSpec::cpp`]'s
//! own arrays and declarator-unwrapping quirk verbatim (see that row's
//! own doc comment: `tree-sitter-hlsl` is literally a fork of
//! `tree-sitter-cpp`), so these tests assert the same declarator-nesting
//! shape C++'s own tests already establish, plus HLSL's own accepted
//! top-level-`cbuffer` parse-gap.

use enforcer_memory::languages::generic::parse_hlsl;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_hlsl";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_via_declarator_unwrap() {
    let src = "float4 add(float4 a, float4 b) {\n    return a + b;\n}\n";
    let parsed = parse_hlsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_function_with_from_symbol_scope() -> TestResult {
    let src = "float4 main() {\n    return add(1, 2);\n}\n";
    let parsed = parse_hlsl(src);
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
    // `SymbolKind::Class`, NOT `SymbolKind::Struct`: `cpp_handle_class_or_struct`
    // (reused verbatim here, see `LangSpec::hlsl`'s own doc comment)
    // classifies BOTH `class_specifier` and `struct_specifier` as Class --
    // `SymbolKind::Struct` is reserved for Rust's own dedicated struct
    // concept (see that enum variant's own doc comment), which C-family
    // languages including HLSL have no equivalent distinct kind for.
    let src = "struct VertexInput {\n    float3 position;\n};\n";
    let parsed = parse_hlsl(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "VertexInput"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_preproc_include_as_import() {
    let src = "#include \"common.hlsli\"\n";
    let parsed = parse_hlsl(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"common.hlsli"), "{paths:?}");
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = r#"
float4 main(float4 x) {
    float4 result = x;
    if (result.x > 0) {
        result = x;
    }
    for (int i = 0; i < 4; i++) {
        result = x;
    }
    return result;
}
"#;
    let parsed = parse_hlsl(src);
    assert!(symbol_kind(&parsed.symbols, "main").is_some());
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.hlsl");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_hlsl(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(symbol_kind(&parsed.symbols, "main").is_some());
    // `SymbolKind::Class`, not `Struct` -- see
    // `extracts_struct_specifier_with_field_defines`'s own doc comment.
    assert_eq!(
        symbol_kind(&parsed.symbols, "VertexInput"),
        Some(&SymbolKind::Class)
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "float4 main() {\n    return add(1, 2);\n}\n";
    let first = parse_hlsl(src);
    let second = parse_hlsl(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_hlsl("float4 ( { this is not valid hlsl @@@");
    let _ = parsed;
}

#[test]
fn top_level_cbuffer_does_not_panic_despite_documented_grammar_gap() {
    // See `LangSpec::hlsl`'s own doc comment: a top-level `cbuffer`
    // block does not parse as the grammar's dedicated
    // `cbuffer_specifier` node (that node kind is only reachable from
    // statement context) -- this is an accepted, documented upstream
    // grammar limitation, not a crash.
    let src = "cbuffer ConstantBuffer : register(b0)\n{\n    float4x4 worldViewProj;\n};\n";
    let parsed = parse_hlsl(src);
    let _ = parsed;
}
