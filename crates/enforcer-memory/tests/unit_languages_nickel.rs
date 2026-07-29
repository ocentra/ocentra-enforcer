//! Hard tests for Nickel, onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_nickel`]) -- there is
//! no bespoke `languages::nickel` extractor to prove zero-regression
//! against (Nickel has never had one in this crate), so these tests
//! assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::nickel`]'s own doc
//! comment directly: the parent-`let_binding`-climb name resolution for
//! an unfielded `fun_expr`, the left-recursive curried-application
//! `applicative` chain, and the `import`-token-as-`applicative`-child
//! shape (NOT the baseline's dead `include`).

use enforcer_memory::languages::generic::parse_nickel;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_nickel";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_named_function_binding_via_parent_let_binding_climb() {
    // Regression guard for the confirmed unfielded `fun_expr` (see
    // `LangSpec::nickel`'s own doc comment): without `nickel_quirk`
    // climbing from `let_binding` to `fun_expr`, this function would be
    // silently invisible (the generic engine's own func/method branch
    // would find no `name_field` on `fun_expr` itself at all).
    let src = "let add = fun x y => x + y in add\n";
    let parsed = parse_nickel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn plain_value_binding_produces_no_symbol() {
    let src = "let isProd = true in isProd\n";
    let parsed = parse_nickel(src);
    assert!(
        symbol_kind(&parsed.symbols, "isProd").is_none(),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_curried_call_with_correct_callee_and_args() -> TestResult {
    // Regression guard for the left-recursive curry chain (see
    // `LangSpec::nickel`'s own doc comment: `f a b` parses as
    // `applicative(t1=applicative(t1=f,t2=a),t2=b)`) -- without
    // `nickel_call_override` walking down `t1`, the callee would be
    // mis-recorded as the OUTER `applicative` node's own text (the whole
    // curried expression) rather than the plain identifier `add`.
    let src = "let add = fun x y => x + y in add 1 2\n";
    let parsed = parse_nickel(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_import_via_applicative_child_shape() -> TestResult {
    let src = "import \"other.ncl\"\n";
    let parsed = parse_nickel(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(!paths.is_empty(), "{paths:?}");
    Ok(())
}

#[test]
fn bare_include_identifier_produces_no_import_edge() {
    // Confirmed dead baseline entry, not a spec-writing gap -- see
    // `LangSpec::nickel`'s own doc comment: `include` is ALWAYS a plain
    // generic `ident` in this grammar version (there is no structural way
    // to distinguish it from an ordinary function call to a variable
    // happening to be named `include`).
    let src = "include \"foo.txt\"\n";
    let parsed = parse_nickel(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn bare_atom_produces_no_call_and_no_panic() {
    // Confirmed real grammar shape (see `nickel_call_override`'s own doc
    // comment): a non-applied atom still wraps in an `applicative` node
    // with neither `t1` nor `t2` present -- must not be mis-recorded as
    // a zero-argument call.
    let src = "42\n";
    let parsed = parse_nickel(src);
    assert!(parsed.calls.is_empty(), "{:?}", parsed.calls);
}

#[test]
fn extracts_branch_heavy_expression_without_panicking() {
    let src = "let x = if true then 1 else 2 in x\n";
    let parsed = parse_nickel(src);
    let _ = parsed;
}

#[test]
fn parses_fixture_config_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("config.ncl");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_nickel(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    assert!(!parsed.imports.is_empty(), "{:?}", parsed.imports);
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "let add = fun x y => x + y in add 1 2\n";
    let first = parse_nickel(src);
    let second = parse_nickel(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_nickel("let ( { this is not valid nickel @@@");
    let _ = parsed;
}
