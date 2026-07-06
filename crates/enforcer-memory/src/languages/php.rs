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
use tree_sitter::{Node, Parser};

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
    walk(tree.root_node(), source.as_bytes(), &mut out, None, false);
    out
}

/// `enclosing` is the name of the containing `class`/`interface`/`trait`
/// body (if any). `enclosing_extends_test_case` is set while walking a
/// class body whose own `extends` clause names `TestCase` (any
/// namespace segment) -- every method in that class is tagged
/// [`SymbolKind::Test`], matching PHPUnit's "extends TestCase" contract.
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    enclosing_extends_test_case: bool,
) {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                let extends_test_case = emit_class_heritage_edges(node, src, &name, line, out);
                for decorator_name in attribute_names(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name,
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
                    walk_children(body, src, out, Some(name.as_str()), extends_test_case);
                }
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
                // An interface's `extends` clause lists supertype
                // interfaces (PHP interfaces can extend several) --
                // all modeled as INHERITS, matching TS/C#'s
                // `interface Sub extends Base1, Base2` treatment.
                for base_name in heritage_names(node, src, "extends") {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: base_name,
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
                    walk_children(body, src, out, Some(name.as_str()), false);
                }
                return;
            }
        }
        "trait_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line: node.start_position().row + 1,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(body, src, out, Some(name.as_str()), false);
                }
                return;
            }
        }
        "namespace_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(text) = name_node.utf8_text(src) {
                    out.symbols.push(SymbolRef {
                        name: text.to_string(),
                        kind: SymbolKind::Module,
                        line: node.start_position().row + 1,
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
                for decorator_name in &attributes {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator_name.clone(),
                        line,
                    });
                }
                for route in routes_from_attributes(node, src, line) {
                    out.routes.push(route);
                }
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: name,
                        line,
                    });
                }
            }
        }
        "const_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "const_element" {
                        // `const_element`'s own name is a positional
                        // `name` child, not a named field, in this
                        // grammar version (same shape as `attribute`'s
                        // name -- see `attribute_name_text`).
                        let name_node = (0..child.child_count())
                            .filter_map(|j| child.child(j))
                            .find(|c| c.kind() == "name");
                        if let Some(name_node) = name_node {
                            if let Ok(text) = name_node.utf8_text(src) {
                                let line = node.start_position().row + 1;
                                out.symbols.push(SymbolRef {
                                    name: text.to_string(),
                                    kind: SymbolKind::Constant,
                                    line,
                                });
                                if let Some(container) = enclosing {
                                    out.defines.push(DefinesRef {
                                        container_name: container.to_string(),
                                        member_name: text.to_string(),
                                        line,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        "namespace_use_declaration" => {
            for path in namespace_use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
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
                    callee: callee.clone(),
                    line: node.start_position().row + 1,
                });
                if let Some(constant) = constant_from_define_call(&callee, node, src) {
                    out.symbols.push(constant);
                }
            }
        }
        "member_call_expression" | "nullsafe_member_call_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                if let Ok(text) = name.utf8_text(src) {
                    out.calls.push(CallRef {
                        callee: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
        "scoped_call_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                let callee = qualify_scoped_call(node, name, src);
                out.calls.push(CallRef {
                    callee,
                    line: node.start_position().row + 1,
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

    walk_children(node, src, out, enclosing, enclosing_extends_test_case);
}

fn walk_children(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    enclosing_extends_test_case: bool,
) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, src, out, enclosing, enclosing_extends_test_case);
        }
    }
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
    let Some(clause) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == clause_kind)
    else {
        return out;
    };
    for i in 0..clause.child_count() {
        if let Some(entry) = clause.child(i) {
            if entry.kind() == "name" || entry.kind() == "qualified_name" {
                if let Ok(text) = entry.utf8_text(src) {
                    out.push(text.to_string());
                }
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
            sub_name: type_name.to_string(),
            super_name: base_name,
            line,
        });
    }
    for iface_name in heritage_names(node, src, "implements") {
        out.implements.push(ImplementsRef {
            type_name: type_name.to_string(),
            trait_name: iface_name,
            line,
        });
    }
    extends_test_case
}

