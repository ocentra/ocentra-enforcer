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

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImplementsRef, ImportRef, InheritsRef, Language, ParsedFile,
    ReceiverHint, RouteRef, SymbolKind, SymbolRef, TypeRefRef,
};
use tree_sitter::{Node, Parser};

/// HTTP methods this extractor recognizes in route-style call
/// expressions (`app.get(...)`, `router.post(...)`) and decorators
/// (`@Get()`, `@Post()`).
const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// The innermost function/method a call expression is lexically inside
/// of, if any -- see `rust.rs`'s identical `FnScope` for the full
/// rationale.
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

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
    walk(
        tree.root_node(),
        source.as_bytes(),
        &mut out,
        None,
        FnScope::default(),
    );
    out
}

/// `enclosing` is the name of the containing `class`/`interface` body
/// (if any) -- distinguishes [`SymbolKind::Method`] from
/// [`SymbolKind::Function`] and feeds DEFINES edges. `fn_scope` is the
/// innermost function/method a call expression sits inside of.
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    kind: test_or_function(&name),
                    name: name.clone(),
                    line,
                });
                for type_ref in signature_type_refs(node, src) {
                    out.type_refs.push(TypeRefRef {
                        from_name: name.clone(),
                        type_name: type_ref,
                        line,
                    });
                }
                for decorator_name in preceding_decorators(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name,
                        line,
                    });
                }
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        enclosing,
                        FnScope {
                            name: Some(name.as_str()),
                            line: Some(line),
                        },
                    );
                }
                return;
            }
        }
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                for (kind, super_name) in heritage_refs(node, src) {
                    match kind {
                        HeritageKind::Extends => out.inherits.push(InheritsRef {
                            sub_name: name.clone(),
                            super_name,
                            line,
                        }),
                        HeritageKind::Implements => out.implements.push(ImplementsRef {
                            type_name: name.clone(),
                            trait_name: super_name,
                            line,
                        }),
                    }
                }
                for decorator_name in preceding_decorators(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name,
                        line,
                    });
                }
                walk_children(node, src, out, Some(name.as_str()), fn_scope);
                return;
            }
        }
        "interface_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for (kind, super_name) in heritage_refs(node, src) {
                    if matches!(kind, HeritageKind::Extends) {
                        out.inherits.push(InheritsRef {
                            sub_name: name.clone(),
                            super_name,
                            line,
                        });
                    }
                }
            }
        }
        "type_alias_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::TypeAlias,
                    line: node.start_position().row + 1,
                });
            }
        }
        "enum_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
        }
        "module" | "internal_module" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
        }
        "method_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let kind = if is_test_name(&name) {
                    SymbolKind::Test
                } else if enclosing.is_some() {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    kind,
                    name: name.clone(),
                    line,
                });
                for type_ref in signature_type_refs(node, src) {
                    out.type_refs.push(TypeRefRef {
                        from_name: name.clone(),
                        type_name: type_ref,
                        line,
                    });
                }
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: name.clone(),
                        line,
                    });
                }
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        enclosing,
                        FnScope {
                            name: Some(name.as_str()),
                            line: Some(line),
                        },
                    );
                }
                return;
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            if let Some(binding) = named_arrow_or_const_binding(node, src) {
                out.symbols.push(binding);
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
                let (receiver_text, receiver_hint) = receiver_of_call(function, src);
                out.calls.push(CallRef {
                    callee: callee.clone(),
                    line: node.start_position().row + 1,
                    from_symbol: fn_scope.name.map(str::to_string),
                    from_symbol_line: fn_scope.line,
                    receiver_text,
                    receiver_hint,
                    arg_texts: call_arg_texts(node, src),
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

    walk_children(node, src, out, enclosing, fn_scope);
}

fn walk_children(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, out, enclosing, fn_scope);
    }
}

/// For a `member_expression`-shaped callee (`x.foo`, `this.foo`), the
/// receiver text plus a cheap syntactic hint.
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "member_expression" {
        return (None, None);
    }
    let Some(receiver) = function_node.child_by_field_name("object") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "new_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(receiver.kind(), "string" | "number" | "true" | "false") {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Each argument expression's own source text, in written order.
