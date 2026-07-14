//! PHP-specific extensions for the generic tree-sitter walker.
//!
//! This module owns PHP declarations, attributes, routes, imports, calls, and test classification.

use super::*;

/// HTTP methods recognized in `Route::get(...)`-style static calls --
/// same list `languages/php.rs`'s `HTTP_METHODS` const duplicates.
const PHP_HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// Every `name` entry inside a `base_clause`/`class_interface_clause`
/// child matching `keyword` -- mirrors `languages/php.rs`'s
/// `heritage_names` byte-for-byte.
fn php_heritage_names(node: Node<'_>, src: &[u8], keyword: &str) -> Vec<String> {
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

/// A `class_declaration`'s heritage edges, reporting whether the base
/// class is `TestCase` -- mirrors `languages/php.rs`'s
/// `emit_class_heritage_edges` byte-for-byte.
fn php_emit_class_heritage_edges(
    node: Node<'_>,
    src: &[u8],
    type_name: &str,
    line: usize,
    out: &mut ParsedFile,
) -> bool {
    let mut extends_test_case = false;
    for base_name in php_heritage_names(node, src, "extends") {
        if base_name.rsplit('\\').next() == Some("TestCase") {
            extends_test_case = true;
        }
        out.inherits.push(InheritsRef {
            sub_name: type_name.to_string(),
            super_name: base_name,
            line,
        });
    }
    for iface_name in php_heritage_names(node, src, "implements") {
        out.implements.push(ImplementsRef {
            type_name: type_name.to_string(),
            trait_name: iface_name,
            line,
        });
    }
    extends_test_case
}

/// PHP 8 `#[Attribute]`/`#[Attribute(...)]` groups directly preceding
/// a declaration -- mirrors `languages/php.rs`'s `attribute_names`
/// byte-for-byte.
fn php_attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
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
                    if let Some(name) = php_attribute_name_text(attr, src) {
                        out.push(name);
                    }
                }
            }
        }
    }
    out
}

/// Mirrors `languages/php.rs`'s `attribute_name_text` byte-for-byte.
fn php_attribute_name_text(attr: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = attr.walk();
    let name_node = attr
        .children(&mut cursor)
        .find(|c| matches!(c.kind(), "name" | "qualified_name"))?;
    name_node.utf8_text(src).ok().map(str::to_string)
}

fn php_is_test_attribute(attribute_name: &str) -> bool {
    matches!(
        attribute_name.rsplit('\\').next().unwrap_or(attribute_name),
        "Test"
    )
}

/// Symfony-style `#[Route("/path")]` attributes on a method -- mirrors
/// `languages/php.rs`'s `routes_from_attributes` byte-for-byte.
fn php_routes_from_attributes(method_node: Node<'_>, src: &[u8], line: usize) -> Vec<RouteRef> {
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
                    if let Some(route) = php_route_from_symfony_attribute(attr, src, line) {
                        out.push(route);
                    }
                }
            }
        }
    }
    out
}

fn php_route_from_symfony_attribute(attr: Node<'_>, src: &[u8], line: usize) -> Option<RouteRef> {
    let name = php_attribute_name_text(attr, src)?;
    if name.rsplit('\\').next().unwrap_or(&name) != "Route" {
        return None;
    }
    let path = php_attribute_first_string_arg(attr, src).unwrap_or_default();
    Some(RouteRef {
        method: String::new(),
        path,
        line,
    })
}

fn php_attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
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
                    return Some(php_strip_string_literal(text));
                }
            }
        }
    }
    None
}

/// Laravel-style `Route::get('/path', ...)` static calls -- mirrors
/// `languages/php.rs`'s `route_from_scoped_call` byte-for-byte.
fn php_route_from_scoped_call(
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
    if !PHP_HTTP_METHODS.contains(&method_name.as_str()) {
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
        method: method_name.to_uppercase(),
        path: php_strip_string_literal(raw),
        line: call_node.start_position().row + 1,
    })
}

fn php_qualify_scoped_call(call_node: Node<'_>, name_node: Node<'_>, src: &[u8]) -> String {
    let name = name_node.utf8_text(src).unwrap_or("");
    match call_node
        .child_by_field_name("scope")
        .and_then(|s| s.utf8_text(src).ok())
    {
        Some(scope) => format!("{scope}::{name}"),
        None => name.to_string(),
    }
}

fn php_strip_string_literal(text: &str) -> String {
    text.trim_matches(|c| c == '"' || c == '\'').to_string()
}

