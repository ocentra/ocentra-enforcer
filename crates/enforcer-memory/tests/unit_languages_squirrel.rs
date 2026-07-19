//! Hard tests for Squirrel onboarded directly through the generic
//! spec-table engine
//! ([`enforcer_memory::languages::generic::parse_squirrel`] --
//! language-parity wave G2.2e). Squirrel has no pre-existing bespoke
//! `languages::squirrel` extractor, so these tests assert directly
//! against the grammar's own real shape -- both the vendored
//! `tree-sitter-squirrel` grammar's own `node-types.json` and a real
//! parse tree dump (a scratch `cargo run` against a minimal crate
//! binding that grammar's vendored `parser.c` directly), which
//! confirmed this grammar is almost entirely field-free for the
//! constructs this row cares about -- see `LangSpec::squirrel`'s own
//! doc comment for the specifics -- not byte-for-byte parity with
//! prior behavior. Grammar sourced from a locally vendored
//! `tree-sitter-squirrel-local` path-dependency, NOT the published
//! `tree-sitter-squirrel` 1.0.0 crate directly -- that crate hard-pins
//! `tree-sitter = "~0.20.9"` as a normal dependency, which cannot
//! unify with this workspace's `tree-sitter = "0.25"` core at all (see
//! `crates/enforcer-memory/vendor/tree-sitter-squirrel-local/src/lib.rs`'s
//! own module doc for the full finding).

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_memory::languages::generic::parse_squirrel;
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
fn extracts_free_function_symbol_via_positional_identifier() {
    let src = "function makeDog(name) {\n    local d = 1;\n    return d;\n}\n";
    let parsed = parse_squirrel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "makeDog"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn extracts_class_with_extends_as_inherits_edge_and_method_symbols() {
    let src = r#"
class Animal {
    function speak() {
        return "...";
    }
}

class Dog extends Animal {
    function speak() {
        return "Woof";
    }
}
"#;
    let parsed = parse_squirrel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Animal"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "speak"),
        Some(&SymbolKind::Method)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
}

#[test]
fn constructor_member_is_not_extracted_as_a_method() {
    // Matches the baseline's own real (limited) depth: a Squirrel
    // class's `constructor(...) {...}` shorthand member has no inner
    // `function_declaration` node at all (confirmed via the parse-tree
    // dump), so it is correctly invisible here too -- see
    // `LangSpec::squirrel`'s own doc comment for the full finding.
    let src = "class Animal {\n    constructor(n) {\n        local x = n;\n    }\n}\n";
    let parsed = parse_squirrel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Animal"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed.symbols.iter().all(|s| s.kind != SymbolKind::Method),
        "expected no Method symbol from a constructor member, got {:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_enum_declaration_symbol() {
    let src = "enum Color {\n    Red,\n    Green,\n    Blue = 3\n}\n";
    let parsed = parse_squirrel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum)
    );
}

#[test]
fn extracts_call_edges_including_qualified_receiver() -> TestResult {
    let src = "function f(h) {\n    helper();\n    h.register();\n}\n";
    let parsed = parse_squirrel(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
    assert!(callees.contains(&"h.register"), "{callees:?}");
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "h.register")
        .ok_or("expected an h.register call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("h"));
    assert_eq!(call.receiver_hint, Some(ReceiverHint::Identifier));
    Ok(())
}

#[test]
fn extracts_call_argument_texts_from_positional_call_args() -> TestResult {
    let src = "function f() {\n    print(\"hi\", 1);\n}\n";
    let parsed = parse_squirrel(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "print")
        .ok_or("expected a print call")?;
    assert_eq!(call.arg_texts, vec!["\"hi\"".to_string(), "1".to_string()]);
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = "function f() {\n    helper();\n}\n";
    let parsed = parse_squirrel(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("f"));
    Ok(())
}

#[test]
fn extracts_branch_heavy_function_without_panicking() {
    let src = r#"
function f(x) {
    if (x > 0) {
        print("positive");
    } else {
        print("non-positive");
    }
    switch (x) {
        case 1:
            print("one");
            break;
        default:
            print("other");
    }
    while (x < 10) {
        x += 1;
    }
}
"#;
    let parsed = parse_squirrel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "f"),
        Some(&SymbolKind::Function)
    );
}

#[test]
fn extends_keyword_never_produces_a_spurious_import() {
    // `LangSpec::squirrel`'s own `import_types` is intentionally empty
    // (NOT the baseline's own dead `{"extends"}` entry) -- see its own
    // doc comment for why porting it verbatim would invent a new,
    // wrong IMPORTS edge here (this engine recurses everywhere, unlike
    // the baseline's own shallow root-only generic import scan).
    let src = "class Dog extends Animal {\n}\n";
    let parsed = parse_squirrel(src);
    assert!(parsed.imports.is_empty(), "{:?}", parsed.imports);
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src =
        "class Dog extends Animal {\n    function speak() {\n        return \"Woof\";\n    }\n}\n";
    let first = parse_squirrel(src);
    let second = parse_squirrel(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_squirrel("class ( { this is not valid squirrel @@@");
    let _ = parsed;
}

#[test]
fn real_fixture_file_parses_and_extracts_expected_symbols() {
    let src = include_str!("fixtures/memory/lang_squirrel/widget.nut");
    let parsed = parse_squirrel(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Animal"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Dog"),
        Some(&SymbolKind::Class)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum)
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "makeDog"),
        Some(&SymbolKind::Function)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
}