/// PHP 8 `#[Attribute]`/`#[Attribute(...)]`/`#[Attr1, Attr2]` groups
/// directly preceding a declaration.
fn attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(candidate) = node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(group) = candidate.child(j) {
                if group.kind() != "attribute_group" {
                    continue;
                }
                for k in 0..group.child_count() {
                    if let Some(attr) = group.child(k) {
                        if attr.kind() == "attribute" {
                            if let Some(name) = attribute_name_text(attr, src) {
                                out.push(name);
                            }
                        }
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
    (0..attr.child_count())
        .filter_map(|i| attr.child(i))
        .find(|c| matches!(c.kind(), "name" | "qualified_name"))
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
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
    for i in 0..method_node.child_count() {
        let Some(candidate) = method_node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(group) = candidate.child(j) {
                if group.kind() != "attribute_group" {
                    continue;
                }
                for k in 0..group.child_count() {
                    if let Some(attr) = group.child(k) {
                        if attr.kind() == "attribute" {
                            if let Some(route) = route_from_symfony_attribute(attr, src, line) {
                                out.push(route);
                            }
                        }
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
        method: String::new(),
        path,
        line,
    })
}

fn attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
    // `attribute`'s own argument list is a positional `arguments`
    // child (no named field on this node, unlike
    // `function_call_expression`/`scoped_call_expression` where
    // `arguments` IS a named field).
    let args = (0..attr.child_count())
        .filter_map(|i| attr.child(i))
        .find(|c| c.kind() == "arguments")?;
    for i in 0..args.child_count() {
        let arg = args.child(i)?;
        if arg.kind() != "argument" {
            continue;
        }
        for j in 0..arg.child_count() {
            if let Some(candidate) = arg.child(j) {
                if matches!(candidate.kind(), "string" | "encapsed_string") {
                    if let Ok(text) = candidate.utf8_text(src) {
                        return Some(strip_php_string_literal(text));
                    }
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
    let first_string = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .filter(|n| n.kind() == "argument")
        .find_map(|arg| {
            (0..arg.child_count())
                .filter_map(|i| arg.child(i))
                .find(|c| matches!(c.kind(), "string" | "encapsed_string"))
        })?;
    let raw = first_string.utf8_text(src).ok()?;
    Some(RouteRef {
        method: method_name.to_uppercase(),
        path: strip_php_string_literal(raw),
        line: call_node.start_position().row + 1,
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
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
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
                for j in 0..child.child_count() {
                    if let Some(clause) = child.child(j) {
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
    for i in 0..clause.child_count() {
        let child = clause.child(i)?;
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
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(
            child.kind(),
            "require_expression"
                | "require_once_expression"
                | "include_expression"
                | "include_once_expression"
        ) {
            for j in 0..child.child_count() {
                if let Some(target) = child.child(j) {
                    if matches!(target.kind(), "string" | "encapsed_string") {
                        if let Ok(text) = target.utf8_text(src) {
                            return Some(ImportRef {
                                module_path: strip_php_string_literal(text),
                                line: node.start_position().row + 1,
                            });
                        }
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
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
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
        name,
        kind: SymbolKind::Lambda,
        line: node.start_position().row + 1,
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
    let first_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "argument")?;
    let literal = (0..first_arg.child_count())
        .filter_map(|i| first_arg.child(i))
        .find(|n| matches!(n.kind(), "string" | "encapsed_string"))?;
    let text = literal.utf8_text(src).ok()?;
    let name = strip_php_string_literal(text);
    if name.is_empty() {
        return None;
    }
    Some(SymbolRef {
        name,
        kind: SymbolKind::Constant,
        line: call_node.start_position().row + 1,
    })
}