/// Mirrors `languages/php.rs`'s `namespace_use_paths` byte-for-byte.
fn php_namespace_use_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "namespace_use_clause" => {
                if let Some(path) = php_namespace_use_clause_path(child, src) {
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
                        if let Some(tail) = php_namespace_use_clause_path(clause, src) {
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

fn php_namespace_use_clause_path(clause: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if matches!(child.kind(), "qualified_name" | "name") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `require`/`require_once`/`include`/`include_once` expressions --
/// mirrors `languages/php.rs`'s `require_include_import` byte-for-byte.
fn php_require_include_import(node: Node<'_>, src: &[u8]) -> Option<ImportRef> {
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
                            module_path: php_strip_string_literal(text),
                            line: node.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }
    None
}

/// Parameter and return types on a method/function's signature --
/// mirrors `languages/php.rs`'s `signature_type_refs` byte-for-byte.
fn php_signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// `$f = function($x) { ... };` / `$g = fn($x) => ...;` -- mirrors
/// `languages/php.rs`'s `named_closure_binding` byte-for-byte.
fn php_named_closure_binding(node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
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

/// `define("NAME", value)` -- mirrors `languages/php.rs`'s
/// `constant_from_define_call` byte-for-byte.
fn php_constant_from_define_call(
    callee: &str,
    call_node: Node<'_>,
    src: &[u8],
) -> Option<SymbolRef> {
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
    let name = php_strip_string_literal(text);
    if name.is_empty() {
        return None;
    }
    Some(SymbolRef {
        name,
        kind: SymbolKind::Constant,
        line: call_node.start_position().row + 1,
    })
}

/// Each argument expression's own source text -- mirrors
/// `languages/php.rs`'s `call_arg_texts` byte-for-byte.
fn php_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// The receiver of a `member_call_expression`/
/// `nullsafe_member_call_expression`, plus a cheap syntactic hint --
/// mirrors `languages/php.rs`'s `receiver_of_member_call`
/// byte-for-byte.
fn php_receiver_of_member_call(
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

/// PHPUnit "extends TestCase" test-detection: walk up from a method
/// node to its lexically-enclosing `class_declaration` (if any) and
/// check whether *that* class's own heritage names `TestCase` --
/// computed on demand from tree structure alone (via `Node::parent()`)
/// rather than threaded state, since [`Quirks::on_method_defined`]'s
/// signature carries no extra per-language scope field the way
/// `languages/php.rs`'s own `WalkScope::enclosing_extends_test_case`
/// does. Observably identical: a method's enclosing class in valid PHP
/// is always its nearest `class_declaration` ancestor, so re-deriving
/// it this way finds the exact same class the bespoke walk was already
/// tracking.
fn php_enclosing_extends_test_case(method_node: Node<'_>, src: &[u8]) -> bool {
    let mut current = method_node.parent();
    while let Some(node) = current {
        if node.kind() == "class_declaration" {
            return php_heritage_names(node, src, "extends")
                .iter()
                .any(|base_name| base_name.rsplit('\\').next() == Some("TestCase"));
        }
        current = node.parent();
    }
    false
}

/// Everything PHP's flat `LangSpec` arrays cannot express: class/trait/
/// interface declarations (heritage, attributes, DEFINES-scoped body
/// walk), namespace Module symbol, class constants (`const NAME = ..`),
/// `use`-path imports, `require`/`include` imports, named-closure-
/// binding Lambda, and all four call-shaped node kinds (each with its
/// own field-name shape, receiver logic, and route detection) -- wired
/// as this wave's [`Quirks::on_unmatched_node`] hook. Method-level
/// extras (attribute test-check/type-refs/decorates/routes, PHPUnit
/// "extends TestCase"/`test`-name-inside-class detection) are a
/// separate [`Quirks::on_method_defined`] hook (see
/// [`php_on_method_defined`]).
fn php_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                php_emit_class_heritage_edges(node, src, &name, line, out);
                for decorator_name in php_attribute_names(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
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
                php_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                php_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "trait_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line: node.start_position().row + 1,
                });
                php_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                php_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "interface_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for base_name in php_heritage_names(node, src, "extends") {
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
                php_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                php_walk_scoped(node, src, enclosing, out);
            }
            true
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
            false
        }
        "const_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "const_element" {
                    let mut child_cursor = child.walk();
                    let name_node = child
                        .children(&mut child_cursor)
                        .find(|candidate| candidate.kind() == "name");
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
            false
        }
        "namespace_use_declaration" => {
            for path in php_namespace_use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "expression_statement" => {
            if let Some(import) = php_require_include_import(node, src) {
                out.imports.push(import);
            }
            false
        }
        // No arms for `function_call_expression`/
        // `member_call_expression`/`nullsafe_member_call_expression`/
        // `scoped_call_expression` here: all four are members of
        // `LangSpec::php()`'s own `call_types`, so `walk`'s call
        // branch invokes [`php_call_override`] for them *before* this
        // `on_unmatched_node` catch-all would ever run -- see
        // `LangSpec::php()`'s own doc comment for why `call_override`
        // (which receives the caller's `fn_scope`) is used instead of
        // handling these here (this hook's signature carries no
        // `fn_scope`, only `enclosing`).
        "assignment_expression" => {
            if let Some(binding) = php_named_closure_binding(node, src) {
                out.symbols.push(binding);
            }
            false
        }
        _ => false,
    }
}

