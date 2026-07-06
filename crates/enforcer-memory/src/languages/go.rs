//! Go extraction via `tree-sitter-go`: functions, receiver methods,
//! structs, interfaces, type aliases, package-level constants/
//! variables, the package clause (as [`SymbolKind::Module`]), imports,
//! calls, best-effort INHERITS for embedded struct fields, DECORATES
//! for nothing (Go has no annotation syntax -- intentionally not
//! emitted), TYPE_REF for signature types, DEFINES for
//! struct/interface -> member, `*_test.go` `TestXxx` test detection,
//! and best-effort `net/http`-style route extraction.
//!
//! Unresolved by design (same rationale as `rust.rs`/`typescript.rs`):
//! import paths and call callees are recorded as written in source,
//! not resolved to graph node ids here.
//!
//! Go interface satisfaction is structural (duck-typed) rather than a
//! written `implements` clause -- there is no syntax node this
//! extractor could read an IMPLEMENTS edge off of, so (per this lane's
//! mission) it is honestly not extracted here at all, rather than
//! guessed at.

use crate::parsers::{
    CallRef, DefinesRef, ImportRef, InheritsRef, ParsedFile, ReceiverHint, RouteRef, SymbolKind,
    SymbolRef, TypeRefRef,
};
use tree_sitter::{Node, Parser};

/// The innermost function/method a call expression is lexically inside
/// of, if any -- same bundled-scope pattern as `rust.rs`'s `FnScope`
/// (a call's "from_symbol" is the enclosing function/method, threaded
/// alongside the walk rather than widening every positional arg).
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

/// HTTP methods this extractor recognizes in `net/http`/mux-style route
/// registration calls (`mux.HandleFunc("/path", handler)`,
/// `router.GET("/path", handler)`).
const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

pub fn parse(source: &str, is_test_file: bool) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
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
        is_test_file,
        FnScope::default(),
    );
    out
}

fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    is_test_file: bool,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "package_clause" => {
            if let Some(name) = first_named_child_text(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
        }
        "function_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let kind = if is_test_file && is_test_function_name(&name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                for type_ref in signature_type_refs(node, src) {
                    out.type_refs.push(TypeRefRef {
                        from_name: name.clone(),
                        type_name: type_ref,
                        line,
                    });
                }
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        is_test_file,
                        FnScope {
                            name: Some(name.as_str()),
                            line: Some(line),
                        },
                    );
                }
                return;
            }
        }
        "method_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                let kind = if is_test_file && is_test_function_name(&name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Method
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                for type_ref in signature_type_refs(node, src) {
                    out.type_refs.push(TypeRefRef {
                        from_name: name.clone(),
                        type_name: type_ref,
                        line,
                    });
                }
                if let Some(receiver_type) = receiver_type_name(node, src) {
                    out.defines.push(DefinesRef {
                        container_name: receiver_type,
                        member_name: name.clone(),
                        line,
                    });
                }
                if let Some(body) = node.child_by_field_name("body") {
                    walk_children(
                        body,
                        src,
                        out,
                        is_test_file,
                        FnScope {
                            name: Some(name.as_str()),
                            line: Some(line),
                        },
                    );
                }
                return;
            }
        }
        "type_declaration" => {
            walk_type_declaration(node, src, out);
        }
        "const_declaration" => {
            for name in spec_names(node, "const_spec", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Constant,
                    line: node.start_position().row + 1,
                });
            }
        }
        "var_declaration" => {
            for name in spec_names(node, "var_spec", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Variable,
                    line: node.start_position().row + 1,
                });
            }
        }
        "import_declaration" => {
            for path in import_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                let callee = match function.utf8_text(src) {
                    Ok(text) => text.to_string(),
                    Err(_) => String::new(),
                };
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
                if let Some(route) = route_from_call(&callee, node, src) {
                    out.routes.push(route);
                }
            }
        }
        _ => {}
    }

    walk_children(node, src, out, is_test_file, fn_scope);
}

fn walk_children(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    is_test_file: bool,
    fn_scope: FnScope<'_>,
) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, src, out, is_test_file, fn_scope);
        }
    }
}

