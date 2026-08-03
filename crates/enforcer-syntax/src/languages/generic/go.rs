//! Go-specific extensions for the generic tree-sitter walker.
//!
//! This module owns Go-only declarations, receiver-based DEFINES edges,
//! import extraction, test naming, and route detection.

use super::{
    child_text, parse_with_spec, syntax_children, DefinesRef, ImportRef, InheritsRef, LangSpec,
    Node, ParsedFile, Quirks, RouteRef, SymbolKind, SymbolRef, HTTP_METHODS,
};

/// Raw source bytes owned by one Go extraction pass.
///
/// The generic walker still supplies byte slices at its hook boundary, while
/// Go-specific helpers receive this value object to keep that boundary local.
#[derive(Clone, Copy)]
struct GoSource<'a>(&'a [u8]);

impl<'a> GoSource<'a> {
    fn bytes(self) -> &'a [u8] {
        self.0
    }
}

fn go_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    let source = GoSource(src);
    match node.kind() {
        "type_declaration" => {
            walk_go_type_declaration(node, source, out);
            // Matches `languages/go.rs`'s `walk_type_declaration` call
            // site: it does not recurse into a `type_declaration`'s own
            // children afterward (a type spec's field/method-elem
            // names are plain identifiers, not call expressions, so
            // there is nothing further the generic walk would find).
            true
        }
        "const_declaration" => {
            for name in go_spec_names(node, "const_spec", source) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Constant,
                    line: (node.start_position().row + 1).into(),
                });
            }
            // Return `false` (not "fully handled"): a `const_spec`'s
            // value expression can itself contain a call
            // (`const N = compute()`), which `languages/go.rs` still
            // finds because its own `walk` falls through to
            // `walk_children` after this match arm. Matching that here
            // means generic recursion must continue into this node's
            // children too.
            false
        }
        "var_declaration" => {
            for name in go_spec_names(node, "var_spec", source) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Variable,
                    line: (node.start_position().row + 1).into(),
                });
            }
            // Same rationale as `const_declaration` above.
            false
        }
        "import_declaration" => {
            for path in go_import_paths(node, source) {
                out.imports.push(ImportRef {
                    module_path: (path).into(),
                    line: (node.start_position().row + 1).into(),
                });
            }
            true
        }
        _ => false,
    }
}

/// Go's method-receiver DEFINES container: `func (w *Widget) Draw()`'s
/// container is `Widget` (the receiver's type name, pointer-stripped),
/// not whatever `enclosing` the generic walker was threading (Go
/// methods are not lexically nested inside their type's body at all,
/// unlike Rust `impl` blocks or TS/Python class bodies) -- wired as
/// [`Quirks::on_method_defined`].
fn go_on_method_defined(node: Node<'_>, name: &str, line: usize, src: &[u8], out: &mut ParsedFile) {
    if node.kind() != "method_declaration" {
        return;
    }
    if let Some(receiver_type) = go_receiver_type_name(node, GoSource(src)) {
        out.defines.push(DefinesRef {
            container_name: (receiver_type).into(),
            member_name: (name.to_string()).into(),
            line: line.into(),
        });
    }
}

fn walk_go_type_declaration(node: Node<'_>, source: GoSource<'_>, out: &mut ParsedFile) {
    for spec in syntax_children(node) {
        if spec.kind() == "type_alias" {
            if let Some(name) = child_text(spec, "name", source.bytes()) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::TypeAlias,
                    line: (spec.start_position().row + 1).into(),
                });
            }
            continue;
        }
        if spec.kind() != "type_spec" {
            continue;
        }
        let Some(name) = child_text(spec, "name", source.bytes()) else {
            continue;
        };
        let line = spec.start_position().row + 1;
        let Some(type_node) = spec.child_by_field_name("type") else {
            continue;
        };
        match type_node.kind() {
            "struct_type" => {
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Struct,
                    line: line.into(),
                });
                for (member_name, embedded) in go_struct_fields(type_node, source) {
                    if embedded {
                        out.inherits.push(InheritsRef {
                            sub_name: (name.clone()).into(),
                            super_name: (member_name).into(),
                            line: line.into(),
                        });
                    } else {
                        out.defines.push(DefinesRef {
                            container_name: (name.clone()).into(),
                            member_name: member_name.into(),
                            line: line.into(),
                        });
                    }
                }
            }
            "interface_type" => {
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Interface,
                    line: line.into(),
                });
                for method_name in go_interface_methods(type_node, source) {
                    out.defines.push(DefinesRef {
                        container_name: (name.clone()).into(),
                        member_name: (method_name).into(),
                        line: line.into(),
                    });
                }
            }
            _ => {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::TypeAlias,
                    line: line.into(),
                });
            }
        }
    }
}

