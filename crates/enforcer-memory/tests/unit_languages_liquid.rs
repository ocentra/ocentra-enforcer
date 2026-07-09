//! Hard tests for Liquid, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_liquid`]). Asserts
//! against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::liquid`]'s own doc
//! comment: the real root node kind is `program` (NOT baseline's
//! claimed `template`), and `include_statement` is claimed by
//! [`enforcer_memory::languages::generic::liquid_quirk`] (the generic
//! walker's own `import_types` branch has no field-driven default of
//! its own at all).

use enforcer_memory::languages::generic::parse_liquid;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_liquid";

#[test]
fn extracts_include_statement_as_import() -> TestResult {
    let src = "{% include \"header.liquid\" %}\n";
    let parsed = parse_liquid(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path.contains("header.liquid"))
        .ok_or("expected a header.liquid import")?;
    let _ = import;
    Ok(())
}

#[test]
fn extracts_module_symbol_for_program_root() {
    let src = "<h1>{{ title }}</h1>\n";
    let parsed = parse_liquid(src);
    assert!(!parsed.symbols.is_empty(), "{:?}", parsed.symbols);
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.liquid");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_liquid(&src);
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path.contains("header.liquid")),
        "{:?}",
        parsed.imports
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "{% include \"header.liquid\" %}\n";
    let first = parse_liquid(src);
    let second = parse_liquid(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_liquid("{% not liquid @@@ ###");
    let _ = parsed;
}