/// X06 type-aware resolution: for a `selector_expression`-shaped callee
/// (`x.Foo`, `w.inner.Foo`), the receiver (`x`/`w.inner`) text plus a
/// cheap syntactic hint. `None`/`None` for a plain identifier callee
/// (`foo(...)`) -- there is no receiver to report. Go has no
/// `self`/`this` keyword (the receiver is an ordinary named parameter),
/// so [`ReceiverHint::SelfOrThis`] is never emitted here; a bare
/// identifier receiver is [`ReceiverHint::Identifier`] and resolution
/// looks its declared type up via the enclosing symbol's TYPE_REFs.
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "selector_expression" {
        return (None, None);
    }
    let Some(receiver) = function_node.child_by_field_name("operand") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if receiver.kind() == "call_expression" && is_new_call(receiver, src) {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "interpreted_string_literal"
            | "raw_string_literal"
            | "int_literal"
            | "float_literal"
            | "imaginary_literal"
            | "rune_literal"
            | "true"
            | "false"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Whether a `call_expression` is (heuristically) a constructor call:
/// its callee's final `.`-segment starts with `New` (`NewWidget(...)`,
/// `pkg.NewClient(...)`) -- Go has no constructor syntax, so this is
/// the idiomatic name-convention best-effort signal, same rationale as
/// `rust.rs`'s `Foo::new` heuristic.
fn is_new_call(call_node: Node<'_>, src: &[u8]) -> bool {
    let Some(function) = call_node.child_by_field_name("function") else {
        return false;
    };
    let Ok(text) = function.utf8_text(src) else {
        return false;
    };
    text.rsplit('.')
        .next()
        .is_some_and(|segment| segment.starts_with("New"))
}

/// Each argument expression's own source text, in written order.
fn call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = call_node.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..args.child_count() {
        if let Some(child) = args.child(i) {
            if matches!(child.kind(), "(" | ")" | ",") {
                continue;
            }
            if let Ok(text) = child.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// `type X struct { ... }` / `type X interface { ... }` / `type X =
/// Y` / `type X Y` -- one `type_declaration` may hold multiple
/// `type_spec`s (`type ( A struct{}; B interface{} )`).
fn walk_type_declaration(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    for i in 0..node.child_count() {
        let Some(spec) = node.child(i) else { continue };
        if spec.kind() == "type_alias" {
            // `type X = Y` -- the grammar models this as its own node
            // kind (distinct from `type_spec`), always a plain alias
            // regardless of what `Y` is (never struct/interface-shaped).
            if let Some(name) = child_text(spec, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::TypeAlias,
                    line: spec.start_position().row + 1,
                });
            }
            continue;
        }
        if spec.kind() != "type_spec" {
            continue;
        }
        let Some(name) = child_text(spec, "name", src) else {
            continue;
        };
        let line = spec.start_position().row + 1;
        let Some(type_node) = spec.child_by_field_name("type") else {
            continue;
        };
        match type_node.kind() {
            "struct_type" => {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Struct,
                    line,
                });
                for (member_name, embedded) in struct_fields(type_node, src) {
                    if embedded {
                        out.inherits.push(InheritsRef {
                            sub_name: name.clone(),
                            super_name: member_name,
                            line,
                        });
                    } else {
                        out.defines.push(DefinesRef {
                            container_name: name.clone(),
                            member_name,
                            line,
                        });
                    }
                }
            }
            "interface_type" => {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for method_name in interface_methods(type_node, src) {
                    out.defines.push(DefinesRef {
                        container_name: name.clone(),
                        member_name: method_name,
                        line,
                    });
                }
            }
            _ => {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::TypeAlias,
                    line,
                });
            }
        }
    }
}

/// One `field_declaration_list`'s fields: `(member_name, is_embedded)`.
/// A field with no `name` field (just a bare type) is an embedded
/// field -- Go's best-effort structural-inheritance signal.
fn struct_fields(struct_node: Node<'_>, src: &[u8]) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let Some(list) = (0..struct_node.child_count())
        .filter_map(|i| struct_node.child(i))
        .find(|n| n.kind() == "field_declaration_list")
    else {
        return out;
    };
    for i in 0..list.child_count() {
        let Some(field) = list.child(i) else { continue };
        if field.kind() != "field_declaration" {
            continue;
        }
        if let Some(name) = child_text(field, "name", src) {
            out.push((name, false));
        } else if let Some(type_node) = field.child_by_field_name("type") {
            if let Ok(text) = type_node.utf8_text(src) {
                out.push((text.trim_start_matches('*').to_string(), true));
            }
        }
    }
    out
}

