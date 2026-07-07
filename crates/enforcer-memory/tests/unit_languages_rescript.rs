//! Hard tests for ReScript onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_rescript`] --
//! language-parity wave G2.2e). ReScript has no pre-existing bespoke
//! `languages::rescript` extractor, so these tests assert directly
//! against the grammar's own real shape -- both the `arborium-rescript`
//! crate's own `node-types.json` and a real parse tree dump (a scratch
//! `cargo run` against a minimal crate depending on that grammar's
//! vendored `parser.c` directly), which confirmed the SAME real
//! name-resolution gap the baseline's own `cbm_resolve_func_name`
//! already has a dedicated case for -- see `LangSpec::rescript`'s own
//! doc comment for the specifics -- not byte-for-byte parity with
//! prior behavior.

use enforcer_memory::languages::generic::parse_rescript;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_let_bound_function_symbol_via_parent_pattern() {
    // `function`'s own name lives on the enclosing `let_binding`'s
    // `pattern` field, not a field of `function` itself -- see
    // `LangSpec::rescript`'s own doc comment.
    let src = "let add = (a, b) => {\n  a + b\n}\n";
    let parsed = parse_rescript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn plain_value_binding_is_not_a_function_symbol() {
    // `let x = 42`'s `let_binding.body` is a bare `number`, never
    // `function` -- confirmed via the parse-tree dump, this natural
    // exclusion needs no extra check in the quirk itself.
    let src = "let x = 42\n";
    let parsed = parse_rescript(src);
    assert!(
        symbol_kind(&parsed.symbols, "x").is_none(),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_module_and_type_declaration_symbols() {
    let src =
        "module Point = {\n  type t = {x: int, y: int}\n}\n\ntype color = Red | Green | Blue\n";
    let parsed = parse_rescript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "color"),
        Some(&SymbolKind::TypeAlias)
    );
    // `type t` nested inside `module Point`'s own body is still
    // visited (the class quirk recurses into the binding's own
    // `definition`/`body`).
    assert_eq!(
        symbol_kind(&parsed.symbols, "t"),
        Some(&SymbolKind::TypeAlias)
    );
}

#[test]
fn extracts_open_and_include_statements_as_imports() {
    let src = "open Belt\ninclude MyModule\n";
    let parsed = parse_rescript(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Belt"), "{paths:?}");
    assert!(paths.contains(&"MyModule"), "{paths:?}");
}

#[test]
fn extracts_call_edges_including_qualified_module_path_callee() -> TestResult {
    let src = "let helper = (x) => {\n  Js.log(\"hi\")\n  x\n}\n\nlet result = helper(1)\n";
    let parsed = parse_rescript(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"Js.log"), "{callees:?}");
    assert!(callees.contains(&"helper"), "{callees:?}");
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Js.log")
        .ok_or("expected a Js.log call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("helper"));
    Ok(())
}

#[test]
fn extracts_decorator_as_decorates_edge() {
    let src = "@react.component\nlet make = () => {\n  React.string(\"hi\")\n}\n";
    let parsed = parse_rescript(src);
    assert!(
        parsed
            .decorates
            .iter()
            .any(|d| d.target_name == "make" && d.decorator_name == "@react.component"),
        "{:?}",
        parsed.decorates
    );
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = "let f = (x) => {\n  if x > 0 {\n    Js.log(\"positive\")\n  } else {\n    Js.log(\"non-positive\")\n  }\n  switch x {\n  | 0 => Js.log(\"zero\")\n  | _ => Js.log(\"other\")\n  }\n}\n";
    let parsed = parse_rescript(src);
    assert!(symbol_kind(&parsed.symbols, "f").is_some());
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = "open Belt\n\nlet add = (a, b) => {\n  a + b\n}\n\nlet result = add(1, 2)\n";
    let first = parse_rescript(src);
    let second = parse_rescript(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_rescript("let x = @@@ this is not valid rescript");
    let _ = parsed;
}

#[test]
fn real_fixture_file_parses_and_extracts_expected_symbols() {
    let src = include_str!("fixtures/memory/lang_rescript/Widget.res");
    let parsed = parse_rescript(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "add"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "color"),
        Some(&SymbolKind::TypeAlias)
    );
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Belt"), "{paths:?}");
    assert!(paths.contains(&"MyModule"), "{paths:?}");
    assert!(
        parsed
            .decorates
            .iter()
            .any(|d| d.target_name == "make" && d.decorator_name == "@react.component"),
        "{:?}",
        parsed.decorates
    );
}
