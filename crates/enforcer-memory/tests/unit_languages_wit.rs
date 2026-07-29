//! Hard tests for WIT, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_wit`]) -- there is no
//! bespoke `languages::wit` extractor to prove zero-regression against
//! (WIT has never had one in this crate), so these tests assert
//! against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::wit`]'s own doc
//! comment directly, including the confirmed real-grammar-bug finding
//! that the world-level inline function-export shorthand
//! (`export greet: func(...) -> T;`) is broken in this exact published
//! grammar version -- these tests deliberately avoid that shorthand
//! and exercise the equivalent `interface`-nested form instead.

use enforcer_memory::languages::generic::parse_wit;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_wit";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_interface_and_nested_func_item_as_function() {
    let src =
        "package example:host;\n\ninterface types {\n    greet: func(name: string) -> string;\n}\n";
    let parsed = parse_wit(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "types"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_record_and_its_field_defines_edge() {
    let src =
        "package example:host;\n\ninterface types {\n    record point {\n        x: u32,\n        y: u32,\n    }\n}\n";
    let parsed = parse_wit(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "point"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "point" && d.member_name == "x"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_resource_and_its_method_as_method_kind() {
    // Regression guard: `func_item` inside a `resource_item`'s
    // `methods` field must classify as Method (nesting-based, same
    // mechanism Rust's own `function_item` already relies on), not
    // Function.
    let src = "package example:host;\n\ninterface types {\n    resource counter {\n        constructor(start: u32);\n        increment: func();\n    }\n}\n";
    let parsed = parse_wit(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "counter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "increment"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_variant_enum_flags_as_enum_kind() {
    let src = "package example:host;\n\ninterface types {\n    variant shape {\n        circle(u32),\n        square(u32),\n    }\n\n    enum color {\n        red,\n        green,\n    }\n\n    flags permissions {\n        read,\n        write,\n    }\n}\n";
    let parsed = parse_wit(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "shape"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "color"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "permissions"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_world_import_as_dependency_edge() -> TestResult {
    let src =
        "package example:host;\n\ninterface types {}\n\nworld my-world {\n    import types;\n}\n";
    let parsed = parse_wit(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_world_qualified_export_as_dependency_edge() -> TestResult {
    let src = "package example:host;\n\nworld my-world {\n    export example:host/types;\n}\n";
    let parsed = parse_wit(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    assert!(
        !parsed.symbols.iter().any(|s| s.kind == SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn parses_fixture_host_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("host.wit");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_wit(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "types"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "greet"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src =
        "package example:host;\n\ninterface types {\n    greet: func(name: string) -> string;\n}\n";
    let first = parse_wit(src);
    let second = parse_wit(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_wit("interface ( { this is not valid wit @@@");
    let _ = parsed;
}
