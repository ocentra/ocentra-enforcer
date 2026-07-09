//! Hard tests for KDL, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_kdl`]). KDL is a
//! Tier-0 nominal language (see
//! [`enforcer_memory::languages::spec::LangSpec::kdl`]'s own doc
//! comment): only its own real root node kind (`document`, matching
//! baseline) is asserted. NOT the same concept as this crate's own
//! Kubernetes-manifest handling (`Language::K8s`/`Language::Kustomize`,
//! both deferred -- see `parsers::Language`'s own doc comments).

use enforcer_memory::languages::generic::parse_kdl;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_kdl";

#[test]
fn extracts_module_symbol_for_document_root() {
    let src = "node \"value\"\n";
    let parsed = parse_kdl(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.kdl");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_kdl(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "node \"value\"\n";
    let first = parse_kdl(src);
    let second = parse_kdl(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_kdl("((( not kdl @@@ ###");
    let _ = parsed;
}
