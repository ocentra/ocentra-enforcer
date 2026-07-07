//! Hard tests for Wolfram Language, onboarded directly through the
//! generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_wolfram`]) -- there is
//! no bespoke `languages::wolfram` extractor to prove zero-regression
//! against (Wolfram has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::wolfram`]'s own doc
//! comment directly: `set_delayed`'s positional (zero-field) function
//! naming off a nested `apply` head, `apply`'s positional callee
//! reconstruction, `Needs[...]`/`get_top`'s IMPORTS detection, and
//! nested-definition descent into a function's own RHS.

use enforcer_memory::languages::generic::parse_wolfram;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_wolfram";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_set_delayed_with_apply_head_as_function() {
    let src = r#"helper[x_] := x + 1"#;
    let parsed = parse_wolfram(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_bare_set_delayed_as_function() {
    // `f := body` with no `[...]` parameter list at all -- still a
    // function-shaped rule definition per the baseline's own
    // `resolve_wolfram_func_name`'s second positional arm.
    let src = r#"helper := 1"#;
    let parsed = parse_wolfram(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn bare_symbol_assignment_is_treated_as_a_named_def_matching_the_baseline() {
    // `x = 1` (a bare `set_top`) has the SAME AST shape as a positional
    // rule head this row's `wolfram_set_function_name` recognizes -- and
    // the C baseline's own `resolve_wolfram_func_name` documents this
    // explicitly ("For a bare `Name = value` (set_top with no apply), the
    // LHS is the symbol itself... Accept all three forms so multiple defs
    // in one file each resolve to a distinct name instead of collapsing"),
    // so this is matched-baseline-behavior, not a bug: a bare variable
    // assignment IS recorded as a named Function-kind symbol too, exactly
    // like the baseline itself does.
    let src = r#"x = 1"#;
    let parsed = parse_wolfram(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "x"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_needs_as_imports_edge() {
    let src = r#"Needs["WidgetLib`"]"#;
    let parsed = parse_wolfram(src);
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "WidgetLib`"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_get_top_as_imports_edge() {
    let src = r#"<<"WidgetLib.wl""#;
    let parsed = parse_wolfram(src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "WidgetLib.wl"),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn extracts_apply_call_with_from_symbol_scope() -> TestResult {
    let src = r#"draw[x_] := helper[x]"#;
    let parsed = parse_wolfram(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_wolfram("helper[ { this is not valid wolfram @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.wl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_wolfram(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "WidgetLib`"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
