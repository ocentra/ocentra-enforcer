//! Hard tests for Objective-C, onboarded directly through the generic
//! spec-table engine ([`enforcer_syntax::languages::generic::parse_objc`])
//! -- there is no bespoke `languages::objc` extractor to prove
//! zero-regression against (Objective-C has never had one in this
//! crate), so these tests assert against the grammar-shape ground truth
//! recorded in [`enforcer_syntax::languages::spec::LangSpec::objc`]'s
//! own doc comment directly: `class_interface`/`class_implementation`
//! positional naming + `superclass`-field INHERITS, multi-keyword
//! selector naming for both method definitions/declarations AND
//! `message_expression` calls (verifying both sides agree on the same
//! joined `"setName:withAge:"` text), plain-C-family delegation
//! (`function_definition`/`struct_specifier`/`preproc_include`), and
//! DEFINES-scoped method bodies.

use enforcer_domain::memory_types::ReceiverHint;
use enforcer_syntax::languages::generic::parse_objc;
use enforcer_syntax::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_objc";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_syntax::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_class_interface_and_implementation_symbols() {
    let src = r#"
@interface Widget : NSObject
- (void)draw;
@end

@implementation Widget
- (void)draw {
}
@end
"#;
    let parsed = parse_objc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
    // "draw" is defined twice (interface declaration + implementation
    // definition), both must classify as Method.
    let draw_kinds: Vec<&SymbolKind> = parsed
        .symbols
        .iter()
        .filter(|s| s.name == "draw")
        .map(|s| &s.kind)
        .collect();
    assert!(!draw_kinds.is_empty(), "{:?}", parsed.symbols);
    assert!(
        draw_kinds.iter().all(|k| **k == SymbolKind::Method),
        "{draw_kinds:?}"
    );
}

#[test]
fn extracts_superclass_as_inherits() {
    let src = r#"
@interface Widget : Animal
@end
"#;
    let parsed = parse_objc(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Animal")));
}

#[test]
fn extracts_multi_keyword_selector_name() {
    let src = r#"
@implementation Widget
- (void)setName:(NSString *)name withAge:(int)age {
}
@end
"#;
    let parsed = parse_objc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "setName:withAge:"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_zero_argument_selector_with_no_trailing_colon() {
    let src = r#"
@implementation Widget
- (void)bark {
}
@end
"#;
    let parsed = parse_objc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "bark"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_method_defines_inside_implementation() {
    let src = r#"
@implementation Widget
- (void)draw {
}
- (void)resize {
}
@end
"#;
    let parsed = parse_objc(src);
    let defines: Vec<(&str, &str)> = parsed
        .defines
        .iter()
        .map(|d| (d.container_name.as_str(), d.member_name.as_str()))
        .collect();
    assert!(defines.contains(&("Widget", "draw")));
    assert!(defines.contains(&("Widget", "resize")));
}

#[test]
fn method_body_traverses_each_message_expression_in_source_order() {
    let src = r#"
@implementation Widget
- (void)draw {
    [canvas prepare];
    [canvas render];
}
@end
"#;
    let parsed = parse_objc(src);
    let calls: Vec<(&str, Option<&str>)> = parsed
        .calls
        .iter()
        .map(|call| (call.callee.as_str(), call.from_symbol.as_deref()))
        .collect();

    assert_eq!(
        calls,
        vec![("prepare", Some("draw")), ("render", Some("draw"))]
    );
}

#[test]
fn extracts_message_expression_zero_argument_selector() -> TestResult {
    let src = r#"
@implementation Widget
- (void)draw {
    [self speak];
}
@end
"#;
    let parsed = parse_objc(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "speak")
        .ok_or("expected a speak message send")?;
    assert_eq!(call.receiver_text.as_deref(), Some("self"), "{call:?}");
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::SelfOrThis),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_message_expression_multi_keyword_selector() -> TestResult {
    // The call site's selector text must agree byte-for-byte with the
    // definition-side selector ("setName:withAge:" both ways).
    let src = r#"
@implementation Widget
- (void)draw {
    [self setName:@"Rex" withAge:3];
}
@end
"#;
    let parsed = parse_objc(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "setName:withAge:")
        .ok_or("expected a setName:withAge: message send")?;
    assert_eq!(call.receiver_text.as_deref(), Some("self"), "{call:?}");
    Ok(())
}

#[test]
fn message_expression_records_only_argument_texts() -> TestResult {
    let src = r#"
@implementation Widget
- (void)draw {
    [self setName:@"Rex" withAge:3];
}
@end
"#;
    let parsed = parse_objc(src);
    let call = parsed
        .calls
        .iter()
        .find(|call| call.callee == "setName:withAge:")
        .ok_or("expected a setName:withAge: message send")?;

    assert_eq!(call.arg_texts, vec!["@\"Rex\"", "3"]);
    Ok(())
}

#[test]
fn class_receiver_message_send_is_new_expression_hint() -> TestResult {
    let src = r#"
@implementation Widget
- (void)draw {
    [NSString stringWithFormat:@"%@", self];
}
@end
"#;
    let parsed = parse_objc(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "stringWithFormat:")
        .ok_or("expected a stringWithFormat: message send")?;
    assert_eq!(
        call.receiver_hint,
        Some(ReceiverHint::NewExpression),
        "{call:?}"
    );
    Ok(())
}

#[test]
fn extracts_plain_c_call_expression() -> TestResult {
    let src = r#"
void helper(void) {
}

void draw(void) {
    helper();
}
"#;
    let parsed = parse_objc(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"));
    Ok(())
}

#[test]
fn message_send_inside_method_records_from_symbol_scope() -> TestResult {
    let src = r#"
@implementation Widget
- (void)draw {
    [self speak];
}
@end
"#;
    let parsed = parse_objc(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "speak")
        .ok_or("expected a speak message send")?;
    assert_eq!(call.from_symbol.as_deref(), Some("draw"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_plain_c_struct_delegated_to_c_quirk() {
    let src = r#"
struct Point {
    int x;
    int y;
};
"#;
    let parsed = parse_objc(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Point"),
        Some(&SymbolKind::Struct),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_preproc_include_as_imports() {
    let src = r#"
#import <Foundation/Foundation.h>
#include "local.h"
"#;
    let parsed = parse_objc(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"Foundation/Foundation.h"));
    assert!(paths.contains(&"local.h"));
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_objc("@interface ( { this is not valid objc @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("Widget.m");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_objc(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "setName:withAge:"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "Foundation/Foundation.h"),
        "{:?}",
        parsed.imports
    );
    assert!(
        parsed
            .inherits
            .iter()
            .any(|i| i.sub_name == "Widget" && i.super_name == "Animal"),
        "{:?}",
        parsed.inherits
    );
    Ok(())
}
