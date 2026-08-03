//! Hard tests for Elm, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_elm`])
//! -- there is no bespoke `languages::elm` extractor to prove
//! zero-regression against (Elm has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::elm`]'s own doc comment
//! directly: `value_declaration`'s own name resolution off the nested
//! `functionDeclarationLeft` field's first child, its own real `"body"`
//! field, and `function_call_expr`'s module-qualifier-dropping callee
//! reconstruction.

use enforcer_syntax::languages::generic::parse_elm;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_elm";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_value_declaration_name_off_function_declaration_left() {
    let src = "module M exposing (helper)\n\nhelper r =\n    r * r\n";
    let parsed = parse_elm(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn call_inside_value_body_is_found_via_real_body_field() -> TestResult {
    let src = "module M exposing (area)\n\narea shape =\n    helper shape\n";
    let parsed = parse_elm(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a nested helper call inside the value body")?;
    assert_eq!(call.from_symbol.as_deref(), Some("area"), "{call:?}");
    Ok(())
}

#[test]
fn module_qualified_call_drops_the_module_prefix() -> TestResult {
    let src = "module M exposing (run)\n\nrun xs =\n    List.length xs\n";
    let parsed = parse_elm(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "length")
        .ok_or("expected a length call with the List. prefix dropped")?;
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "List.length"),
        "{:?}",
        parsed.calls
    );
    let _ = call;
    Ok(())
}

#[test]
fn multi_argument_call_records_every_arg_field() -> TestResult {
    // A real, confirmed finding: `helper a b c` parses as ONE
    // `function_call_expr` with THREE repeated `arg` field entries (NOT
    // three nested single-arg curried applications) -- reading only the
    // FIRST `arg` field (`child_by_field_name`, not
    // `children_by_field_name`) would silently drop every argument past
    // the first. See `elm_call_arg_texts`'s own doc comment.
    let src = "module M exposing (run)\n\nrun a b c =\n    helper a b c\n";
    let parsed = parse_elm(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_type_declaration_as_class() {
    let src = "module M exposing (Shape)\n\ntype Shape\n    = Circle Float\n    | Square Float\n";
    let parsed = parse_elm(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_clause_module_name() -> TestResult {
    let src = "module M exposing (x)\n\nimport List exposing (map)\n\nx = 1\n";
    let parsed = parse_elm(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "List")
        .ok_or("expected a List import")?;
    let _ = import;
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_elm("module ??? this is not valid elm @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("Widget.elm");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_elm(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "area"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Shape"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "List"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
