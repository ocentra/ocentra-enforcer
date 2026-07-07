//! Hard tests for Sway, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_sway`])
//! -- there is no bespoke `languages::sway` extractor to prove
//! zero-regression against (Sway has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::sway`]'s own doc comment
//! directly: Rust-shaped `function_item`/`call_expression` fields,
//! `impl_item`'s `type`-field naming/scoping, `struct_item`/`abi_item`'s
//! Struct/Interface relabeling, and `use_declaration`'s `argument`-field
//! IMPORTS.

use enforcer_memory::languages::generic::parse_sway;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_sway";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function() {
    let src = r#"
fn helper(x: u64) -> u64 {
    x + 1
}
"#;
    let parsed = parse_sway(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_as_struct_kind() {
    let src = r#"
struct Widget {
    label: str,
}
"#;
    let parsed = parse_sway(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_abi_as_interface_kind() {
    let src = r#"
abi Widget {
    fn draw();
}
"#;
    let parsed = parse_sway(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_impl_item_type_field_as_container() -> TestResult {
    let src = r#"
struct Widget {
    label: str,
}

impl Widget {
    fn draw(self) {
        helper(self.label);
    }
}
"#;
    let parsed = parse_sway(src);
    let defines = parsed
        .defines
        .iter()
        .find(|d| d.container_name == "Widget" && d.member_name == "draw")
        .ok_or("expected draw method DEFINES on Widget")?;
    let _ = defines;
    Ok(())
}

#[test]
fn extracts_impl_trait_for_type_as_implements_edge() -> TestResult {
    let src = r#"
trait Drawable {
    fn draw(self);
}

impl Drawable for Widget {
    fn draw(self) {}
}
"#;
    let parsed = parse_sway(src);
    let implements = parsed
        .implements
        .iter()
        .find(|i| i.type_name == "Widget" && i.trait_name == "Drawable")
        .ok_or("expected Widget IMPLEMENTS Drawable")?;
    let _ = implements;
    Ok(())
}

#[test]
fn extracts_use_declaration_argument_field_as_imports_edge() {
    let src = r#"
use std::storage::storage_api::*;
"#;
    let parsed = parse_sway(src);
    assert!(
        !parsed.imports.is_empty(),
        "expected at least one import: {:?}",
        parsed.imports
    );
}

#[test]
fn extracts_function_call_with_real_fields() -> TestResult {
    let src = r#"
fn draw(x: u64) {
    helper(x);
}
"#;
    let parsed = parse_sway(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn if_expression_is_recognized_as_a_branch_node() {
    let src = r#"
fn draw(x: u64) -> u64 {
    if x > 0 {
        helper(x)
    } else {
        0
    }
}
"#;
    let parsed = parse_sway(src);
    let helper_calls = parsed.calls.iter().filter(|c| c.callee == "helper").count();
    assert_eq!(helper_calls, 1, "{:?}", parsed.calls);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_sway("fn ( { this is not valid sway @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.sw");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_sway(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}
