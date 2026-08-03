//! C++ extraction via `tree-sitter-cpp`: everything `languages::c`
//! extracts (functions, structs, enums, typedefs, `#define` value
//! macros, globals, `#include` imports, calls) plus C++-specific
//! surface: `class`, member methods (including out-of-line
//! `Class::method(...) { ... }` definitions), best-effort abstract
//! classes tagged [`SymbolKind::Interface`], `namespace` as
//! [`SymbolKind::Module`], named lambda variable bindings, and
//! `INHERITS` edges from `class X : public Base`.
//!
//! `IMPLEMENTS`/`DECORATES`: C++ has no first-class interface-
//! implementation or decorator/attribute-macro syntax comparable to
//! Rust `impl Trait for T` or Python/TS decorators -- `[[attribute]]`
//! syntax exists but has no analogous "decorates a symbol with a named
//! behavior" semantics this crate's DECORATES edge models, so this
//! extractor deliberately emits neither edge kind (documented here per
//! the workpack's "n/a -- document" instruction, matching `[`ParsedFile`]`'s
//! empty-by-default fields for any edge kind an extractor does not
//! produce).
//!
//! Baseline reference: `C:\Projects\codebase-memory-mcp` is a C, not
//! C++, codebase, so this extractor's baseline-indexing proof
//! (`tests/unit_languages_cpp.rs`) targets synthetic fixtures rather
//! than that repo's own sources -- see `tests/unit_languages_c.rs` for
//! the baseline-indexing test that *does* apply (C, not C++).

use super::has_unsafe_tree_sitter_input;
use crate::boundary::{Retained, RetainedDisplay};
use crate::parsers::{
    CallRef, DefinesRef, ImportRef, InheritsRef, ParsedFile, SymbolKind, SymbolRef,
};
use enforcer_domain::memory_types::ReceiverHint;
use tree_sitter::{Node, Parser};

/// The innermost function/method a call expression is lexically inside
/// of, if any -- see `rust.rs`'s identical `FnScope` for the full
/// rationale.
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

/// Parse C++ `source`. Never panics on malformed input, same
/// error-recovery rationale as every other extractor in this crate.
///
/// `is_test_file` is the file-level test signal (a path under `test/`
/// or matching `*_test.cpp`/`*_test.cc`, decided by the caller from
/// `rel_path`, same convention as `languages::c::parse`): promotes
/// every free function/method to [`SymbolKind::Test`] regardless of its
/// own name, on top of this module's `TEST`/`TEST_F`/`TEST_P` gtest
/// macro detection and `test_`/`_test` name-convention heuristic (both
/// of which apply independently of this flag).
pub fn parse(source: &str, is_test_file: bool) -> ParsedFile {
    if has_unsafe_tree_sitter_input(source) {
        return ParsedFile::default();
    }
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
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
            if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
                symbol.kind = SymbolKind::Test;
            }
        }
    }
    out
}

