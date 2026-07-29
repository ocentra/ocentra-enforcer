//! Hard tests for FORM (the symbolic-manipulation language, not Go
//! Template's own `form` naming or anything HTML-form-related),
//! onboarded directly through the generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_form`]) --
//! language-parity wave G2.6 (found genuinely missing during the G2.5
//! closeout audit, no bespoke `languages::form` extractor ever
//! existed). Real syntax lifted directly from the baseline's own
//! `tools/tree-sitter-form/test/corpus/procedures.txt` test corpus
//! (`#procedure name(args) ... #endprocedure`, `#call name(args)`), not
//! guessed.

use enforcer_memory::languages::generic::parse_form;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_form";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn procedure_definition_is_a_function_symbol() {
    let src = "#procedure simplify(expr)\n  id x = 1;\n#endprocedure\n";
    let parsed = parse_form(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "simplify"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn call_statement_is_recorded_as_a_call() -> TestResult {
    let src = "#call simplify(result)\n";
    let parsed = parse_form(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "simplify")
        .ok_or("expected a simplify call")?;
    assert_eq!(call.from_symbol, None, "{call:?}");
    Ok(())
}

#[test]
fn call_inside_procedure_body_records_from_symbol_scope() -> TestResult {
    let src = "#procedure greet(x)\n  #call draw(x)\n#endprocedure\n";
    let parsed = parse_form(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "draw")
        .ok_or("expected a draw call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("greet"), "{call:?}");
    Ok(())
}

#[test]
fn include_directive_records_the_included_path() {
    let src = "#include \"widget_defs.h\"\n";
    let parsed = parse_form(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"widget_defs.h"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_form("#procedure @@@ this is not valid FORM (((");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.frm");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_form(&src);
    assert!(
        symbol_kind(&parsed.symbols, "greet") == Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "draw"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "greet"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "widget_defs.h"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
