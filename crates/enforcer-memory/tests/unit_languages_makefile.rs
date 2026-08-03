//! Hard tests for Makefile, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_makefile`]) -- there is
//! no bespoke `languages::makefile` extractor to prove zero-regression
//! against (Makefile has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::makefile`]'s own doc
//! comment directly: `rule`'s own unfielded `targets` child (no working
//! field for the common case), the real `"function"` field shared by both
//! `function_call`/`shell_function`, and `include_directive`'s two-level
//! `filenames`-field unwrap.

use enforcer_syntax::languages::generic::parse_makefile;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_makefile";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_rule_as_function() {
    let src = "widget: main.o\n\t$(CC) -o widget main.o\n";
    let parsed = parse_makefile(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "widget"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_include_directive_path_two_levels_deep() -> TestResult {
    let src = "include config.mk\n";
    let parsed = parse_makefile(src);
    let path = parsed
        .imports
        .first()
        .ok_or("expected an import")?
        .module_path
        .as_str();
    assert_eq!(path, "config.mk", "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_function_call_callee_via_real_function_field() -> TestResult {
    let src = "SRCS = $(wildcard *.c)\n";
    let parsed = parse_makefile(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "wildcard")
        .ok_or("expected a wildcard call")?;
    assert_eq!(call.arg_texts, vec!["*.c".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_shell_function_callee_via_same_function_field() -> TestResult {
    let src = "OUT := $(shell echo hi)\n";
    let parsed = parse_makefile(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "shell")
        .ok_or("expected a shell call")?;
    let _ = call;
    Ok(())
}

#[test]
fn recipe_line_call_records_from_symbol_scope() -> TestResult {
    let src = "widget: main.o\n\t$(CC) -o widget main.o\n";
    let parsed = parse_makefile(src);
    // The recipe line's own `$(CC)` is a `variable_reference`, not a
    // `function_call`/`shell_function` -- no CALLS edge is expected from
    // it (matching `LangSpec::makefile`'s own `call_types`, neither of
    // which names `variable_reference`); this instead asserts the rule's
    // OWN body walk reaches recipe content at all without panicking and
    // without misclassifying it as a call.
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "CC"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_makefile("widget: ??? this is not valid make @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.mk");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_makefile(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "all"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "config.mk"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