/// `enclosing` is the name of the containing `class`/`struct`/
/// `namespace` (if any) -- distinguishes [`SymbolKind::Method`] from a
/// free [`SymbolKind::Function`] and feeds DEFINES edges, same role as
/// `rust.rs`'s/`typescript.rs`'s `enclosing` parameter.
fn walk(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    match node.kind() {
        "function_definition" => {
            handle_function_definition(node, src, out, enclosing, fn_scope);
            return; // children (body) already walked by the handler.
        }
        "class_specifier" | "struct_specifier" => {
            handle_class_or_struct(node, src, out, enclosing, fn_scope);
            return;
        }
        "namespace_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: (name.retained()).into(),
                    kind: SymbolKind::Module,
                    line: (node.start_position().row + 1).into(),
                });
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
        "alias_declaration" => {
            // C++11 `using Alias = Type;` -- a type alias with a
            // dedicated grammar node, unlike typedef's declarator-based
            // shape.
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.into(),
                    kind: SymbolKind::TypeAlias,
                    line: (node.start_position().row + 1).into(),
                });
            }
        }
        "declaration" => {
            if let Some(binding) = named_lambda_binding(node, src) {
                out.symbols.push(binding);
            } else {
                for symbol in top_level_declaration_symbols(node, src, enclosing.is_some()) {
                    out.symbols.push(symbol);
                }
            }
        }
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
                let callee = function.utf8_text(src).unwrap_or("").retained_display();
                if let Some(test_name) = gtest_macro_test_name(&callee, node, src) {
                    out.symbols.push(SymbolRef {
                        name: (test_name).into(),
                        kind: SymbolKind::Test,
                        line: (node.start_position().row + 1).into(),
                    });
                }
                let (receiver_text, receiver_hint) = receiver_of_call(function, src);
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
        }
        "declaration_list" if enclosing.is_some() => {
            // A namespace's body: keep the enclosing namespace name in
            // scope for its direct declaration children (function
            // prototypes, nested classes) without this arm itself
            // emitting anything.
        }
        "field_declaration_list" if enclosing.is_some() => {
            // A class/struct body already dispatched via
            // `handle_class_or_struct` -- reaching here only via a
            // nested class/struct inside another class/struct's own
            // body, which `handle_class_or_struct`'s own
            // `walk_children` call already covers member-by-member.
        }
        "assignment_expression" | "field_expression" => {
            // Fast-path skip: neither node kind introduces a symbol on
            // its own, and both can contain a `lambda_expression` this
            // walk's default recursion already reaches -- no special
            // handling needed beyond the default `walk_children` below.
        }
        "expression_statement" => {
            if let Some(binding) = named_lambda_binding_from_assignment(node, src) {
                out.symbols.push(binding);
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

/// `class`/`struct` declaration: emits [`SymbolKind::Class`] (or
/// [`SymbolKind::Interface`] when every member function is a pure
/// virtual -- the closest C++ has to an "abstract class as interface"
/// best-effort signal), `INHERITS` edges from its base-class clause,
/// DEFINES edges to member field names, and recurses into the body with
/// this class as the new `enclosing` scope for member methods.
fn handle_class_or_struct(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    let Some(name) = child_text(node, "name", src) else {
        walk_children(node, src, out, enclosing, fn_scope);
        return;
    };
    let line = node.start_position().row + 1;
    let body = node.child_by_field_name("body");

    let kind = if body
        .map(|b| is_abstract_class_body(b, src))
        .unwrap_or(false)
    {
        SymbolKind::Interface
    } else {
        SymbolKind::Class
    };
    out.symbols.push(SymbolRef {
        name: (name.retained()).into(),
        kind,
        line: line.into(),
    });

    for base_name in base_class_names(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: (name.retained()).into(),
            super_name: (base_name).into(),
            line: line.into(),
        });
    }

    if let Some(body) = body {
        for field_name in field_names(body, src) {
            out.defines.push(DefinesRef {
                container_name: (name.retained()).into(),
                member_name: (field_name).into(),
                line: line.into(),
            });
        }
    }

    walk_children(node, src, out, Some(name.as_str()), fn_scope);
}

/// Best-effort "interface" heuristic: a class/struct body whose every
/// `field_declaration`-shaped member function declarator carries a
/// `pure_virtual_clause` (`virtual void f() = 0;`), and which declares
/// at least one such member. A body with no member functions at all
/// (a plain data struct) is `false` here, not vacuously `true`.
fn is_abstract_class_body(body: Node<'_>, src: &[u8]) -> bool {
    let mut saw_virtual_member = false;
    let mut body_cursor = body.walk();
    for child in body.children(&mut body_cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }
        // A pure virtual member (`virtual void f() = 0;`) is, per this
        // grammar version, not a dedicated `pure_virtual_clause` node
        // but a literal `=` token immediately followed by a
        // `number_literal` of `"0"` among the field_declaration's own
        // children (verified against the grammar's own parse tree,
        // same "verified against the grammar's own output" rationale
        // as `typescript.rs`'s decorator-field comment).
        let mut child_cursor = child.walk();
        let field_children: Vec<Node<'_>> = child.children(&mut child_cursor).collect();
        let has_pure_virtual = field_children
            .iter()
            .any(|c| c.kind() == "pure_virtual_clause")
            || field_children.iter().enumerate().any(|(idx, c)| {
                c.kind() == "="
                    && field_children
                        .get(idx + 1)
                        .map(|next| {
                            next.kind() == "number_literal" && next.utf8_text(src).ok() == Some("0")
                        })
                        .unwrap_or(false)
            });
        let declares_function = child
            .child_by_field_name("declarator")
            .map(|d| d.kind() == "function_declarator")
            .unwrap_or(false);
        if declares_function {
            if !has_pure_virtual {
                return false;
            }
            saw_virtual_member = true;
        }
    }
    saw_virtual_member
}

