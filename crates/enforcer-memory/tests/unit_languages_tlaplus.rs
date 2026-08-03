//! Hard tests for TLA+, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_tlaplus`]) -- there is
//! no bespoke `languages::tlaplus` extractor to prove zero-regression
//! against (TLA+ has never had one in this crate), so these tests assert
//! against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::tlaplus`]'s own doc
//! comment directly: `operator_definition`'s own real `name`/
//! `definition` fields, `bound_op`'s own repeated `[parameter]`-field
//! CALLS reconstruction, `function_evaluation`'s own field-less
//! first-named-child callee (the corrected finding that const's own doc
//! comment documents), and `extends`/`instance`'s own repeated
//! `identifier_ref` IMPORTS list.

use enforcer_syntax::languages::generic::parse_tlaplus;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_tlaplus";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_operator_definition_as_function() {
    let src = "---- MODULE Widget ----\nHelper(x) == x + 1\n====\n";
    let parsed = parse_tlaplus(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_definition_array_form_as_function() {
    let src = "---- MODULE Widget ----\nf[x \\in Nat] == x + 1\n====\n";
    let parsed = parse_tlaplus(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "f"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn bound_op_call_records_callee_and_arguments() -> TestResult {
    let src = "---- MODULE Widget ----\nHelper(x) == x + 1\nArea(shape) == Helper(shape)\n====\n";
    let parsed = parse_tlaplus(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Helper")
        .ok_or("expected a Helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("Area"), "{call:?}");
    assert_eq!(call.arg_texts, vec!["shape".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn function_evaluation_call_records_bracketed_argument() -> TestResult {
    let src = "---- MODULE Widget ----\nf[x \\in Nat] == x + 1\nArea(shape) == f[shape]\n====\n";
    let parsed = parse_tlaplus(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "f")
        .ok_or("expected an f call")?;
    assert_eq!(call.arg_texts, vec!["shape".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extends_records_each_module_name_as_a_separate_import() -> TestResult {
    let src = "---- MODULE Widget ----\nEXTENDS Naturals, Sequences\n====\n";
    let parsed = parse_tlaplus(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    if !paths.contains(&"Naturals") || !paths.contains(&"Sequences") {
        return Err(format!("expected both Naturals and Sequences, got {paths:?}").into());
    }
    Ok(())
}

#[test]
fn instance_records_module_name_as_import() -> TestResult {
    let src = "---- MODULE Widget ----\nINSTANCE Other\n====\n";
    let parsed = parse_tlaplus(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "Other")
        .ok_or("expected an Other import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_tlaplus("---- MODULE @@@ this is not valid tla ###");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.tla");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_tlaplus(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Area"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "Naturals"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "Helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
