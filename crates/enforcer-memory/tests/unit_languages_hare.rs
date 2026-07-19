//! Hard tests for Hare, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_hare`])
//! -- there is no bespoke `languages::hare` extractor to prove
//! zero-regression against (Hare has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::hare`]'s own doc comment
//! directly: `function_declaration`'s real `"name"`/`"body"` fields,
//! `type_declaration`'s positional `identifier` name, `use_statement`'s
//! positional imported path, and `call_expression`'s `"callee"` field
//! plus positional (non-wrapped) argument siblings.

use enforcer_memory::languages::generic::parse_hare;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_hare";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_declaration_symbol() {
    let src = "fn add(a: int, b: int) int = {\n\treturn a + b;\n};\n";
    let parsed = parse_hare(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_inside_function_with_from_symbol_scope() -> TestResult {
    let src = "fn main() void = {\n\thelper(1, 2);\n};\n";
    let parsed = parse_hare(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"));
    Ok(())
}

#[test]
fn extracts_call_with_multiple_positional_args() -> TestResult {
    // Regression guard for the "no wrapping arguments-list node"
    // finding (see `LangSpec::hare`'s own doc comment): without
    // `hare_call_override` scanning the callee's remaining siblings
    // directly, `arg_texts` would come back empty.
    let src = "fn main() void = {\n\thelper(1, 2, 3);\n};\n";
    let parsed = parse_hare(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_type_declaration_as_class_via_positional_name() {
    let src = "type animal = struct {\n\tname: str,\n};\n";
    let parsed = parse_hare(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "animal"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_use_statement_as_import_via_positional_identifier() {
    let src = "use fmt;\n";
    let parsed = parse_hare(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"fmt"), "{paths:?}");
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = r#"
fn main() void = {
	let x = 0;
	if (x > 0) {
		helper();
	};
	for (let i = 0z; i < 10z; i += 1) {
		helper();
	};
};
"#;
    let parsed = parse_hare(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.ha");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_hare(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "animal"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function)
    );
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"fmt"), "{paths:?}");
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "fn main() void = {\n\thelper();\n};\n";
    let first = parse_hare(src);
    let second = parse_hare(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_hare("fn ( { this is not valid hare @@@");
    let _ = parsed;
}
