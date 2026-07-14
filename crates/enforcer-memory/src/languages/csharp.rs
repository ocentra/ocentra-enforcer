//! C# extraction via `tree-sitter-c-sharp`.
//!
//! Symbols: `Class`, `Interface`/`SymbolKind::Interface`, `Struct`,
//! `Enum`, `Method`, `Constant` (`const`/`static readonly` fields),
//! `Module` (`namespace`), and named `Lambda` (a local function or a
//! named delegate/`Func<..>` variable bound to a lambda expression).
//! Edges: IMPORTS (`using`), CALLS, INHERITS (base class -- the first
//! entry in a `base_list` that is not one of the type's own
//! `implements` targets is heuristically treated as the base class,
//! same ambiguity the C# language itself has at the syntax level: a
//! `base_list` entry could be a class or an interface, and only symbol
//! resolution -- out of scope for this syntactic extractor -- can tell
//! them apart for certain), IMPLEMENTS (best-effort: conventionally
//! interface names in C# start with `I` followed by an uppercase
//! letter; entries matching that convention are modeled as IMPLEMENTS,
//! everything else in the list after the first entry is also IMPLEMENTS
//! since C# forbids multiple class inheritance), DECORATES
//! (`[Attribute]`), TYPE_REF (parameter/return types), DEFINES.
//!
//! Tests: methods carrying a `[Test]`/`[Fact]`/`[TestMethod]` attribute
//! (NUnit/xUnit/MSTest respectively) are tagged [`SymbolKind::Test`]
//! instead of [`SymbolKind::Method`].
//!
//! Routes: ASP.NET attribute routing (`[HttpGet("...")]`,
//! `[HttpPost]`, ..., `[Route("...")]` on a method or combined with a
//! controller-level `[Route("...")]` prefix is NOT stitched together --
//! this extractor records each attribute's own literal path, matching
//! this crate's "unresolved, as-written" edge philosophy elsewhere) and
//! minimal-API endpoint calls (`app.MapGet("/path", ...)`,
//! `app.MapPost(...)`, etc).

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImplementsRef, ImportRef, InheritsRef, ParsedFile,
    ReceiverHint, RouteRef, SymbolKind, SymbolRef, TypeRefRef,
};
use tree_sitter::{Node, Parser};

/// The innermost method a call expression is lexically inside of, if
/// any -- see `rust.rs`'s identical `FnScope` for the full rationale.
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

/// HTTP methods recognized in `[Http*]` attributes and `Map*`
/// minimal-API calls (`[HttpGet]` -> `get`, `app.MapGet(...)` -> `get`).
const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

pub fn parse(source: &str) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
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