fn interface_methods(interface_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..interface_node.child_count() {
        let Some(child) = interface_node.child(i) else {
            continue;
        };
        if child.kind() == "method_elem" {
            if let Some(name) = child_text(child, "name", src) {
                out.push(name);
            }
        }
    }
    out
}

/// `func (r *Receiver) Method(...)` -- the receiver's underlying type
/// name (pointer `*` stripped), used as the DEFINES container.
fn receiver_type_name(method_node: Node<'_>, src: &[u8]) -> Option<String> {
    let receiver = method_node.child_by_field_name("receiver")?;
    // `receiver` is a `parameter_list` with exactly one `parameter_declaration`.
    for i in 0..receiver.child_count() {
        let child = receiver.child(i)?;
        if child.kind() == "parameter_declaration" {
            let type_node = child.child_by_field_name("type")?;
            let text = type_node.utf8_text(src).ok()?;
            return Some(text.trim_start_matches('*').to_string());
        }
    }
    None
}

/// Parameter and result types on a `function_declaration`/
/// `method_declaration`'s signature.
fn signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if param.kind() == "parameter_declaration" {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(result) = node.child_by_field_name("result") {
        if let Ok(text) = result.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// `const ( A = 1; B = 2 )` / `var ( A int; B string )` -- flatten
/// every `const_spec`/`var_spec` name across every "," in its `name`
/// field (Go allows `A, B = 1, 2` in one spec).
fn spec_names(decl_node: Node<'_>, spec_kind: &str, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..decl_node.child_count() {
        let Some(spec) = decl_node.child(i) else {
            continue;
        };
        if spec.kind() != spec_kind {
            continue;
        }
        for j in 0..spec.child_count() {
            if let Some(child) = spec.child(j) {
                if child.kind() == "identifier" {
                    if let Ok(text) = child.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Flatten an `import_declaration`'s `import_spec`/`import_spec_list`
/// children into every imported path string, quotes stripped.
fn import_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "import_spec" => {
                if let Some(path) = import_spec_path(child, src) {
                    out.push(path);
                }
            }
            "import_spec_list" => {
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "import_spec" {
                            if let Some(path) = import_spec_path(spec, src) {
                                out.push(path);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn import_spec_path(spec: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = spec.child_by_field_name("path")?;
    let raw = path_node.utf8_text(src).ok()?;
    Some(raw.trim_matches('"').to_string())
}

fn first_named_child_text(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.is_named() {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// `TestXxx(t *testing.T)`-shaped Go test function name convention.
fn is_test_function_name(name: &str) -> bool {
    name.starts_with("Test") || name.starts_with("Benchmark") || name.starts_with("Example")
}

/// Recognize `net/http`-style/mux-style route registration:
/// `<obj>.<Method|HandleFunc>("<path>", handler)` where `<Method>` is
/// an HTTP verb (case-insensitive, e.g. gin/gorilla-mux's `router.GET`)
/// or the literal `HandleFunc`/`Handle` (bare `net/http`, method
/// unknown -- recorded as `ANY`).
fn route_from_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let last_segment = callee.rsplit('.').next()?;
    let method = if last_segment.eq_ignore_ascii_case("HandleFunc")
        || last_segment.eq_ignore_ascii_case("Handle")
    {
        "ANY".to_string()
    } else {
        let lower = last_segment.to_lowercase();
        if !HTTP_METHODS.contains(&lower.as_str()) {
            return None;
        }
        lower.to_uppercase()
    };
    let args = call_node.child_by_field_name("arguments")?;
    let first_string_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "interpreted_string_literal" || n.kind() == "raw_string_literal")?;
    let raw = first_string_arg.utf8_text(src).ok()?;
    let path = raw.trim_matches(|c| c == '"' || c == '`').to_string();
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    Some(RouteRef {
        method,
        path,
        line: call_node.start_position().row + 1,
    })
}
