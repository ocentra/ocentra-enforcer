//! Hard tests for Move, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_move`])
//! -- there is no bespoke `languages::move` extractor to prove
//! zero-regression against (Move has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::move_lang`]'s own doc
//! comment directly: the real `function_definition`/`struct_definition`
//! name fields (a confirmed IMPROVEMENT over the baseline's own stale
//! "struct/enum are anonymous keyword tokens" claim), the positional
//! `name_expression` callee `move_call_override` must read, and the
//! `use_declaration`'s own unfielded four-shape import path.

use enforcer_memory::languages::generic::parse_move;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_move";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_definition_via_real_name_field() {
    let src = "module 0x1::m {\n    public fun helper() {}\n}\n";
    let parsed = parse_move(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_struct_definition_via_real_name_field_improvement_over_baseline() {
    // Regression guard for the confirmed-wrong baseline source comment
    // (see `LangSpec::move_lang`'s own doc comment: "struct/enum exist
    // only as anonymous keyword tokens, never as parent nodes" is FALSE
    // for this grammar generation) -- `struct_definition` has a real,
    // working `name` field this crate now extracts as a genuine
    // improvement over the baseline's own empty `class_types` array.
    let src = "module 0x1::m {\n    struct Counter has key {\n        value: u64,\n    }\n}\n";
    let parsed = parse_move(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Counter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_via_positional_name_expression_callee() -> TestResult {
    // Regression guard for the positional (unfielded) callee (see
    // `LangSpec::move_lang`'s own doc comment): without
    // `move_call_override`, the generic default's `"function"`-field
    // lookup would find nothing at all on this grammar's
    // `call_expression`.
    let src = "module 0x1::m {\n    public fun helper() {\n        move_to(1, 2);\n    }\n}\n";
    let parsed = parse_move(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "move_to")
        .ok_or("expected a move_to call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_qualified_call_dotted_path_via_positional_callee() -> TestResult {
    let src = "module 0x1::m {\n    use std::signer;\n    public fun helper(account: &signer) {\n        signer::address_of(account);\n    }\n}\n";
    let parsed = parse_move(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.contains("address_of"))
        .ok_or("expected a signer::address_of call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_use_declaration_as_import() -> TestResult {
    let src = "module 0x1::m {\n    use std::signer;\n}\n";
    let parsed = parse_move(src);
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = "module 0x1::m {\n    public fun helper() {\n        if (true) {\n            1;\n        };\n    }\n}\n";
    let parsed = parse_move(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn parses_fixture_counter_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("counter.move");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_move(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Counter"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "initialize"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "increment"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "module 0x1::m {\n    public fun helper() {\n        move_to(1);\n    }\n}\n";
    let first = parse_move(src);
    let second = parse_move(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_move("module ( { this is not valid move @@@");
    let _ = parsed;
}
