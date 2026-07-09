//! Hard tests for Assembly, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_assembly`]). Tier-0
//! (see [`enforcer_memory::languages::spec::LangSpec::assembly`]'s own
//! doc comment): `label` has no real `name` field (confirmed via a
//! real `node-types.json` dump), so
//! [`enforcer_memory::languages::generic::assembly_quirk`] reads its
//! name positionally; labels have no call/branch extraction at all
//! (they do not wrap the following instructions as children).

use enforcer_memory::languages::generic::parse_assembly;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_assembly";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_label_as_function_symbol_via_positional_name() {
    let src = "main:\n    mov eax, 1\n    ret\n";
    let parsed = parse_assembly(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_multiple_labels() {
    let src = "main:\n    call foo\nfoo:\n    ret\n";
    let parsed = parse_assembly(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_fixture_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.s");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_assembly(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "main:\n    ret\n";
    let first = parse_assembly(src);
    let second = parse_assembly(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_assembly("@@@ not valid assembly ###");
    let _ = parsed;
}
