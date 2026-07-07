//! Hard tests for D, onboarded directly through the generic spec-table
//! engine ([`enforcer_memory::languages::generic::parse_d`]) -- there is
//! no bespoke `languages::d` extractor to prove zero-regression against
//! (D has never had one in this crate), so these tests assert against
//! the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::d`]'s own doc comment
//! directly: free functions, a class with a constructor/method and
//! multi-base heritage, a struct with fields, a module declaration, an
//! import, ordinary calls, and a `new`-expression constructor call.

use enforcer_memory::languages::generic::parse_d;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_d";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_free_function() {
    let src = r#"
int helper() {
    return 1;
}
"#;
    let parsed = parse_d(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_with_method_and_defines_edge() {
    let src = r#"
class Animal {
    void speak() {
    }
}
"#;
    let parsed = parse_d(src);
    let kinds: Vec<(&str, SymbolKind)> = parsed
        .symbols
        .iter()
        .map(|s| (s.name.as_str(), s.kind))
        .collect();
    assert!(kinds.contains(&("Animal", SymbolKind::Class)), "{kinds:?}");
    assert!(kinds.contains(&("speak", SymbolKind::Method)), "{kinds:?}");
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "Animal" && d.member_name == "speak"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_multi_base_heritage_as_inherits() {
    let src = "class Dog : Animal, Serializable {\n}\n";
    let parsed = parse_d(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Dog", "Animal")), "{inherits:?}");
    assert!(inherits.contains(&("Dog", "Serializable")), "{inherits:?}");
}

#[test]
fn extracts_struct_with_field_defines() {
    let src = r#"
struct Point {
    int x;
    int y;
}
"#;
    let parsed = parse_d(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Point", "x")), "{defines:?}");
    assert!(defines.contains(&("Point", "y")), "{defines:?}");
}

#[test]
fn extracts_interface() {
    let src = "interface Serializable {\n    string serialize();\n}\n";
    let parsed = parse_d(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Serializable"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_module_declaration_as_module_symbol() {
    let src = "module myapp.widgets;\n";
    let parsed = parse_d(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "myapp.widgets"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_import_declaration() {
    let src = "import std.stdio;\n";
    let parsed = parse_d(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"std.stdio"), "{paths:?}");
}

#[test]
fn extracts_call_inside_function_with_scope() -> TestResult {
    let src = r#"
int helper() {
    return add(1, 2);
}

int add(int a, int b) {
    return a + b;
}
"#;
    let parsed = parse_d(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "add")
        .ok_or("expected an add call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("helper"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_new_expression_constructor_call() -> TestResult {
    let src = r#"
class Dog {
}

void main() {
    auto d = new Dog();
}
"#;
    let parsed = parse_d(src);
    assert!(
        parsed.calls.iter().any(|c| c.callee.contains("Dog")),
        "{:?}",
        parsed.calls
    );
    Ok(())
}

#[test]
fn constructor_is_recorded_as_nameless_method() {
    let src = r#"
class Animal {
    this(string name) {
    }
}
"#;
    let parsed = parse_d(src);
    assert!(
        parsed
            .symbols
            .iter()
            .any(|s| s.kind == SymbolKind::Method && s.name.is_empty()),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_d("class ((( this is not valid d @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.d");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_d(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Animal"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Dog"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Point"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.imports.iter().any(|i| i.module_path == "std.stdio"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Dog" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
    Ok(())
}
