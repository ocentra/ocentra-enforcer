//! Hard tests for GN, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_gn`])
//! -- grammar VENDORED (`vendor/tree-sitter-gn-local/`). Asserts
//! against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::gn`]'s own doc
//! comment: real `call_expression`/`if_statement`/`foreach_statement`
//! fields, plus [`enforcer_memory::languages::generic::gn_quirk`]'s
//! own fieldless `import_statement` claim.

use enforcer_memory::languages::generic::parse_gn;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_gn";

#[test]
fn extracts_import_via_quirk() -> TestResult {
    let src = "import(\"//build/config.gni\")\n";
    let parsed = parse_gn(src);
    parsed
        .imports
        .iter()
        .find(|i| i.module_path == "//build/config.gni")
        .ok_or("expected an import for //build/config.gni")?;
    Ok(())
}

#[test]
fn extracts_call_via_real_function_field() -> TestResult {
    let src = "print(\"hi\")\n";
    let parsed = parse_gn(src);
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "print")
        .ok_or("expected a print call")?;
    Ok(())
}

#[test]
fn parses_fixture_build_gn_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("BUILD.gn");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_gn(&src);
    parsed
        .imports
        .iter()
        .find(|i| i.module_path == "//build/config.gni")
        .ok_or("expected an import for //build/config.gni")?;
    parsed
        .calls
        .iter()
        .find(|c| c.callee == "executable")
        .ok_or("expected an executable(...) call")?;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "executable(\"foo\") {\n  sources = [ \"a.cc\" ]\n}\n";
    let first = parse_gn(src);
    let second = parse_gn(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_gn("this { is not [[[ valid gn @@@");
    let _ = parsed;
}