fn call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = call_node.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cursor = args.walk();
    for child in args.children(&mut cursor) {
        if matches!(child.kind(), "(" | ")" | ",") {
            continue;
        }
        if let Ok(text) = child.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

enum HeritageKind {
    Extends,
    Implements,
}

/// `class Sub extends Base implements I1, I2` / `interface Sub extends
/// Base1, Base2` -- the `class_heritage`/`extends_clause`/
/// `implements_clause` children.
fn heritage_refs(node: Node<'_>, src: &[u8]) -> Vec<(HeritageKind, String)> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_heritage" => {
                let mut child_cursor = child.walk();
                for clause in child.children(&mut child_cursor) {
                    collect_heritage_clause(clause, src, &mut out);
                }
            }
            "extends_clause" | "implements_clause" => {
                collect_heritage_clause(child, src, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn collect_heritage_clause(clause: Node<'_>, src: &[u8], out: &mut Vec<(HeritageKind, String)>) {
    let is_extends = match clause.kind() {
        "extends_clause" => true,
        "implements_clause" => false,
        _ => return,
    };
    let mut cursor = clause.walk();
    for entry in clause.children(&mut cursor) {
        if matches!(
            entry.kind(),
            "identifier" | "type_identifier" | "nested_type_identifier"
        ) {
            if let Ok(text) = entry.utf8_text(src) {
                out.push((
                    if is_extends {
                        HeritageKind::Extends
                    } else {
                        HeritageKind::Implements
                    },
                    text.to_string(),
                ));
            }
        }
    }
}

/// Decorators on a class/method/function declaration (`@Injectable()
/// class X`, `class C { @Prop() name: string; }`). `tree-sitter-
/// typescript` attaches a decorator to its target as the `decorator:`
/// field on the target node itself (verified against the grammar's own
/// s-expression output), not as a preceding sibling -- this also walks
/// `prev_sibling()` as a defensive fallback for any grammar shape this
/// field-based lookup does not cover, so a decorator is never silently
/// missed either way.
fn preceding_decorators(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(field_decorator) = node.child_by_field_name("decorator") {
        if let Some(name) = decorator_name(field_decorator, src) {
            out.push(name);
        }
    }
    let mut sibling = node.prev_sibling();
    while let Some(candidate) = sibling {
        if candidate.kind() != "decorator" {
            break;
        }
        if let Some(name) = decorator_name(candidate, src) {
            out.push(name);
        }
        sibling = candidate.prev_sibling();
    }
    out.reverse();
    out
}

fn decorator_name(decorator_node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = decorator_node.walk();
    for child in decorator_node.children(&mut cursor) {
        match child.kind() {
            "call_expression" => {
                let function = child.child_by_field_name("function")?;
                return function.utf8_text(src).ok().map(str::to_string);
            }
            "identifier" => {
                return child.utf8_text(src).ok().map(str::to_string);
            }
            _ => {}
        }
    }
    None
}

/// Parameter and return types on a function/method's signature.
fn signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for param in params.children(&mut cursor) {
            if let Some(type_node) = param.child_by_field_name("type") {
                if let Ok(text) = type_node.utf8_text(src) {
                    out.push(text.trim_start_matches(':').trim().to_string());
                }
            }
        }
    }
    if let Some(return_type) = node.child_by_field_name("return_type") {
        if let Ok(text) = return_type.utf8_text(src) {
            out.push(text.trim_start_matches(':').trim().to_string());
        }
    }
    out
}

/// `const f = (x) => ...` / `const f = function() {}` -- a named
/// arrow-function/lambda binding, best-effort [`SymbolKind::Lambda`]
/// source.
fn named_arrow_or_const_binding(node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    let mut cursor = node.walk();
    for declarator in node.children(&mut cursor) {
        if declarator.kind() != "variable_declarator" {
            continue;
        }
        let name_node = declarator.child_by_field_name("name")?;
        if name_node.kind() != "identifier" {
            continue;
        }
        let value = declarator.child_by_field_name("value")?;
        let name = name_node.utf8_text(src).ok()?.to_string();
        let line = node.start_position().row + 1;
        if matches!(value.kind(), "arrow_function" | "function_expression") {
            return Some(SymbolRef {
                name,
                kind: SymbolKind::Lambda,
                line,
            });
        }
    }
    None
}

fn test_or_function(name: &str) -> SymbolKind {
    if is_test_name(name) {
        SymbolKind::Test
    } else {
        SymbolKind::Function
    }
}

fn is_test_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test") || lower.starts_with("it_") || lower == "it"
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
    let mut args_cursor = args.walk();
    let first_string_arg = args
        .children(&mut args_cursor)
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
    let mut decorator_cursor = decorator_node.walk();
    let call = decorator_node
        .children(&mut decorator_cursor)
        .find(|n| n.kind() == "call_expression")?;
    let function = call.child_by_field_name("function")?;
    let name = function.utf8_text(src).ok()?;
    let method = HTTP_METHODS
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(name))?;
    let path = call
        .child_by_field_name("arguments")
        .and_then(|args| {
            let mut args_cursor = args.walk();
            let string_node = args
                .children(&mut args_cursor)
                .find(|node| node.kind() == "string");
            string_node.and_then(|node| node.utf8_text(src).ok())
        })
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
