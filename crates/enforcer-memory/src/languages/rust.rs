//! Rust extraction via `tree-sitter-rust`: functions (including
//! `#[test]`-annotated ones, tagged as [`SymbolKind::Test`]), types
//! (`struct`/`enum`/`trait`), `use` imports, and call expressions.
//!
//! Unresolved by design: import/call targets are recorded as written in
//! source (e.g. `crate::graph::MemoryGraph`, `foo::bar`) -- resolving
//! them to concrete graph node ids is [`crate::code_graph`]'s job once
//! every file in the repo has been parsed and every symbol's id is
//! known.

use crate::parsers::{
    CallRef, DecoratesRef, DefinesRef, ImplementsRef, ImportRef, InheritsRef, ParsedFile,
    SymbolKind, SymbolRef, TypeRefRef,
};
use enforcer_domain::memory_types::ReceiverHint;
use tree_sitter::{Node, Parser};

/// The innermost function/method a call expression is lexically inside
/// of, if any -- threaded alongside `enclosing` (the containing
/// `impl`/`trait`/`mod` *type* name) because a call's "from_symbol" is
/// the function/method, not the type, while DEFINES/Method-vs-Function
/// tagging still needs the type name. Kept as a small local struct
/// (rather than widening `walk`'s own positional args further) per this
/// crate's "bundle related params" convention (see `code_graph.rs`'s
/// `NewFileParams`).
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

