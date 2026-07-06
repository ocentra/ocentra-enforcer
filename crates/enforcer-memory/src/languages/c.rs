//! C extraction via `tree-sitter-c`: functions, structs, enums, typedef
//! type aliases, `#define`-value macros and top-level `const`/plain
//! globals (best-effort split into [`SymbolKind::Constant`] vs
//! [`SymbolKind::Variable`]), `#include` imports, and call expressions.
//!
//! Baseline reference: `C:\Projects\codebase-memory-mcp` is itself
//! written in C, so its own `src/**/*.c` files are the ground-truth
//! fixture for "does this extractor recognize real-world C" -- see
//! `tests/unit_languages_c.rs`'s baseline-indexing test.
//!
//! Unresolved by design, same rationale as `rust.rs`: import/call
//! targets are recorded as written in source; resolving them to
//! concrete graph node ids is [`crate::code_graph`]'s job.

use crate::parsers::{CallRef, DefinesRef, ImportRef, ParsedFile, SymbolKind, SymbolRef};
use tree_sitter::{Node, Parser};

/// Parse C `source`. Never panics on malformed input -- same
/// error-recovery rationale as every other extractor in this crate:
/// tree-sitter always produces *a* tree, marking unparseable spans as
/// `ERROR` nodes this walk simply does not match against.
///
/// `is_test_file` is the file-level test signal (a path under `test/`
/// or matching `*_test.c`, decided by the caller from `rel_path` --
/// same "test-ness travels with the file, not just the symbol name"
/// convention as `languages::go`'s `_test.go` handling): when `true`,
/// every free function extracted from this file is tagged
/// [`SymbolKind::Test`] regardless of its own name, in addition to this
/// module's own `test_`/`_test` name-convention heuristic (which still
/// applies independently for files this flag is `false` for).
pub fn parse(source: &str, is_test_file: bool) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .is_err()
    {
        return ParsedFile::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParsedFile::default();
    };

    let mut out = ParsedFile::default();
    walk(tree.root_node(), source.as_bytes(), &mut out, None);
    if is_test_file {
        for symbol in &mut out.symbols {
            if symbol.kind == SymbolKind::Function {
                symbol.kind = SymbolKind::Test;
            }
        }
    }
    // `#define`-value macros are preprocessor directives, not AST nodes
    // this walk's node-kind `match` visits structurally the same way as
    // declarations -- tree-sitter-c does expose them as
    // `preproc_def`/`preproc_function_def` nodes, handled inside `walk`
    // below like everything else; no separate pass needed.
    out
}

/// `enclosing` is the name of the containing `struct`/`union` (if any)
/// -- used only to emit a best-effort DEFINES edge from a struct to its
/// member field names (the workpack's "DEFINES optional" note for C).
fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile, enclosing: Option<&str>) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = function_name(node, src) {
                let line = node.start_position().row + 1;
                let kind = if is_test_function(&name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef { name, kind, line });
            }
        }
        "struct_specifier" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Struct,
                    line,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    for field_name in struct_field_names(body, src) {
                        out.defines.push(DefinesRef {
                            container_name: name.clone(),
                            member_name: field_name,
                            line,
                        });
                    }
                }
                walk_children(node, src, out, Some(name.as_str()));
                return;
            }
        }
        "enum_specifier" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
        }
        "type_definition" => {
            for alias_name in typedef_alias_names(node, src) {
                out.symbols.push(SymbolRef {
                    name: alias_name,
                    kind: SymbolKind::TypeAlias,
                    line: node.start_position().row + 1,
                });
            }
        }
        "declaration" => {
            for symbol in top_level_declaration_symbols(node, src, enclosing.is_some()) {
                out.symbols.push(symbol);
            }
        }
        // `#define NAME value` -- a value macro (a bare `#define NAME`
        // with no replacement text, e.g. an include guard, is
        // deliberately excluded: it has no `value` field at all).
        "preproc_def" if node.child_by_field_name("value").is_some() => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Constant,
                    line: node.start_position().row + 1,
                });
            }
        }
        "preproc_include" => {
            if let Some(path) = include_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                out.calls.push(CallRef {
                    callee: function.utf8_text(src).unwrap_or("").to_string(),
                    line: node.start_position().row + 1,
                });
            }
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

/// A `function_definition` node's name, unwrapping
/// `pointer_declarator`/`function_declarator` layers (`int *foo(...)`,
/// `void (*foo)(...)` best-effort) down to the innermost `identifier`.
fn function_name(function_node: Node<'_>, src: &[u8]) -> Option<String> {
    let declarator = function_node.child_by_field_name("declarator")?;
    innermost_declarator_identifier(declarator, src)
}

