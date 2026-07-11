//! C#-specific extensions for the generic tree-sitter walker.
//!
//! This module owns C# declarations, attributes, routes, imports, calls, and test classification.

use super::*;

/// HTTP methods recognized in `[Http*]` attributes and `Map*`
/// minimal-API calls -- same list `languages/csharp.rs`'s
/// `HTTP_METHODS` const duplicates.
const CSHARP_HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// Every base-list entry's identifier text -- mirrors
/// `languages/csharp.rs`'s `base_list_names` byte-for-byte.
fn csharp_base_list_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(base_list) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == "base_list")
    else {
        return out;
    };
    for i in 0..base_list.child_count() {
        let Some(entry) = base_list.child(i) else {
            continue;
        };
        if matches!(
            entry.kind(),
            "identifier" | "generic_name" | "qualified_name"
        ) {
            if let Ok(text) = entry.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// Split a `class`/`struct` declaration's base list into INHERITS +
/// IMPLEMENTS -- mirrors `languages/csharp.rs`'s `emit_base_list_edges`
/// byte-for-byte.
fn csharp_emit_base_list_edges(
    node: Node<'_>,
    src: &[u8],
    type_name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let names = csharp_base_list_names(node, src);
    let mut iter = names.into_iter();
    let Some(first) = iter.next() else {
        return;
    };
    if csharp_looks_like_interface_name(&first) {
        out.implements.push(ImplementsRef {
            type_name: type_name.to_string(),
            trait_name: first,
            line,
        });
    } else {
        out.inherits.push(InheritsRef {
            sub_name: type_name.to_string(),
            super_name: first,
            line,
        });
    }
    for remaining in iter {
        out.implements.push(ImplementsRef {
            type_name: type_name.to_string(),
            trait_name: remaining,
            line,
        });
    }
}

/// C# interface-naming convention -- mirrors `languages/csharp.rs`'s
/// `looks_like_interface_name` byte-for-byte.
fn csharp_looks_like_interface_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('I')) && matches!(chars.next(), Some(c) if c.is_uppercase())
}