/// Generic per-child recursion under `enclosing` -- the shared "just
/// recurse, same as `walk_children`" helper `php_quirk`'s
/// class/interface/trait arms use. `fn_scope` resets to
/// [`FnScope::default`] rather than threading through whatever the
/// caller had: a class/interface/trait *declaration* itself is not
/// call-shaped, so the only way this would ever observably differ from
/// the bespoke walk is a call appearing directly inside a nested
/// class's field initializer while that whole class declaration is
/// itself lexically inside another function's body (PHP permits
/// declaring a class inside a function) -- an edge case no existing
/// fixture/test exercises either way.
fn php_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::php();
    let quirks = php_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, &ctx, out, enclosing, FnScope::default());
    }
}

/// Full override for all four of PHP's call-shaped node kinds -- wired
/// as [`Quirks::call_override`] (not `on_unmatched_node`) specifically
/// so `from_symbol`/`from_symbol_line` can be populated from the
/// caller's actual `fn_scope` (`on_unmatched_node`'s signature carries
/// no such parameter, only `enclosing`), matching
/// `languages/php.rs`'s own four call-shape arms byte-for-byte.
fn php_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "function_call_expression" => {
            let Some(function) = node.child_by_field_name("function") else {
                return false;
            };
            let callee = function.utf8_text(src).unwrap_or("").to_string();
            out.calls.push(CallRef {
                callee: callee.clone(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: php_call_arg_texts(node, src),
            });
            if let Some(constant) = php_constant_from_define_call(&callee, node, src) {
                out.symbols.push(constant);
            }
            true
        }
        "member_call_expression" | "nullsafe_member_call_expression" => {
            let Some(name) = node.child_by_field_name("name") else {
                return false;
            };
            let Ok(text) = name.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = php_receiver_of_member_call(node, src);
            out.calls.push(CallRef {
                callee: text.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: php_call_arg_texts(node, src),
            });
            true
        }
        "scoped_call_expression" => {
            let Some(name) = node.child_by_field_name("name") else {
                return false;
            };
            let callee = php_qualify_scoped_call(node, name, src);
            let receiver_hint = node
                .child_by_field_name("scope")
                .and_then(|s| s.utf8_text(src).ok())
                .map(|scope| {
                    if scope.rsplit('.').next() == Some("new") || scope == "new" {
                        ReceiverHint::NewExpression
                    } else {
                        ReceiverHint::Other
                    }
                });
            out.calls.push(CallRef {
                callee,
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: node
                    .child_by_field_name("scope")
                    .and_then(|s| s.utf8_text(src).ok())
                    .map(str::to_string),
                receiver_hint,
                arg_texts: php_call_arg_texts(node, src),
            });
            if let Some(route) = php_route_from_scoped_call(node, name, src) {
                out.routes.push(route);
            }
            true
        }
        _ => false,
    }
}

/// Attribute test-check/TYPE_REF/DECORATES/routes plus PHPUnit
/// "extends TestCase"/`test`-name-inside-class reclassification for a
/// just-recorded PHP method/function symbol -- wired as
/// [`Quirks::on_method_defined`], mirrors `languages/php.rs`'s
/// `method_declaration`/`function_definition` arm.
fn php_on_method_defined(
    node: Node<'_>,
    name: &str,
    line: usize,
    src: &[u8],
    out: &mut ParsedFile,
) {
    if !matches!(node.kind(), "method_declaration" | "function_definition") {
        return;
    }
    let attributes = php_attribute_names(node, src);
    let enclosing_is_method = node.kind() == "method_declaration";
    let is_test = php_enclosing_extends_test_case(node, src)
        || attributes.iter().any(|a| php_is_test_attribute(a))
        || (enclosing_is_method && name.to_lowercase().starts_with("test"));
    if is_test {
        if let Some(last) = out.symbols.last_mut() {
            if last.name == name && last.line == line {
                last.kind = SymbolKind::Test;
            }
        }
    }
    for type_ref in php_signature_type_refs(node, src) {
        out.type_refs.push(crate::parsers::TypeRefRef {
            from_name: name.to_string(),
            type_name: type_ref,
            line,
        });
    }
    for decorator_name in &attributes {
        out.decorates.push(crate::parsers::DecoratesRef {
            target_name: name.to_string(),
            decorator_name: decorator_name.clone(),
            line,
        });
    }
    for route in php_routes_from_attributes(node, src, line) {
        out.routes.push(route);
    }
}

/// PHP's [`Quirks`] row: everything (see `LangSpec::php()`'s doc
/// comment for why `call_types` is empty and every call shape is
/// claimed directly by [`php_quirk`]).
pub fn php_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(php_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: Some(Box::new(php_on_method_defined)),
        call_override: Some(Box::new(php_call_override)),
    }
}

/// Parse PHP source through the generic engine (this wave's PHP
/// zero-regression proof against `languages::php::parse`).
pub fn parse_php(source: &str) -> ParsedFile {
    let spec = LangSpec::php();
    let quirks = php_quirks();
    let language: tree_sitter::Language = tree_sitter_php::LANGUAGE_PHP.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}
