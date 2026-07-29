//! Hard tests for BibTeX, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_bibtex`]). Tier-0 (see
//! [`enforcer_memory::languages::spec::LangSpec::bibtex`]'s own doc
//! comment): `command`'s real `name` field resolves through the
//! ordinary generic call path -- no quirk needed at all.

use enforcer_memory::languages::generic::parse_bibtex;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_bibtex";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "@article{key1,\n  title = {A Title}\n}\n";
    let parsed = parse_bibtex(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn extracts_command_macro_as_call_via_real_name_field() {
    let src = "@article{key1,\n  title = {\\LaTeX}\n}\n";
    let parsed = parse_bibtex(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee == "\\LaTeX"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.bib");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_bibtex(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "@article{key1,\n  title = {A Title}\n}\n";
    let first = parse_bibtex(src);
    let second = parse_bibtex(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_bibtex("not really bibtex @@@ ###");
    let _ = parsed;
}
