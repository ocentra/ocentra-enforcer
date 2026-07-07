//! Hard tests for Jsonnet, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_jsonnet`]) -- there is
//! no bespoke `languages::jsonnet` extractor to prove zero-regression
//! against (Jsonnet has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::jsonnet`]'s own doc
//! comment directly: the function-vs-value `bind`/`field`
//! disambiguation (no dedicated function-shaped node exists at all in
//! this grammar), the sibling-lookup `suffix_apply` callee, and the
//! dedicated `import_expr` wrapper node (not the baseline's dead
//! `import`/`importstr` bare names).

use enforcer_memory::languages::generic::parse_jsonnet;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_jsonnet";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_named_local_function_via_bind_params_child() {
    // Regression guard for the confirmed "no dedicated function node at
    // all" grammar shape (see `LangSpec::jsonnet`'s own doc comment) --
    // `local greeting(name) = ...;` flattens `params` directly as a
    // sibling of `bind`, not wrapped in any function-expression node.
    let src = "local greeting(name) = \"hi\" + name;\ngreeting\n";
    let parsed = parse_jsonnet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greeting"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_named_function_via_explicit_function_keyword_binding() {
    let src = "local f = function(x) x + 1;\nf\n";
    let parsed = parse_jsonnet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "f"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn plain_value_binding_produces_no_symbol() {
    let src = "local isProd = true;\nisProd\n";
    let parsed = parse_jsonnet(src);
    assert!(
        symbol_kind(&parsed.symbols, "isProd").is_none(),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_object_method_sugar_field_as_function() {
    let src = "{\n  fn(x): x + 1,\n}\n";
    let parsed = parse_jsonnet(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "fn"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn plain_object_field_produces_no_symbol() {
    let src = "{\n  name: \"widget\",\n}\n";
    let parsed = parse_jsonnet(src);
    assert!(
        symbol_kind(&parsed.symbols, "name").is_none(),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_via_sibling_suffix_apply_lookup() -> TestResult {
    // Regression guard for the sibling-lookup callee (see
    // `LangSpec::jsonnet`'s own doc comment): `suffix_apply` has no
    // callee field of its own at all -- without `jsonnet_call_override`
    // reading the preceding sibling, the callee would be unresolvable.
    let src = "local greeting(name) = \"hi\" + name;\ngreeting(\"world\")\n";
    let parsed = parse_jsonnet(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greeting")
        .ok_or("expected a greeting call")?;
    assert_eq!(call.arg_texts, vec!["\"world\"".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_import_expr_as_import() -> TestResult {
    let src = "import \"other.libsonnet\"\n";
    let parsed = parse_jsonnet(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(!paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn extracts_branch_heavy_expression_without_panicking() {
    let src = "{\n  value: if true then 1 else 2,\n}\n";
    let parsed = parse_jsonnet(src);
    let _ = parsed;
}

#[test]
fn parses_fixture_config_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("config.jsonnet");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_jsonnet(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "greeting"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "fn"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "greeting")
        .ok_or("expected a greeting call")?;
    let _ = call;
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "local greeting(name) = \"hi\" + name;\ngreeting(\"world\")\n";
    let first = parse_jsonnet(src);
    let second = parse_jsonnet(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_jsonnet("local ( { this is not valid jsonnet @@@");
    let _ = parsed;
}
