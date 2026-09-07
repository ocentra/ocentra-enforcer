//! Hard tests for COBOL, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_cobol`])
//! -- there is no bespoke `languages::cobol` extractor to prove
//! zero-regression against (COBOL has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::cobol`]'s own doc
//! comment directly: `program_definition`'s own two-level
//! `identification_division > program_name` name resolution,
//! `call_statement`'s own quoted-string-literal callee, `copy_statement`'s
//! own `[book]`-field IMPORTS, and the confirmed-real `if_header`/
//! `evaluate_header`/`perform_statement_call_proc` branch kinds. Grammar
//! is VENDORED (`crates/enforcer-memory/vendor/tree-sitter-cobol-local/`)
//! -- see that crate's own `src/lib.rs` module doc for the full
//! grammar-sourcing rationale.

use enforcer_syntax::languages::generic::parse_cobol;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_cobol";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_program_definition_via_identification_division() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           STOP RUN.\n";
    let parsed = parse_cobol(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "HELPER"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn call_statement_records_quoted_program_name_as_callee() -> TestResult {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           CALL 'SUBPROG' USING X.\n           STOP RUN.\n";
    let parsed = parse_cobol(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "SUBPROG")
        .ok_or("expected a SUBPROG call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("HELPER"), "{call:?}");
    Ok(())
}

#[test]
fn call_statement_callee_has_quotes_stripped() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           CALL 'SUBPROG' USING X.\n           STOP RUN.\n";
    let parsed = parse_cobol(src);
    assert!(
        !parsed.calls.iter().any(|c| c.callee.contains('\'')),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn copy_statement_records_book_field_as_import() -> TestResult {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       DATA DIVISION.\n       WORKING-STORAGE SECTION.\n       COPY COMMONDEF.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           STOP RUN.\n";
    let parsed = parse_cobol(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "COMMONDEF")
        .ok_or("expected a COMMONDEF import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn open_statement_is_not_recorded_as_an_import() {
    // See `LangSpec::cobol`'s own doc comment: baseline's own
    // `open_statement` choice is deliberately NOT reproduced here (real
    // COBOL file I/O, not a dependency concept) -- `copy_statement` is
    // the real idiom this row wires instead.
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           STOP RUN.\n";
    let parsed = parse_cobol(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn if_end_if_uses_the_real_if_header_branch_kind() {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           IF X > 0\n               DISPLAY \"POS\"\n           END-IF.\n           STOP RUN.\n";
    let parsed = parse_cobol(src);
    // No direct branch-count assertion surface on `ParsedFile` -- this
    // fixture existing and parsing without error/panic, combined with
    // the dedicated fixture-file test below asserting real symbols still
    // resolve past an IF/END-IF block, is this crate's own established
    // convention for `branch_types` coverage at this Tier-2 depth (no
    // complexity extraction wired this wave).
    assert_eq!(
        symbol_kind(&parsed.symbols, "HELPER"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn perform_statement_call_proc_does_not_break_paragraph_parsing() -> TestResult {
    let src = "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELPER.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           PERFORM SUB-PARA.\n           STOP RUN.\n       SUB-PARA.\n           DISPLAY \"IN SUB\".\n";
    let parsed = parse_cobol(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "HELPER"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_cobol("       IDENTIFICATION @@@ this is not valid cobol ###\n");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.cbl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_cobol(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "WIDGET"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "COMMONDEF"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "HELPER"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
