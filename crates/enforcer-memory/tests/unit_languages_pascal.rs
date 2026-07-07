//! Hard tests for Pascal, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_pascal`])
//! -- there is no bespoke `languages::pascal` extractor to prove
//! zero-regression against (Pascal has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::pascal`]'s own doc
//! comment directly: `declType`'s own `"name"` field naming every
//! `declClass`/`declIntf`/... shape, `declClass`'s `"parent"`-tagged
//! heritage list as INHERITS, `defProc`'s out-of-line
//! `header`/`body`-split implementation (incl. dotted `TDog.Bark`
//! Method-vs-Function classification), and the `exprCall`/`exprDot`
//! call-shape split (including the parenless-call baseline gap this row
//! closes).

use enforcer_memory::languages::generic::parse_pascal;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_pascal";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

fn wrap_unit(body: &str) -> String {
    format!("unit MyUnit;\ninterface\nimplementation\n{body}\nend.\n")
}

#[test]
fn extracts_forward_declared_procedure_as_function() {
    let src = "unit MyUnit;\ninterface\nprocedure DoWork;\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "DoWork"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_function_keyword_declaration_same_as_procedure() {
    let src =
        "unit MyUnit;\ninterface\nfunction Add(a, b: Integer): Integer;\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Add"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_with_inheritance() {
    let src = r#"unit MyUnit;

interface

type
  TAnimal = class
  end;

  TDog = class(TAnimal, IFoo)
  public
    procedure Bark;
  end;

implementation

procedure TDog.Bark;
begin
end;

end.
"#;
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "TAnimal"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "TDog"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "TDog" && i.super_name == "TAnimal"),
        "{:?}",
        parsed.inherits
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "TDog" && i.super_name == "IFoo"),
        "{:?}",
        parsed.inherits
    );
}

#[test]
fn class_with_no_parent_has_no_inherits_edge() {
    let src = "unit MyUnit;\ninterface\ntype\n  TAnimal = class\n  end;\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "TAnimal"),
        Some(&SymbolKind::Class)
    );
    assert!(parsed.inherits.is_empty(), "{:?}", parsed.inherits);
}

