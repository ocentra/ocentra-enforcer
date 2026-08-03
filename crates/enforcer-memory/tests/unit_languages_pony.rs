//! Hard tests for Pony, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_pony`])
//! -- there is no bespoke `languages::pony` extractor to prove
//! zero-regression against (Pony has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::pony`]'s own doc comment
//! directly: `method`/`constructor`/`class_definition`/`actor_definition`/
//! `primitive_definition`'s entirely positional (unfielded) naming, the
//! `is`-clause INHERITS-edge improvement, and `call_expression`'s
//! `"callee"` field plus its separate, unfielded `arguments` sibling.

use enforcer_syntax::languages::generic::parse_pony;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_pony";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_definition_via_positional_name() {
    let src = "class Animal\n  var name: String\n";
    let parsed = parse_pony(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Animal"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_primitive_definition_via_positional_name() {
    let src = "primitive Helpers\n  fun add(a: I64, b: I64): I64 =>\n    a + b\n";
    let parsed = parse_pony(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Helpers"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_method_as_method_symbol_via_positional_name_with_defines_edge() -> TestResult {
    let src = "class Animal\n  fun bark(): String =>\n    \"woof\"\n";
    let parsed = parse_pony(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "bark"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    let defines = parsed
        .defines
        .iter()
        .find(|d| d.container_name == "Animal" && d.member_name == "bark")
        .ok_or("expected Animal DEFINES bark")?;
    let _ = defines;
    Ok(())
}

#[test]
fn extracts_actor_is_clause_as_inherits_edge() -> TestResult {
    let src = "actor Dog is Animal\n  fun bark(): String =>\n    \"woof\"\n";
    let parsed = parse_pony(src);
    let inherit = parsed
        .inherits
        .iter()
        .find(|i| i.sub_name == "Dog" && i.super_name == "Animal")
        .ok_or("expected Dog INHERITS Animal")?;
    let _ = inherit;
    Ok(())
}

#[test]
fn extracts_call_with_unfielded_arguments_sibling() -> TestResult {
    // Regression guard for the "separate, unfielded `arguments` sibling
    // with `positional`-tagged children" finding (see `LangSpec::pony`'s
    // own doc comment): without `pony_call_override` locating the
    // `arguments` sibling by kind, `arg_texts` would come back empty.
    let src = "primitive Helpers\n  fun call_it(): I64 =>\n    add(1, 2)\n";
    let parsed = parse_pony(src);
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
fn extracts_use_statement_string_literal_as_import() {
    let src = "use \"collections\"\n";
    let parsed = parse_pony(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"collections"));
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.pony");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_pony(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Animal"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Helpers"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "add"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "primitive Helpers\n  fun add(a: I64, b: I64): I64 =>\n    a + b\n";
    let first = parse_pony(src);
    let second = parse_pony(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_pony("class ( { this is not valid pony @@@");
    let _ = parsed;
}
