//! Hard tests for NASM, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_nasm`])
//! -- there is no bespoke `languages::nasm` extractor to prove
//! zero-regression against (NASM has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::nasm`]'s own doc comment
//! directly: `label`'s real `"name"` field, `actual_instruction`'s
//! `"instruction"`/`"operands"` fields (recording EVERY instruction as a
//! CALLS edge, matching the baseline's own real modeling choice), and
//! `preproc_include`'s `"path"` field.

use enforcer_memory::languages::generic::parse_nasm;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_nasm";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_label_as_function_symbol() {
    let src = "print_msg:\n    ret\n";
    let parsed = parse_nasm(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "print_msg"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_instruction_as_call_edge() -> TestResult {
    let src = "_start:\n    call print_msg\n    ret\n";
    let parsed = parse_nasm(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "call")
        .ok_or("expected a call instruction recorded as a CALLS edge")?;
    assert_eq!(call.from_symbol.as_deref(), Some("_start"));
    Ok(())
}

#[test]
fn extracts_ordinary_instruction_as_call_edge_matching_baseline_modeling() -> TestResult {
    // Regression guard for the baseline's own real (if unusual) choice
    // to record EVERY instruction, not just `call`/`jmp`-family ones, as
    // a CALLS edge (see `LangSpec::nasm`'s own doc comment).
    let src = "_start:\n    mov eax, 1\n";
    let parsed = parse_nasm(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "mov")
        .ok_or("expected a mov instruction recorded as a CALLS edge")?;
    assert!(call.arg_texts.iter().any(|a| a == "eax"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_preproc_include_as_import() {
    let src = "%include \"common.inc\"\n";
    let parsed = parse_nasm(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"common.inc"));
}

#[test]
fn extracts_preproc_def_as_function_symbol() {
    let src = "%define BUFSIZE 128\n";
    let parsed = parse_nasm(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "BUFSIZE"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.nasm");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_nasm(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "_start"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "print_msg"),
        Some(&SymbolKind::Function)
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "call"),
        "{:?}",
        parsed.calls
    );
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"common.inc"));
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "_start:\n    call print_msg\n    ret\n";
    let first = parse_nasm(src);
    let second = parse_nasm(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_nasm("this is not @@@ valid nasm at all !!!");
    let _ = parsed;
}
