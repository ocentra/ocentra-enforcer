//! Generic spec-table-driven tree-sitter walker, mirroring the C
//! baseline's shared `extract_defs.c`/`extract_calls.c`/
//! `extract_imports.c`: one walk, driven entirely by a
//! [`crate::languages::spec::LangSpec`]'s node-kind name arrays, that
//! emits the exact same [`crate::parsers::ParsedFile`] shape every
//! bespoke `languages/*.rs` extractor already emits -- so
//! [`crate::code_graph`]/[`crate::complexity`]/[`crate::resolution`]
//! never need to know whether a given file went through this walker
//! or a bespoke one.
//!
//! # Quirk hook
//! A handful of per-language behaviors cannot be expressed as a flat
//! node-kind name list (Go's struct-vs-interface `type_spec` split and
//! embedded-field INHERITS detection, TS's `extends`/`implements`
//! heritage clauses, ...). [`Quirks`] is the seam for those --
//! `LangSpec`-driven callers pass a `Quirks` value (empty/no-op by
//! default) whose hooks the generic walker calls at well-defined
//! points, mirroring the baseline's `if (lang == CBM_LANG_X)`
//! one-offs in `extract_defs.c`/`extract_calls.c` without polluting
//! this module's generic path with per-language `match` arms.
//!
//! # G1 scope
//! [`parse_go`] is the only language fully routed through this walker
//! this wave (the plan's explicit "migrate Go FIRST" zero-regression
//! proof) -- see `spec.rs`'s module doc for why the other 9 `LangSpec`
//! rows are data-complete but not yet dispatched here.

use crate::languages::spec::LangSpec;
use crate::parsers::{
    CallRef, DefinesRef, ImportRef, InheritsRef, ParsedFile, ReceiverHint, RouteRef, SymbolKind,
    SymbolRef,
};
use tree_sitter::{Node, Parser};

/// The innermost function/method a call expression is lexically inside
/// of, if any -- same bundled-scope pattern every bespoke extractor
/// already uses (`FnScope` in `rust.rs`/`go.rs`/`typescript.rs`/...).
#[derive(Debug, Clone, Copy, Default)]
struct FnScope<'a> {
    name: Option<&'a str>,
    line: Option<usize>,
}

/// Signature for [`Quirks::on_unmatched_node`] -- factored into its own
/// alias (rather than inlined in the field) purely to keep the type
/// readable; clippy's `type_complexity` lint flags inline
/// `Box<dyn Fn(...) -> ...>` signatures with more than a couple of
/// parameters.
type UnmatchedNodeHook = Box<dyn Fn(Node<'_>, &[u8], &mut ParsedFile) -> bool>;

/// Signature for [`Quirks::route_from_call`].
type RouteFromCallHook = Box<dyn Fn(&str, Node<'_>, &[u8]) -> Option<RouteRef>>;

/// Signature for [`Quirks::on_method_defined`].
type MethodDefinedHook = Box<dyn Fn(Node<'_>, &str, usize, &[u8], &mut ParsedFile)>;

/// Per-language override hooks the generic walker calls at fixed
/// points, mirroring the baseline's per-language quirk branches.
/// Every hook defaults to a no-op (`None`/empty) via
/// [`Quirks::default`], so a `LangSpec` with no quirks at all still
/// walks correctly through the fully generic path.
pub struct Quirks {
    /// Called for every node the generic walker does not otherwise
    /// recognize via `LangSpec`'s flat arrays, before generic
    /// recursion into its children. Returning `true` tells the walker
    /// this node (and its subtree) was fully handled by the quirk and
    /// should not be recursed into generically again.
    pub on_unmatched_node: Option<UnmatchedNodeHook>,
    /// Test-name convention predicate (`TestXxx` for Go, `test_*`/
    /// `Test*` for others, ...). Defaults to "never a test" (`false`
    /// for every name) -- callers that care route file-level test
    /// detection through this hook the same way `go::parse`'s
    /// `is_test_file` parameter does.
    pub is_test_name: fn(&str) -> bool,
    /// Recognize an HTTP-route-registration call
    /// (`mux.HandleFunc("/x", h)`, `app.get("/x", h)`, ...). Defaults
    /// to "never a route".
    pub route_from_call: Option<RouteFromCallHook>,
    /// Called immediately after the generic walker records a
    /// function/method symbol, for languages whose DEFINES container
    /// is not simply "the lexically enclosing class/struct body" --
    /// e.g. Go's `func (w *Widget) Draw()`, where the container
    /// (`Widget`) comes from the receiver clause, not from syntactic
    /// nesting (Go methods are not nested inside their type's body at
    /// all). Defaults to a no-op.
    pub on_method_defined: Option<MethodDefinedHook>,
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            on_unmatched_node: None,
            is_test_name: |_| false,
            route_from_call: None,
            on_method_defined: None,
        }
    }
}

/// Shared walk context threaded through the recursive walk -- bundles
/// the `LangSpec`, source bytes, quirks, and output sink so the walk
/// functions below stay narrow in their own parameter lists.
struct Ctx<'a> {
    spec: &'a LangSpec,
    src: &'a [u8],
    quirks: &'a Quirks,
    is_test_file: bool,
}

