//! PHP extraction via `tree-sitter-php`.
//!
//! Grammar-variant note: `tree-sitter-php` 0.24.x exposes two entry
//! points -- `LANGUAGE_PHP` (accepts a full on-disk `.php` file:
//! optional leading inline HTML, `<?php ... ?>` tags, trailing inline
//! HTML) and `LANGUAGE_PHP_ONLY` (grammar rooted directly at `program`,
//! no tag/HTML handling, meant for embedding a PHP-only fragment inside
//! another host grammar). This extractor parses whole files as they
//! exist on disk, where a leading `<?php` tag (and occasionally
//! trailing inline HTML in legacy templates) is the norm, so it uses
//! `LANGUAGE_PHP`.
//!
//! Symbols: `Class`, `Interface`, `Function` (top-level), `Method`
//! (inside a class/interface/trait body), `Constant` (`const NAME = ..`
//! class constants and top-level `define("NAME", ..)` calls), named
//! `Lambda` (`$f = function(...) {}` / `$f = fn(...) => ...` assigned to
//! a plain variable), `Module` (`namespace ...;`). Edges: IMPORTS
//! (`use App\Foo;` / `use function ...` / `require`/`include`
//! expressions), CALLS, INHERITS (`extends`), IMPLEMENTS
//! (`implements`), DECORATES (PHP 8 `#[Attribute]`), TYPE_REF
//! (typed parameters/return types), DEFINES.
//!
//! Tests: PHPUnit-style methods -- a method inside a class whose own
//! name starts with `test`, OR that carries a `#[Test]` attribute, OR
//! that lives in a class that `extends TestCase` (last one is a
//! best-effort heuristic scoped to the whole class, not just methods
//! that look test-like by name).
//!
//! Routes: Laravel-style `Route::get('/path', ...)` /
//! `Route::post(...)` static calls, and Symfony-style `#[Route("/path")]`
//! attributes (best-effort, same as ASP.NET's `[Route]` in
//! `languages/csharp.rs`: the literal path as written, no prefix
//! stitching with a class-level `#[Route]`).

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImplementsRef, ImportRef, InheritsRef, ParsedFile, RouteRef,
    SymbolKind, SymbolRef, TypeRefRef,
};
use enforcer_domain::memory_types::ReceiverHint;
use tree_sitter::{Node, Parser};

/// The innermost function/method a call expression is lexically inside
/// of, if any -- see `rust.rs`'s identical `FnScope` for the full
/// rationale.
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

/// Bundled lexical-walk context: the containing class/interface/trait
/// name, the PHPUnit "extends TestCase" flag, and the innermost
/// function/method scope -- grouped into one struct (rather than three
/// positional params) so `walk`/`walk_children` stay under clippy's
/// default too-many-arguments threshold without an `#[allow]`, same
/// "bundle related params" convention as `code_graph.rs`'s
/// `NewFileParams`.
#[derive(Debug, Clone, Copy, Default)]
struct WalkScope<'a> {
    enclosing: Option<&'a str>,
    enclosing_extends_test_case: bool,
    fn_scope: FnScope<'a>,
}

/// HTTP methods recognized in `Route::get(...)`-style static calls.
const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

pub fn parse(source: &str) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
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
        WalkScope::default(),
    );
    out
}

