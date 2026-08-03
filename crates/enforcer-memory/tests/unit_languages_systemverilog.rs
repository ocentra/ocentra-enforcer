//! Hard tests for SystemVerilog, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_systemverilog`]) --
//! there is no bespoke `languages::systemverilog` extractor to prove
//! zero-regression against (SystemVerilog has never had one in this
//! crate), so these tests assert against the grammar-shape ground truth
//! recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::systemverilog`]'s own
//! doc comment directly: `function_body_declaration`/`class_declaration`/
//! `module_declaration`'s own real, DIRECT `[name]` field (a genuine
//! improvement over plain Verilog's doubly-nested unfielded wrapper), and
//! the confirmed real capability GAP CLOSED relative to plain Verilog: a
//! bare-statement call with parenthesized arguments (`helper(1);`) parses
//! cleanly here via `tf_call`, unlike the plain-Verilog parse error the
//! sibling `unit_languages_verilog.rs` test suite documents.

use enforcer_syntax::languages::generic::parse_systemverilog;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_systemverilog";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_via_real_direct_name_field() {
    let src = "module widget;\n  function int helper(int x);\n    return x + 1;\n  endfunction\nendmodule\n";
    let parsed = parse_systemverilog(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_module_via_real_direct_name_field() {
    let src = "module widget;\nendmodule\n";
    let parsed = parse_systemverilog(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_via_real_direct_name_field() {
    let src = "class Area;\n  function int compute(int shape);\n    return shape;\n  endfunction\nendclass\n";
    let parsed = parse_systemverilog(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Area"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn bare_statement_call_with_arguments_produces_a_call_node() -> TestResult {
    // See this module's own doc comment: the exact form that is a parse
    // ERROR in plain Verilog (`unit_languages_verilog.rs`'s own
    // `argless_task_enable_produces_a_call_node` test documents the
    // weaker form that DOES work there) parses cleanly here via
    // `subroutine_call_statement -> subroutine_call -> tf_call`.
    let src = "module widget;\n  function int helper(int x);\n    return x + 1;\n  endfunction\n  initial begin\n    helper(1);\n  end\nendmodule\n";
    let parsed = parse_systemverilog(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert!(call.line > 0, "{call:?}");
    Ok(())
}

#[test]
fn system_tf_call_records_dollar_prefixed_callee() -> TestResult {
    let src = "module widget;\n  initial $display(\"hi\");\nendmodule\n";
    let parsed = parse_systemverilog(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "$display")
        .ok_or("expected a $display call")?;
    assert_eq!(call.arg_texts.len(), 1, "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_systemverilog("module @@@ this is not valid systemverilog ###");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.sv");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_systemverilog(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Area"),
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