/// Parse `source` with tree-sitter `language`, then walk the tree
/// generically per `spec`/`quirks`. `is_test_file` feeds
/// `quirks.is_test_name` the same file-level gate `go::parse` already
/// uses for `_test.go` files.
pub fn parse_with_spec(
    source: &str,
    language: &tree_sitter::Language,
    spec: &LangSpec,
    quirks: &Quirks,
    is_test_file: bool,
) -> ParsedFile {
    let mut parser = Parser::new();
    if parser.set_language(language).is_err() {
        return ParsedFile::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParsedFile::default();
    };

    let mut out = ParsedFile::default();
    let ctx = Ctx {
        spec,
        src: source.as_bytes(),
        quirks,
        is_test_file,
    };
    walk(tree.root_node(), &ctx, &mut out, None, FnScope::default());
    out
}

fn walk(
    node: Node<'_>,
    ctx: &Ctx<'_>,
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    let kind = node.kind();
    let spec = ctx.spec;

    if spec.module_types.contains(&kind) {
        if let Some(name) = first_named_child_text(node, ctx.src) {
            out.symbols.push(SymbolRef {
                name,
                kind: SymbolKind::Module,
                line: node.start_position().row + 1,
            });
        }
        walk_children(node, ctx, out, enclosing, fn_scope);
        return;
    }

    if spec.import_types.contains(&kind) {
        // Generic import handling is intentionally minimal: most
        // grammars nest their path string arbitrarily deep (Go's
        // `import ( "a"; "b" )`, C's `#include <a.h>`), which the
        // quirk hook is far better placed to walk than a flat
        // "the import path is always in field X" assumption. Callers
        // needing generic import edges supply an `on_unmatched_node`
        // quirk; the walker still recurses into this node's children
        // generically afterward so nested call/import nodes inside a
        // grouped import block are still visited.
        if let Some(handler) = &ctx.quirks.on_unmatched_node {
            if handler(node, ctx.src, out) {
                return;
            }
        }
        walk_children(node, ctx, out, enclosing, fn_scope);
        return;
    }

    if spec.func_types.contains(&kind) || spec.method_types.contains(&kind) {
        if let Some(name) = child_text(node, spec.name_field, ctx.src) {
            let line = node.start_position().row + 1;
            // A node kind exclusive to `method_types` (not also listed
            // in `func_types`) is always a method regardless of lexical
            // nesting -- covers grammars like Go's, where a method
            // declaration is syntactically top-level (tied to its
            // receiver type, not nested in a class/impl body) but is
            // still semantically a method. A node kind shared by both
            // arrays (e.g. Rust's `function_item` used for both free
            // functions and impl methods) falls back to the nesting
            // check.
            let is_method = spec.method_types.contains(&kind)
                && (!spec.func_types.contains(&kind) || enclosing.is_some());
            let kind_out = if ctx.is_test_file && (ctx.quirks.is_test_name)(&name) {
                SymbolKind::Test
            } else if is_method {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: kind_out,
                line,
            });
            if let Some(container) = enclosing {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name.clone(),
                    line,
                });
            }
            if let Some(hook) = &ctx.quirks.on_method_defined {
                hook(node, &name, line, ctx.src, out);
            }
            if let Some(body) = node.child_by_field_name(spec.body_field) {
                walk_children(
                    body,
                    ctx,
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

    if spec.class_types.contains(&kind)
        || spec.interface_types.contains(&kind)
        || spec.enum_types.contains(&kind)
        || spec.alias_types.contains(&kind)
    {
        // Class/struct/interface/enum/alias shapes vary too much
        // across grammars (Go's struct-vs-interface split lives inside
        // one `type_spec` node's `type` field, not the node kind
        // itself) for a single generic branch to classify correctly --
        // this is the quirk hook's primary job. If no quirk claims the
        // node, fall back to a generic Class/Interface/Enum/TypeAlias
        // symbol keyed off `name_field` so the language is never
        // silently dropped.
        if let Some(handler) = &ctx.quirks.on_unmatched_node {
            if handler(node, ctx.src, out) {
                return;
            }
        }
        if let Some(name) = child_text(node, spec.name_field, ctx.src) {
            let line = node.start_position().row + 1;
            let generic_kind = if spec.interface_types.contains(&kind) {
                SymbolKind::Interface
            } else if spec.enum_types.contains(&kind) {
                SymbolKind::Enum
            } else if spec.alias_types.contains(&kind) {
                SymbolKind::TypeAlias
            } else {
                SymbolKind::Class
            };
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: generic_kind,
                line,
            });
            walk_children(node, ctx, out, Some(name.as_str()), fn_scope);
            return;
        }
    }

    if spec.field_types.contains(&kind) {
        if let (Some(container), Some(name)) =
            (enclosing, child_text(node, spec.name_field, ctx.src))
        {
            out.defines.push(DefinesRef {
                container_name: container.to_string(),
                member_name: name,
                line: node.start_position().row + 1,
            });
        }
    }

    if spec.call_types.contains(&kind) {
        if let Some(function) = node.child_by_field_name(spec.call_function_field) {
            if let Ok(callee) = function.utf8_text(ctx.src) {
                let callee = callee.to_string();
                let (receiver_text, receiver_hint) = receiver_of_call(function, ctx.src);
                out.calls.push(CallRef {
                    callee: callee.clone(),
                    line: node.start_position().row + 1,
                    from_symbol: fn_scope.name.map(str::to_string),
                    from_symbol_line: fn_scope.line,
                    receiver_text,
                    receiver_hint,
                    arg_texts: call_arg_texts(node, spec.call_arguments_field, ctx.src),
                });
                if let Some(route_fn) = &ctx.quirks.route_from_call {
                    if let Some(route) = route_fn(&callee, node, ctx.src) {
                        out.routes.push(route);
                    }
                }
            }
        }
    }

    // Any node kind not otherwise recognized above still gets a
    // chance at the quirk hook (e.g. Go's `const_declaration`/
    // `var_declaration`) before generic recursion.
    if let Some(handler) = &ctx.quirks.on_unmatched_node {
        if handler(node, ctx.src, out) {
            return;
        }
    }

    walk_children(node, ctx, out, enclosing, fn_scope);
}

