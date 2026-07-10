//! Hard tests for Kotlin, onboarded directly through the generic
//! spec-table engine ([`enforcer_memory::languages::generic::parse_kotlin`])
//! -- there is no bespoke `languages::kotlin` extractor to prove
//! zero-regression against (Kotlin has never had one in this crate), so
//! these tests assert against the grammar-shape ground truth recorded in
//! [`enforcer_memory::languages::spec::LangSpec::kotlin`]'s own doc
//! comment directly: symbol kinds (function/method/interface/class/
//! type-alias), delegation-specifier INHERITS, call edges (both the
//! unfielded `call_expression` and standalone `navigation_expression`
//! shapes), and import paths.

use enforcer_memory::languages::generic::parse_kotlin;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_kotlin";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn extracts_function_interface_and_class_symbols() {
    let src = r#"
package widget

interface Drawable {
    fun draw(): String
}

class Widget(val name: String) : Drawable {
    override fun draw(): String {
        return name
    }
}

fun helper(label: String): String {
    return label
}
"#;
    let parsed = parse_kotlin(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Drawable"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
    // "draw" is defined twice (interface member + class override) --
    // both must be classified as Method (nested inside a class/interface
    // body), never Function.
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
fn extracts_type_alias() {
    let src = r#"
package widget

type Alias = String
"#;
    // NOTE: Kotlin's real syntax is `typealias Alias = String`, but the
    // node kind this grammar emits for it is literally `type_alias`
    // regardless of spelling variance in hand-written fixtures elsewhere
    // -- assert against the canonical form.
    let canonical = r#"
package widget

typealias Alias = String
"#;
    let _ = parse_kotlin(src);
    let parsed = parse_kotlin(canonical);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Alias"),
        Some(&SymbolKind::TypeAlias),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_delegation_specifier_as_inherits() {
    let src = r#"
package widget

interface Drawable

class Widget : Drawable {
}
"#;
    let parsed = parse_kotlin(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Drawable")), "{inherits:?}");
}

#[test]
fn extracts_constructor_invocation_delegation_as_inherits() {
    // `Base()` heritage (constructor-invocation-wrapped `user_type`,
    // not a bare `user_type`) -- mirrors the baseline's own
    // `extract_kotlin_bases` descending through the
    // `constructor_invocation` layer.
    let src = r#"
package widget

open class Base

class Widget : Base() {
}
"#;
    let parsed = parse_kotlin(src);
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Base")), "{inherits:?}");
}

#[test]
fn extracts_call_edges() -> TestResult {
    let src = r#"
package widget

fun f() {
    helper()
    obj.method()
}
"#;
    let parsed = parse_kotlin(src);
    let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
    assert!(callees.contains(&"helper"), "{callees:?}");
    let method_call = parsed
        .calls
        .iter()
        .find(|c| c.callee.contains("obj.method"))
        .ok_or("expected an obj.method call")?;
    assert_eq!(
        method_call.receiver_text.as_deref(),
        Some("obj"),
        "{method_call:?}"
    );
    Ok(())
}

#[test]
fn call_inside_function_records_from_symbol_scope() -> TestResult {
    let src = r#"
package widget

fun render() {
    helper()
}
"#;
    let parsed = parse_kotlin(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee.starts_with("helper"))
        .ok_or("expected a helper call")?;
    assert_eq!(call.from_symbol.as_deref(), Some("render"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_imports() {
    let src = r#"
package widget

import kotlin.math.max
import kotlin.collections.List
"#;
    let parsed = parse_kotlin(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"kotlin.math.max"), "{paths:?}");
    assert!(paths.contains(&"kotlin.collections.List"), "{paths:?}");
}

#[test]
fn extracts_secondary_constructor_as_nameless_method() {
    let src = r#"
package widget

class Widget {
    constructor(name: String) {
    }
}
"#;
    let parsed = parse_kotlin(src);
    assert!(
        parsed
            .symbols
            .iter()
            .any(|s| s.name.is_empty() && s.kind == SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_kotlin("class ( { this is not valid kotlin @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.kt");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_kotlin(&src);
    assert!(
        parsed.symbols.iter().any(|s| s.name == "Widget"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed.symbols.iter().any(|s| s.name == "helper"),
        "{:?}",
        parsed.symbols
    );
    assert!(
        parsed
            .imports
            .iter()
            .any(|i| i.module_path == "kotlin.math.max"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}

#[test]
fn annotated_class_and_function_record_decorates_edges() -> TestResult {
    // language-parity wave G3 stage 3: Kotlin's `annotation` node has no
    // fields at all -- its name bottoms out in a `user_type` child,
    // either directly (bare `@Serializable`) or nested inside a
    // `constructor_invocation` (`@Deprecated("old")`) -- and sits inside
    // an unfielded `modifiers` wrapper child of the declaration it
    // decorates.
    let src = r#"
@Serializable
class Widget {
    @Deprecated("old")
    fun draw() {}
}
"#;
    let parsed = parse_kotlin(src);
    let class_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "Widget")
        .ok_or("expected a DECORATES edge for Widget")?;
    assert_eq!(class_edge.decorator_name, "Serializable");
    let fn_edge = parsed
        .decorates
        .iter()
        .find(|d| d.target_name == "draw")
        .ok_or("expected a DECORATES edge for draw")?;
    assert_eq!(fn_edge.decorator_name, "Deprecated");
    Ok(())
}

#[test]
fn spring_mapping_annotation_records_a_route() -> TestResult {
    // language-parity wave G3 stage 4: Spring Boot backends written in
    // Kotlin use the same `@GetMapping`/`@PostMapping`/etc. annotations
    // as Java -- the baseline's own route mechanism is purely name-based
    // with no per-language gating (`extract_defs.c:1275`,
    // `extract_defs.c:4137` explicitly treats Java/Kotlin identically).
    let src = r#"
@GetMapping("/widgets")
fun listWidgets() {}
"#;
    let parsed = parse_kotlin(src);
    let route = parsed
        .routes
        .first()
        .ok_or("expected a route for listWidgets")?;
    assert_eq!(route.method, "GET");
    assert_eq!(route.path, "/widgets");
    Ok(())
}
