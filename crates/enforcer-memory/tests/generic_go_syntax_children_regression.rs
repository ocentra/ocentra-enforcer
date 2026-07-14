use enforcer_memory::parsers::{parse_file, Language, SymbolKind};

#[test]
fn syntax_child_iteration_preserves_go_members_and_grouped_import_order() -> Result<(), String> {
    let go = parse_file(
        Language::Go,
        "package widget\n\nimport (\n    \"fmt\"\n    \"net/http\"\n)\n\ntype Base struct{}\ntype Widget struct {\n    Base\n    Name string\n}\ntype Runner interface {\n    Run()\n}\nconst Answer = 42\nvar Enabled = true\nfunc (w *Widget) Draw() {}\n",
        "widget.go",
    )
    .ok_or("Go parser did not return a parsed file")?;

    let imports: Vec<&str> = go
        .imports
        .iter()
        .map(|import| import.module_path.as_str())
        .collect();
    assert_eq!(imports, vec!["fmt", "net/http"]);
    let symbols: Vec<(&str, SymbolKind)> = go
        .symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    assert_eq!(
        symbols,
        vec![
            ("widget", SymbolKind::Module),
            ("Base", SymbolKind::Struct),
            ("Widget", SymbolKind::Struct),
            ("Runner", SymbolKind::Interface),
            ("Answer", SymbolKind::Constant),
            ("Enabled", SymbolKind::Variable),
            ("Draw", SymbolKind::Method),
        ]
    );
    let inherits: Vec<(&str, &str)> = go
        .inherits
        .iter()
        .map(|inheritance| {
            (
                inheritance.sub_name.as_str(),
                inheritance.super_name.as_str(),
            )
        })
        .collect();
    assert_eq!(inherits, vec![("Widget", "Base")]);
    let members: Vec<(&str, &str)> = go
        .defines
        .iter()
        .map(|define| (define.container_name.as_str(), define.member_name.as_str()))
        .collect();
    assert_eq!(
        members,
        vec![("Widget", "Name"), ("Runner", "Run"), ("Widget", "Draw")]
    );
    Ok(())
}
