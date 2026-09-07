//! Hard tests for Apex, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_apex`])
//! -- there is no bespoke `languages::apex` extractor to prove
//! zero-regression against (Apex has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_syntax::languages::spec::LangSpec::apex`]'s own doc
//! comment directly: class/interface/enum heritage (this grammar is
//! Java-shaped throughout), field DEFINES, annotations as DECORATES, and
//! full `method_invocation` callee reconstruction.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_syntax::languages::generic::parse_apex;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_apex";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_and_method_symbols() {
    let src = r#"
public class Widget {
    public String draw() {
        return 'widget';
    }
}
"#;
    let parsed = parse_apex(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "draw"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_interface_and_enum_symbols() {
    let src = r#"
public interface Drawable {
    String draw();
}

public enum Color {
    RED,
    GREEN
}
"#;
    let parsed = parse_apex(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Drawable"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Color"),
        Some(&SymbolKind::Enum),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_class_superclass_as_inherits() -> TestResult {
    let src = r#"
public class Widget extends BaseWidget {
}
"#;
    let parsed = parse_apex(src);
    let inherit = parsed
        .inherits
        .iter()
        .find(|i| i.sub_name == "Widget")
        .ok_or("expected Widget to inherit from something")?;
    assert_eq!(inherit.super_name, "BaseWidget", "{inherit:?}");
    Ok(())
}

#[test]
fn extracts_class_interfaces_as_implements() -> TestResult {
    let src = r#"
public class Widget implements Drawable {
}
"#;
    let parsed = parse_apex(src);
    let implements = parsed
        .implements
        .iter()
        .find(|i| i.type_name == "Widget")
        .ok_or("expected Widget to implement something")?;
    assert_eq!(implements.trait_name, "Drawable", "{implements:?}");
    Ok(())
}

#[test]
fn extracts_interface_extends_as_inherits() -> TestResult {
    let src = r#"
public interface Sub extends Base {
}
"#;
    let parsed = parse_apex(src);
    let inherit = parsed
        .inherits
        .iter()
        .find(|i| i.sub_name == "Sub")
        .ok_or("expected Sub to inherit from something")?;
    assert_eq!(inherit.super_name, "Base", "{inherit:?}");
    Ok(())
}

#[test]
fn class_with_no_heritage_has_no_inherits_or_implements() {
    let src = r#"
public class Standalone {
}
"#;
    let parsed = parse_apex(src);
    assert!(parsed.inherits.is_empty(), "{:?}", parsed.inherits);
    assert!(parsed.implements.is_empty(), "{:?}", parsed.implements);
}

#[test]
fn extracts_field_defines_inside_class_body() {
    let src = r#"
public class Widget {
    public String name;
    public Integer age;
}
"#;
    let parsed = parse_apex(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "name")));
    assert!(defines.contains(&("Widget", "age")));
}

#[test]
fn extracts_method_defines_inside_class_body() {
    let src = r#"
public class Widget {
    public void draw() {
    }

    public void resize() {
    }
}
"#;
    let parsed = parse_apex(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")));
    assert!(defines.contains(&("Widget", "resize")));
}

#[test]
fn extracts_annotation_as_decorates() -> TestResult {
    let src = r#"
@RestResource(urlMapping='/widgets/*')
global class WidgetService {
}
"#;
    let parsed = parse_apex(src);
    let decorate = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "WidgetService")
        .ok_or("expected a decorator on WidgetService")?;
    assert_eq!(decorate.decorator_name, "RestResource", "{decorate:?}");
    Ok(())
}

#[test]
fn extracts_receiver_qualified_call() -> TestResult {
    let src = r#"
public class Widget {
    public void draw() {
        helper.render();
    }
}
"#;
    let parsed = parse_apex(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper.render")
        .ok_or("expected a helper.render call")?;
    assert_eq!(call.receiver_text.as_deref(), Some("helper"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::Identifier),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn unqualified_call_has_no_receiver() -> TestResult {
    let src = r#"
public class Widget {
    public void draw() {
        helper();
    }
}
"#;
    let parsed = parse_apex(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.receiver_text, None, "{call:?}");
    Ok(())
}

#[test]
fn call_with_arguments_records_arg_texts() -> TestResult {
    let src = r#"
public class Widget {
    public void draw() {
        helper.resize(10, 20);
    }
}
"#;
    let parsed = parse_apex(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper.resize")
        .ok_or("expected a helper.resize call")?;
    assert_eq!(
        call.arg_texts,
        vec!["10".to_string(), "20".to_string()],
        "{call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_method_records_from_symbol_scope() -> TestResult {
    let src = r#"
public class Widget {
    public void render() {
        helper();
    }
}
"#;
    let parsed = parse_apex(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_apex("class ( { this is not valid apex @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("Widget.cls");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_apex(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "draw"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Widget" && i.super_name == "BaseWidget"),
        "{:?}",
        parsed.inherits
    );
    assert!(
        parsed
            .implements
            .iter()
            .any(|i| i.type_name == "Widget" && i.trait_name == "Drawable"),
        "{:?}",
        parsed.implements
    );
    Ok(())
}
