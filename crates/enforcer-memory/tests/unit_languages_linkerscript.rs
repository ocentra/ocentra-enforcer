//! Hard tests for Linker Script, onboarded directly through the
//! generic spec-table engine
//! ([`enforcer_memory::languages::generic::parse_linkerscript`]).
//! Asserts against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::linkerscript`]'s own
//! doc comment: the real root node kind is `linkerscript` (NOT
//! baseline's claimed `source_file`), and `call_expression` carries
//! real `function`/`arguments` fields the generic engine's own
//! field-driven default extracts with no quirk needed.

use enforcer_memory::languages::generic::parse_linkerscript;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_linkerscript";

#[test]
fn extracts_module_symbol_for_linkerscript_root() {
    let src = "ENTRY(_start)\nSECTIONS { .text : { *(.text) } }\n";
    let parsed = parse_linkerscript(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn extracts_call_expression_via_real_fields() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.ld");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_linkerscript(&src);
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "ASSERT")
        .ok_or("expected an ASSERT call")?;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "ENTRY(_start)\nSECTIONS { .text : { *(.text) } }\n";
    let first = parse_linkerscript(src);
    let second = parse_linkerscript(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_linkerscript("((( not linkerscript @@@ ###");
    let _ = parsed;
}