/// `scope.enclosing` is the name of the containing `class`/`interface`/
/// `trait` body (if any). `scope.enclosing_extends_test_case` is set
/// while walking a class body whose own `extends` clause names
/// `TestCase` (any namespace segment) -- every method in that class is
/// tagged [`SymbolKind::Test`], matching PHPUnit's "extends TestCase"
/// contract. `scope.fn_scope` is the innermost function/method a call
/// expression sits inside of.
fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile, scope: WalkScope<'_>) {
    let WalkScope {
        enclosing,
        enclosing_extends_test_case,
        fn_scope,
    } = scope;
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Class,
                    line: line.into(),
                });
                let extends_test_case = emit_class_heritage_edges(node, src, &name, line, out);
                for decorator_name in attribute_names(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: (name.clone()).into(),
                        decorator_name: decorator_name.into(),
                        line: line.into(),
                    });
                }
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: (container.to_string()).into(),
                        member_name: (name.clone()).into(),
                        line: line.into(),
                    });
                }
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        WalkScope {
                            enclosing: Some(name.as_str()),
                            enclosing_extends_test_case: extends_test_case,
                            fn_scope,
                        },
                    );
                }
                return;
            }
        }
        "interface_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Interface,
                    line: line.into(),
                });
                // An interface's `extends` clause lists supertype
                // interfaces (PHP interfaces can extend several) --
                // all modeled as INHERITS, matching TS/C#'s
                // `interface Sub extends Base1, Base2` treatment.
                for base_name in heritage_names(node, src, "extends") {
                    out.inherits.push(InheritsRef {
                        sub_name: (name.clone()).into(),
                        super_name: (base_name).into(),
                        line: line.into(),
                    });
                }
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: (container.to_string()).into(),
                        member_name: (name.clone()).into(),
                        line: line.into(),
                    });
                }
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        WalkScope {
                            enclosing: Some(name.as_str()),
                            enclosing_extends_test_case: false,
                            fn_scope,
                        },
                    );
                }
                return;
            }
        }
        "trait_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Class,
                    line: (node.start_position().row + 1).into(),
                });
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        WalkScope {
                            enclosing: Some(name.as_str()),
                            enclosing_extends_test_case: false,
                            fn_scope,
                        },
                    );
                }
                return;
            }
        }
        "namespace_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(text) = name_node.utf8_text(src) {
                    out.symbols.push(SymbolRef {
                        name: (text.to_string()).into(),
                        kind: SymbolKind::Module,
                        line: (node.start_position().row + 1).into(),
                    });
                }
            }
        }
        "method_declaration" | "function_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let attributes = attribute_names(node, src);
                let is_test = enclosing_extends_test_case
                    || attributes.iter().any(|a| is_test_attribute(a))
                    || (enclosing.is_some() && name.to_lowercase().starts_with("test"));
                let kind = if is_test {
                    SymbolKind::Test
                } else if enclosing.is_some() {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    kind,
                    name: (name.clone()).into(),
                    line: line.into(),
                });
                for type_ref in signature_type_refs(node, src) {
                    out.type_refs.push(TypeRefRef {
                        from_name: (name.clone()).into(),
                        type_name: (type_ref).into(),
                        line: line.into(),
                    });
                }
                for decorator_name in &attributes {
                    out.decorates.push(DecoratesRef {
                        target_name: (name.clone()).into(),
                        decorator_name: (decorator_name.clone()).into(),
                        line: line.into(),
                    });
                }
                for route in routes_from_attributes(node, src, line) {
                    out.routes.push(route);
                }
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: (container.to_string()).into(),
                        member_name: (name.clone()).into(),
                        line: line.into(),
                    });
                }
                let method_scope = FnScope {
                    name: Some(name.as_str()),
                    line: Some(line),
                };
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        WalkScope {
                            enclosing,
                            enclosing_extends_test_case,
                            fn_scope: method_scope,
                        },
                    );
                }
                return;
            }
        }
        "const_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "const_element" {
                    // `const_element`'s own name is a positional
                    // `name` child, not a named field, in this
                    // grammar version (same shape as `attribute`'s
                    // name -- see `attribute_name_text`).
                    let mut child_cursor = child.walk();
                    let name_node = child
                        .children(&mut child_cursor)
                        .find(|candidate| candidate.kind() == "name");
                    if let Some(name_node) = name_node {
                        if let Ok(text) = name_node.utf8_text(src) {
                            let line = node.start_position().row + 1;
                            out.symbols.push(SymbolRef {
                                name: (text.to_string()).into(),
                                kind: SymbolKind::Constant,
                                line: line.into(),
                            });
                            if let Some(container) = enclosing {
                                out.defines.push(DefinesRef {
                                    container_name: (container.to_string()).into(),
                                    member_name: (text.to_string()).into(),
                                    line: line.into(),
                                });
                            }
                        }
                    }
                }
            }
        }
        "namespace_use_declaration" => {
            for path in namespace_use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: (path).into(),
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "expression_statement" => {
            // `require`/`include`(_once) are unary-expression-shaped
            // keyword forms in this grammar, not function calls --
            // handled here rather than in the call-expression arm
            // below.
            if let Some(import) = require_include_import(node, src) {
                out.imports.push(import);
            }
        }
        "function_call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                let callee = function.utf8_text(src).unwrap_or("").to_string();
                out.calls.push(CallRef {
                    callee: (callee.clone()).into(),
                    line: (node.start_position().row + 1).into(),
                    from_symbol: (fn_scope.name.map(str::to_string)).map(Into::into),
                    from_symbol_line: (fn_scope.line).map(Into::into),
                    receiver_text: None,
                    receiver_hint: None,
                    arg_texts: (call_arg_texts(node, src))
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                });
                if let Some(constant) = constant_from_define_call(&callee, node, src) {
                    out.symbols.push(constant);
                }
            }
        }
        "member_call_expression" | "nullsafe_member_call_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                if let Ok(text) = name.utf8_text(src) {
                    let (receiver_text, receiver_hint) = receiver_of_member_call(node, src);
                    out.calls.push(CallRef {
                        callee: (text.to_string()).into(),
                        line: (node.start_position().row + 1).into(),
                        from_symbol: (fn_scope.name.map(str::to_string)).map(Into::into),
                        from_symbol_line: (fn_scope.line).map(Into::into),
                        receiver_text: receiver_text.map(Into::into),
                        receiver_hint,
                        arg_texts: (call_arg_texts(node, src))
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    });
                }
            }
        }
        "scoped_call_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                let callee = qualify_scoped_call(node, name, src);
                let receiver_hint = call_node_scope(node, src).map(|scope| {
                    if scope.rsplit('.').next() == Some("new") || scope == "new" {
                        ReceiverHint::NewExpression
                    } else {
                        ReceiverHint::Other
                    }
                });
                out.calls.push(CallRef {
                    callee: callee.into(),
                    line: (node.start_position().row + 1).into(),
                    from_symbol: (fn_scope.name.map(str::to_string)).map(Into::into),
                    from_symbol_line: (fn_scope.line).map(Into::into),
                    receiver_text: (call_node_scope(node, src)).map(Into::into),
                    receiver_hint,
                    arg_texts: (call_arg_texts(node, src))
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                });
                if let Some(route) = route_from_scoped_call(node, name, src) {
                    out.routes.push(route);
                }
            }
        }
        "assignment_expression" => {
            if let Some(binding) = named_closure_binding(node, src) {
                out.symbols.push(binding);
            }
        }
        _ => {}
    }

    walk_children(node, src, out, scope);
}

