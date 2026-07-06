//! Python extraction via `tree-sitter-python`: functions (including
//! `test_*`-named pytest functions and `unittest`-style methods, tagged
//! [`SymbolKind::Test`]), classes, imports, calls, and Flask/FastAPI
//! route decorators.

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImportRef, InheritsRef, ParsedFile, RouteRef, SymbolKind,
    SymbolRef,
};
use tree_sitter::{Node, Parser};

const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

pub fn parse(source: &str) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .is_err()
    {
        return ParsedFile::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParsedFile::default();
    };

    let mut out = ParsedFile::default();
    walk(tree.root_node(), source.as_bytes(), &mut out, None);
    out
}

/// `enclosing` is the name of the containing `class_definition` (if
/// any) -- distinguishes [`SymbolKind::Method`] from
/// [`SymbolKind::Function`] and feeds DEFINES edges.
fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile, enclosing: Option<&str>) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let kind = if enclosing.is_some() {
                    if is_test_name(&name) {
                        SymbolKind::Test
                    } else {
                        SymbolKind::Method
                    }
                } else {
                    test_or_function(&name)
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: name,
                        line,
                    });
                }
            }
        }
        "class_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                for base in base_class_names(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: base,
                        line,
                    });
                }
                walk_children(node, src, out, Some(name.as_str()));
                return;
            }
        }
        "decorated_definition" => {
            if let Some(route) = route_from_decorated(node, src) {
                out.routes.push(route);
            }
            for (target, decorator) in decorators_on(node, src) {
                out.decorates.push(DecoratesRef {
                    target_name: target,
                    decorator_name: decorator,
                    line: node.start_position().row + 1,
                });
            }
        }
        "import_statement" => {
            for path in dotted_names_under(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "import_from_statement" => {
            if let Some(module_node) = node.child_by_field_name("module_name") {
                if let Ok(module) = module_node.utf8_text(src) {
                    out.imports.push(ImportRef {
                        module_path: module.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
        "call" => {
            if let Some(function) = node.child_by_field_name("function") {
                out.calls.push(CallRef {
                    callee: function.utf8_text(src).unwrap_or("").to_string(),
                    line: node.start_position().row + 1,
                });
            }
        }
        "expression_statement" => {
            if let Some(binding) = named_lambda_binding(node, src) {
                out.symbols.push(binding);
            }
        }
        _ => {}
    }

    walk_children(node, src, out, enclosing);
}

fn walk_children(node: Node<'_>, src: &[u8], out: &mut ParsedFile, enclosing: Option<&str>) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, src, out, enclosing);
        }
    }
}

fn test_or_function(name: &str) -> SymbolKind {
    if is_test_name(name) {
        SymbolKind::Test
    } else {
        SymbolKind::Function
    }
}

fn is_test_name(name: &str) -> bool {
    name.starts_with("test_") || name.starts_with("test")
}

/// `class Sub(Base1, Base2):` -- the base-class argument list.
fn base_class_names(class_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(superclasses) = class_node.child_by_field_name("superclasses") {
        for i in 0..superclasses.child_count() {
            if let Some(child) = superclasses.child(i) {
                if matches!(child.kind(), "identifier" | "attribute") {
                    if let Ok(text) = child.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Every `@decorator`/`@decorator(...)` on a `decorated_definition`,
/// paired with the name of the definition it decorates.
fn decorators_on(node: Node<'_>, src: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(target_name) = node
        .child_by_field_name("definition")
        .and_then(|def| child_text(def, "name", src))
    else {
        return out;
    };
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "decorator" {
            continue;
        }
        let decorator_expr = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.kind() != "@");
        let Some(expr) = decorator_expr else { continue };
        let name = match expr.kind() {
            "call" => expr
                .child_by_field_name("function")
                .and_then(|f| f.utf8_text(src).ok()),
            _ => expr.utf8_text(src).ok(),
        };
        if let Some(name) = name {
            out.push((target_name.clone(), name.to_string()));
        }
    }
    out
}

/// `f = lambda x: ...` -- a named lambda binding, best-effort
/// [`SymbolKind::Lambda`] source.
fn named_lambda_binding(expr_stmt: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    let assignment = (0..expr_stmt.child_count())
        .filter_map(|i| expr_stmt.child(i))
        .find(|n| n.kind() == "assignment")?;
    let left = assignment.child_by_field_name("left")?;
    if left.kind() != "identifier" {
        return None;
    }
    let right = assignment.child_by_field_name("right")?;
    if right.kind() != "lambda" {
        return None;
    }
    let name = left.utf8_text(src).ok()?.to_string();
    Some(SymbolRef {
        name,
        kind: SymbolKind::Lambda,
        line: expr_stmt.start_position().row + 1,
    })
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

fn dotted_names_under(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "dotted_name" => {
                    if let Ok(text) = child.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
                "aliased_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(text) = name_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Recognize Flask/FastAPI-style decorators: `@app.route("/x")`,
/// `@app.get("/x")`, `@router.post("/x")`.
fn route_from_decorated(decorated_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    for i in 0..decorated_node.child_count() {
        let child = decorated_node.child(i)?;
        if child.kind() != "decorator" {
            continue;
        }
        // A `decorator` node wraps either a bare `identifier`/
        // `attribute` or a `call` (when it has arguments).
        let call = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.kind() == "call")?;
        let function = call.child_by_field_name("function")?;
        let function_text = function.utf8_text(src).unwrap_or("");
        let method_word = function_text.rsplit('.').next().unwrap_or("");
        let method = if method_word.eq_ignore_ascii_case("route") {
            "GET".to_string()
        } else if HTTP_METHODS
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(method_word))
        {
            method_word.to_uppercase()
        } else {
            continue;
        };
        let path = call
            .child_by_field_name("arguments")
            .and_then(|args| {
                (0..args.child_count())
                    .filter_map(|k| args.child(k))
                    .find(|n| n.kind() == "string")
            })
            .and_then(|n| n.utf8_text(src).ok())
            .map(|raw| raw.trim_matches(|c| c == '"' || c == '\'').to_string())
            .unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        return Some(RouteRef {
            method,
            path,
            line: decorated_node.start_position().row + 1,
        });
    }
    None
}
