//! Java extraction via `tree-sitter-java`: classes, interfaces, enums,
//! methods, static-final constants, the package declaration (as
//! [`SymbolKind::Module`]), imports, calls, INHERITS (`extends`),
//! IMPLEMENTS (`implements`), DECORATES (annotations), TYPE_REF
//! (signature types), DEFINES (class/interface -> member), `@Test`
//! annotation test detection, and best-effort Spring
//! `@GetMapping`/`@PostMapping`/... route extraction.
//!
//! Unresolved by design (same rationale as `rust.rs`/`typescript.rs`):
//! import paths and call callees are recorded as written in source,
//! not resolved to graph node ids here.

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImplementsRef, ImportRef, InheritsRef, ParsedFile, RouteRef,
    SymbolKind, SymbolRef, TypeRefRef,
};
use tree_sitter::{Node, Parser};

/// Spring MVC mapping annotations this extractor recognizes as routes,
/// paired with the HTTP method they imply (`@RequestMapping` is
/// intentionally excluded -- its method comes from a `method =
/// RequestMethod.X` argument this best-effort slice does not parse, so
/// it is honestly skipped rather than guessed at).
const MAPPING_ANNOTATIONS: &[(&str, &str)] = &[
    ("GetMapping", "GET"),
    ("PostMapping", "POST"),
    ("PutMapping", "PUT"),
    ("PatchMapping", "PATCH"),
    ("DeleteMapping", "DELETE"),
];

pub fn parse(source: &str) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
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

