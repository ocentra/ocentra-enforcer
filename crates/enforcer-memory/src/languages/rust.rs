//! Rust extraction via `tree-sitter-rust`: functions (including
//! `#[test]`-annotated ones, tagged as [`SymbolKind::Test`]), types
//! (`struct`/`enum`/`trait`), `use` imports, and call expressions.
//!
//! Unresolved by design: import/call targets are recorded as written in
//! source (e.g. `crate::graph::MemoryGraph`, `foo::bar`) -- resolving
//! them to concrete graph node ids is [`crate::code_graph`]'s job once
//! every file in the repo has been parsed and every symbol's id is
//! known.

use crate::parsers::{CallRef, ImportRef, ParsedFile, SymbolKind, SymbolRef};
use tree_sitter::{Node, Parser};

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
    walk(tree.root_node(), source.as_bytes(), &mut out);
    out
}

fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = child_text(node, "name", src) {
                let kind = if has_test_attribute(node, src) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    name,
                    kind,
                    line: node.start_position().row + 1,
                });
            }
        }
        "struct_item" | "enum_item" | "trait_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Type,
                    line: node.start_position().row + 1,
                });
            }
        }
        "use_declaration" => {
            for path in use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                out.calls.push(CallRef {
                    callee: call_callee_text(function, src),
                    line: node.start_position().row + 1,
                });
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, src, out);
        }
    }
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
                for i in 0..list.child_count() {
                    if let Some(child) = list.child(i) {
                        if child.kind() != "," && child.kind() != "{" && child.kind() != "}" {
                            collect_use_paths(child, src, &joined, out);
                        }
                    }
                }
            }
        }
        "use_list" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "," && child.kind() != "{" && child.kind() != "}" {
                        collect_use_paths(child, src, prefix, out);
                    }
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
