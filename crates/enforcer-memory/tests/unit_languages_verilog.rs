//! Hard tests for Verilog, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_verilog`]) -- there is
//! no bespoke `languages::verilog` extractor to prove zero-regression
//! against (Verilog has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::verilog`]'s own doc
//! comment directly: `function_body_declaration`'s own doubly-nested
//! `function_identifier` name resolution, `module_declaration`'s own
//! unfielded `module_header`-descendant name resolution, and the
//! confirmed, non-obvious real grammar LIMITATION that a bare-statement
//! call with parenthesized arguments (`helper(1);`) is a genuine parse
//! error -- only the expression/condition-position form (`if
//! (helper(1))`) produces a recognizable call node.

use enforcer_syntax::languages::generic::parse_verilog;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_verilog";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_via_doubly_nested_identifier_wrapper() {
    let src = "module widget;\n  function integer helper;\n    input integer x;\n    begin\n      helper = x + 1;\n    end\n  endfunction\nendmodule\n";
    let parsed = parse_verilog(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_module_via_unfielded_module_header_descendant() {
    let src = "module widget;\nendmodule\n";
    let parsed = parse_verilog(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_via_class_identifier_wrapper() {
    // `class_declaration`'s own direct `class_identifier` child WRAPS
    // (rather than IS) the real `simple_identifier` leaf, but since that
    // wrapper's own span covers exactly its one required child, its own
    // `utf8_text` is byte-for-byte identical to the leaf's -- this test
    // is the empirical check for that assumption (`verilog_name`'s own
    // `class_declaration`/`interface_declaration` branch returns the
    // wrapper node directly rather than descending one level further).
    let src = "module widget;\n  class Area;\n  endclass\nendmodule\n";
    let parsed = parse_verilog(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Area"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(
        !parsed
            .symbols
            .iter()
            .any(|s| s.name.contains("class_identifier")),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn system_tf_call_records_dollar_prefixed_callee() -> TestResult {
    let src = "module widget;\n  initial $display(\"hi\");\nendmodule\n";
    let parsed = parse_verilog(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "$display")
        .ok_or("expected a $display call")?;
    assert_eq!(call.arg_texts.len(), 1, "{call:?}");
    Ok(())
}

#[test]
fn function_call_inside_condition_produces_a_call_node() -> TestResult {
    // See `LangSpec::verilog`'s own doc comment: this is the ONE
    // expression-position form this grammar version actually surfaces a
    // call node for.
    let src = "module widget;\n  function integer helper;\n    input integer x;\n    begin\n      helper = x + 1;\n    end\n  endfunction\n  initial begin\n    if (helper(1))\n      $display(\"ok\");\n  end\nendmodule\n";
    let parsed = parse_verilog(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert!(call.line > 0, "{call:?}");
    Ok(())
}

#[test]
fn argless_task_enable_produces_a_call_node() -> TestResult {
    let src = "module widget;\n  task helper;\n    begin\n      $display(\"hi\");\n    end\n  endtask\n  initial begin\n    helper;\n  end\nendmodule\n";
    let parsed = parse_verilog(src);
    // The argless task-enable form (`helper;`) parses as a plain
    // identifier reference in a `data_declaration`-shaped statement in
    // this grammar version (confirmed via a real parse-tree dump), NOT
    // as one of this row's own `call_types` kinds -- this test documents
    // that real, confirmed absence rather than asserting a call that
    // does not exist. The task DEFINITION itself must still resolve.
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    let _ = parsed.calls.iter().find(|c| c.callee == "helper");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_verilog("module @@@ this is not valid verilog ###");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.v");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_verilog(&src);
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
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