/// `class Sub : public Base1, private Base2` -- every base class name
/// in the `base_class_clause`, access specifiers (`public`/`private`/
/// `protected`) intentionally dropped (this crate's INHERITS edge
/// records only the sub/super name pair, same shape as `rust.rs`'s
/// supertrait bounds).
fn base_class_names(class_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut class_cursor = class_node.walk();
    for child in class_node.children(&mut class_cursor) {
        if child.kind() != "base_class_clause" {
            continue;
        }
        let mut child_cursor = child.walk();
        for entry in child.children(&mut child_cursor) {
            if matches!(
                entry.kind(),
                "type_identifier" | "qualified_identifier" | "template_type"
            ) {
                if let Ok(text) = entry.utf8_text(src) {
                    out.push(text.retained_display());
                }
            }
        }
    }
    out
}

/// Field names inside a class/struct `field_declaration_list` body
/// (data members only -- member function declarations are skipped
/// here since they are separately walked as [`SymbolKind::Method`] by
/// the default `function_definition` handling once `walk_children`
/// descends into this same body).
fn field_names(body: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() != "field_declaration" {
            continue;
        }
        if let Some(declarator) = child.child_by_field_name("declarator") {
            if declarator.kind() == "function_declarator" {
                continue; // a method prototype, not a data member.
            }
            if let Some(name) = innermost_declarator_identifier(declarator, src) {
                out.push(name);
            }
        }
    }
    out
}

/// `function_definition`: a free function, an in-class member method,
/// or an out-of-line `Class::method(...) { ... }` definition. The
/// out-of-line case is recognized by the declarator's innermost name
/// being a `qualified_identifier` (`Class::method`) -- its `scope` part
/// becomes the DEFINES container instead of (or in addition to) the
/// lexical `enclosing` scope, matching the workpack's explicit "Method
/// (member fns incl. out-of-line Class::method definitions)"
/// requirement.
fn handle_function_definition(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        walk_children(node, src, out, enclosing, fn_scope);
        return;
    };
    let Some((name, out_of_line_scope)) = declarator_name_and_scope(declarator, src) else {
        walk_children(node, src, out, enclosing, fn_scope);
        return;
    };
    let line = node.start_position().row + 1;

    // gtest `TEST(Suite, Name)` / `TEST_F(Fixture, Name)` /
    // `TEST_P(Fixture, Name)`: this grammar version parses the macro
    // invocation itself as an ordinary `function_definition` (verified
    // against the grammar's own parse tree -- there is no separate
    // "macro call" node kind it falls back to), so gtest detection
    // lives here rather than on `call_expression` as it would for a
    // real function call.
    if let Some(test_name) = gtest_macro_test_name(&name, declarator, src) {
        out.symbols.push(SymbolRef {
            name: (test_name).into(),
            kind: SymbolKind::Test,
            line: line.into(),
        });
        walk_children(node, src, out, enclosing, fn_scope);
        return;
    }

    let container = out_of_line_scope.as_deref().or(enclosing);
    let kind = if is_test_function(&name) {
        SymbolKind::Test
    } else if container.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };
    out.symbols.push(SymbolRef {
        name: (name.retained()).into(),
        kind,
        line: line.into(),
    });
    if let Some(container) = container {
        out.defines.push(DefinesRef {
            container_name: (container.retained_display()).into(),
            member_name: (name.retained()).into(),
            line: line.into(),
        });
    }
    // Descend into the body (and any nested lambdas/classes) with the
    // *lexical* `enclosing` unchanged -- an out-of-line method's body
    // is not itself inside `Class`'s lexical scope, so further nested
    // symbols should not spuriously inherit `Class` as their own
    // `enclosing` (matches how `rust.rs`/`typescript.rs` scope nested
    // walks to the innermost lexical container only). `fn_scope` DOES
    // update to this function/method, though -- a call inside its body
    // is "from" this symbol regardless of lexical-vs-out-of-line scope.
    walk_children(
        node,
        src,
        out,
        enclosing,
        FnScope {
            name: Some(name.as_str()),
            line: Some(line),
        },
    );
}

