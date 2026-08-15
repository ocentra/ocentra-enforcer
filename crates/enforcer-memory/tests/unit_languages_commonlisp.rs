//! Hard tests for Common Lisp, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_commonlisp`]) -- there
//! is no bespoke `languages::commonlisp` extractor to prove
//! zero-regression against (Common Lisp has never had one in this
//! crate), so these tests assert against the grammar-shape ground truth
//! recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::commonlisp`]'s own doc
//! comment directly: `defun`'s own two-level `defun_header` name/body
//! resolution, `in-package`/`require` IMPORTS, and every `list_lit`'s
//! unfiltered head-symbol CALLS (matching Clojure's own posture).

use enforcer_syntax::languages::generic::parse_commonlisp;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_commonlisp";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_defun_as_function() {
    let src = "(defun helper (x)\n  (+ x 1))\n";
    let parsed = parse_commonlisp(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn defun_head_keyword_is_not_recorded_as_a_call() {
    // CORRECTED from this test's own first draft, which wrongly assumed
    // Clojure's own unfiltered `defn`-head-is-also-a-call posture
    // extended to Common Lisp too. Real grammar-shape difference,
    // confirmed via this very test failing during this wave's own
    // verification, not by inspection: Clojure's grammar treats EVERY
    // form uniformly as a `list_lit` with a bare `sym_lit` head, but this
    // grammar's `defun` is instead a DEDICATED node type (with its own
    // `defun_header`/`function_name`/`[value]` fields) wrapped INSIDE the
    // outer `list_lit`, not itself a bare-symbol-headed list --
    // `commonlisp_call_override`'s own `sym_lit`/`identifier`-only head
    // check (see `LangSpec::commonlisp`'s own doc comment) correctly
    // never matches a `defun`-headed `list_lit`'s first named child
    // (which is the `defun` node itself, not a symbol).
    let src = "(defun helper (x)\n  (+ x 1))\n";
    let parsed = parse_commonlisp(src);
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "defun"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn call_inside_defun_body_records_from_symbol_scope() -> TestResult {
    let src = "(defun helper (x)\n  (other x))\n";
    let parsed = parse_commonlisp(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "other")
        .ok_or("expected an other call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("helper"), "{call:?}");
    Ok(())
}

#[test]
fn in_package_records_module_path_as_import() -> TestResult {
    let src = "(in-package :widget)\n";
    let parsed = parse_commonlisp(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "widget")
        .ok_or("expected a widget import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn require_records_module_path_as_import() -> TestResult {
    let src = "(require :other-package)\n";
    let parsed = parse_commonlisp(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "other-package")
        .ok_or("expected an other-package import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = "(defun helper (x)\n  (other x))\n";
    let parsed = parse_commonlisp(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn nested_list_lit_records_both_calls() -> TestResult {
    let src = "(defun area (shape)\n  (other (helper shape)))\n";
    let parsed = parse_commonlisp(src);
    let has_other = parsed.calls.iter().any(|c| c.callee == "other");
    let has_helper = parsed.calls.iter().any(|c| c.callee == "helper");
    assert!(has_other && has_helper, "{:?}", parsed.calls);
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_commonlisp("(defun ((( this is not valid lisp");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.lisp");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_commonlisp(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "area"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "widget"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "other-package"),
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