fn go_struct_fields(struct_node: Node<'_>, source: GoSource<'_>) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let Some(list) = (0..struct_node.child_count())
        .filter_map(|i| struct_node.child(i))
        .find(|n| n.kind() == "field_declaration_list")
    else {
        return out;
    };
    for field in syntax_children(list) {
        if field.kind() != "field_declaration" {
            continue;
        }
        if let Some(name) = child_text(field, "name", source.bytes()) {
            out.push((name, false));
        } else if let Some(type_node) = field.child_by_field_name("type") {
            if let Ok(text) = type_node.utf8_text(source.bytes()) {
                out.push((text.trim_start_matches('*').to_string(), true));
            }
        }
    }
    out
}

fn go_interface_methods(interface_node: Node<'_>, source: GoSource<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for child in syntax_children(interface_node) {
        if child.kind() == "method_elem" {
            if let Some(name) = child_text(child, "name", source.bytes()) {
                out.push(name);
            }
        }
    }
    out
}

fn go_receiver_type_name(method_node: Node<'_>, source: GoSource<'_>) -> Option<String> {
    let receiver = method_node.child_by_field_name("receiver")?;
    for child in syntax_children(receiver) {
        if child.kind() == "parameter_declaration" {
            let type_node = child.child_by_field_name("type")?;
            let text = type_node.utf8_text(source.bytes()).ok()?;
            return Some(text.trim_start_matches('*').to_string());
        }
    }
    None
}

fn go_spec_names(decl_node: Node<'_>, spec_kind: &str, source: GoSource<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for spec in syntax_children(decl_node) {
        if spec.kind() != spec_kind {
            continue;
        }
        for child in syntax_children(spec) {
            if child.kind() == "identifier" {
                if let Ok(text) = child.utf8_text(source.bytes()) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

fn go_import_paths(node: Node<'_>, source: GoSource<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for child in syntax_children(node) {
        match child.kind() {
            "import_spec" => {
                if let Some(path) = go_import_spec_path(child, source) {
                    out.push(path);
                }
            }
            "import_spec_list" => {
                for spec in syntax_children(child) {
                    if spec.kind() == "import_spec" {
                        if let Some(path) = go_import_spec_path(spec, source) {
                            out.push(path);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn go_import_spec_path(spec: Node<'_>, source: GoSource<'_>) -> Option<String> {
    let path_node = spec.child_by_field_name("path")?;
    let raw = path_node.utf8_text(source.bytes()).ok()?;
    Some(raw.trim_matches('"').to_string())
}

fn go_is_test_name(name: &str) -> bool {
    name.starts_with("Test") || name.starts_with("Benchmark") || name.starts_with("Example")
}

/// Recognize `net/http`-style/mux-style route registration, same
/// rules as `languages/go.rs`'s `route_from_call`.
fn go_route_from_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let source = GoSource(src);
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
    let raw = first_string_arg.utf8_text(source.bytes()).ok()?;
    let path = raw.trim_matches(|c| c == '"' || c == '`').to_string();
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    Some(RouteRef {
        method: method.into(),
        path: path.into(),
        line: (call_node.start_position().row + 1).into(),
    })
}

/// Go's [`Quirks`] row: embedded-field INHERITS, struct-vs-interface
/// `type_spec` split, `const`/`var` flattening, grouped import paths,
/// `TestXxx`/`BenchmarkXxx`/`ExampleXxx` test-name convention, and
/// `net/http`/mux-style route detection.
pub fn go_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(go_quirk)),
        is_test_name: go_is_test_name,
        route_from_call: Some(Box::new(|ctx| {
            go_route_from_call(ctx.callee, ctx.call_node, ctx.source)
        })),
        on_method_defined: Some(Box::new(|ctx| {
            go_on_method_defined(ctx.node, ctx.name, ctx.line, ctx.source, ctx.output)
        })),
        call_override: None,
    }
}

/// Parse Go source through the generic engine (this wave's
/// zero-regression proof against `languages::go::parse`). `rel_path`
/// test-file gating is the caller's job (same contract as
/// `languages::go::parse`'s `is_test_file` parameter).
pub fn parse_go(source: &str, is_test_file: bool) -> ParsedFile {
    let spec = LangSpec::go();
    let quirks = go_quirks();
    let language: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, is_test_file)
}
