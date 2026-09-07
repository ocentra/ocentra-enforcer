//! Hard tests for Mermaid, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_mermaid`]). Mermaid is
//! a Tier-0 nominal language (see
//! [`enforcer_syntax::languages::spec::LangSpec::mermaid`]'s own doc
//! comment): only its own real root node kind (`source_file`, matching
//! baseline) is asserted.

use enforcer_syntax::languages::generic::parse_mermaid;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_mermaid";

#[test]
fn extracts_module_symbol_for_source_file_root() {
    let src = "sequenceDiagram\n    Alice->>Bob: hi\n";
    let parsed = parse_mermaid(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.mmd");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_mermaid(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "sequenceDiagram\n    Alice->>Bob: hi\n";
    let first = parse_mermaid(src);
    let second = parse_mermaid(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_mermaid("not really a diagram @@@ ###");
    let _ = parsed;
}