fn innermost_declarator_identifier(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => node.utf8_text(src).ok().map(str::to_string),
        "function_declarator" | "pointer_declarator" | "parenthesized_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|inner| innermost_declarator_identifier(inner, src)),
        _ => None,
    }
}

/// Field names inside a `struct`/`union` body -- one name per
/// `field_declaration`'s declarator (best-effort: does not expand
/// bit-field widths or nested anonymous struct/union members beyond
/// their own declared field name, if any).
fn struct_field_names(body: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..body.child_count() {
        let Some(child) = body.child(i) else { continue };
        if child.kind() != "field_declaration" {
            continue;
        }
        if let Some(declarator) = child.child_by_field_name("declarator") {
            if let Some(name) = innermost_declarator_identifier(declarator, src) {
                out.push(name);
            }
        }
    }
    out
}

/// `typedef struct {...} Foo;` / `typedef int MyInt;` / `typedef int
/// (*FnPtr)(int);` -- every declarator name a `type_definition`
/// introduces (there can be more than one: `typedef int A, B;`). The
/// alias name itself is a bare `type_identifier` child (not wrapped in
/// any declarator kind `innermost_declarator_identifier` recognizes),
/// while the *underlying* type being aliased may itself contribute a
/// `type_identifier` (e.g. `struct Point` in `typedef struct Point
/// PointAlias;`) that must NOT be collected as an alias name -- so this
/// takes only the *last* top-level `type_identifier`/declarator-shaped
/// child, matching the grammar's fixed "type-spec then alias name(s)"
/// ordering.
fn typedef_alias_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "typedef" | "struct_specifier" | "union_specifier" | "enum_specifier" | ";" => {
                continue; // the type-specifier side, not an alias name.
            }
            "type_identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
            _ => {
                if let Some(name) = innermost_declarator_identifier(child, src) {
                    out.push(name);
                }
            }
        }
    }
    out
}

/// A top-level (or struct/union-scoped) `declaration` node: global
/// variables and `const`-qualified globals, split into
/// [`SymbolKind::Constant`] (has a `const` type qualifier) vs
/// [`SymbolKind::Variable`] (does not). Skips declarations already
/// covered by a more specific node kind (function prototypes with no
/// body are still `declaration` nodes in tree-sitter-c, but a bare
/// prototype has no meaningful "variable" identity here, so a
/// `function_declarator` child is excluded).
fn top_level_declaration_symbols(
    node: Node<'_>,
    src: &[u8],
    inside_struct: bool,
) -> Vec<SymbolRef> {
    if inside_struct {
        // Struct/union field declarations are handled by
        // `struct_field_names`, not re-emitted as top-level globals.
        return Vec::new();
    }
    let is_const = declaration_has_const(node, src);
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "init_declarator" && child.kind() != "identifier" {
            continue;
        }
        let declarator = if child.kind() == "init_declarator" {
            child.child_by_field_name("declarator")
        } else {
            Some(child)
        };
        let Some(declarator) = declarator else {
            continue;
        };
        if declarator.kind() == "function_declarator" {
            continue; // bare prototype, not a variable/constant.
        }
        if let Some(name) = innermost_declarator_identifier(declarator, src) {
            out.push(SymbolRef {
                name,
                kind: if is_const {
                    SymbolKind::Constant
                } else {
                    SymbolKind::Variable
                },
                line: node.start_position().row + 1,
            });
        }
    }
    out
}

fn declaration_has_const(node: Node<'_>, src: &[u8]) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "type_qualifier" {
                if let Ok(text) = child.utf8_text(src) {
                    if text == "const" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// `#include "path"` / `#include <path>` -- the path as written,
/// stripped of its `"..."`/`<...>` delimiters (unresolved, same
/// rationale as every other import extractor in this crate).
fn include_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = node.child_by_field_name("path")?;
    let raw = path_node.utf8_text(src).ok()?;
    Some(
        raw.trim_matches(|c| c == '"' || c == '<' || c == '>')
            .to_string(),
    )
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// gtest-family `TEST(Suite, Name)`/`TEST_F(Fixture, Name)` macros
/// don't apply to plain C (gtest is C++-only), so C's test heuristic is
/// name-convention only: a function whose name starts with `test_` or
/// ends with `_test`, matching this workpack's "files under test/ or
/// *_test.c" file-level signal with a symbol-level fallback for tests
/// declared in a non-`*_test.c`-named file.
fn is_test_function(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test_") || lower.ends_with("_test")
}
