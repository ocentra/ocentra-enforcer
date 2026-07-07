//! Hard tests for TSX (TypeScript-JSX), onboarded through the generic
//! spec-table engine's [`enforcer_memory::languages::generic::parse_tsx`]
//! -- reuses [`enforcer_memory::languages::generic::typescript_quirks`]
//! unchanged (see [`enforcer_memory::languages::spec::LangSpec::tsx`]'s
//! doc comment: every node-kind array is identical to plain
//! TypeScript's), with only the grammar entry point swapped
//! (`tree_sitter_typescript::LANGUAGE_TSX` instead of
//! `LANGUAGE_TYPESCRIPT`). These tests exist primarily to prove that
//! grammar swap actually works end-to-end on real JSX syntax (which the
//! plain TypeScript grammar cannot parse at all) while every ordinary
//! TS construct (class heritage, decorators, calls, imports) still
//! extracts identically to `unit_languages_typescript.rs`'s own
//! TypeScript-grammar assertions.

use enforcer_memory::languages::generic::parse_tsx;
use enforcer_memory::parsers::SymbolKind;
use std::error::Error;
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/lang_tsx";

fn symbol_kind<'a>(
    symbols: &'a [enforcer_memory::parsers::SymbolRef],
    name: &str,
) -> Option<&'a SymbolKind> {
    symbols.iter().find(|s| s.name == name).map(|s| &s.kind)
}

#[test]
fn parses_jsx_returning_function_component_without_panic() {
    let src = r#"
function Widget(props: { name: string }) {
    if (props.name === "") {
        return <span>unnamed</span>;
    }
    return <span>{props.name}</span>;
}
"#;
    let parsed = parse_tsx(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn parses_class_component_with_jsx_render_method() {
    // A bare (non-dotted) base class name -- `ts_collect_heritage_clause`
    // (shared, unchanged TS logic this row reuses per `LangSpec::tsx`'s
    // own doc comment) only recognizes `identifier`/`type_identifier`/
    // `nested_type_identifier` heritage-clause entries; a dotted base
    // like `React.Component` parses as a `member_expression`, which
    // that pre-existing match does not cover (a real gap, confirmed
    // with a standalone debug harness against the TSX grammar, but a
    // pre-existing one in shared TS code this wave reuses unchanged
    // rather than something introduced here -- flagged separately
    // rather than silently worked around by this test).
    let src = r#"
import Component from "react";

class Widget extends Component {
    render() {
        return <div className="widget">{this.props.name}</div>;
    }
}
"#;
    let parsed = parse_tsx(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Class),
        "{:?}",
        parsed.symbols
    );
    let inherits: Vec<(&str, &str)> = parsed
        .inherits
        .iter()
        .map(|i| (i.sub_name.as_str(), i.super_name.as_str()))
        .collect();
    assert!(inherits.contains(&("Widget", "Component")), "{inherits:?}");
    assert_eq!(
        symbol_kind(&parsed.symbols, "render"),
        Some(&SymbolKind::Method),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn extracts_call_edges_inside_jsx_expression_container() -> TestResult {
    let src = r#"
function Widget(props: { name: string }) {
    return <span>{helper(props.name)}</span>;
}

function helper(label: string): string {
    return label.toUpperCase();
}
"#;
    let parsed = parse_tsx(src);
    let call = parsed
        .calls
        .iter()
        .find(|c| c.callee == "helper")
        .ok_or("expected a helper call inside the JSX expression container")?;
    assert_eq!(call.from_symbol.as_deref(), Some("Widget"), "{call:?}");
    Ok(())
}

#[test]
fn extracts_decorator_same_as_plain_typescript() {
    let src = r#"
@Component
class Widget {
}
"#;
    let parsed = parse_tsx(src);
    let decorators: Vec<(&str, &str)> = parsed
        .decorates
        .iter()
        .map(|d| (d.target_name.as_str(), d.decorator_name.as_str()))
        .collect();
    assert!(
        decorators.contains(&("Widget", "Component")),
        "{decorators:?}"
    );
}

#[test]
fn extracts_import_statement() {
    let src = r#"import React from "react";"#;
    let parsed = parse_tsx(src);
    let paths: Vec<&str> = parsed
        .imports
        .iter()
        .map(|i| i.module_path.as_str())
        .collect();
    assert!(paths.contains(&"react"), "{paths:?}");
}

#[test]
fn plain_typescript_source_with_no_jsx_still_parses() {
    // TSX's grammar is a strict superset of plain TypeScript syntax
    // (see `LangSpec::tsx`'s doc comment) -- a file with zero JSX
    // syntax must still parse identically to `parse_typescript`.
    let src = r#"
interface Widget {
    name: string;
}

function helper(label: string): string {
    return label.toUpperCase();
}
"#;
    let parsed = parse_tsx(src);
    assert_eq!(
        symbol_kind(&parsed.symbols, "Widget"),
        Some(&SymbolKind::Interface),
        "{:?}",
        parsed.symbols
    );
    assert_eq!(
        symbol_kind(&parsed.symbols, "helper"),
        Some(&SymbolKind::Function),
        "{:?}",
        parsed.symbols
    );
}

#[test]
fn malformed_source_does_not_panic() {
    let parsed = parse_tsx("function ( { <this is not valid tsx @@@");
    let _ = parsed;
}

#[test]
fn fixture_file_parses_without_panic() -> TestResult {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join(FIXTURE_DIR).join("widget.tsx");
    let src = fs::read_to_string(&fixture)?;
    let parsed = parse_tsx(&src);
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
        parsed.imports.iter().any(|i| i.module_path == "react"),
        "{:?}",
        parsed.imports
    );
    Ok(())
}