/// `enclosing` is the name of the containing class/interface/enum (if
/// any) -- feeds DEFINES edges and, for constants, is not otherwise
/// needed (Java fields/methods are always members of *some* type).
fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile, enclosing: Option<&str>) {
    match node.kind() {
        "package_declaration" => {
            if let Some(name) = package_name(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
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
                if let Some(super_name) = superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                for interface_name in super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                for decorator in annotations(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator,
                        line,
                    });
                }
                walk_children(node, src, out, Some(name.as_str()));
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
                for extended in extends_interfaces(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: extended,
                        line,
                    });
                }
                for decorator in annotations(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator,
                        line,
                    });
                }
                walk_children(node, src, out, Some(name.as_str()));
                return;
            }
        }
        "enum_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
                    line,
                });
                for interface_name in super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                walk_children(node, src, out, Some(name.as_str()));
                return;
            }
        }
        "method_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let decorators = annotations(node, src);
                let kind = if decorators.iter().any(|d| d == "Test") {
                    SymbolKind::Test
                } else if enclosing.is_some() {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                for decorator in decorators {
                    out.decorates.push(DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator.clone(),
                        line,
                    });
                    if let Some(route) = route_from_mapping(&decorator, node, src) {
                        out.routes.push(route);
                    }
                }
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
                        member_name: name,
                        line,
                    });
                }
            }
        }
        "field_declaration" => {
            let modifiers = modifier_texts(node, src);
            let is_constant = modifiers.contains(&"static".to_string())
                && modifiers.contains(&"final".to_string());
            if is_constant {
                let line = node.start_position().row + 1;
                for name in field_names(node, src) {
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
            }
        }
        "import_declaration" => {
            if let Some(path) = import_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "method_invocation" => {
            let callee = method_invocation_callee(node, src);
            out.calls.push(CallRef {
                callee,
                line: node.start_position().row + 1,
                ..CallRef::default()
            });
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

/// `package a.b.c;` -- the dotted package path, flattened from the
/// `scoped_identifier`/`identifier` child.
fn package_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(child.kind(), "scoped_identifier" | "identifier") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `class Sub extends Base` -- the `superclass` clause's type text.
fn superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    let superclass = class_node.child_by_field_name("superclass")?;
    // `superclass` wraps a `_type` child directly (its only named child).
    let type_node = (0..superclass.child_count())
        .filter_map(|i| superclass.child(i))
        .find(|n| n.is_named())?;
    type_node.utf8_text(src).ok().map(str::to_string)
}

/// `class C implements I1, I2` / `enum E implements I1, I2` -- the
/// `super_interfaces` -> `type_list` children.
fn super_interfaces(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(interfaces) = node.child_by_field_name("interfaces") else {
        return out;
    };
    let Some(type_list) = (0..interfaces.child_count())
        .filter_map(|i| interfaces.child(i))
        .find(|n| n.kind() == "type_list")
    else {
        return out;
    };
    for i in 0..type_list.child_count() {
        if let Some(child) = type_list.child(i) {
            if child.is_named() {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// `interface Sub extends Base1, Base2` -- the `extends_interfaces` ->
/// `type_list` children.
fn extends_interfaces(interface_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..interface_node.child_count() {
        let Some(child) = interface_node.child(i) else {
            continue;
        };
        if child.kind() != "extends_interfaces" {
            continue;
        }
        let Some(type_list) = (0..child.child_count())
            .filter_map(|i| child.child(i))
            .find(|n| n.kind() == "type_list")
        else {
            continue;
        };
        for j in 0..type_list.child_count() {
            if let Some(entry) = type_list.child(j) {
                if entry.is_named() {
                    if let Ok(text) = entry.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Annotations (`@Foo`, `@Foo(...)`) attached to a declaration via its
/// `modifiers` child.
fn annotations(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "modifiers" {
            continue;
        }
        for j in 0..child.child_count() {
            let Some(modifier) = child.child(j) else {
                continue;
            };
            match modifier.kind() {
                "marker_annotation" | "annotation" => {
                    if let Some(name_node) = modifier.child_by_field_name("name") {
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

fn modifier_texts(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "modifiers" {
            continue;
        }
        for j in 0..child.child_count() {
            if let Some(modifier) = child.child(j) {
                if !modifier.is_named() {
                    if let Ok(text) = modifier.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// `int A = 1, B = 2;` -- flatten every `variable_declarator`'s name.
fn field_names(field_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..field_node.child_count() {
        let Some(child) = field_node.child(i) else {
            continue;
        };
        if child.kind() != "variable_declarator" {
            continue;
        }
        if let Some(name_node) = child.child_by_field_name("name") {
            if let Ok(text) = name_node.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// Parameter and return types on a `method_declaration`'s signature.
fn signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if param.kind() == "formal_parameter" {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(return_type) = node.child_by_field_name("type") {
        if let Ok(text) = return_type.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// `import a.b.C;` / `import static a.b.C.foo;` / `import a.b.*;` --
/// the dotted path as written (the `static`/wildcard-`*` shape is
/// preserved verbatim in the reconstructed text).
fn import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut parts = Vec::new();
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        match child.kind() {
            "identifier" | "scoped_identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    parts.push(text.to_string());
                }
            }
            "asterisk" => parts.push("*".to_string()),
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

/// `obj.method(...)` / `method(...)` (`object` field absent for a
/// same-class call) -- reconstructs the callee text as written,
/// including the receiver where present.
fn method_invocation_callee(node: Node<'_>, src: &[u8]) -> String {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("");
    match node
        .child_by_field_name("object")
        .and_then(|n| n.utf8_text(src).ok())
    {
        Some(object) => format!("{object}.{name}"),
        None => name.to_string(),
    }
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// `@GetMapping("/path")` / `@PostMapping(path = "/path")` ->
/// [`RouteRef`]. Best-effort: only handles a bare string-literal
/// argument or a single `path = "..."`/`value = "..."` named argument;
/// anything else yields an empty path rather than guessing.
fn route_from_mapping(decorator_name: &str, method_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let http_method = MAPPING_ANNOTATIONS
        .iter()
        .find(|(name, _)| *name == decorator_name)
        .map(|(_, method)| method.to_string())?;
    let path = mapping_path_argument(decorator_name, method_node, src).unwrap_or_default();
    Some(RouteRef {
        method: http_method,
        path,
        line: method_node.start_position().row + 1,
    })
}

fn mapping_path_argument(
    decorator_name: &str,
    method_node: Node<'_>,
    src: &[u8],
) -> Option<String> {
    for i in 0..method_node.child_count() {
        let child = method_node.child(i)?;
        if child.kind() != "modifiers" {
            continue;
        }
        for j in 0..child.child_count() {
            let annotation = child.child(j)?;
            if annotation.kind() != "annotation" {
                continue;
            }
            let name_matches = annotation
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                == Some(decorator_name);
            if !name_matches {
                continue;
            }
            let args = annotation.child_by_field_name("arguments")?;
            for k in 0..args.child_count() {
                let arg = args.child(k)?;
                match arg.kind() {
                    "string_literal" => {
                        let raw = arg.utf8_text(src).ok()?;
                        return Some(raw.trim_matches('"').to_string());
                    }
                    "element_value_pair" => {
                        let key = arg
                            .child_by_field_name("key")
                            .and_then(|n| n.utf8_text(src).ok());
                        if matches!(key, Some("path") | Some("value")) {
                            let value = arg.child_by_field_name("value")?;
                            let raw = value.utf8_text(src).ok()?;
                            return Some(raw.trim_matches('"').to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    None
}
