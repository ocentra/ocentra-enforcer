//! Hard tests for Go Mod (`go.mod` file grammar), onboarded directly
//! through the generic spec-table engine
//! ([`enforcer_syntax::languages::generic::parse_gomod`]) -- grammar
//! VENDORED (`vendor/tree-sitter-gomod-local/`). Asserts the real
//! `require_directive` -> IMPORTS correction documented in
//! [`enforcer_syntax::languages::spec::LangSpec::gomod`]'s own doc
//! comment (baseline's own `gomod_import_types` names a dead
//! `"require"` node kind that does not exist in this real grammar).

use enforcer_syntax::languages::generic::parse_gomod;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_gomod";

#[test]
fn extracts_require_directive_as_import() -> TestResult {
    let src = "module example.com/foo\n\nrequire github.com/bar/baz v1.2.3\n";
    let parsed = parse_gomod(src);
    assert!(parsed
        .imports
        .iter()
        .any(|i| i.module_path == "github.com/bar/baz"));
    Ok(())
}

#[test]
fn extracts_grouped_require_block_entries() -> TestResult {
    let src = "module example.com/foo\n\nrequire (\n\tgithub.com/x/y v0.1.0\n)\n";
    let parsed = parse_gomod(src);
    parsed
        .imports
        .iter()
        .find(|i| i.module_path == "github.com/x/y")
        .ok_or("expected an import for github.com/x/y")?;
    Ok(())
}

#[test]
fn does_not_import_replace_directive() {
    let src = "module example.com/foo\n\nreplace github.com/bar/baz => ../baz\n";
    let parsed = parse_gomod(src);
    assert!(
        !parsed.imports.iter().any(|i| i.module_path.contains("baz")),
        "{:?}",
        parsed.imports
    );
}

#[test]
fn parses_fixture_go_mod_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("go.mod");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_gomod(&src);
    assert!(parsed
        .imports
        .iter()
        .any(|i| i.module_path == "github.com/bar/baz"));
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "module example.com/foo\n\ngo 1.21\n";
    let first = parse_gomod(src);
    let second = parse_gomod(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let source = "this is not [[[ valid go.mod @@@";
    assert_eq!(parse_gomod(source), parse_gomod(source));
}
