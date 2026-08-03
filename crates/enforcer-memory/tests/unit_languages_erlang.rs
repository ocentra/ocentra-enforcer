//! Hard tests for Erlang, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_erlang`])
//! -- there is no bespoke `languages::erlang` extractor to prove
//! zero-regression against (Erlang has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::erlang`]'s own doc
//! comment directly: `function_clause`'s real `name`/`body` fields (no
//! quirk needed for the base case), `type_alias`'s own two-level
//! `type_name` unwrap, `import_attribute`'s own `module`-field IMPORTS
//! (deliberately NOT the baseline's own self-referential
//! `module_attribute` choice), and `call`'s own `expr`-field callee
//! reconstruction (dropping a remote call's `io:` qualifier, matching
//! the baseline's own real depth).

use enforcer_syntax::languages::generic::parse_erlang;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_erlang";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_clause_as_function() {
    let src = "helper(X) ->\n    X + 1.\n";
    let parsed = parse_erlang(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_both_clauses_of_a_multi_clause_function() {
    let src = r#"
area({circle, R}) ->
    3.14 * R * R;
area({rectangle, W, H}) ->
    W * H.
"#;
    let parsed = parse_erlang(src);
    let area_count = parsed.symbols.iter().filter(|s| s.name == "area").count();
    assert_eq!(area_count, 2, "{:?}", parsed.symbols);
}

#[test]
fn extracts_type_alias_name_past_the_arity_wrapper() {
    // `type_alias`'s own `name` field points at an intermediate
    // `type_name` wrapper node (`shape()`, including the parenthesized
    // arity suffix) -- the recorded symbol name must be the bare
    // `"shape"`, not the wrapper's own `"shape()"` text.
    let src = "-type shape() :: {circle, float()} | {rectangle, float(), float()}.\n";
    let parsed = parse_erlang(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "shape"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
    assert!(
        !parsed.symbols.iter().any(|s| s.name == "shape()"),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_attribute_module_path() {
    let src = "-import(lists, [sort/1]).\n";
    let parsed = parse_erlang(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"lists"));
}

#[test]
fn module_attribute_is_not_recorded_as_a_self_import() {
    // See `LangSpec::erlang`'s own doc comment: the baseline's own
    // `parse_generic_imports(ctx, "module_attribute")` records a
    // self-referential "this file imports its own name" ImportRef --
    // this row deliberately does NOT reproduce that.
    let src = "-module(widget).\n";
    let parsed = parse_erlang(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn ordinary_call_is_not_misdetected_as_an_import() {
    let src = "helper(X) ->\n    other(X).\n";
    let parsed = parse_erlang(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn extracts_unqualified_call_callee() -> TestResult {
    let src = "helper(X) ->\n    other(X).\n";
    let parsed = parse_erlang(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "other")
        .ok_or("expected an other call")?;
    assert_eq!(call.arg_texts, vec!["X".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn remote_call_records_only_the_unqualified_half() -> TestResult {
    // `io:format(...)` wraps an inner `call` (callee `format`) inside a
    // sibling `remote` node carrying the `io:` qualifier separately --
    // matching the baseline's own `extract_erlang_callee` (which has no
    // `"remote"`-specific arm at all), this row records only
    // `"format"`, not a reconstructed `"io:format"`.
    let src = "helper(X) ->\n    io:format(\"~p\", [X]).\n";
    let parsed = parse_erlang(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "format")
        .ok_or("expected a format call")?;
    assert_eq!(call.arg_texts.len(), 2, "{call:?}");
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "io:format"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn binary_operator_is_not_recorded_as_a_call() {
    // Confirmed via a real parse-tree dump: Erlang arithmetic/comparison
    // operators (`X + 1`, `R * R`) are a DIFFERENT node kind
    // (`binary_op_expr`), never `LangSpec::erlang::call_types`' own
    // `"call"` -- unlike Haskell's `infix`/OCaml's `infix_expression`,
    // which this crate DOES record as calls (see `LangSpec::haskell`'s/
    // `LangSpec::ocaml`'s own doc comments), Erlang genuinely has no
    // operator-as-call convention to mirror here.
    let src = "helper(X) ->\n    X + 1.\n";
    let parsed = parse_erlang(src);
    assert!(
        !parsed.calls.iter().any(|c| c.callee == "+"),
        "{:?}",
        parsed.calls
    );
}

#[test]
fn call_inside_function_clause_records_from_symbol_scope() -> TestResult {
    let src = "helper(X) ->\n    other(X).\n";
    let parsed = parse_erlang(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "other")
        .ok_or("expected an other call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("helper"), "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_erlang("-module(??? this is not valid erlang @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.erl");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_erlang(&src);
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
        parsed.symbols.iter().any(|s| s.name == "shape"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "lists"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "helper"),
        "{:?}",
        parsed.calls
    );
    assert!(
        parsed.calls.iter().any(|c| c.callee == "area"),
        "{:?}",
        parsed.calls
    );
    Ok(())
}
