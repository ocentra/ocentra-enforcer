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

use super::has_unsafe_tree_sitter_input;
use crate::parsers::{CallRef, DefinesRef, ImportRef, ParsedFile, SymbolKind, SymbolRef};
use tree_sitter::{Node, Parser};

/// The innermost function a call expression is lexically inside of, if
/// any -- see `rust.rs`'s identical `FnScope` for the full rationale.
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

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
    if has_unsafe_tree_sitter_input(source) {
        return ParsedFile::default();
    }
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
    walk(
        tree.root_node(),
        source.as_bytes(),
        &mut out,
        None,
        FnScope::default(),
    );
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
/// `fn_scope` is the innermost function a call expression sits inside.
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = function_name(node, src) {
                let line = node.start_position().row + 1;
                let kind = if is_test_function(&name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind,
                    line: line.into(),
                });
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
        "struct_specifier" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: (name.clone()).into(),
                    kind: SymbolKind::Struct,
                    line: line.into(),
                });
                if let Some(body) = node.child_by_field_name("body") {
                    for field_name in struct_field_names(body, src) {
                        out.defines.push(DefinesRef {
                            container_name: (name.clone()).into(),
                            member_name: (field_name).into(),
                            line: line.into(),
                        });
                    }
                }
                walk_children(node, src, out, Some(name.as_str()), fn_scope);
                return;
            }
        }
        "enum_specifier" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::Enum,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "type_definition" => {
            for alias_name in typedef_alias_names(node, src) {
                out.symbols.push(SymbolRef {
                    name: (alias_name).into(),
                    kind: SymbolKind::TypeAlias,
                    line: (node.start_position().row + 1).into(),
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
                    name: name.into(),
                    kind: SymbolKind::Constant,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "preproc_include" => {
            if let Some(path) = include_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: (path).into(),
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                out.calls.push(CallRef {
                    callee: (function.utf8_text(src).unwrap_or("").to_string()).into(),
                    line: (node.start_position().row + 1).into(),
                    from_symbol: (fn_scope.name.map(str::to_string)).map(Into::into),
                    from_symbol_line: (fn_scope.line).map(Into::into),
                    // C has no method-call syntax (`x.foo()` is a
                    // struct field access to a function *pointer*
                    // value, not a receiver-dispatched call) -- no
                    // receiver to report, matching this crate's
                    // "unresolved as-written" posture elsewhere.
                    receiver_text: None,
                    receiver_hint: None,
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
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
                name: name.into(),
                kind: if is_const {
                    SymbolKind::Constant
                } else {
                    SymbolKind::Variable
                },
                line: (node.start_position().row + 1).into(),
            });
        }
    }
    out
}

fn declaration_has_const(node: Node<'_>, src: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_qualifier" {
            if let Ok(text) = child.utf8_text(src) {
                if text == "const" {
                    return true;
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