fn walk_children(
    node: Node<'_>,
    ctx: &Ctx<'_>,
    out: &mut ParsedFile,
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
) {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, ctx, out, enclosing, fn_scope);
        }
    }
}

/// A method-call-shaped callee's receiver text plus a cheap syntactic
/// hint -- generic across grammars that model a method call as
/// `<object-like-node>.<name>` with the object in some field. Since
/// the object field name itself varies (`operand` for Go, `object` for
/// TS, `value` for Rust, ...), this only handles the common shape of
/// "the callee node has exactly one non-punctuation, non-name-field
/// child that looks like an expression" heuristically via the second
/// named child; languages needing precise receiver semantics should
/// supply their own via a quirk (kept generic-only for the wave-G1
/// zero-regression proof on Go, whose `selector_expression` shape this
/// already matches exactly).
fn receiver_of_call(function_node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "selector_expression" && function_node.kind() != "member_expression"
    {
        return (None, None);
    }
    let field = if function_node.kind() == "selector_expression" {
        "operand"
    } else {
        "object"
    };
    let Some(receiver) = function_node.child_by_field_name(field) else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "self" || text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "call_expression" && is_new_call(receiver, src) {
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
            | "string"
            | "number"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Go's `NewXxx(...)` constructor-idiom heuristic -- same rationale as
/// `languages/go.rs`'s `is_new_call`.
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
fn call_arg_texts(call_node: Node<'_>, arguments_field: &str, src: &[u8]) -> Vec<String> {
    let Some(args) = call_node.child_by_field_name(arguments_field) else {
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

/// HTTP methods recognized in route-registration calls -- shared by
/// every language's `route_from_call` quirk (same list every bespoke
/// extractor already duplicates).
const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// Go's specific `type_declaration`/`type_spec` struct-vs-interface
/// split, embedded-field INHERITS, interface method DEFINES,
/// `const`/`var` spec flattening, and import-path extraction --
/// everything Go's flat `LangSpec` arrays cannot express, wired as
/// this wave's [`Quirks::on_unmatched_node`] hook (the method-receiver
/// DEFINES edge is a separate [`Quirks::on_method_defined`] hook --
/// see [`go_on_method_defined`] -- since `method_declaration` is
/// already claimed by the generic walker's own func/method branch
/// before `on_unmatched_node` ever sees it). Mirrors
/// `languages/go.rs`'s `walk_type_declaration`/`spec_names`/
/// `import_paths` byte-for-byte in output shape (not code) so
/// `parse_go`'s output matches `go::parse`'s on every existing
/// fixture/test.
fn go_quirk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "type_declaration" => {
            walk_go_type_declaration(node, src, out);
            // Matches `languages/go.rs`'s `walk_type_declaration` call
            // site: it does not recurse into a `type_declaration`'s own
            // children afterward (a type spec's field/method-elem
            // names are plain identifiers, not call expressions, so
            // there is nothing further the generic walk would find).
            true
        }
        "const_declaration" => {
            for name in go_spec_names(node, "const_spec", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Constant,
                    line: node.start_position().row + 1,
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
            for name in go_spec_names(node, "var_spec", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Variable,
                    line: node.start_position().row + 1,
                });
            }
            // Same rationale as `const_declaration` above.
            false
        }
        "import_declaration" => {
            for path in go_import_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
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
    if let Some(receiver_type) = go_receiver_type_name(node, src) {
        out.defines.push(DefinesRef {
            container_name: receiver_type,
            member_name: name.to_string(),
            line,
        });
    }
}

fn walk_go_type_declaration(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    for i in 0..node.child_count() {
        let Some(spec) = node.child(i) else { continue };
        if spec.kind() == "type_alias" {
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
                for (member_name, embedded) in go_struct_fields(type_node, src) {
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
                for method_name in go_interface_methods(type_node, src) {
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

fn go_struct_fields(struct_node: Node<'_>, src: &[u8]) -> Vec<(String, bool)> {
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

fn go_interface_methods(interface_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

fn go_receiver_type_name(method_node: Node<'_>, src: &[u8]) -> Option<String> {
    let receiver = method_node.child_by_field_name("receiver")?;
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

fn go_spec_names(decl_node: Node<'_>, spec_kind: &str, src: &[u8]) -> Vec<String> {
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

fn go_import_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "import_spec" => {
                if let Some(path) = go_import_spec_path(child, src) {
                    out.push(path);
                }
            }
            "import_spec_list" => {
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "import_spec" {
                            if let Some(path) = go_import_spec_path(spec, src) {
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

fn go_import_spec_path(spec: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = spec.child_by_field_name("path")?;
    let raw = path_node.utf8_text(src).ok()?;
    Some(raw.trim_matches('"').to_string())
}

fn go_is_test_name(name: &str) -> bool {
    name.starts_with("Test") || name.starts_with("Benchmark") || name.starts_with("Example")
}

/// Recognize `net/http`-style/mux-style route registration, same
/// rules as `languages/go.rs`'s `route_from_call`.
fn go_route_from_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
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

/// Go's [`Quirks`] row: embedded-field INHERITS, struct-vs-interface
/// `type_spec` split, `const`/`var` flattening, grouped import paths,
/// `TestXxx`/`BenchmarkXxx`/`ExampleXxx` test-name convention, and
/// `net/http`/mux-style route detection.
pub fn go_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(go_quirk)),
        is_test_name: go_is_test_name,
        route_from_call: Some(Box::new(go_route_from_call)),
        on_method_defined: Some(Box::new(go_on_method_defined)),
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