/// `[Attribute]`/`[Attribute(...)]` lists directly preceding a
/// declaration -- mirrors `languages/csharp.rs`'s `attribute_names`
/// byte-for-byte.
fn csharp_attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(candidate) = node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(attr) = candidate.child(j) {
                if attr.kind() == "attribute" {
                    if let Some(name_node) = attr.child_by_field_name("name") {
                        if let Ok(text) = name_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

fn csharp_is_test_attribute(attribute_name: &str) -> bool {
    matches!(
        attribute_name.rsplit('.').next().unwrap_or(attribute_name),
        "Test" | "Fact" | "TestMethod" | "Theory"
    )
}

/// One route per matching `[Http*("...")]`/`[Route("...")]` attribute
/// on a method -- mirrors `languages/csharp.rs`'s
/// `routes_from_attributes` byte-for-byte.
fn csharp_routes_from_attributes(method_node: Node<'_>, src: &[u8], line: usize) -> Vec<RouteRef> {
    let mut out = Vec::new();
    for i in 0..method_node.child_count() {
        let Some(candidate) = method_node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(attr) = candidate.child(j) {
                if attr.kind() == "attribute" {
                    if let Some(route) = csharp_route_from_attribute(attr, src, line) {
                        out.push(route);
                    }
                }
            }
        }
    }
    out
}

fn csharp_route_from_attribute(attr: Node<'_>, src: &[u8], line: usize) -> Option<RouteRef> {
    let name_node = attr.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?;
    let method = if name.eq_ignore_ascii_case("Route") {
        String::new()
    } else if let Some(stripped) = name.strip_prefix("Http") {
        let candidate = stripped.to_lowercase();
        if !CSHARP_HTTP_METHODS.contains(&candidate.as_str()) {
            return None;
        }
        candidate.to_uppercase()
    } else {
        return None;
    };
    let path = csharp_attribute_first_string_arg(attr, src).unwrap_or_default();
    Some(RouteRef { method, path, line })
}

/// Mirrors `languages/csharp.rs`'s `attribute_first_string_arg`
/// byte-for-byte.
fn csharp_attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
    let args = (0..attr.child_count())
        .filter_map(|i| attr.child(i))
        .find(|c| c.kind() == "attribute_argument_list")?;
    for i in 0..args.child_count() {
        let arg = args.child(i)?;
        let literal = if arg.kind() == "string_literal" {
            Some(arg)
        } else {
            (0..arg.child_count())
                .filter_map(|i| arg.child(i))
                .find(|c| c.kind() == "string_literal")
        };
        if let Some(lit) = literal {
            if let Ok(text) = lit.utf8_text(src) {
                return Some(text.trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Minimal-API endpoint calls (`app.MapGet("/path", handler)`) --
/// mirrors `languages/csharp.rs`'s `route_from_map_call` byte-for-byte,
/// wired as [`Quirks::route_from_call`].
fn csharp_route_from_map_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let last_segment = callee.rsplit('.').next()?;
    let method = last_segment.strip_prefix("Map")?.to_lowercase();
    if !CSHARP_HTTP_METHODS.contains(&method.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "string_literal" || n.kind() == "argument")
        .and_then(|n| {
            if n.kind() == "argument" {
                n.child(0)
            } else {
                Some(n)
            }
        })?;
    let raw = first_string_arg.utf8_text(src).ok()?;
    if !raw.starts_with('"') {
        return None;
    }
    let path = raw.trim_matches('"').to_string();
    Some(RouteRef {
        method: method.to_uppercase(),
        path,
        line: call_node.start_position().row + 1,
    })
}

/// Mirrors `languages/csharp.rs`'s `using_directive_path`
/// byte-for-byte.
fn csharp_using_directive_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    if let Some(alias) = node.child_by_field_name("name") {
        return alias.utf8_text(src).ok().map(str::to_string);
    }
    let text = node.utf8_text(src).ok()?;
    let trimmed = text
        .trim_start_matches("using")
        .trim()
        .trim_start_matches("static")
        .trim()
        .trim_end_matches(';')
        .trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Parameter and return types on a method's signature -- mirrors
/// `languages/csharp.rs`'s `signature_type_refs` byte-for-byte.
fn csharp_signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if param.kind() == "parameter" {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(return_type) = node.child_by_field_name("returns") {
        if let Ok(text) = return_type.utf8_text(src) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

/// Mirrors `languages/csharp.rs`'s `is_const_or_static_readonly`
/// byte-for-byte.
fn csharp_is_const_or_static_readonly(node: Node<'_>, src: &[u8]) -> bool {
    let mut has_const = false;
    let mut has_static = false;
    let mut has_readonly = false;
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "modifier" {
            continue;
        }
        if let Ok(text) = child.utf8_text(src) {
            match text {
                "const" => has_const = true,
                "static" => has_static = true,
                "readonly" => has_readonly = true,
                _ => {}
            }
        }
    }
    has_const || (has_static && has_readonly)
}

/// Mirrors `languages/csharp.rs`'s `field_declarator_names`
/// byte-for-byte.
fn csharp_field_declarator_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(decl) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == "variable_declaration")
    else {
        return out;
    };
    for i in 0..decl.child_count() {
        if let Some(child) = decl.child(i) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(text) = name_node.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Everything C#'s flat `LangSpec` arrays cannot express: class/struct
/// base-list INHERITS+IMPLEMENTS split + attribute DECORATES +
/// DEFINES-scoped body walk, interface base-list all-INHERITS + DEFINES
/// (no scoped body walk -- matches `languages/csharp.rs`'s own
/// `interface_declaration` arm, which never re-scopes `enclosing` to
/// its own name), enum DEFINES (same no-rescoping), namespace Module
/// symbol (also no re-scoping -- `languages/csharp.rs`'s own
/// `namespace_declaration` arm has no scoped recursion either, unlike
/// TS's/C++'s namespace handling), `local_function_statement` as
/// [`SymbolKind::Lambda`] (never Function/Method), `const`/`static
/// readonly`-gated field Constant + DEFINES, and custom `using`-path
/// extraction -- wired as this wave's [`Quirks::on_unmatched_node`]
/// hook. Method-level extras (attribute test-check/type-refs/
/// decorates/routes, expression-bodied-member fallback) are a separate
/// [`Quirks::on_method_defined`] hook (see [`csharp_on_method_defined`]).
fn csharp_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" | "struct_declaration" => {
            // Fully claimed (`true`): both kinds are members of
            // `LangSpec::csharp()`'s own `class_types`, so this quirk
            // is invoked once from the class-shape branch's own early
            // check -- returning `false` would fall through to that
            // branch's `name_field`-keyed fallback, which (unlike C/
            // C++) is *not* neutralized here (C#'s `name_field` really
            // is `"name"`), so it would double-push a second, wrongly-
            // classified (always `SymbolKind::Class`, never `Struct`,
            // and missing every one of the edges/DEFINES below)
            // symbol for the same node. `true` prevents that.
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let kind = if node.kind() == "struct_declaration" {
                    SymbolKind::Struct
                } else {
                    SymbolKind::Class
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                csharp_emit_base_list_edges(node, src, &name, line, out);
                for decorator_name in csharp_attribute_names(node, src) {
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
                csharp_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                csharp_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "interface_declaration" => {
            // Fully claimed for the same double-invocation reason as
            // `class_declaration` above. No scoped body walk (matches
            // the bespoke arm exactly).
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for base_name in csharp_base_list_names(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: base_name,
                        line,
                    });
                }
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: name,
                        line,
                    });
                }
            }
            csharp_walk_scoped(node, src, enclosing, out);
            true
        }
        "enum_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
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
            csharp_walk_scoped(node, src, enclosing, out);
            true
        }
        "namespace_declaration" | "file_scoped_namespace_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
            // `enclosing` unchanged (not re-scoped to the namespace's
            // own name): matches `languages/csharp.rs`'s own
            // `namespace_declaration` arm exactly, which has no scoped
            // recursion of its own.
            csharp_walk_scoped(node, src, enclosing, out);
            true
        }
        "field_declaration" if csharp_is_const_or_static_readonly(node, src) => {
            for name in csharp_field_declarator_names(node, src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Constant,
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
            false
        }
        "local_function_statement" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Lambda,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "using_directive" => {
            if let Some(path) = csharp_using_directive_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        _ => false,
    }
}