/// The innermost identifier name of a declarator, plus (for a
/// `qualified_identifier`-shaped name, i.e. `Class::method`) the
/// scope's name as the out-of-line container.
fn declarator_name_and_scope(node: Node<'_>, src: &[u8]) -> Option<(String, Option<String>)> {
    match node.kind() {
        "function_declarator" | "pointer_declarator" | "reference_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|inner| declarator_name_and_scope(inner, src)),
        "qualified_identifier" => {
            let name_node = node.child_by_field_name("name")?;
            let name = name_node.utf8_text(src).ok()?.retained_display();
            let scope = node
                .child_by_field_name("scope")
                .and_then(|s| s.utf8_text(src).ok())
                .map(str::to_string);
            Some((name, scope))
        }
        "identifier" | "field_identifier" | "destructor_name" | "operator_name" => node
            .utf8_text(src)
            .ok()
            .map(|s| (s.retained_display(), None)),
        _ => None,
    }
}

fn innermost_declarator_identifier(node: Node<'_>, src: &[u8]) -> Option<String> {
    declarator_name_and_scope(node, src).map(|(name, _)| name)
}

/// `typedef struct {...} Foo;` / `typedef int MyInt;` -- every
/// declarator name a `type_definition` introduces. Same "skip the
/// type-specifier side's own `type_identifier`" care as `languages::c`'s
/// `typedef_alias_names` (`typedef struct Point PointAlias;`'s `Point`
/// must not be collected as an alias name).
fn typedef_alias_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "typedef" | "struct_specifier" | "union_specifier" | "enum_specifier"
            | "class_specifier" | ";" => {
                continue;
            }
            "type_identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.retained_display());
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

