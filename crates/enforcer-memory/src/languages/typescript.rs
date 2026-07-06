//! TypeScript/JavaScript extraction via `tree-sitter-typescript`.
//!
//! The workpack groups "TypeScript/JavaScript" as one requirement, and
//! the TypeScript grammar is a strict superset of JavaScript syntax for
//! everything this extractor cares about (functions, classes,
//! interfaces, imports, calls, route decorators/handlers) -- so both
//! [`crate::parsers::Language::TypeScript`] and
//! [`crate::parsers::Language::JavaScript`] are parsed with the same
//! grammar here rather than pulling in a second, narrower grammar
//! crate. `.tsx` is intentionally routed through the plain
//! `LANGUAGE_TYPESCRIPT` grammar (not `LANGUAGE_TSX`) for this slice --
//! JSX syntax nodes simply fall outside every node kind this extractor
//! matches, which is a silent-miss on JSX-only symbols, not a parse
//! failure (the file still gets its file node either way).

use crate::parsers::{CallRef, ImportRef, Language, ParsedFile, RouteRef, SymbolKind, SymbolRef};
use tree_sitter::{Node, Parser};

/// HTTP methods this extractor recognizes in route-style call
/// expressions (`app.get(...)`, `router.post(...)`) and decorators
/// (`@Get()`, `@Post()`).
const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

pub fn parse(source: &str, _language: Language) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .is_err()
    {
        return ParsedFile::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParsedFile::default();
    };

    let mut out = ParsedFile::default();
    walk(tree.root_node(), source.as_bytes(), &mut out);
    out
}

fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    kind: test_or_function(&name),
                    name,
                    line: node.start_position().row + 1,
                });
            }
        }
        "class_declaration" | "interface_declaration" | "type_alias_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Type,
                    line: node.start_position().row + 1,
                });
            }
        }
        "method_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    kind: test_or_function(&name),
                    name,
                    line: node.start_position().row + 1,
                });
            }
        }
        "import_statement" => {
            if let Some(source_node) = node.child_by_field_name("source") {
                let raw = source_node.utf8_text(src).unwrap_or("");
                let module_path = raw.trim_matches(|c| c == '"' || c == '\'').to_string();
                if !module_path.is_empty() {
                    out.imports.push(ImportRef {
                        module_path,
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                let callee = function.utf8_text(src).unwrap_or("").to_string();
                out.calls.push(CallRef {
                    callee: callee.clone(),
                    line: node.start_position().row + 1,
                });
                if let Some(route) = route_from_call(&callee, node, src) {
                    out.routes.push(route);
                }
            }
        }
        "decorator" => {
            if let Some(route) = route_from_decorator(node, src) {
                out.routes.push(route);
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, src, out);
        }
    }
}

fn test_or_function(name: &str) -> SymbolKind {
    let lower = name.to_lowercase();
    if lower.starts_with("test") || lower.starts_with("it_") || lower == "it" {
        SymbolKind::Test
    } else {
        SymbolKind::Function
    }
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Recognize Express/Fastify/Axios-router-style calls: `<obj>.<method>(
/// "<path>", ...)` where `<method>` is an HTTP verb and the first
/// argument is a string literal path.
fn route_from_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let method = callee.rsplit('.').next()?.to_lowercase();
    if !HTTP_METHODS.contains(&method.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "string")?;
    let raw = first_string_arg.utf8_text(src).ok()?;
    let path = raw
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string();
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    Some(RouteRef {
        method: method.to_uppercase(),
        path,
        line: call_node.start_position().row + 1,
    })
}

/// Recognize NestJS-style decorators: `@Get("/path")`, `@Post()`.
fn route_from_decorator(decorator_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let call = (0..decorator_node.child_count())
        .filter_map(|i| decorator_node.child(i))
        .find(|n| n.kind() == "call_expression")?;
    let function = call.child_by_field_name("function")?;
    let name = function.utf8_text(src).ok()?;
    let method = HTTP_METHODS
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(name))?;
    let path = call
        .child_by_field_name("arguments")
        .and_then(|args| {
            (0..args.child_count())
                .filter_map(|i| args.child(i))
                .find(|n| n.kind() == "string")
        })
        .and_then(|n| n.utf8_text(src).ok())
        .map(|raw| {
            raw.trim_matches(|c| c == '"' || c == '\'' || c == '`')
                .to_string()
        })
        .unwrap_or_default();
    Some(RouteRef {
        method: method.to_uppercase(),
        path,
        line: decorator_node.start_position().row + 1,
    })
}
