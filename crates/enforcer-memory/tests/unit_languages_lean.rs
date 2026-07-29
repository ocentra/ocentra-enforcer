//! Hard tests for Lean, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_lean`])
//! -- there is no bespoke `languages::lean` extractor to prove
//! zero-regression against (Lean has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::lean`]'s own doc comment
//! directly: `definition`'s own real, direct `name` field covering
//! def/theorem/instance/abbrev, `structure`/`inductive`'s own SEPARATE
//! top-level node kind (not a `definition` variant -- the corrected
//! finding that const's own doc comment documents), `application`'s own
//! real `name`/`arguments` fields (no quirk needed), and `import`'s own
//! repeated `[module]`-field dotted-path reconstruction.

use enforcer_memory::languages::generic::parse_lean;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_lean";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_def_as_function() {
    let src = "def helper (x : Nat) : Nat := x + 1\n";
    let parsed = parse_lean(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_theorem_as_function() {
    // Same `definition` node kind as `def` -- see `LangSpec::lean`'s own
    // doc comment.
    let src = "theorem area_thm (x : Nat) : x = x := rfl\n";
    let parsed = parse_lean(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "area_thm"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_instance_as_function() {
    // `instance`-form `definition` has NO `kind` field value at all
    // (confirmed via `node-types.json`) but still resolves through the
    // ordinary generic path since its own `name` field is real -- unless
    // the instance is anonymous, which this fixture avoids.
    let src = "instance helperInst : Inhabited Nat := ⟨0⟩\n";
    let parsed = parse_lean(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helperInst"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_structure_as_a_separate_class_symbol() {
    // See `LangSpec::lean`'s own doc comment: `structure` is its OWN
    // top-level node kind, not a `definition` variant.
    let src = "structure Point where\n  x : Nat\n  y : Nat\n";
    let parsed = parse_lean(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_inductive_as_a_separate_class_symbol() {
    let src = "inductive Color where\n  | Red\n  | Green\n";
    let parsed = parse_lean(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn application_call_uses_the_real_name_arguments_fields() -> TestResult {
    let src = "def area (shape : Nat) : Nat := helper shape\n";
    let parsed = parse_lean(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("area"), "{call:?}");
    Ok(())
}

#[test]
fn import_records_dotted_module_path() -> TestResult {
    let src = "import Mathlib.Data.Nat.Basic\n";
    let parsed = parse_lean(src);
    let import = parsed
        .imports
        .iter()
        .find(|i| i.module_path == "Mathlib.Data.Nat.Basic")
        .ok_or("expected a Mathlib.Data.Nat.Basic import")?;
    assert!(import.line > 0, "{import:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_lean("def ??? this is not valid lean @@@");
    let _ = parsed;
}

#[test]
fn malformed_unicode_source_does_not_panic() {
    let source = "3\u{1b}⤸:\u{b}{`\"\rLѭ¡+\u{46c64}\u{cf078}\u{b}{\u{85a78}𠟄Ѩ\u{a1447}.\0\"\\Ð&'%Y\t'`*\u{1b}?\u{c06a9}\u{7661a};/\u{feff}`¬\u{7f}%E\0s\r\u{76b7c}\u{7f}\u{be2a3}-\r娠\r\u{6c6bd}\"=\u{9d471}-\r\u{1b}須\u{b}Ⱥ0u\u{9f52b}M";
    let parsed = parse_lean(source);
    assert!(parsed.symbols.is_empty(), "{:?}", parsed.symbols);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.lean");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_lean(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "area"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Point"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "Mathlib.Data.Nat.Basic"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