#[test]
fn extracts_interface_declaration() {
    let src = "unit MyUnit;\ninterface\ntype\n  IFoo = interface\n    procedure Bark;\n  end;\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "IFoo"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_record_declaration_as_class() {
    let src = "unit MyUnit;\ninterface\ntype\n  TPoint = record\n    X: Integer;\n    Y: Integer;\n  end;\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "TPoint"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "TPoint" && d.member_name == "X"),
        "{:?}",
        parsed.defines
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "TPoint" && d.member_name == "Y"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn extracts_property_declaration_as_defines() {
    let src = r#"unit MyUnit;
interface
type
  TDog = class
  private
    FName: string;
  public
    property Name: string read FName write FName;
  end;
implementation
end.
"#;
    let parsed = parse_pascal(src);
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "TDog" && d.member_name == "FName"),
        "{:?}",
        parsed.defines
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "TDog" && d.member_name == "Name"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn out_of_line_method_implementation_is_method_with_defines() {
    let src = r#"unit MyUnit;

interface

type
  TDog = class
  public
    procedure Bark;
  end;

implementation

procedure TDog.Bark;
begin
  Helper(1);
end;

end.
"#;
    let parsed = parse_pascal(src);
    let bark_kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "TDog.Bark")
        .map(|s| &s.kind)
        .collect();
    assert!(
        bark_kinds.contains(&&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .defines
            .iter()
            .any(|d| d.container_name == "TDog" && d.member_name == "Bark"),
        "{:?}",
        parsed.defines
    );
}

#[test]
fn out_of_line_method_body_call_records_from_symbol_scope() -> TestResult {
    let src = r#"unit MyUnit;
interface
implementation
procedure TDog.Bark;
begin
  Helper(1);
end;
end.
"#;
    let parsed = parse_pascal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Helper")
        .ok_or("expected a Helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("TDog.Bark"));
    Ok(())
}

#[test]
fn plain_top_level_implementation_is_function_not_method() {
    let src = &wrap_unit("procedure P;\nbegin\nend;");
    let parsed = parse_pascal(src);
    let kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "P")
        .map(|s| &s.kind)
        .collect();
    assert!(
        kinds.contains(&&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_with_parens_and_args() -> TestResult {
    let src = &wrap_unit("procedure P;\nbegin\n  Helper(1, 2);\nend;");
    let parsed = parse_pascal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Helper")
        .ok_or("expected a Helper call")?;
    assert_eq!(
        call.arg_texts,
        vec!["1".to_string(), "2".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_parenless_dot_call_as_call_edge() -> TestResult {
    // Regression guard for the baseline-gap finding (see
    // `LangSpec::pascal`'s own doc comment): `Obj.Draw;` with no parens
    // at all must still surface as a CALLS edge.
    let src = &wrap_unit("procedure P;\nbegin\n  Obj.Draw;\nend;");
    let parsed = parse_pascal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Obj.Draw")
        .ok_or("expected an Obj.Draw call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("Obj"));
    Ok(())
}

#[test]
fn extracts_dotted_callee_inside_parenthesized_call() -> TestResult {
    let src = &wrap_unit("procedure P;\nbegin\n  Exception.Create('bad');\nend;");
    let parsed = parse_pascal(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "Exception.Create")
        .ok_or("expected an Exception.Create call")?;
    assert_eq!(call.arg_texts, vec!["'bad'".to_string()], "{call:?}");
    Ok(())
}

#[test]
fn extracts_uses_clause_with_multiple_units_as_imports() {
    let src = "unit MyUnit;\ninterface\nuses SysUtils, Classes, Foo.Bar;\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"SysUtils"), "{paths:?}");
    assert!(paths.contains(&"Classes"), "{paths:?}");
    assert!(paths.contains(&"Foo.Bar"), "{paths:?}");
}

#[test]
fn extracts_unit_name_as_module_symbol() {
    let src = "unit MyUnit;\ninterface\nimplementation\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyUnit"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_program_name_as_module_symbol() {
    let src = "program MyProg;\nbegin\nend.\n";
    let parsed = parse_pascal(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "MyProg"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_branch_heavy_procedure_without_panicking() {
    let src = &wrap_unit(
        r#"procedure P;
begin
  if x > 0 then
    y := 1
  else
    y := 2;
  while x > 0 do
    x := x - 1;
  for i := 0 to 10 do
  begin
  end;
  repeat
    x := x - 1;
  until x = 0;
  try
    x := 1;
  except
    y := 2;
  end;
  case x of
    1: y := 1;
  end;
end;"#,
    );
    let parsed = parse_pascal(src);
    let kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "P")
        .map(|s| &s.kind)
        .collect();
    assert!(!kinds.is_empty());
}

#[test]
fn parses_fixture_widget_without_panicking() -> TestResult {
    let path = Path::new(FIXTURE_DIR).join("widget.pas");
    let src = fs::read_to_string(&path)?;
    let parsed = parse_pascal(&src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Module),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "TDog"),
        Some(&SymbolKind::Class)
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "TDog" && i.super_name == "TAnimal"),
        "{:?}",
        parsed.inherits
    );
    let bark_kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "TDog.Bark")
        .map(|s| &s.kind)
        .collect();
    assert!(
        bark_kinds.contains(&&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
    Ok(())
}

#[test]
fn incremental_reindex_is_deterministic() {
    let src = &wrap_unit("procedure P;\nbegin\n  Helper();\nend;");
    let first = parse_pascal(src);
    let second = parse_pascal(src);
    assert_eq!(first, second);
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_pascal("unit ( { this is not valid pascal @@@");
    let _ = parsed;
}