/// Generic per-child recursion under `enclosing` -- the shared "just
/// recurse, same as `walk_children`" helper `csharp_quirk`'s arms use.
fn csharp_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::csharp();
    let quirks = csharp_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, FnScope::default());
        }
    }
}

/// Attribute test-check/TYPE_REF/DECORATES/routes for a just-recorded
/// C# method symbol -- wired as [`Quirks::on_method_defined`], mirrors
/// `languages/csharp.rs`'s `method_declaration` arm. Note: the
/// expression-bodied-member fallback (`int F() => x.Y();`, no `body`
/// field) is already handled correctly by the *generic* engine's own
/// func/method branch, which recurses into `spec.body_field`
/// (`"body"`) only `if let Some(body) = ...` -- when absent, it simply
/// does not recurse further there, same as the bespoke arm's own
/// `else` branch falling back to `walk_children(node, ...)` over the
/// whole node: neither call ever finds a body-less method's `=>`
/// expression sibling specially, so this is observably identical
/// without needing its own hook logic.
fn csharp_on_method_defined(
    node: Node<'_>,
    name: &str,
    line: usize,
    src: &[u8],
    out: &mut ParsedFile,
) {
    if node.kind() != "method_declaration" {
        return;
    }
    let attributes = csharp_attribute_names(node, src);
    if attributes.iter().any(|a| csharp_is_test_attribute(a)) {
        if let Some(last) = out.symbols.last_mut() {
            if last.name == name && last.line == line {
                last.kind = SymbolKind::Test;
            }
        }
    }
    for type_ref in csharp_signature_type_refs(node, src) {
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
    for route in csharp_routes_from_attributes(node, src, line) {
        out.routes.push(route);
    }
}

/// C#'s [`Quirks`] row: class/struct base-list INHERITS+IMPLEMENTS +
/// attribute DECORATES + DEFINES-scoped body walk, interface/enum/
/// namespace DEFINES (no re-scoping), local-function Lambda,
/// const/static-readonly field Constant + DEFINES, custom `using`-path
/// extraction, method attribute test-check/TYPE_REF/DECORATES/routes,
/// and minimal-API `Map*` call routes.
pub fn csharp_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(csharp_quirk)),
        is_test_name: |_| false,
        route_from_call: Some(Box::new(csharp_route_from_map_call)),
        on_method_defined: Some(Box::new(csharp_on_method_defined)),
        call_override: None,
    }
}

/// Parse C# source through the generic engine (this wave's C#
/// zero-regression proof against `languages::csharp::parse`).
pub fn parse_csharp(source: &str) -> ParsedFile {
    let spec = LangSpec::csharp();
    let quirks = csharp_quirks();
    let language: tree_sitter::Language = tree_sitter_c_sharp::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}
