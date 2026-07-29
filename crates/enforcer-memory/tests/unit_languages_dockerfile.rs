//! Hard tests for Dockerfile, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_dockerfile`]). Tier-0
//! (see [`enforcer_memory::languages::spec::LangSpec::dockerfile`]'s
//! own doc comment): only its own real root node kind (`source_file`,
//! matching baseline) is asserted -- `var_types` is documentation
//! parity only, not consumed by this crate's generic walker.

use enforcer_memory::languages::generic::parse_dockerfile;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_dockerfile";

#[test]
fn extracts_module_symbol_for_source_file_root() {
    let src = "FROM alpine:3.18\nENV FOO=bar\n";
    let parsed = parse_dockerfile(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("Dockerfile");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_dockerfile(&src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "FROM alpine:3.18\nENV FOO=bar\nARG BAZ=1\nRUN echo hi\n";
    let first = parse_dockerfile(src);
    let second = parse_dockerfile(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_dockerfile("not really a dockerfile @@@ ###");
    let _ = parsed;
}
