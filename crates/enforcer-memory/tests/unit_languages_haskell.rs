//! Hard tests for Haskell, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_haskell`])
//! -- there is no bespoke `languages::haskell` extractor to prove
//! zero-regression against (Haskell has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::haskell`]'s own doc
//! comment directly: symbol kinds (`function`/`bind` as Function,
//! `data_type`/`class`/`instance` as Class), `import`'s `module`-field
//! IMPORTS, curried `apply` callee-head recovery, and `infix` operator-
//! as-callee reconstruction.

use enforcer_memory::languages::generic::parse_haskell;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_haskell";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_symbol() {
    let src = "helper x = x + 1\n";
    let parsed = parse_haskell(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn signature_does_not_double_the_function_symbol() {
    // `helper :: Int -> Int` is a type annotation, syntactically
    // adjacent to but distinct from the actual `helper x = ...`
    // definition -- see `LangSpec::haskell`'s own doc comment for why
    // this row's `func_types` deliberately omits `signature`.
    let src = r#"
helper :: Int -> Int
helper x = x + 1
"#;
    let parsed = parse_haskell(src);
    let helper_count = parsed.symbols.iter().filter(|s| s.name == "helper").count();
    assert_eq!(helper_count, 1, "{:?}", parsed.symbols);
}

#[test]
fn extracts_data_type_as_class() {
    let src = "data Shape = Circle Double | Rectangle Double Double\n";
    let parsed = parse_haskell(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Shape"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_and_instance_as_class() {
    let src = r#"
class Greet a where
  greet :: a -> String

instance Greet Shape where
  greet x = "hi"
"#;
    let parsed = parse_haskell(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Greet"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_newtype_as_class() {
    let src = "newtype Age = Age Int\n";
    let parsed = parse_haskell(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Age"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_module_path() {
    let src = "import Data.List (sort)\n";
    let parsed = parse_haskell(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Data.List"), "{paths:?}");
}

#[test]
fn extracts_qualified_import_module_path() {
    let src = "import qualified Data.Map as Map\n";
    let parsed = parse_haskell(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Data.Map"), "{paths:?}");
}

#[test]
fn ordinary_definition_is_not_misdetected_as_an_import() {
    let src = "helper x = x + 1\n";
    let parsed = parse_haskell(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_curried_apply_callee() -> TestResult {
    let src = r#"
helper x = x + 1
draw s = helper 3
"#;
    let parsed = parse_haskell(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.arg_texts, vec!["3".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_multi_arg_curried_apply_callee() -> TestResult {
    // `f a b` nests as `apply(function=apply(function=f,argument=a),
    // argument=b)` -- the head recovery must descend past BOTH curry
    // levels to find the real callee `f`, and `arg_texts` must list
    // both arguments in written order.
    let src = r#"
combine a b = a + b
draw = combine 1 2
"#;
    let parsed = parse_haskell(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "combine")
        .ok_or("expected a combine call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_infix_operator_as_callee() -> TestResult {
    let src = "helper x = x + 1\n";
    let parsed = parse_haskell(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "+")
        .ok_or("expected a + call")?;
    assert_eq!(
        call.arg_texts,
        vec!["x".to_string(), "1".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn nested_infix_chain_records_both_operators() {
    // `pi * r * r` right-associates as
    // `infix(left=pi,op=*,right=infix(left=r,op=*,right=r))` -- both
    // levels are independently-walked `infix` nodes, so both should be
    // recorded (not just the outer one).
    let src = "area r = pi * r * r\n";
    let parsed = parse_haskell(src);
    let star_calls: Vec<_> = parsed.calls.iter().filter(|c| c.callee == "*").collect();
    assert_eq!(star_calls.len(), 2, "{:?}", parsed.calls);
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
helper x = x + 1
draw s = helper 3
"#;
    let parsed = parse_haskell(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_haskell("module ??? where this is not valid haskell @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.hs");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_haskell(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Shape"),
        "{:?}",
        parsed.symbols
    );
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
        parsed.imports.iter().any(|i| i.module_path == "Data.List"),
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
