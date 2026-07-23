//! Hard tests for Agda, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_agda`]) --
//! language-parity wave G2.6 (found genuinely missing during the G2.5
//! closeout audit, no bespoke `languages::agda` extractor ever existed).
//! Every assertion here is against real grammar shapes confirmed via a
//! `cargo run` probe with the vendored grammar (not `node-types.json`
//! alone -- this grammar exposes zero field names anywhere, see
//! [`enforcer_memory::languages::spec::LangSpec::agda`]'s own doc
//! comment), not guessed from the baseline's C spec table.

use enforcer_memory::languages::generic::parse_agda;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_agda";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn equation_clause_is_a_function_symbol() {
    let src = "f : Nat\nf = zero\n";
    let parsed = parse_agda(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "f"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn signature_and_equation_clause_both_record_the_same_name() {
    let src = "greet : Nat -> Nat\ngreet x = x\n";
    let parsed = parse_agda(src);
    let count = parsed.symbols.iter().filter(|s| s.name == "greet").count();
    assert_eq!(count, 2, "{:?}", parsed.symbols);
}

#[test]
fn data_declaration_is_a_class_symbol() {
    let src = "data Bool : Set where\n  true : Bool\n  false : Bool\n";
    let parsed = parse_agda(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Bool"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn record_declaration_is_a_class_symbol() {
    let src = "record Point : Set where\n  field\n    x : Nat\n";
    let parsed = parse_agda(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn record_block_does_not_hide_a_following_function() {
    let src = "record Point : Set where\n  field\n    x : Nat\n\ngreet : Nat -> Nat\ngreet x = x\n";
    let parsed = parse_agda(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class),
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
fn application_inside_equation_body_is_a_call() -> TestResult {
    let src = "greet : Nat -> Nat\ngreet x = draw x\n";
    let parsed = parse_agda(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "draw")
        .ok_or("expected a draw call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("greet"), "{call:?}");
    Ok(())
}

#[test]
fn type_signature_arrow_is_not_misdetected_as_a_call() {
    let src = "f : Nat -> Nat\nf x = x\n";
    let parsed = parse_agda(src);
    assert!(
        parsed.calls.iter().all(|c| c.callee != "Nat"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn open_import_records_the_dotted_module_path() {
    let src = "open import Data.Nat\n";
    let parsed = parse_agda(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Data.Nat"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_agda("data @@@ this is not valid agda ((( ");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("Widget.agda");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_agda(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "greet"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        symbol_kind(&parsed.symbols, "Point") == Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "draw"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "Data.Nat"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
