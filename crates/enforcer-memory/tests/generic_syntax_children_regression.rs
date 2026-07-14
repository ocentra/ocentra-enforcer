use enforcer_memory::languages::generic::{
    parse_ada, parse_apex, parse_cairo, parse_fsharp, parse_julia, parse_powershell,
};
use enforcer_memory::parsers::SymbolKind;

#[test]
fn syntax_child_iteration_preserves_generic_language_scope_and_order() {
    let powershell = parse_powershell("class Dog : Animal {\n    [void] Speak() {\n    }\n}\n");
    let power_shell_symbols: Vec<(&str, SymbolKind)> = powershell
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    assert_eq!(
        power_shell_symbols,
        vec![("Dog", SymbolKind::Class), ("Speak", SymbolKind::Method)]
    );
    assert_eq!(powershell.inherits.len(), 1);
    assert_eq!(powershell.inherits[0].sub_name, "Dog");
    assert_eq!(powershell.inherits[0].super_name, "Animal");

    let fsharp = parse_fsharp("let helper value =\n    printfn \"helper\"\n");
    assert_eq!(fsharp.symbols.len(), 1);
    assert_eq!(fsharp.symbols[0].name, "helper");
    assert_eq!(fsharp.symbols[0].kind, SymbolKind::Function);

    let ada = parse_ada(
        "package body Widget is\n   procedure Draw is\n   begin\n      null;\n   end Draw;\nend Widget;\n",
    );
    let ada_symbols: Vec<(&str, SymbolKind)> = ada
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    assert_eq!(
        ada_symbols,
        vec![
            ("Widget", SymbolKind::Class),
            ("Draw", SymbolKind::Function)
        ]
    );
    assert_eq!(ada.defines.len(), 1);
    assert_eq!(ada.defines[0].container_name, "Widget");
    assert_eq!(ada.defines[0].member_name, "Draw");
}

#[test]
fn syntax_child_iteration_preserves_apex_interface_and_field_traversal() {
    let apex = parse_apex(
        "public interface Sub extends Base {\n}\npublic class Widget {\n    public String name;\n}\n",
    );
    let apex_symbols: Vec<(&str, SymbolKind)> = apex
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    assert_eq!(
        apex_symbols,
        vec![
            ("Sub", SymbolKind::Interface),
            ("Widget", SymbolKind::Class)
        ]
    );
    assert_eq!(apex.inherits.len(), 1);
    assert_eq!(apex.inherits[0].sub_name, "Sub");
    assert_eq!(apex.inherits[0].super_name, "Base");
    assert_eq!(apex.defines.len(), 1);
    assert_eq!(apex.defines[0].container_name, "Widget");
    assert_eq!(apex.defines[0].member_name, "name");
}

#[test]
fn syntax_child_iteration_preserves_julia_function_scope() {
    let julia = parse_julia("function draw(value)\n    helper(value)\nend\n");
    assert_eq!(julia.symbols.len(), 1);
    assert_eq!(julia.symbols[0].name, "draw");
    assert_eq!(julia.symbols[0].kind, SymbolKind::Function);
    assert_eq!(julia.calls.len(), 1);
    assert_eq!(julia.calls[0].callee, "helper");
    assert_eq!(julia.calls[0].from_symbol.as_deref(), Some("draw"));
}

#[test]
fn syntax_child_iteration_preserves_cairo_call_arguments_and_scope() {
    let cairo = parse_cairo("fn main() {\n    helper(1, 2, 3);\n}\n");
    assert!(cairo
        .symbols
        .iter()
        .any(|symbol| symbol.name == "main" && symbol.kind == SymbolKind::Function));
    assert!(cairo.calls.iter().any(|call| {
        call.callee == "helper"
            && call.from_symbol.as_deref() == Some("main")
            && call.arg_texts == vec!["1", "2", "3"]
    }));
}
