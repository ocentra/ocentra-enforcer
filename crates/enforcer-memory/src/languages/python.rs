//! Python extraction via `tree-sitter-python`: functions (including
//! `test_*`-named pytest functions and `unittest`-style methods, tagged
//! [`SymbolKind::Test`]), classes, imports, calls, and Flask/FastAPI
//! route decorators.

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImportRef, InheritsRef, ParsedFile, ReceiverHint, RouteRef,
    SymbolKind, SymbolRef,
};
use tree_sitter::{Node, Parser};

const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// The innermost function/method a call expression is lexically inside
/// of, if any -- see `rust.rs`'s identical `FnScope` for the full
/// rationale (kept as its own type per file rather than shared, since
/// each extractor module is independently high-churn per this crate's
/// "no shared mutable state across language extractors" convention).
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

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
    walk(
        tree.root_node(),
        source.as_bytes(),
        &mut out,
        None,
        FnScope::default(),
    );
    out
}

/// `enclosing` is the name of the containing `class_definition` (if
/// any) -- distinguishes [`SymbolKind::Method`] from
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
                walk_children(node, src, out, Some(name.as_str()), fn_scope);
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
                let (receiver_text, receiver_hint) = receiver_of_call(function, src);
                out.calls.push(CallRef {
                    callee: function.utf8_text(src).unwrap_or("").to_string(),
                    line: node.start_position().row + 1,
                    from_symbol: fn_scope.name.map(str::to_string),
                    from_symbol_line: fn_scope.line,
                    receiver_text,
                    receiver_hint,
                    arg_texts: call_arg_texts(node, src),
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

    walk_children(node, src, out, enclosing, fn_scope);
}

fn walk_children(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    for child in node.children(&mut node.walk()) {
        walk(child, src, out, enclosing, fn_scope);
    }
}

/// For an `attribute`-shaped callee (`x.foo`, `self.foo`), the
/// receiver (`x`/`self`) text plus a cheap syntactic hint. `None`/
/// `None` for a plain identifier callee (`foo`).
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "attribute" {
        return (None, None);
    }
    let Some(receiver) = function_node.child_by_field_name("object") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "self" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "call" && is_new_call(receiver, src) {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "string" | "integer" | "float" | "true" | "false"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Python has no dedicated `new`-expression syntax -- a call is
/// heuristically treated as constructor-shaped when its own callee
/// looks like a class name (`PascalCase` convention), same
/// name-convention rationale as `rust.rs`'s `Foo::new(...)` heuristic.
fn is_new_call(call_node: Node<'_>, src: &[u8]) -> bool {
    let Some(function) = call_node.child_by_field_name("function") else {
        return false;
    };
    let Ok(text) = function.utf8_text(src) else {
        return false;
    };
    let last = text.rsplit('.').next().unwrap_or(text);
    last.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Each argument expression's own source text, in written order.
fn call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = call_node.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for child in args.children(&mut args.walk()) {
        if matches!(child.kind(), "(" | ")" | ",") {
            continue;
        }
        if let Ok(text) = child.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
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
        for child in superclasses.children(&mut superclasses.walk()) {
            if matches!(child.kind(), "identifier" | "attribute") {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
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
    for child in node.children(&mut node.walk()) {
        if child.kind() != "decorator" {
            continue;
        }
        let decorator_expr = child.children(&mut child.walk()).find(|n| n.kind() != "@");
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
    let assignment = expr_stmt
        .children(&mut expr_stmt.walk())
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
    for child in node.children(&mut node.walk()) {
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
    out
}

/// Recognize Flask/FastAPI-style decorators: `@app.route("/x")`,
/// `@app.get("/x")`, `@router.post("/x")`.
fn route_from_decorated(decorated_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    for child in decorated_node.children(&mut decorated_node.walk()) {
        if child.kind() != "decorator" {
            continue;
        }
        // A `decorator` node wraps either a bare `identifier`/
        // `attribute` or a `call` (when it has arguments).
        let call = child
            .children(&mut child.walk())
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
                args.children(&mut args.walk())
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
