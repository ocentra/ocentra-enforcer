//! Hard tests for LLVM IR, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_llvm_ir`]) -- there is
//! no bespoke `languages::llvm_ir` extractor to prove zero-regression
//! against (LLVM IR has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::llvm_ir`]'s own doc
//! comment directly: the positional `fn_define`/`declare` name
//! extraction [`enforcer_memory::languages::generic::llvm_quirk`]
//! performs (neither node has a `"name"` field of its own), the
//! leading `@`-sigil-stripping convention, and the real
//! `instruction_call`/`instruction_invoke` node kinds (NOT the
//! baseline's phantom bare `"call"`/`"invoke"`).

use enforcer_memory::languages::generic::parse_llvm_ir;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_llvmir";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_fn_define_with_sigil_stripped_name() {
    let src = "define i32 @main() {\nentry:\n  ret i32 0\n}\n";
    let parsed = parse_llvm_ir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "main"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_declare_with_sigil_stripped_name() {
    let src = "declare i32 @foo(i32)\n";
    let parsed = parse_llvm_ir(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "foo"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_via_real_instruction_call_node() -> TestResult {
    let src = "define i32 @main() {\nentry:\n  %1 = call i32 @foo(i32 42)\n  ret i32 %1\n}\n";
    let parsed = parse_llvm_ir(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.contains("foo"))
        .ok_or("expected a call to foo")?;
    assert_eq!(call.from_symbol.as_deref(), Some("main"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_call_inside_body_field_via_manual_quirk_recursion() {
    // Regression guard: `fn_define`'s own `body` field must be
    // recursed into manually by `llvm_quirk` (the generic func_types
    // branch never reaches it since `fn_define` has no `"name"` field
    // for the generic path to succeed on first).
    let src = "define i32 @main() {\nentry:\n  %1 = call i32 @helper()\n  ret i32 %1\n}\n";
    let parsed = parse_llvm_ir(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee.contains("helper")),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn parses_fixture_example_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("example.ll");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_llvm_ir(&src);
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
    assert!(
        parsed.calls.iter().any(|c| c.callee.contains("foo")),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "define i32 @main() {\nentry:\n  ret i32 0\n}\n";
    let first = parse_llvm_ir(src);
    let second = parse_llvm_ir(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_llvm_ir("define ( { this is not valid llvm ir @@@");
    let _ = parsed;
}