/// Parse Rust `source`. Never panics on malformed input: a source file
/// that fails to parse cleanly still yields whatever tree-sitter's
/// error-recovering parser could extract (tree-sitter always produces
/// *a* tree, marking unparseable spans as `ERROR` nodes that this walk
/// simply does not match against).
pub fn parse(source: &str) -> ParsedFile {
    let mut parser = Parser::new();
    // `set_language` only fails if the grammar's ABI version is
    // incompatible with this `tree-sitter` core version, which is a
    // build-time invariant (locked by Cargo.toml), not a runtime
    // per-file condition -- there is nothing a caller could do
    // differently, so an empty result is the correct, total response.
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
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

/// `enclosing` is the name of the containing `impl`/`trait`/`mod` block
/// (if any) -- used to tag a `function_item` as [`SymbolKind::Method`]
/// instead of [`SymbolKind::Function`] and to emit a DEFINES edge from
/// the container to the member. `fn_scope` is the innermost
/// function/method a call expression sits inside of -- see [`FnScope`].
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let kind = if has_test_attribute(node, src) {
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
                for decorator in attribute_decorators(node, src) {
                    out.decorates.push(DecoratesRef {
                        target_name: (name.clone()).into(),
                        decorator_name: (decorator).into(),
                        line: line.into(),
                    });
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
        "struct_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Struct,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "enum_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Enum,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "trait_item" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Interface,
                    line: line.into(),
                });
                for supertrait in trait_bounds(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: (name.clone()).into(),
                        super_name: (supertrait).into(),
                        line: line.into(),
                    });
                }
                walk_children(node, src, out, Some(name.as_str()), fn_scope);
                return;
            }
        }
        "mod_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Module,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "type_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::TypeAlias,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "const_item" | "static_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Constant,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "let_declaration" => {
            if let Some(closure_name) = named_closure_binding(node, src) {
                out.symbols.push(SymbolRef {
                    name: (closure_name).into(),
                    kind: SymbolKind::Lambda,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "impl_item" => {
            let type_name = node
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_string);
            let trait_name = node
                .child_by_field_name("trait")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_string);
            if let (Some(type_name), Some(trait_name)) = (&type_name, &trait_name) {
                out.implements.push(ImplementsRef {
                    type_name: (type_name.clone()).into(),
                    trait_name: (trait_name.clone()).into(),
                    line: (node.start_position().row + 1).into(),
                });
            }
            walk_children(node, src, out, type_name.as_deref(), fn_scope);
            return;
        }
        "use_declaration" => {
            for path in use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: (path).into(),
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                let (receiver_text, receiver_hint) = receiver_of_call(function, src);
                out.calls.push(CallRef {
                    callee: (call_callee_text(function, src)).into(),
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

/// `#[attribute]`/`#[attribute(...)]` macros directly preceding a
/// `function_item`, best-effort DECORATES source (excludes the
/// `#[test]`-family attributes, which are already modeled via
/// [`SymbolKind::Test`] rather than a decoration edge).
fn attribute_decorators(function_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut sibling = function_node.prev_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            "attribute_item" => {
                if let Ok(text) = node.utf8_text(src) {
                    if !attribute_is_test(text) {
                        let inner = text
                            .trim_start_matches('#')
                            .trim_start_matches('[')
                            .trim_end_matches(']')
                            .trim();
                        let name = inner.split(['(', ' ']).next().unwrap_or(inner);
                        if !name.is_empty() {
                            out.push(name.to_string());
                        }
                    }
                }
                sibling = node.prev_sibling();
            }
            "line_comment" | "block_comment" => sibling = node.prev_sibling(),
            _ => break,
        }
    }
    out.reverse();
    out
}

/// Parameter and return types on a `function_item`'s signature,
/// as-written (best-effort TYPE_REF source).
fn signature_type_refs(function_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = function_node.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for param in params.children(&mut cursor) {
            if param.kind() == "parameter" {
                if let Some(type_node) = param.child_by_field_name("type") {
                    if let Ok(text) = type_node.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    if let Some(return_type) = function_node.child_by_field_name("return_type") {
        if let Ok(text) = return_type.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// `trait Sub: Super1 + Super2` -- the supertrait bounds list.
fn trait_bounds(trait_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(bounds) = trait_node.child_by_field_name("bounds") {
        let mut cursor = bounds.walk();
        for child in bounds.children(&mut cursor) {
            if child.kind() != "+" {
                if let Ok(text) = child.utf8_text(src) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        out.push(trimmed.to_string());
                    }
                }
            }
        }
    }
    out
}

/// `let f = |x| ...` / `let f = move |x| ...` -- a named closure
/// binding, best-effort [`SymbolKind::Lambda`] source.
fn named_closure_binding(let_node: Node<'_>, src: &[u8]) -> Option<String> {
    let pattern = let_node.child_by_field_name("pattern")?;
    if pattern.kind() != "identifier" {
        return None;
    }
    let value = let_node.child_by_field_name("value")?;
    if value.kind() != "closure_expression" {
        return None;
    }
    pattern.utf8_text(src).ok().map(str::to_string)
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Innermost callee identifier text, stripping generic argument lists
/// and keeping the full `a::b::c` / `self.method` path as written.
fn call_callee_text(node: Node<'_>, src: &[u8]) -> String {
    node.utf8_text(src).unwrap_or("").to_string()
}

/// X06 type-aware resolution: for a `field_expression`-shaped callee
/// (`x.foo`, `self.foo`), the receiver (`x`/`self`) text plus a cheap
/// syntactic hint. `None`/`None` for a plain path callee (`foo`,
/// `a::b::foo`) -- there is no receiver to report.
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "field_expression" {
        return (None, None);
    }
    let Some(receiver) = function_node.child_by_field_name("value") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "self" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "call_expression" && is_new_call(receiver, src) {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "integer_literal" | "string_literal" | "boolean_literal" | "float_literal"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Whether a `call_expression` node is (heuristically) a constructor
/// call: its callee's final path segment is `new` (`Foo::new(...)`) --
/// Rust has no dedicated `new`-expression syntax, so this is a
/// name-convention best-effort signal, same rationale as every other
/// "as-written, unresolved" heuristic in this crate.
fn is_new_call(call_node: Node<'_>, src: &[u8]) -> bool {
    let Some(function) = call_node.child_by_field_name("function") else {
        return false;
    };
    let Ok(text) = function.utf8_text(src) else {
        return false;
    };
    text.rsplit("::").next() == Some("new")
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

/// Whether a `function_item` is preceded by a `#[test]`-family
/// attribute (`#[test]`, `#[tokio::test]`, `#[async_std::test]`, ...) --
/// matched by the attribute's last path segment being `test`.
fn has_test_attribute(function_node: Node<'_>, src: &[u8]) -> bool {
    let mut sibling = function_node.prev_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            "attribute_item" | "line_comment" | "block_comment" => {
                if node.kind() == "attribute_item" {
                    if let Ok(text) = node.utf8_text(src) {
                        if attribute_is_test(text) {
                            return true;
                        }
                    }
                }
                sibling = node.prev_sibling();
            }
            _ => break,
        }
    }
    false
}

fn attribute_is_test(attribute_text: &str) -> bool {
    // Matches `#[test]`, `#[tokio::test]`, `#[async_std::test]`,
    // `#[test_case(..)]` is deliberately excluded (it decorates a
    // generator, not a runnable test itself, and would need argument
    // parsing this slice does not attempt).
    let inner = attribute_text
        .trim_start_matches('#')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    inner == "test" || inner.ends_with("::test")
}

/// Flatten a `use_declaration`'s tree (`use_list`, `scoped_use_list`,
/// `use_as_clause`, `use_wildcard`, plain path) into every concrete
/// imported path it declares.
fn use_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(argument) = node.child_by_field_name("argument") {
        collect_use_paths(argument, src, "", &mut paths);
    }
    if paths.is_empty() {
        // Fall back to the raw declaration text (minus `use`/`;`) so a
        // shape this walk does not special-case still yields something
        // rather than silently dropping the import.
        if let Ok(text) = node.utf8_text(src) {
            let trimmed = text
                .trim_start_matches("use")
                .trim()
                .trim_end_matches(';')
                .trim();
            if !trimmed.is_empty() {
                paths.push(trimmed.to_string());
            }
        }
    }
    paths
}

fn collect_use_paths(node: Node<'_>, src: &[u8], prefix: &str, out: &mut Vec<String>) {
    match node.kind() {
        "scoped_use_list" => {
            let base = node
                .child_by_field_name("path")
                .and_then(|n| n.utf8_text(src).ok())
                .unwrap_or("");
            let joined = join_prefix(prefix, base);
            if let Some(list) = node.child_by_field_name("list") {
                let mut cursor = list.walk();
                for child in list.children(&mut cursor) {
                    if child.kind() != "," && child.kind() != "{" && child.kind() != "}" {
                        collect_use_paths(child, src, &joined, out);
                    }
                }
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "," && child.kind() != "{" && child.kind() != "}" {
                    collect_use_paths(child, src, prefix, out);
                }
            }
        }
        "use_as_clause" => {
            if let Some(path_node) = node.child_by_field_name("path") {
                let text = path_node.utf8_text(src).unwrap_or("");
                out.push(join_prefix(prefix, text));
            }
        }
        "use_wildcard" => {
            let text = node.utf8_text(src).unwrap_or("*");
            out.push(join_prefix(prefix, text));
        }
        _ => {
            let text = node.utf8_text(src).unwrap_or("");
            if !text.is_empty() {
                out.push(join_prefix(prefix, text));
            }
        }
    }
}

fn join_prefix(prefix: &str, tail: &str) -> String {
    if prefix.is_empty() {
        tail.to_string()
    } else {
        format!("{prefix}::{tail}")
    }
}
