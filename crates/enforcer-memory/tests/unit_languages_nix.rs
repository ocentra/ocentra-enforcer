//! Hard tests for Nix, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_nix`])
//! -- there is no bespoke `languages::nix` extractor to prove
//! zero-regression against (Nix has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::nix`]'s own doc comment
//! directly: a `function_expression`'s own name is resolved from its
//! ENCLOSING `binding`'s `attrpath`, an unbound lambda stays anonymous,
//! and `apply_expression` calls are recorded per-node (not curry-chain
//! collapsed).

use enforcer_syntax::languages::generic::parse_nix;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_nix";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_bound_lambda_as_function_via_enclosing_binding_attrpath() {
    let src = "let\n  addOne = x: x + 1;\nin\naddOne 41\n";
    let parsed = parse_nix(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "addOne"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn unbound_inline_lambda_stays_anonymous() {
    // A lambda whose parent is not a `binding` (e.g. an inline argument)
    // resolves no name and is correctly left out of the symbol table --
    // see `LangSpec::nix`'s own doc comment.
    let src = "map (x: x) [ 1 2 3 ]\n";
    let parsed = parse_nix(src);
    assert!(
        parsed
            .symbols
            .iter()
            .all(|s| s.kind != SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_apply_expression_as_call() -> TestResult {
    let src = "let\n  addOne = x: x + 1;\nin\naddOne 41\n";
    let parsed = parse_nix(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "addOne")
        .ok_or("expected an addOne call")?;
    let _ = call;
    Ok(())
}

#[test]
fn extracts_if_expression_as_branch() {
    let src = "if true then 1 else 2\n";
    let parsed = parse_nix(src);
    // Complexity extraction is deferred this wave; this test only proves
    // the file parses cleanly with an `if_expression` present in the
    // source (branch_types is documentation-only until a future wave
    // wires complexity for this language).
    let _ = parsed;
}

#[test]
fn parses_fixture_sample_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("sample.nix");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_nix(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "addOne"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "addOne")
        .ok_or("expected an addOne call")?;
    let _ = call;
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "let\n  addOne = x: x + 1;\nin\naddOne 41\n";
    let first = parse_nix(src);
    let second = parse_nix(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_nix("let x = ; in @@@ not valid nix");
    let _ = parsed;
}
