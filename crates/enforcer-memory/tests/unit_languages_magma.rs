//! Hard tests for Magma, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_magma`])
//! -- there is no bespoke `languages::magma` extractor to prove
//! zero-regression against (Magma has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::magma`]'s own doc
//! comment directly: the real `call` (not baseline's phantom
//! `call_expression`) node kind, `program` root, and `load_directive`/
//! `import_directive` import shapes.

use enforcer_syntax::languages::generic::parse_magma;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_magma";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_symbol() {
    let src = "function add(a, b)\n    return a + b;\nend function;\n";
    let parsed = parse_magma(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_procedure_definition_symbol() {
    let src = "procedure Greet(name)\n    print name;\nend procedure;\n";
    let parsed = parse_magma(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_via_real_call_node_kind() -> TestResult {
    // Regression guard for the real, load-bearing baseline correction
    // (see `LangSpec::magma`'s own doc comment): the baseline's own
    // `magma_call_types = {"call_expression"}` names a node kind that
    // does not exist in this grammar at all -- without the correction to
    // `["call"]`, this test would find zero calls.
    let src = "x := add(1, 2);\n";
    let parsed = parse_magma(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_load_directive_as_import() {
    let src = "load \"setup.m\";\n";
    let parsed = parse_magma(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"setup.m"));
}

#[test]
fn extracts_import_directive_with_filename_field() {
    let src = "import \"foo.m\": bar;\n";
    let parsed = parse_magma(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"foo.m"));
}

#[test]
fn extracts_branch_heavy_source_without_panicking() {
    let src = r#"
x := 1;
if x gt 0 then
    print "positive";
end if;
for i := 1 to 10 do
    print i;
end for;
"#;
    let parsed = parse_magma(src);
    let _ = parsed;
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.magma");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_magma(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greet"),
        Some(&SymbolKind::Function)
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "add"),
        "{:?}",
        parsed.calls
    );
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"setup.m"));
    assert!(paths.contains(&"foo.m"));
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "x := add(1, 2);\n";
    let first = parse_magma(src);
    let second = parse_magma(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_magma("function ( { this is not valid magma @@@");
    let _ = parsed;
}
