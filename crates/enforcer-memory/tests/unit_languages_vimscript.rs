//! Hard tests for VimScript, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_vimscript`]) -- there is
//! no bespoke `languages::vimscript` extractor to prove zero-regression
//! against (VimScript has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::vimscript`]'s own doc
//! comment directly: `function_definition` (the OUTER node) is the sole
//! `func_types` entry (NOT the nested `function_declaration`, which has
//! no `body` field of its own -- a real correctness bug the baseline's
//! own double-listing would have caused), and `import_types` is
//! deliberately empty (the baseline's own `"include"` entry names an
//! anonymous token, never a real named node).

use enforcer_syntax::languages::generic::parse_vimscript;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_vimscript";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_name_off_nested_declaration() {
    let src = "function! Greet(name)\n  echo \"hi\"\nendfunction\n";
    let parsed = parse_vimscript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn call_inside_function_body_is_found_via_real_body_field() -> TestResult {
    // `function_declaration` (the nested name-carrying child) has NO
    // `body` field of its own -- the real body is a SIBLING `body` field
    // of the OUTER `function_definition`. This asserts the quirk's own
    // recursion into that real field actually reaches a nested call, not
    // just that the function symbol itself is recorded (see
    // `LangSpec::vimscript`'s own doc comment for the full "double-listing
    // would silently lose every nested call" finding this guards against).
    let src = "function! Greet(name)\n  call Helper(a:name)\nendfunction\n";
    let parsed = parse_vimscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Helper")
        .ok_or("expected a nested Helper call inside the function body")?;
    assert_eq!(call.from_symbol.as_deref(), Some("Greet"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_call_expression_callee_via_real_function_field() -> TestResult {
    let src = "call Helper(a:name)\n";
    let parsed = parse_vimscript(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Helper")
        .ok_or("expected a Helper call")?;
    let _ = call;
    Ok(())
}

#[test]
fn no_import_types_recorded_for_dead_baseline_entry() {
    // The baseline's own `vim_import_types = ["include"]` names an
    // anonymous keyword token, not a real named node -- see
    // `LangSpec::vimscript`'s own doc comment. No source construct should
    // ever produce an ImportRef for this language.
    let src = "let g:foo = 1\n";
    let parsed = parse_vimscript(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn lambda_expression_is_not_recorded_as_a_named_function() {
    // `lambda_expression` is deliberately dropped from `func_types` (an
    // anonymous closure literal with no name to recover) -- see
    // `LangSpec::vimscript`'s own doc comment.
    let src = "let g:f = {x -> x + 1}\n";
    let parsed = parse_vimscript(src);
    assert!(
        !parsed
            .symbols
            .iter()
            .any(|s| matches!(s.kind, SymbolKind::Function)),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_vimscript("function! ??? this is not valid vim @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.vim");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_vimscript(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Greet"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "Helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
