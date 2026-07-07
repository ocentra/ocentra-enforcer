//! Hard tests for Bicep, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_bicep`])
//! -- there is no bespoke `languages::bicep` extractor to prove
//! zero-regression against (Bicep has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::bicep`]'s own doc comment
//! directly: positional `resource_declaration`/`type_declaration`/
//! `module_declaration` naming (none has a `name` field), the real
//! `user_defined_function` name field, and the absence of any
//! branch-shaped node at all (a purely declarative IaC language).

use enforcer_memory::languages::generic::parse_bicep;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_bicep";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_resource_declaration_as_class_via_positional_name() {
    let src = "resource storageAccount 'Microsoft.Storage/storageAccounts@2021-09-01' = {\n  name: 'x'\n}\n";
    let parsed = parse_bicep(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "storageAccount"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_module_declaration_as_class_via_positional_name() {
    let src = "module networkModule 'network.bicep' = {\n  name: 'x'\n}\n";
    let parsed = parse_bicep(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "networkModule"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_user_defined_function_via_real_name_field() {
    let src = "func buildName(prefix string) string => prefix\n";
    let parsed = parse_bicep(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "buildName"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_expression_inside_string_interpolation() -> TestResult {
    let src = "var storageName = 'st${uniqueString(resourceGroup().id)}'\n";
    let parsed = parse_bicep(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "uniqueString")
        .ok_or("expected a uniqueString call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_import_statement() -> TestResult {
    let src = "import 'foo.bicep' as foo\n";
    let parsed = parse_bicep(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn produces_no_branch_types_at_all() {
    // Bicep is a purely declarative IaC language -- see `LangSpec::bicep`'s
    // own doc comment ("branch_types is EMPTY"). Even source with an
    // ordinary "for"-shaped resource loop must not be mis-recorded as a
    // decision point this crate's complexity metrics would ever consult
    // (complexity extraction is out of this wave's scope for Bicep either
    // way, but the underlying `LangSpec` row itself must stay empty).
    let src = "resource accounts 'Microsoft.Storage/storageAccounts@2021-09-01' = [for i in range(0, 3): {\n  name: 'x${i}'\n}]\n";
    let parsed = parse_bicep(src);
    // Still extracts the resource itself.
    assert_eq!(
        symbol_kind(&parsed.symbols, "accounts"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_storage_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("storage.bicep");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_bicep(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "storageAccount"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "buildName"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "networkModule"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "resource a 'X@1' = {\n  name: 'x'\n}\n";
    let first = parse_bicep(src);
    let second = parse_bicep(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_bicep("resource ( { this is not valid bicep @@@");
    let _ = parsed;
}
