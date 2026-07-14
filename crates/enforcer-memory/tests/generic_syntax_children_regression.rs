use enforcer_memory::languages::generic::{parse_ada, parse_fsharp, parse_powershell};
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
