//! Hard tests for Fortran, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_fortran`])
//! -- there is no bespoke `languages::fortran` extractor to prove
//! zero-regression against (Fortran has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::fortran`]'s own doc
//! comment directly: `function`/`subroutine`'s own name resolution off
//! the nested `*_statement` child's real `"name"` field, and the two
//! distinct callee field names (`"function"` for `call_expression`,
//! `"subroutine"` for `subroutine_call` -- the real, confirmed baseline
//! gap this row fills).

use enforcer_syntax::languages::generic::parse_fortran;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_fortran";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_name_off_nested_statement() {
    let src = "function area(r) result(a)\n  real :: r, a\n  a = r * r\nend function area\n";
    let parsed = parse_fortran(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "area"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_subroutine_name_off_nested_statement() {
    let src = "subroutine greet(name)\n  character(len=*) :: name\n  print *, name\nend subroutine greet\n";
    let parsed = parse_fortran(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_subroutine_call_callee_via_subroutine_field() -> TestResult {
    // The `"subroutine_call"` node kind and its own `"subroutine"` field
    // (distinct from `call_expression`'s `"function"` field) are a real,
    // confirmed addition beyond the baseline's own `fortran_call_types`
    // array -- see `LangSpec::fortran`'s own doc comment. Also asserts
    // `arg_texts` explicitly: `subroutine_call` has NO named field for its
    // own `argument_list` child at all (found by kind instead) -- a real
    // bug this row's own initial implementation had (passed the wrong
    // FIELD NAME, `"argument_list"`, confusing a node KIND name for a
    // field name) that a from_symbol-only assertion here would not have
    // caught.
    let src =
        "subroutine greet(name)\n  character(len=*) :: name\n  call helper(name)\nend subroutine greet\n";
    let parsed = parse_fortran(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper subroutine call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("greet"), "{call:?}");
    assert_eq!(call.arg_texts, vec!["name".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_call_expression_callee_via_function_field() -> TestResult {
    // A real, confirmed grammar quirk: `call_expression`'s own
    // `"function"` field (`"required": false` in `node-types.json`) is
    // genuinely ABSENT for this common bare-identifier callee shape
    // (confirmed via a real parse-tree dump, not merely assumed) -- the
    // node's own first named child (a plain `identifier`) is the real
    // callee. Also asserts `arg_texts` explicitly: the real field name
    // for this node's own arguments is `"arguments"`, NOT
    // `"argument_list"` (that is the WRAPPER NODE's own kind name) -- a
    // real bug this row's own initial implementation had that a
    // from_symbol-only assertion here would not have caught.
    let src =
        "function area(r) result(a)\n  real :: r, a\n  a = helper(r) * 2\nend function area\n";
    let parsed = parse_fortran(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper function call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("area"), "{call:?}");
    assert_eq!(call.arg_texts, vec!["r".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_use_statement_module_path() -> TestResult {
    let src = "module widget\n  use iso_fortran_env\nend module widget\n";
    let parsed = parse_fortran(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path.contains("iso_fortran_env")),
        "{:?}",
        parsed.imports
    );
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_fortran("function ??? this is not valid fortran @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.f90");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_fortran(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "area"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "greet"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