/// Same split as `languages::c`'s top-level declaration handling:
/// `const`-qualified globals become [`SymbolKind::Constant`], everything
/// else [`SymbolKind::Variable`] -- skipped entirely when inside a
/// class/struct body (member field declarations there are handled by
/// `field_names`) or a namespace (still module-scoped globals, treated
/// the same as top-level for this best-effort pass).
fn top_level_declaration_symbols(
    node: Node<'_>,
    src: &[u8],
    inside_class_or_struct: bool,
) -> Vec<SymbolRef> {
    if inside_class_or_struct {
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
            continue;
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

/// `auto f = [](...) { ... };` / `auto f = [captures](...) { ... };` --
/// a named lambda variable binding on a `declaration` node, best-effort
/// [`SymbolKind::Lambda`] source, mirroring `rust.rs`'s
/// `named_closure_binding` / `typescript.rs`'s
/// `named_arrow_or_const_binding`.
fn named_lambda_binding(declaration_node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    let mut cursor = declaration_node.walk();
    for inner in declaration_node.children(&mut cursor) {
        if inner.kind() != "init_declarator" {
            continue;
        }
        let declarator = inner.child_by_field_name("declarator")?;
        let value = inner.child_by_field_name("value")?;
        if value.kind() == "lambda_expression" {
            let name = innermost_declarator_identifier(declarator, src)?;
            return Some(SymbolRef {
                name: name.into(),
                kind: SymbolKind::Lambda,
                line: (declaration_node.start_position().row + 1).into(),
            });
        }
    }
    None
}

/// `f = [](...) { ... };` -- a lambda assigned (not declared-and-
/// initialized) to an existing bare identifier, on an
/// `expression_statement` wrapping an `assignment_expression`.
fn named_lambda_binding_from_assignment(
    expression_statement_node: Node<'_>,
    src: &[u8],
) -> Option<SymbolRef> {
    let child = expression_statement_node.child(0)?;
    if child.kind() != "assignment_expression" {
        return None;
    }
    let left = child.child_by_field_name("left")?;
    let right = child.child_by_field_name("right")?;
    if right.kind() == "lambda_expression" && left.kind() == "identifier" {
        let name = left.utf8_text(src).ok()?.retained_display();
        return Some(SymbolRef {
            name: name.into(),
            kind: SymbolKind::Lambda,
            line: (expression_statement_node.start_position().row + 1).into(),
        });
    }
    None
}

fn include_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = node.child_by_field_name("path")?;
    let raw = path_node.utf8_text(src).ok()?;
    Some(
        raw.trim_matches(|c| c == '"' || c == '<' || c == '>')
            .retained_display(),
    )
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// gtest-family `TEST(Suite, Name)`/`TEST_F(Fixture, Name)` are
/// function-call-shaped macros in tree-sitter's grammar (no dedicated
/// node kind), so they are recognized directly here rather than via the
/// name-convention fallback: `TEST`/`TEST_F`/`TEST_P` invoked with two
/// identifier-ish arguments. `languages::c`'s name-convention heuristic
/// (`test_`-prefixed / `_test`-suffixed) remains the fallback for
/// everything else, matching the workpack's "gtest TEST()/TEST_F macros,
/// files under test/ or *_test.c(pp)" dual signal.
fn is_test_function(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test_") || lower.ends_with("_test")
}

/// Recognize a gtest `TEST(Suite, Name)` / `TEST_F(Fixture, Name)` /
/// `TEST_P(Fixture, Name)` invocation and synthesize a test symbol name
/// `Suite.Name`, matching gtest's own `Suite.Name` test-id convention
/// (and this crate's need for a single string to key a
/// [`SymbolKind::Test`] symbol by, since the macro itself introduces no
/// ordinary declarator name).
/// `declarator` is the `function_definition`'s own `declarator` field
/// (a `function_declarator` for the `TEST(Suite, Name)` macro-as-
/// function-definition shape this grammar version produces -- see
/// `handle_function_definition`'s call site doc). Its two
/// comma-separated "parameters" are grammatically ambiguous
/// identifiers: tree-sitter-cpp's own heuristic classifies each as
/// either a plain `identifier` or a `type_identifier` (both observed
/// against the grammar's own parse tree for `TEST(MathSuite,
/// AddsNumbers)`), so both are accepted here.
fn gtest_macro_test_name(callee: &str, declarator: Node<'_>, src: &[u8]) -> Option<String> {
    if !matches!(callee, "TEST" | "TEST_F" | "TEST_P") {
        return None;
    }
    let params = declarator.child_by_field_name("parameters")?;
    let mut params_cursor = params.walk();
    let idents: Vec<&str> = params
        .children(&mut params_cursor)
        .filter_map(|n| match n.kind() {
            "identifier" | "type_identifier" => n.utf8_text(src).ok(),
            // `TEST(MathSuite, AddsNumbers)`: each bare comma-separated
            // name parses as a `parameter_declaration` whose sole child
            // is a `type_identifier` (there is no `declarator` field at
            // all for a type-only, unnamed parameter -- verified
            // against the grammar's own parse tree) -- take that
            // child's text directly.
            "parameter_declaration" => n
                .child_by_field_name("declarator")
                .or_else(|| n.child_by_field_name("type"))
                .or_else(|| n.child(0))
                .and_then(|d| d.utf8_text(src).ok()),
            _ => None,
        })
        .collect();
    if idents.len() < 2 {
        return None;
    }
    let first = idents.first()?;
    let second = idents.get(1)?;
    Some(format!("{first}.{second}"))
}

/// For a `field_expression`-shaped callee (`x.foo()`/`x->foo()`, both
/// parse as `field_expression` in this grammar), the receiver text plus
/// a cheap syntactic hint.
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "field_expression" {
        return (None, None);
    }
    let Some(receiver) = function_node.child_by_field_name("argument") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "new_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "string_literal" | "number_literal" | "true" | "false"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.retained_display()), Some(hint))
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
            out.push(text.retained_display());
        }
    }
    out
}
