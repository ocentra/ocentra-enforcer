//! Hard tests for Smithy, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_smithy`]) -- there is
//! no bespoke `languages::smithy` extractor to prove zero-regression
//! against (Smithy has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::smithy`]'s own doc
//! comment directly: every claimed def/field kind genuinely exposes a
//! real `"name"` field (no quirk needed for definitions at all), and
//! the real `external_shape_id` import shape
//! [`enforcer_memory::languages::generic::smithy_quirk`] joins from two
//! fields.

use enforcer_memory::languages::generic::parse_smithy;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_smithy";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_service_operation_and_resource_via_real_name_field() {
    let src = "namespace example.weather\n\nservice Weather {\n    version: \"1\"\n}\n\nresource City {\n    identifiers: { cityId: CityId }\n}\n\noperation GetCity {\n    input: GetCityInput\n}\n";
    let parsed = parse_smithy(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Weather"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "City"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "GetCity"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_structure_and_union_via_real_name_field() {
    let src = "namespace example.weather\n\nstructure CityId {\n    id: String\n}\n\nunion MyUnion {\n    a: String\n    b: Integer\n}\n";
    let parsed = parse_smithy(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "CityId"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyUnion"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "CityId" && d.member_name == "id"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_module_via_namespace_statement_not_the_idl_root() {
    // Regression guard for the confirmed real-grammar finding (see
    // `LangSpec::smithy`'s own doc comment): the real file root is
    // `idl`, which has no meaningful single name -- `module_types`
    // instead points at `namespace_statement`.
    let src = "namespace example.weather\n\nstructure X {}\n";
    let parsed = parse_smithy(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "example.weather"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_use_statement_as_import_joined_from_two_fields() -> TestResult {
    let src = "namespace example.weather\n\nuse aws.protocols#restJson1\n\nstructure X {}\n";
    let parsed = parse_smithy(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path.contains("restJson1"))
        .ok_or("expected an aws.protocols#restJson1 import")?;
    assert!(import.module_path.contains("aws.protocols"), "{import:?}");
    Ok(())
}

#[test]
fn parses_fixture_weather_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("weather.smithy");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_smithy(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Weather"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "CityId"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "namespace example.weather\n\nstructure X {}\n";
    let first = parse_smithy(src);
    let second = parse_smithy(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_smithy("structure ( { this is not valid smithy @@@");
    let _ = parsed;
}