/// `enclosing` is the name of the containing `class`/`struct`/
/// `interface` body (if any) -- distinguishes a member method from a
/// free function (C# has none at file scope, but local functions still
/// route through [`SymbolKind::Function`] when `enclosing` is `None`)
/// and feeds DEFINES edges. `fn_scope` is the innermost method a call
/// expression sits inside of.
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "class_declaration" | "struct_declaration" => {
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
                emit_base_list_edges(node, src, &name, line, out);
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
                // An interface's `base_list` entries are all supertype
                // interfaces (C# interfaces cannot extend a class) --
                // model every entry as INHERITS, matching TS's
                // `interface Sub extends Base1, Base2` treatment.
                for base_name in base_list_names(node, src) {
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
        }
        "namespace_declaration" | "file_scoped_namespace_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
        }
        "method_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let attributes = attribute_names(node, src);
                let kind = if attributes.iter().any(|a| is_test_attribute(a)) {
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
                        member_name: name.clone(),
                        line,
                    });
                }
                let method_scope = FnScope {
                    name: Some(name.as_str()),
                    line: Some(line),
                };
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(body, src, out, enclosing, method_scope);
                } else {
                    // Expression-bodied member (`int F() => x.Y();`): no
                    // `body` field, but an `=>` expression sibling still
                    // needs walking with this method as `fn_scope`.
                    walk_children(node, src, out, enclosing, method_scope);
                }
                return;
            }
        }
        "field_declaration" if is_const_or_static_readonly(node, src) => {
            for name in field_declarator_names(node, src) {
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
        }
        "local_function_statement" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Lambda,
                    line: node.start_position().row + 1,
                });
            }
        }
        "using_directive" => {
            if let Some(path) = using_directive_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "invocation_expression" => {
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
                if let Some(route) = route_from_map_call(&callee, node, src) {
                    out.routes.push(route);
                }
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

/// For a `member_access_expression`-shaped callee (`x.Foo()`), the
/// receiver text plus a cheap syntactic hint.
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "member_access_expression" {
        return (None, None);
    }
    let Some(receiver) = function_node.child_by_field_name("expression") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "object_creation_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "string_literal" | "integer_literal" | "real_literal" | "boolean_literal"
    ) {
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

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Every base-list entry's identifier text (`class C : Base, I1, I2` /
/// `interface I : Base1, Base2`), in source order. `tree-sitter-c-
/// sharp`'s `base_list` node has no named field for its entries (only
/// the literal `:`/`,` tokens plus each type node as a positional
/// child), so this scans direct children by kind.
fn base_list_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(base_list) = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "base_list")
    else {
        return out;
    };
    for entry in base_list.children(&mut base_list.walk()) {
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

/// Split a `class`/`struct` declaration's base list into one INHERITS
/// edge (the base class, at most one -- C# forbids multiple class
/// inheritance) and IMPLEMENTS edges (every remaining entry). The first
/// entry is heuristically the base class UNLESS it matches the `I`+
/// uppercase interface-naming convention, in which case the whole list
/// is treated as interfaces only (a class with no explicit base still
/// implicitly derives from `object`, which this extractor does not
/// model as an edge).
fn emit_base_list_edges(
    node: Node<'_>,
    src: &[u8],
    type_name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let names = base_list_names(node, src);
    let mut iter = names.into_iter();
    let Some(first) = iter.next() else {
        return;
    };
    if looks_like_interface_name(&first) {
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

/// C# interface-naming convention: `I` followed by an uppercase letter
/// (`IFoo`, `IEnumerable`). Best-effort only -- a base-list entry that
/// is actually an interface but does not follow this convention is
/// still modeled as INHERITS (the same ambiguity a purely syntactic
/// extractor cannot resolve without symbol lookup).
fn looks_like_interface_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('I')) && matches!(chars.next(), Some(c) if c.is_uppercase())
}

/// `[Attribute]`/`[Attribute(...)]`/`[Attr1, Attr2]` lists directly
/// preceding a declaration. `tree-sitter-c-sharp` attaches these as
/// preceding `attribute_list` siblings.
fn attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for candidate in node.children(&mut node.walk()) {
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for attr in candidate.children(&mut candidate.walk()) {
            if attr.kind() == "attribute" {
                if let Some(name_node) = attr.child_by_field_name("name") {
                    if let Ok(text) = name_node.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

fn is_test_attribute(attribute_name: &str) -> bool {
    matches!(
        attribute_name.rsplit('.').next().unwrap_or(attribute_name),
        "Test" | "Fact" | "TestMethod" | "Theory"
    )
}

/// One route per matching `[Http*("...")]`/`[Route("...")]` attribute
/// on a method, as-written (no controller-prefix stitching -- see
/// module doc).
fn routes_from_attributes(method_node: Node<'_>, src: &[u8], line: usize) -> Vec<RouteRef> {
    let mut out = Vec::new();
    for candidate in method_node.children(&mut method_node.walk()) {
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for attr in candidate.children(&mut candidate.walk()) {
            if attr.kind() == "attribute" {
                if let Some(route) = route_from_attribute(attr, src, line) {
                    out.push(route);
                }
            }
        }
    }
    out
}

fn route_from_attribute(attr: Node<'_>, src: &[u8], line: usize) -> Option<RouteRef> {
    let name_node = attr.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?;
    let method = if name.eq_ignore_ascii_case("Route") {
        // A bare `[Route("...")]` on a method with no `[Http*]` sibling
        // is HTTP-method-agnostic in ASP.NET (matches any verb) --
        // recorded with an empty method rather than guessing GET, so a
        // consumer can tell the two cases apart.
        String::new()
    } else if let Some(stripped) = name.strip_prefix("Http") {
        let candidate = stripped.to_lowercase();
        if !HTTP_METHODS.contains(&candidate.as_str()) {
            return None;
        }
        candidate.to_uppercase()
    } else {
        return None;
    };
    let path = attribute_first_string_arg(attr, src).unwrap_or_default();
    Some(RouteRef { method, path, line })
}

/// `attribute`'s argument list is an `attribute_argument_list` node
/// (no named field, since the grammar exposes the identifier as
/// `name` but leaves the argument list as a plain positional child) --
/// scan for the first `string_literal` inside it.
fn attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
    let args = attr
        .children(&mut attr.walk())
        .find(|c| c.kind() == "attribute_argument_list")?;
    for arg in args.children(&mut args.walk()) {
        // Each argument is an `attribute_argument` wrapping the
        // literal (or a bare literal for simple positional args,
        // depending on grammar version) -- search either shape.
        let literal = if arg.kind() == "string_literal" {
            Some(arg)
        } else {
            arg.children(&mut arg.walk())
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

/// Minimal-API endpoint calls: `app.MapGet("/path", handler)`,
/// `app.MapPost(...)`, etc.
fn route_from_map_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let last_segment = callee.rsplit('.').next()?;
    let method = last_segment.strip_prefix("Map")?.to_lowercase();
    if !HTTP_METHODS.contains(&method.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string_arg = args
        .children(&mut args.walk())
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

/// `using System.Collections.Generic;` / `using Foo = Bar.Baz;` /
/// `using static System.Math;`.
fn using_directive_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    if let Some(alias) = node.child_by_field_name("name") {
        return alias.utf8_text(src).ok().map(str::to_string);
    }
    // Fallback: strip the leading `using`/`static` keywords and
    // trailing `;` from the raw text.
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

/// Parameter and return types on a method's signature.
fn signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for param in params.children(&mut params.walk()) {
            if param.kind() == "parameter" {
                if let Some(type_node) = param.child_by_field_name("type") {
                    if let Ok(text) = type_node.utf8_text(src) {
                        out.push(text.to_string());
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

/// `const int X = 1;` / `static readonly string Y = "z";` -- a field
/// declaration is treated as a constant when it carries `const`, or
/// both `static` and `readonly` modifiers. `field_declaration` has no
/// named `modifiers` field -- each modifier is its own positional
/// `modifier` child wrapping the literal keyword token.
fn is_const_or_static_readonly(node: Node<'_>, src: &[u8]) -> bool {
    let mut has_const = false;
    let mut has_static = false;
    let mut has_readonly = false;
    for child in node.children(&mut node.walk()) {
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

/// A `field_declaration` may declare several comma-separated names
/// (`const int X = 1, Y = 2;`) inside its positional `variable_declaration`
/// child (no named field for it either).
fn field_declarator_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(decl) = node
        .children(&mut node.walk())
        .find(|c| c.kind() == "variable_declaration")
    else {
        return out;
    };
    for child in decl.children(&mut decl.walk()) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(text) = name_node.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}
