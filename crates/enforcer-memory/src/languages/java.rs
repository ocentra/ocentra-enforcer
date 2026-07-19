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
use enforcer_domain::memory_types::ReceiverHint;
use tree_sitter::{Node, Parser};

/// The innermost method a call expression is lexically inside of, if
/// any -- same bundled-scope pattern as `rust.rs`'s `FnScope` (a call's
/// "from_symbol" is the enclosing method, threaded alongside
/// `enclosing`, which stays the containing *type* name for DEFINES).
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

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
    walk(
        tree.root_node(),
        source.as_bytes(),
        &mut out,
        None,
        FnScope::default(),
    );
    out
}

/// `enclosing` is the name of the containing class/interface/enum (if
/// any) -- feeds DEFINES edges and, for constants, is not otherwise
/// needed (Java fields/methods are always members of *some* type).
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "package_declaration" => {
            if let Some(name) = package_name(node, src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Module,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Class,
                    line: line.into(),
                });
                if let Some(super_name) = superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: (name.clone()).into(),
                        super_name: super_name.into(),
                        line: line.into(),
                    });
                }
                for interface_name in super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: (name.clone()).into(),
                        trait_name: (interface_name).into(),
                        line: line.into(),
                    });
                }
                for decorator in annotations(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: (name.clone()).into(),
                        decorator_name: (decorator).into(),
                        line: line.into(),
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
                    name: (name.clone()).into(),
                    kind: SymbolKind::Interface,
                    line: line.into(),
                });
                for extended in extends_interfaces(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: (name.clone()).into(),
                        super_name: (extended).into(),
                        line: line.into(),
                    });
                }
                for decorator in annotations(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: (name.clone()).into(),
                        decorator_name: (decorator).into(),
                        line: line.into(),
                    });
                }
                walk_children(node, src, out, Some(name.as_str()), fn_scope);
                return;
            }
        }
        "enum_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Enum,
                    line: line.into(),
                });
                for interface_name in super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: (name.clone()).into(),
                        trait_name: (interface_name).into(),
                        line: line.into(),
                    });
                }
                walk_children(node, src, out, Some(name.as_str()), fn_scope);
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
                    name: (name.clone()).into(),
                    kind,
                    line: line.into(),
                });
                for decorator in decorators {
                    out.decorates.push(DecoratesRef {
                        target_name: (name.clone()).into(),
                        decorator_name: (decorator.clone()).into(),
                        line: line.into(),
                    });
                    if let Some(route) = route_from_mapping(&decorator, node, src) {
                        out.routes.push(route);
                    }
                }
                for type_ref in signature_type_refs(node, src) {
                    out.type_refs.push(TypeRefRef {
                        from_name: (name.clone()).into(),
                        type_name: (type_ref).into(),
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
        "field_declaration" => {
            let modifiers = modifier_texts(node, src);
            let is_constant = modifiers.contains(&"static".to_string())
                && modifiers.contains(&"final".to_string());
            if is_constant {
                let line = node.start_position().row + 1;
                for name in field_names(node, src) {
                    out.symbols.push(SymbolRef {
                        name: (name.clone()).into(),
                        kind: SymbolKind::Constant,
                        line: line.into(),
                    });
                    if let Some(container) = enclosing {
                        out.defines.push(DefinesRef {
                            container_name: (container.to_string()).into(),
                            member_name: (name).into(),
                            line: line.into(),
                        });
                    }
                }
            }
        }
        "import_declaration" => {
            if let Some(path) = import_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: (path).into(),
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "method_invocation" => {
            let callee = method_invocation_callee(node, src);
            let (receiver_text, receiver_hint) = receiver_of_call(node, src);
            out.calls.push(CallRef {
                callee: callee.into(),
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

/// X06 type-aware resolution: for a `method_invocation` with an
/// `object` field (`x.foo(...)`, `this.foo(...)`), the receiver's own
/// text plus a cheap syntactic hint. `None`/`None` for an unqualified
/// same-class call (`foo(...)`) -- there is no receiver to report.
fn receiver_of_call(
    invocation_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    let Some(receiver) = invocation_node.child_by_field_name("object") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if receiver.kind() == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "object_creation_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "string_literal"
            | "decimal_integer_literal"
            | "hex_integer_literal"
            | "octal_integer_literal"
            | "binary_integer_literal"
            | "decimal_floating_point_literal"
            | "hex_floating_point_literal"
            | "character_literal"
            | "true"
            | "false"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Each argument expression's own source text, in written order.
fn call_arg_texts(invocation_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = invocation_node.child_by_field_name("arguments") else {
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

/// `package a.b.c;` -- the dotted package path, flattened from the
/// `scoped_identifier`/`identifier` child.
fn package_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
    let mut cursor = superclass.walk();
    let type_node = superclass.children(&mut cursor).find(|n| n.is_named())?;
    type_node.utf8_text(src).ok().map(str::to_string)
}

/// `class C implements I1, I2` / `enum E implements I1, I2` -- the
/// `super_interfaces` -> `type_list` children.
fn super_interfaces(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(interfaces) = node.child_by_field_name("interfaces") else {
        return out;
    };
    let mut interfaces_cursor = interfaces.walk();
    let Some(type_list) = interfaces
        .children(&mut interfaces_cursor)
        .find(|n| n.kind() == "type_list")
    else {
        return out;
    };
    let mut cursor = type_list.walk();
    for child in type_list.children(&mut cursor) {
        if child.is_named() {
            if let Ok(text) = child.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// `interface Sub extends Base1, Base2` -- the `extends_interfaces` ->
/// `type_list` children.
fn extends_interfaces(interface_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = interface_node.walk();
    for child in interface_node.children(&mut cursor) {
        if child.kind() != "extends_interfaces" {
            continue;
        }
        let mut child_cursor = child.walk();
        let Some(type_list) = child
            .children(&mut child_cursor)
            .find(|n| n.kind() == "type_list")
        else {
            continue;
        };
        let mut type_list_cursor = type_list.walk();
        for entry in type_list.children(&mut type_list_cursor) {
            if entry.is_named() {
                if let Ok(text) = entry.utf8_text(src) {
                    out.push(text.to_string());
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "modifiers" {
            continue;
        }
        let mut modifier_cursor = child.walk();
        for modifier in child.children(&mut modifier_cursor) {
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "modifiers" {
            continue;
        }
        let mut modifier_cursor = child.walk();
        for modifier in child.children(&mut modifier_cursor) {
            if !modifier.is_named() {
                if let Ok(text) = modifier.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// `int A = 1, B = 2;` -- flatten every `variable_declarator`'s name.
fn field_names(field_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = field_node.walk();
    for child in field_node.children(&mut cursor) {
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
        let mut cursor = params.walk();
        for param in params.children(&mut cursor) {
            if param.kind() == "formal_parameter" {
                if let Some(type_node) = param.child_by_field_name("type") {
                    if let Ok(text) = type_node.utf8_text(src) {
                        out.push(text.to_string());
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
        method: (http_method).into(),
        path: path.into(),
        line: (method_node.start_position().row + 1).into(),
    })
}

fn mapping_path_argument(
    decorator_name: &str,
    method_node: Node<'_>,
    src: &[u8],
) -> Option<String> {
    let mut cursor = method_node.walk();
    for child in method_node.children(&mut cursor) {
        if child.kind() != "modifiers" {
            continue;
        }
        let mut annotation_cursor = child.walk();
        for annotation in child.children(&mut annotation_cursor) {
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
            let mut args_cursor = args.walk();
            for arg in args.children(&mut args_cursor) {
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