fn walk_children(node: Node<'_>, src: &[u8], out: &mut ParsedFile, scope: WalkScope<'_>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, src, out, scope);
    }
}

/// The receiver ($this, a variable, or another expression) of a
/// `member_call_expression`/`nullsafe_member_call_expression`, plus a
/// cheap syntactic hint.
fn receiver_of_member_call(
    call_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    let Some(receiver) = call_node.child_by_field_name("object") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "$this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "object_creation_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "variable_name" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "string" | "encapsed_string" | "integer" | "float"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// A `scoped_call_expression`'s own `scope` field text (`Route` in
/// `Route::get(...)`, `parent`/`self`/a class name in `Foo::bar()`).
fn call_node_scope(call_node: Node<'_>, src: &[u8]) -> Option<String> {
    call_node
        .child_by_field_name("scope")
        .and_then(|s| s.utf8_text(src).ok())
        .map(str::to_string)
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

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Every `name` entry inside a `base_clause`/`class_interface_clause`
/// child of `node` whose own leading keyword token matches `keyword`
/// (`"extends"` or `"implements"`). Neither clause node exposes a
/// named field for its entries in this grammar, so this scans direct
/// children by kind.
fn heritage_names(node: Node<'_>, src: &[u8], keyword: &str) -> Vec<String> {
    let mut out = Vec::new();
    let clause_kind = if keyword == "extends" {
        "base_clause"
    } else {
        "class_interface_clause"
    };
    let mut node_cursor = node.walk();
    let Some(clause) = node
        .children(&mut node_cursor)
        .find(|c| c.kind() == clause_kind)
    else {
        return out;
    };
    let mut clause_cursor = clause.walk();
    for entry in clause.children(&mut clause_cursor) {
        if entry.kind() == "name" || entry.kind() == "qualified_name" {
            if let Ok(text) = entry.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// A `class_declaration`'s `extends` (single base class, PHP forbids
/// multiple class inheritance) and `implements` clauses. Returns
/// whether the base class is (any-namespace-segment) `TestCase`, for
/// the PHPUnit "extends TestCase" test-detection heuristic.
fn emit_class_heritage_edges(
    node: Node<'_>,
    src: &[u8],
    type_name: &str,
    line: usize,
    out: &mut ParsedFile,
) -> bool {
    let mut extends_test_case = false;
    for base_name in heritage_names(node, src, "extends") {
        if base_name.rsplit('\\').next() == Some("TestCase") {
            extends_test_case = true;
        }
        out.inherits.push(InheritsRef {
            sub_name: (type_name.to_string()).into(),
            super_name: (base_name).into(),
            line: line.into(),
        });
    }
    for iface_name in heritage_names(node, src, "implements") {
        out.implements.push(ImplementsRef {
            type_name: (type_name.to_string()).into(),
            trait_name: (iface_name).into(),
            line: line.into(),
        });
    }
    extends_test_case
}

/// PHP 8 `#[Attribute]`/`#[Attribute(...)]`/`#[Attr1, Attr2]` groups
/// directly preceding a declaration.
fn attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for candidate in node.children(&mut cursor) {
        if candidate.kind() != "attribute_list" {
            continue;
        }
        let mut candidate_cursor = candidate.walk();
        for group in candidate.children(&mut candidate_cursor) {
            if group.kind() != "attribute_group" {
                continue;
            }
            let mut group_cursor = group.walk();
            for attr in group.children(&mut group_cursor) {
                if attr.kind() == "attribute" {
                    if let Some(name) = attribute_name_text(attr, src) {
                        out.push(name);
                    }
                }
            }
        }
    }
    out
}

/// An `attribute` node's own name is a positional `name`/`qualified_name`
/// child, not a named field, in this grammar version.
fn attribute_name_text(attr: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = attr.walk();
    let name_node = attr
        .children(&mut cursor)
        .find(|c| matches!(c.kind(), "name" | "qualified_name"))?;
    name_node.utf8_text(src).ok().map(str::to_string)
}

fn is_test_attribute(attribute_name: &str) -> bool {
    matches!(
        attribute_name.rsplit('\\').next().unwrap_or(attribute_name),
        "Test"
    )
}

/// Symfony-style `#[Route("/path")]` attributes on a method.
fn routes_from_attributes(method_node: Node<'_>, src: &[u8], line: usize) -> Vec<RouteRef> {
    let mut out = Vec::new();
    let mut cursor = method_node.walk();
    for candidate in method_node.children(&mut cursor) {
        if candidate.kind() != "attribute_list" {
            continue;
        }
        let mut candidate_cursor = candidate.walk();
        for group in candidate.children(&mut candidate_cursor) {
            if group.kind() != "attribute_group" {
                continue;
            }
            let mut group_cursor = group.walk();
            for attr in group.children(&mut group_cursor) {
                if attr.kind() == "attribute" {
                    if let Some(route) = route_from_symfony_attribute(attr, src, line) {
                        out.push(route);
                    }
                }
            }
        }
    }
    out
}

fn route_from_symfony_attribute(attr: Node<'_>, src: &[u8], line: usize) -> Option<RouteRef> {
    let name = attribute_name_text(attr, src)?;
    if name.rsplit('\\').next().unwrap_or(&name) != "Route" {
        return None;
    }
    let path = attribute_first_string_arg(attr, src).unwrap_or_default();
    Some(RouteRef {
        // Symfony's `#[Route]` is method-agnostic unless a `methods:`
        // argument is given; parsing that named argument's array value
        // is out of scope for this best-effort extractor (same
        // "unresolved, as-written" posture as elsewhere), so the HTTP
        // method is left empty here -- a consumer wanting the verb
        // must inspect the raw attribute text.
        method: (String::new()).into(),
        path: path.into(),
        line: line.into(),
    })
}

fn attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
    // `attribute`'s own argument list is a positional `arguments`
    // child (no named field on this node, unlike
    // `function_call_expression`/`scoped_call_expression` where
    // `arguments` IS a named field).
    let mut attr_cursor = attr.walk();
    let args = attr
        .children(&mut attr_cursor)
        .find(|c| c.kind() == "arguments")?;
    let mut args_cursor = args.walk();
    for arg in args.children(&mut args_cursor) {
        if arg.kind() != "argument" {
            continue;
        }
        let mut arg_cursor = arg.walk();
        for candidate in arg.children(&mut arg_cursor) {
            if matches!(candidate.kind(), "string" | "encapsed_string") {
                if let Ok(text) = candidate.utf8_text(src) {
                    return Some(strip_php_string_literal(text));
                }
            }
        }
    }
    None
}

/// Laravel-style `Route::get('/path', ...)` static calls.
fn route_from_scoped_call(
    call_node: Node<'_>,
    name_node: Node<'_>,
    src: &[u8],
) -> Option<RouteRef> {
    let scope = call_node.child_by_field_name("scope")?;
    let scope_text = scope.utf8_text(src).ok()?;
    if scope_text.rsplit('\\').next().unwrap_or(scope_text) != "Route" {
        return None;
    }
    let method_name = name_node.utf8_text(src).ok()?.to_lowercase();
    if !HTTP_METHODS.contains(&method_name.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let mut args_cursor = args.walk();
    let first_string = args
        .children(&mut args_cursor)
        .filter(|n| n.kind() == "argument")
        .find_map(|arg| {
            let mut arg_cursor = arg.walk();
            let literal = arg
                .children(&mut arg_cursor)
                .find(|c| matches!(c.kind(), "string" | "encapsed_string"));
            literal
        })?;
    let raw = first_string.utf8_text(src).ok()?;
    Some(RouteRef {
        method: (method_name.to_uppercase()).into(),
        path: (strip_php_string_literal(raw)).into(),
        line: (call_node.start_position().row + 1).into(),
    })
}

fn qualify_scoped_call(call_node: Node<'_>, name_node: Node<'_>, src: &[u8]) -> String {
    let name = name_node.utf8_text(src).unwrap_or("");
    match call_node
        .child_by_field_name("scope")
        .and_then(|s| s.utf8_text(src).ok())
    {
        Some(scope) => format!("{scope}::{name}"),
        None => name.to_string(),
    }
}

fn strip_php_string_literal(text: &str) -> String {
    text.trim_matches(|c| c == '"' || c == '\'').to_string()
}

/// `use App\Foo;` / `use App\Foo as Bar;` / `use function App\helper;`
/// / `use App\{Foo, Bar};` -- every concrete imported path.
fn namespace_use_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "namespace_use_clause" => {
                if let Some(path) = namespace_use_clause_path(child, src) {
                    out.push(path);
                }
            }
            "namespace_use_group" => {
                let prefix = child
                    .child_by_field_name("prefix")
                    .and_then(|n| n.utf8_text(src).ok())
                    .unwrap_or("");
                let mut child_cursor = child.walk();
                for clause in child.children(&mut child_cursor) {
                    if clause.kind() == "namespace_use_clause" {
                        if let Some(tail) = namespace_use_clause_path(clause, src) {
                            out.push(if prefix.is_empty() {
                                tail
                            } else {
                                format!("{prefix}\\{tail}")
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if out.is_empty() {
        if let Ok(text) = node.utf8_text(src) {
            let trimmed = text
                .trim_start_matches("use")
                .trim()
                .trim_end_matches(';')
                .trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn namespace_use_clause_path(clause: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if matches!(child.kind(), "qualified_name" | "name") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `require`/`require_once`/`include`/`include_once` expressions --
/// this grammar models each as its own dedicated expression node kind
/// wrapping the target path expression, not a function call.
fn require_include_import(node: Node<'_>, src: &[u8]) -> Option<ImportRef> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "require_expression"
                | "require_once_expression"
                | "include_expression"
                | "include_once_expression"
        ) {
            let mut child_cursor = child.walk();
            for target in child.children(&mut child_cursor) {
                if matches!(target.kind(), "string" | "encapsed_string") {
                    if let Ok(text) = target.utf8_text(src) {
                        return Some(ImportRef {
                            module_path: (strip_php_string_literal(text)).into(),
                            line: (node.start_position().row + 1).into(),
                        });
                    }
                }
            }
        }
    }
    None
}

/// Parameter and return types on a method/function's signature.
fn signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for param in params.children(&mut cursor) {
            if matches!(
                param.kind(),
                "simple_parameter" | "property_promotion_parameter"
            ) {
                if let Some(type_node) = param.child_by_field_name("type") {
                    if let Ok(text) = type_node.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    if let Some(return_type) = node.child_by_field_name("return_type") {
        if let Ok(text) = return_type.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// `$f = function($x) { ... };` / `$g = fn($x) => ...;` -- a named
/// closure/arrow-function binding, best-effort [`SymbolKind::Lambda`]
/// source (mirrors `typescript.rs`'s `const f = () => ...` treatment).
fn named_closure_binding(node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    let left = node.child_by_field_name("left")?;
    if left.kind() != "variable_name" {
        return None;
    }
    let right = node.child_by_field_name("right")?;
    if !matches!(right.kind(), "anonymous_function" | "arrow_function") {
        return None;
    }
    let name = left
        .utf8_text(src)
        .ok()?
        .trim_start_matches('$')
        .to_string();
    Some(SymbolRef {
        name: name.into(),
        kind: SymbolKind::Lambda,
        line: (node.start_position().row + 1).into(),
    })
}

/// `define("NAME", value)` -- PHP's function-call-shaped constant
/// declaration (as opposed to `const NAME = value;` inside a class,
/// handled by the `const_declaration` arm above).
fn constant_from_define_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    if callee != "define" {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let mut args_cursor = args.walk();
    let first_arg = args
        .children(&mut args_cursor)
        .find(|n| n.kind() == "argument")?;
    let mut first_arg_cursor = first_arg.walk();
    let literal = first_arg
        .children(&mut first_arg_cursor)
        .find(|n| matches!(n.kind(), "string" | "encapsed_string"))?;
    let text = literal.utf8_text(src).ok()?;
    let name = strip_php_string_literal(text);
    if name.is_empty() {
        return None;
    }
    Some(SymbolRef {
        name: name.into(),
        kind: SymbolKind::Constant,
        line: (call_node.start_position().row + 1).into(),
    })
}
