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
//! # G1/G1b scope
//! All 10 of this crate's languages (Go, Rust, TypeScript/JavaScript,
//! Python, Java, C, C++, C#, PHP) are now fully routed through this
//! walker (`parse_go`/`parse_rust`/`parse_typescript`/`parse_python`/
//! `parse_java`/`parse_c`/`parse_cpp`/`parse_csharp`/`parse_php`),
//! dispatched from [`crate::parsers::parse_file`]. Every bespoke
//! `languages/*.rs` extractor is kept in place purely as a
//! zero-regression oracle -- `tests/unit_lang_spec_engine.rs` proves
//! each generic-path function reproduces its bespoke counterpart's
//! output on every existing scenario -- and is never called from
//! `parse_file` anymore. C/C++/PHP needed a full quirk claim rather
//! than the generic engine's own `name_field`-keyed fallback (see
//! `LangSpec::c`/`LangSpec::cpp`/`LangSpec::php`'s own doc comments for
//! why); the rest use a mix of flat-array dispatch plus the
//! [`Quirks`] seam below for their rich-tier behavior (heritage
//! clauses, decorators, routes, ...).

use crate::languages::spec::LangSpec;
use crate::parsers::{
    CallRef, DefinesRef, ImplementsRef, ImportRef, InheritsRef, ParsedFile, ReceiverHint, RouteRef,
    SymbolKind, SymbolRef,
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
/// parameters. `enclosing` is the name of whatever lexically-containing
/// class/struct/impl/trait/namespace `walk` was already tracking at
/// the point this node was reached -- languages whose quirk needs to
/// know it (C++'s in-class `function_definition` handling, in
/// particular) read it directly rather than losing that context the
/// moment control passes into this hook.
type UnmatchedNodeHook = Box<dyn Fn(Node<'_>, Option<&str>, &[u8], &mut ParsedFile) -> bool>;

/// Signature for [`Quirks::route_from_call`].
type RouteFromCallHook = Box<dyn Fn(&str, Node<'_>, &[u8]) -> Option<RouteRef>>;

/// Signature for [`Quirks::on_method_defined`].
type MethodDefinedHook = Box<dyn Fn(Node<'_>, &str, usize, &[u8], &mut ParsedFile)>;

/// Signature for [`Quirks::call_override`]: the call node, the
/// innermost enclosing function/method name + line (`fn_scope`, same
/// pair `walk` threads through everywhere else), the source bytes, and
/// the output sink.
type CallOverrideHook =
    Box<dyn Fn(Node<'_>, Option<&str>, Option<usize>, &[u8], &mut ParsedFile) -> bool>;

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
    /// Called for every `LangSpec::call_types` node *before* the
    /// generic walker's own callee-text-from-one-field reconstruction
    /// runs. Returning `true` tells the walker the call was fully
    /// recorded by the quirk (skip the generic push entirely) --
    /// needed for grammars whose call-shaped node splits the receiver
    /// and method name into two separate fields rather than one field
    /// holding the whole written expression (Java's `method_invocation`
    /// has `object`/`name` fields, unlike Go/Rust/TS/Python's call node,
    /// whose single callee field already contains the full receiver-
    /// qualified text). Defaults to a no-op (every call falls through
    /// to the generic single-field reconstruction).
    pub call_override: Option<CallOverrideHook>,
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            on_unmatched_node: None,
            is_test_name: |_| false,
            route_from_call: None,
            on_method_defined: None,
            call_override: None,
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
            if handler(node, enclosing, ctx.src, out) {
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
            if handler(node, enclosing, ctx.src, out) {
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
        let overridden = ctx
            .quirks
            .call_override
            .as_ref()
            .is_some_and(|handler| handler(node, fn_scope.name, fn_scope.line, ctx.src, out));
        if overridden {
            if let Some(handler) = &ctx.quirks.on_unmatched_node {
                if handler(node, enclosing, ctx.src, out) {
                    return;
                }
            }
            walk_children(node, ctx, out, enclosing, fn_scope);
            return;
        }
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
        if handler(node, enclosing, ctx.src, out) {
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
    // The object/receiver field name and the constructor-idiom test
    // both vary by grammar/convention (Go: `operand` field,
    // `NewXxx(...)` prefix idiom via `.`-paths; TS: `object` field,
    // no built-in `new`-idiom convention beyond an explicit `new`
    // expression the callee text itself would show; Rust: `value`
    // field, `Xxx::new(...)` suffix idiom via `::`-paths) -- resolved
    // per node kind rather than per `LangSpec`, since the field name is
    // implied by the callee-shape node kind itself, not by anything a
    // flat `LangSpec` array could vary.
    let field = match function_node.kind() {
        "selector_expression" => "operand",
        "member_expression" => "object",
        "field_expression" => "value",
        "attribute" => "object",
        "member_access_expression" => "expression",
        _ => return (None, None),
    };
    let Some(receiver) = function_node.child_by_field_name(field) else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "self" || text == "this" {
        ReceiverHint::SelfOrThis
    } else if matches!(receiver.kind(), "call_expression" | "call") && is_new_call(receiver, src) {
        // Go's `call_expression` (`NewXxx(...)` dot-path prefix
        // convention) and Python's `call` (PascalCase-callee
        // convention, `languages/python.rs`'s own `is_new_call`) both
        // funnel through the same node-kind-driven dispatch inside
        // `is_new_call` below -- neither grammar's `call`-shaped node
        // kind collides with the other's.
        ReceiverHint::NewExpression
    } else if matches!(
        receiver.kind(),
        "new_expression" | "object_creation_expression"
    ) {
        // TS/JS (`new_expression`) and C# (`object_creation_expression`)
        // both have a dedicated `new`-expression node kind rather than
        // an ordinary-call-plus-name-convention idiom -- mirrors
        // `languages/typescript.rs`'s and `languages/csharp.rs`'s own
        // `receiver_of_call` exactly.
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
            | "integer_literal"
            | "string_literal"
            | "boolean_literal"
            | "integer"
            | "float"
            | "real_literal"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Constructor-idiom heuristic covering Go's `NewXxx(...)` dot-path
/// prefix convention (`languages/go.rs`'s `is_new_call`), Rust's
/// `Xxx::new(...)` double-colon-path suffix convention
/// (`languages/rust.rs`'s `is_new_call`), and Python's PascalCase-
/// callee convention (`languages/python.rs`'s `is_new_call`) --
/// distinguished by which node kind called in (Python's is the only
/// grammar whose call-shaped node kind is literally `"call"`, so that
/// branch is unambiguous) and, for the shared `call_expression` kind,
/// by which path separator the callee text actually uses (a Go dotted
/// path never contains `::`, and idiomatic Rust paths are exactly the
/// reverse).
fn is_new_call(call_node: Node<'_>, src: &[u8]) -> bool {
    let Some(function) = call_node.child_by_field_name("function") else {
        return false;
    };
    let Ok(text) = function.utf8_text(src) else {
        return false;
    };
    if call_node.kind() == "call" {
        // Python has no dedicated `new`-expression syntax -- treat a
        // call as constructor-shaped when its own callee's final
        // dotted segment looks like a class name (`PascalCase`
        // convention), same rationale as `languages/python.rs`'s
        // `is_new_call`.
        let last = text.rsplit('.').next().unwrap_or(text);
        return last.chars().next().is_some_and(|c| c.is_uppercase());
    }
    if text.contains("::") {
        text.rsplit("::").next() == Some("new")
    } else {
        text.rsplit('.')
            .next()
            .is_some_and(|segment| segment.starts_with("New"))
    }
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
fn go_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
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

/// Rust's `#[test]`-family attribute test-name gate is annotation-based
/// (not filename-based like Go), so it is handled inline in
/// [`rust_on_method_defined`]/the function-kind branch above via
/// [`rust_has_test_attribute`] rather than [`Quirks::is_test_name`]
/// (which only ever sees the bare identifier, not its preceding
/// attributes) -- `is_test_name` stays the default "never a test" for
/// Rust and [`Quirks::on_unmatched_node`] reclassifies the symbol
/// generic already pushed, matching `languages/rust.rs`'s own
/// `has_test_attribute` check byte-for-byte in output shape.
fn rust_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_item" => {
            // Already handled by the generic func/method branch before
            // `on_unmatched_node` is ever consulted for this kind (it
            // `return`s early on a successful name match) -- this arm
            // only runs for the rare case the name field is missing, in
            // which case there is nothing further to do.
            false
        }
        "use_declaration" => {
            for path in rust_use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "struct_item" => {
            // The generic class-shape fallback only distinguishes
            // Interface/Enum/TypeAlias, defaulting everything else to
            // `SymbolKind::Class` -- Rust needs the more specific
            // `SymbolKind::Struct` (matches `languages/rust.rs`'s own
            // `struct_item` arm), so this quirk claims the node fully
            // rather than falling through to that generic default.
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Struct,
                    line: node.start_position().row + 1,
                });
            }
            // `languages/rust.rs`'s `struct_item` arm never sets
            // `enclosing` to the struct's own name when recursing (see
            // `LangSpec::rust()`'s `field_types` doc comment) -- match
            // that by walking children under the *outer* `enclosing`
            // this quirk was invoked with is unavailable here (the
            // quirk signature carries no `enclosing`), but a struct
            // body cannot contain further func/method/class nodes in
            // Rust's grammar, so there is nothing to recurse into that
            // would ever observe the difference; returning `true` with
            // no further recursion is therefore equivalent in every
            // observable output.
            true
        }
        "trait_item" => {
            // Supertrait bounds (`trait Sub: Super1 + Super2`) --
            // pushed here, then fall through (`false`) so the generic
            // class-shape fallback still emits the Interface symbol and
            // recurses with `enclosing` set to the trait name, exactly
            // matching `languages/rust.rs`'s own `trait_item` arm.
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                for supertrait in rust_trait_bounds(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: supertrait,
                        line,
                    });
                }
            }
            false
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
                    type_name: type_name.clone(),
                    trait_name: trait_name.clone(),
                    line: node.start_position().row + 1,
                });
            }
            rust_walk_impl_body(node, src, type_name.as_deref(), out);
            true
        }
        "let_declaration" => {
            if let Some(closure_name) = rust_named_closure_binding(node, src) {
                out.symbols.push(SymbolRef {
                    name: closure_name,
                    kind: SymbolKind::Lambda,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "const_item" | "static_item" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Constant,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        _ => false,
    }
}

/// `impl_item` bodies are not one of `LangSpec::rust()`'s `class_types`
/// (an `impl` block is not itself a type declaration), so the generic
/// walker never recurses into one with `enclosing` set -- this quirk
/// re-implements that one recursive call directly (mirrors
/// `languages/rust.rs`'s `impl_item` arm calling `walk_children` with
/// `enclosing = type_name.as_deref()`), then hands each child straight
/// back to [`walk`] so every nested function/call/use inside the `impl`
/// body still goes through the fully generic path (decorators,
/// type-refs, DEFINES, and all).
fn rust_walk_impl_body(
    impl_node: Node<'_>,
    src: &[u8],
    type_name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::rust();
    let quirks = rust_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..impl_node.child_count() {
        if let Some(child) = impl_node.child(i) {
            walk(child, &ctx, out, type_name, FnScope::default());
        }
    }
}

/// Attribute-macro DECORATES + parameter/return TYPE_REF edges for a
/// just-recorded Rust function/method symbol, plus `#[test]`-family
/// reclassification -- wired as [`Quirks::on_method_defined`], mirrors
/// `languages/rust.rs`'s `function_item` arm.
fn rust_on_method_defined(
    node: Node<'_>,
    name: &str,
    line: usize,
    src: &[u8],
    out: &mut ParsedFile,
) {
    if node.kind() != "function_item" {
        return;
    }
    if rust_has_test_attribute(node, src) {
        if let Some(last) = out.symbols.last_mut() {
            if last.name == name && last.line == line {
                last.kind = SymbolKind::Test;
            }
        }
    }
    for decorator in rust_attribute_decorators(node, src) {
        out.decorates.push(crate::parsers::DecoratesRef {
            target_name: name.to_string(),
            decorator_name: decorator,
            line,
        });
    }
    for type_ref in rust_signature_type_refs(node, src) {
        out.type_refs.push(crate::parsers::TypeRefRef {
            from_name: name.to_string(),
            type_name: type_ref,
            line,
        });
    }
}

fn rust_attribute_decorators(function_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut sibling = function_node.prev_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            "attribute_item" => {
                if let Ok(text) = node.utf8_text(src) {
                    if !rust_attribute_is_test(text) {
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

fn rust_signature_type_refs(function_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = function_node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if param.kind() == "parameter" {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
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

fn rust_trait_bounds(trait_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(bounds) = trait_node.child_by_field_name("bounds") {
        for i in 0..bounds.child_count() {
            if let Some(child) = bounds.child(i) {
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
    }
    out
}

fn rust_named_closure_binding(let_node: Node<'_>, src: &[u8]) -> Option<String> {
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

fn rust_has_test_attribute(function_node: Node<'_>, src: &[u8]) -> bool {
    let mut sibling = function_node.prev_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            "attribute_item" | "line_comment" | "block_comment" => {
                if node.kind() == "attribute_item" {
                    if let Ok(text) = node.utf8_text(src) {
                        if rust_attribute_is_test(text) {
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

fn rust_attribute_is_test(attribute_text: &str) -> bool {
    let inner = attribute_text
        .trim_start_matches('#')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    inner == "test" || inner.ends_with("::test")
}

/// Flatten a `use_declaration`'s tree into every concrete imported path
/// it declares, mirrors `languages/rust.rs`'s `use_paths` byte-for-byte.
fn rust_use_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(argument) = node.child_by_field_name("argument") {
        rust_collect_use_paths(argument, src, "", &mut paths);
    }
    if paths.is_empty() {
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

fn rust_collect_use_paths(node: Node<'_>, src: &[u8], prefix: &str, out: &mut Vec<String>) {
    match node.kind() {
        "scoped_use_list" => {
            let base = node
                .child_by_field_name("path")
                .and_then(|n| n.utf8_text(src).ok())
                .unwrap_or("");
            let joined = rust_join_prefix(prefix, base);
            if let Some(list) = node.child_by_field_name("list") {
                for i in 0..list.child_count() {
                    if let Some(child) = list.child(i) {
                        if child.kind() != "," && child.kind() != "{" && child.kind() != "}" {
                            rust_collect_use_paths(child, src, &joined, out);
                        }
                    }
                }
            }
        }
        "use_list" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() != "," && child.kind() != "{" && child.kind() != "}" {
                        rust_collect_use_paths(child, src, prefix, out);
                    }
                }
            }
        }
        "use_as_clause" => {
            if let Some(path_node) = node.child_by_field_name("path") {
                let text = path_node.utf8_text(src).unwrap_or("");
                out.push(rust_join_prefix(prefix, text));
            }
        }
        "use_wildcard" => {
            let text = node.utf8_text(src).unwrap_or("*");
            out.push(rust_join_prefix(prefix, text));
        }
        _ => {
            let text = node.utf8_text(src).unwrap_or("");
            if !text.is_empty() {
                out.push(rust_join_prefix(prefix, text));
            }
        }
    }
}

fn rust_join_prefix(prefix: &str, tail: &str) -> String {
    if prefix.is_empty() {
        tail.to_string()
    } else {
        format!("{prefix}::{tail}")
    }
}

/// Rust's [`Quirks`] row: trait supertrait-bounds INHERITS, `impl Trait
/// for Type` IMPLEMENTS (with its body walked generically under the
/// `impl` block's type name as `enclosing`), named-closure-binding
/// [`SymbolKind::Lambda`], `const`/`static` item constants,
/// attribute-macro DECORATES, signature TYPE_REF, and `#[test]`-family
/// reclassification.
pub fn rust_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(rust_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: Some(Box::new(rust_on_method_defined)),
        call_override: None,
    }
}

/// Parse Rust source through the generic engine (this wave's Rust
/// zero-regression proof against `languages::rust::parse`).
pub fn parse_rust(source: &str) -> ParsedFile {
    let spec = LangSpec::rust();
    let quirks = rust_quirks();
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// HTTP methods recognized in TS/JS route-registration calls/decorators
/// -- same list `languages/typescript.rs`'s `HTTP_METHODS` const
/// duplicates.
const TS_HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// TypeScript/JavaScript's `TestXxx`/`it`/`it_xxx` test-name
/// convention -- mirrors `languages/typescript.rs`'s `is_test_name`.
fn ts_is_test_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test") || lower.starts_with("it_") || lower == "it"
}

enum TsHeritageKind {
    Extends,
    Implements,
}

/// `class Sub extends Base implements I1, I2` / `interface Sub extends
/// Base1, Base2` -- mirrors `languages/typescript.rs`'s `heritage_refs`
/// byte-for-byte.
fn ts_heritage_refs(node: Node<'_>, src: &[u8]) -> Vec<(TsHeritageKind, String)> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "class_heritage" => {
                for j in 0..child.child_count() {
                    let Some(clause) = child.child(j) else {
                        continue;
                    };
                    ts_collect_heritage_clause(clause, src, &mut out);
                }
            }
            "extends_clause" | "implements_clause" => {
                ts_collect_heritage_clause(child, src, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn ts_collect_heritage_clause(
    clause: Node<'_>,
    src: &[u8],
    out: &mut Vec<(TsHeritageKind, String)>,
) {
    let is_extends = match clause.kind() {
        "extends_clause" => true,
        "implements_clause" => false,
        _ => return,
    };
    for k in 0..clause.child_count() {
        let Some(entry) = clause.child(k) else {
            continue;
        };
        if matches!(
            entry.kind(),
            "identifier" | "type_identifier" | "nested_type_identifier"
        ) {
            if let Ok(text) = entry.utf8_text(src) {
                out.push((
                    if is_extends {
                        TsHeritageKind::Extends
                    } else {
                        TsHeritageKind::Implements
                    },
                    text.to_string(),
                ));
            }
        }
    }
}

/// Decorators on a class/method/function declaration -- mirrors
/// `languages/typescript.rs`'s `preceding_decorators` byte-for-byte
/// (field-based `decorator:` lookup plus a defensive `prev_sibling()`
/// walk).
fn ts_preceding_decorators(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(field_decorator) = node.child_by_field_name("decorator") {
        if let Some(name) = ts_decorator_name(field_decorator, src) {
            out.push(name);
        }
    }
    let mut sibling = node.prev_sibling();
    while let Some(candidate) = sibling {
        if candidate.kind() != "decorator" {
            break;
        }
        if let Some(name) = ts_decorator_name(candidate, src) {
            out.push(name);
        }
        sibling = candidate.prev_sibling();
    }
    out.reverse();
    out
}

fn ts_decorator_name(decorator_node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..decorator_node.child_count() {
        let child = decorator_node.child(i)?;
        match child.kind() {
            "call_expression" => {
                let function = child.child_by_field_name("function")?;
                return function.utf8_text(src).ok().map(str::to_string);
            }
            "identifier" => {
                return child.utf8_text(src).ok().map(str::to_string);
            }
            _ => {}
        }
    }
    None
}

/// Parameter and return types on a function/method's signature --
/// mirrors `languages/typescript.rs`'s `signature_type_refs`
/// byte-for-byte.
fn ts_signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if let Some(type_node) = param.child_by_field_name("type") {
                    if let Ok(text) = type_node.utf8_text(src) {
                        out.push(text.trim_start_matches(':').trim().to_string());
                    }
                }
            }
        }
    }
    if let Some(return_type) = node.child_by_field_name("return_type") {
        if let Ok(text) = return_type.utf8_text(src) {
            out.push(text.trim_start_matches(':').trim().to_string());
        }
    }
    out
}

/// `const f = (x) => ...` / `const f = function() {}` -- mirrors
/// `languages/typescript.rs`'s `named_arrow_or_const_binding`
/// byte-for-byte.
fn ts_named_arrow_or_const_binding(node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    for i in 0..node.child_count() {
        let declarator = node.child(i)?;
        if declarator.kind() != "variable_declarator" {
            continue;
        }
        let name_node = declarator.child_by_field_name("name")?;
        if name_node.kind() != "identifier" {
            continue;
        }
        let value = declarator.child_by_field_name("value")?;
        let name = name_node.utf8_text(src).ok()?.to_string();
        let line = node.start_position().row + 1;
        if matches!(value.kind(), "arrow_function" | "function_expression") {
            return Some(SymbolRef {
                name,
                kind: SymbolKind::Lambda,
                line,
            });
        }
    }
    None
}

/// Recognize Express/Fastify/Axios-router-style calls -- mirrors
/// `languages/typescript.rs`'s `route_from_call` byte-for-byte, wired
/// as [`Quirks::route_from_call`].
fn ts_route_from_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let method = callee.rsplit('.').next()?.to_lowercase();
    if !TS_HTTP_METHODS.contains(&method.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "string")?;
    let raw = first_string_arg.utf8_text(src).ok()?;
    let path = raw
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string();
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    Some(RouteRef {
        method: method.to_uppercase(),
        path,
        line: call_node.start_position().row + 1,
    })
}

/// Recognize NestJS-style decorators (`@Get("/path")`) -- mirrors
/// `languages/typescript.rs`'s `route_from_decorator` byte-for-byte.
fn ts_route_from_decorator(decorator_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let call = (0..decorator_node.child_count())
        .filter_map(|i| decorator_node.child(i))
        .find(|n| n.kind() == "call_expression")?;
    let function = call.child_by_field_name("function")?;
    let name = function.utf8_text(src).ok()?;
    let method = TS_HTTP_METHODS
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(name))?;
    let path = call
        .child_by_field_name("arguments")
        .and_then(|args| {
            (0..args.child_count())
                .filter_map(|i| args.child(i))
                .find(|n| n.kind() == "string")
        })
        .and_then(|n| n.utf8_text(src).ok())
        .map(|raw| {
            raw.trim_matches(|c| c == '"' || c == '\'' || c == '`')
                .to_string()
        })
        .unwrap_or_default();
    Some(RouteRef {
        method: method.to_uppercase(),
        path,
        line: decorator_node.start_position().row + 1,
    })
}

/// Everything TS/JS's flat `LangSpec` arrays cannot express: class
/// heritage (INHERITS/IMPLEMENTS) + decorators + DEFINES-scoped body
/// walk, interface heritage (extends-only INHERITS) with NO
/// DEFINES-scoped body walk (matches `languages/typescript.rs`'s own
/// `interface_declaration` arm, which never calls `walk_children` with
/// `enclosing` set to the interface's own name), function-declaration
/// decorators/type-refs, custom import-path extraction (quoted string
/// trimming, not the generic engine's default), arrow/const-binding
/// [`SymbolKind::Lambda`], and standalone `@Get(...)`-decorator route
/// detection -- wired as this wave's [`Quirks::on_unmatched_node`] hook.
fn ts_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_declaration" => {
            // Already handled by the generic func/method branch before
            // `on_unmatched_node` is consulted for this kind; this arm
            // only runs if the name field was missing.
            false
        }
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                for (kind, super_name) in ts_heritage_refs(node, src) {
                    match kind {
                        TsHeritageKind::Extends => out.inherits.push(InheritsRef {
                            sub_name: name.clone(),
                            super_name,
                            line,
                        }),
                        TsHeritageKind::Implements => out.implements.push(ImplementsRef {
                            type_name: name.clone(),
                            trait_name: super_name,
                            line,
                        }),
                    }
                }
                for decorator_name in ts_preceding_decorators(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
                        target_name: name.clone(),
                        decorator_name,
                        line,
                    });
                }
                ts_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "interface_declaration" => {
            // Fully claimed (`true`): the generic class-shape
            // fallback's own symbol-push would otherwise run *again*
            // right after this quirk returns, double-recording the
            // Interface symbol -- this arm pushes it exactly once,
            // plus extends-only INHERITS, then recurses into the
            // interface body itself with NO scoped-body walk (matches
            // `languages/typescript.rs`'s own `interface_declaration`
            // arm exactly: it never calls `walk_children` with
            // `enclosing` set to the interface's own name, so member
            // signatures never get a DEFINES edge, unlike a class
            // body).
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for (kind, super_name) in ts_heritage_refs(node, src) {
                    if matches!(kind, TsHeritageKind::Extends) {
                        out.inherits.push(InheritsRef {
                            sub_name: name.clone(),
                            super_name,
                            line,
                        });
                    }
                }
            }
            true
        }
        "method_definition" => {
            // Already fully handled by the generic func/method branch
            // (`method_types` claims this kind); `on_method_defined`
            // supplies the type-ref/decorator extras. This arm never
            // actually runs for a well-formed node.
            false
        }
        "lexical_declaration" | "variable_declaration" => {
            if let Some(binding) = ts_named_arrow_or_const_binding(node, src) {
                out.symbols.push(binding);
            }
            false
        }
        "import_statement" => {
            if let Some(source_node) = node.child_by_field_name("source") {
                let raw = source_node.utf8_text(src).unwrap_or("");
                let module_path = raw.trim_matches(|c| c == '"' || c == '\'').to_string();
                if !module_path.is_empty() {
                    out.imports.push(ImportRef {
                        module_path,
                        line: node.start_position().row + 1,
                    });
                }
            }
            true
        }
        "decorator" => {
            if let Some(route) = ts_route_from_decorator(node, src) {
                out.routes.push(route);
            }
            false
        }
        _ => false,
    }
}

/// `class_declaration`'s DEFINES-scoped body walk: not one of
/// `LangSpec::typescript()`'s own class/interface/enum/alias arrays (a
/// class body's *members* are, but the class node itself is already
/// claimed by [`ts_quirk`] before the generic class-shape fallback ever
/// runs) -- this quirk re-implements that one recursive call directly
/// (mirrors `languages/typescript.rs`'s `class_declaration` arm calling
/// `walk_children(node, src, out, Some(name.as_str()), fn_scope)`), then
/// hands each child straight back to [`walk`] so every nested
/// method/call/field inside the class body still goes through the
/// fully generic path.
fn ts_walk_scoped_body(class_node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::typescript();
    let quirks = typescript_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        // `true` unconditionally, same rationale as `parse_typescript`'s
        // own top-level call: TS/JS's test-name convention is never
        // filename-gated, so a nested scoped-body walk must apply the
        // exact same always-on gate the outer walk already uses.
        is_test_file: true,
    };
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// DECORATES + TYPE_REF extras for a just-recorded TS/JS
/// function/method symbol, plus `TestXxx`/`it`/`it_xxx`
/// reclassification -- wired as [`Quirks::on_method_defined`], mirrors
/// `languages/typescript.rs`'s `function_declaration`/
/// `method_definition` arms.
fn ts_on_method_defined(node: Node<'_>, name: &str, line: usize, src: &[u8], out: &mut ParsedFile) {
    if !matches!(node.kind(), "function_declaration" | "method_definition") {
        return;
    }
    for type_ref in ts_signature_type_refs(node, src) {
        out.type_refs.push(crate::parsers::TypeRefRef {
            from_name: name.to_string(),
            type_name: type_ref,
            line,
        });
    }
    if node.kind() == "function_declaration" {
        for decorator_name in ts_preceding_decorators(node, src) {
            out.decorates.push(crate::parsers::DecoratesRef {
                target_name: name.to_string(),
                decorator_name,
                line,
            });
        }
    }
}

/// TypeScript/JavaScript's [`Quirks`] row: class heritage
/// (INHERITS/IMPLEMENTS) + decorators, interface extends-only INHERITS,
/// function/method decorators + TYPE_REF, arrow/const-binding
/// [`SymbolKind::Lambda`], quoted-string import-path extraction,
/// `TestXxx`/`it`/`it_xxx` test-name convention, and both
/// Express-style call routes and NestJS-style decorator routes. Shared
/// unchanged by [`crate::parsers::Language::JavaScript`] (same grammar,
/// same quirks, same as the bespoke `languages/typescript.rs::parse`
/// already treats both languages identically).
pub fn typescript_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(ts_quirk)),
        is_test_name: ts_is_test_name,
        route_from_call: Some(Box::new(ts_route_from_call)),
        on_method_defined: Some(Box::new(ts_on_method_defined)),
        call_override: None,
    }
}

/// Parse TypeScript/JavaScript source through the generic engine (this
/// wave's TS/JS zero-regression proof against
/// `languages::typescript::parse`). Both
/// [`crate::parsers::Language::TypeScript`] and
/// [`crate::parsers::Language::JavaScript`] route here unchanged, same
/// as the bespoke extractor.
/// `is_test_file: true` unconditionally: `languages/typescript.rs`'s
/// own `function_declaration`/`method_definition` arms call
/// `is_test_name` regardless of which file the symbol is in (no
/// filename gating), same rationale as [`parse_python`]'s identical
/// choice.
pub fn parse_typescript(source: &str) -> ParsedFile {
    let spec = LangSpec::typescript();
    let quirks = typescript_quirks();
    let language: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    parse_with_spec(source, &language, &spec, &quirks, true)
}

/// Parse TSX (TypeScript-JSX) source through the generic engine.
/// [`LangSpec::tsx`]'s own doc comment explains why every node-kind
/// array is identical to [`LangSpec::typescript`]'s (the baseline's own
/// `CBM_LANG_TSX` row copies every array from `CBM_LANG_TYPESCRIPT`
/// verbatim too, differing only in which grammar factory it carries) --
/// [`typescript_quirks`] is reused unchanged for the exact same reason.
/// The only real difference from [`parse_typescript`] is the grammar
/// itself: `tree_sitter_typescript::LANGUAGE_TSX` (not
/// `LANGUAGE_TYPESCRIPT`) -- both constants are exported from the one
/// `tree-sitter-typescript` crate already declared in this crate's
/// `Cargo.toml` for plain TypeScript, so onboarding TSX needed no new
/// grammar dependency at all.
pub fn parse_tsx(source: &str) -> ParsedFile {
    let spec = LangSpec::tsx();
    let quirks = typescript_quirks();
    let language: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TSX.into();
    parse_with_spec(source, &language, &spec, &quirks, true)
}

/// HTTP methods recognized in Python route decorators -- same list
/// `languages/python.rs`'s `HTTP_METHODS` const duplicates.
const PY_HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// Python's `test_*`/`test`-prefix convention -- mirrors
/// `languages/python.rs`'s `is_test_name`.
fn py_is_test_name(name: &str) -> bool {
    name.starts_with("test_") || name.starts_with("test")
}

/// `class Sub(Base1, Base2):` -- mirrors `languages/python.rs`'s
/// `base_class_names` byte-for-byte.
fn py_base_class_names(class_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(superclasses) = class_node.child_by_field_name("superclasses") {
        for i in 0..superclasses.child_count() {
            if let Some(child) = superclasses.child(i) {
                if matches!(child.kind(), "identifier" | "attribute") {
                    if let Ok(text) = child.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Every `@decorator`/`@decorator(...)` on a `decorated_definition`,
/// paired with the name of the definition it decorates -- mirrors
/// `languages/python.rs`'s `decorators_on` byte-for-byte.
fn py_decorators_on(node: Node<'_>, src: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(target_name) = node
        .child_by_field_name("definition")
        .and_then(|def| child_text(def, "name", src))
    else {
        return out;
    };
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "decorator" {
            continue;
        }
        let decorator_expr = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.kind() != "@");
        let Some(expr) = decorator_expr else { continue };
        let name = match expr.kind() {
            "call" => expr
                .child_by_field_name("function")
                .and_then(|f| f.utf8_text(src).ok()),
            _ => expr.utf8_text(src).ok(),
        };
        if let Some(name) = name {
            out.push((target_name.clone(), name.to_string()));
        }
    }
    out
}

/// `f = lambda x: ...` -- mirrors `languages/python.rs`'s
/// `named_lambda_binding` byte-for-byte.
fn py_named_lambda_binding(expr_stmt: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    let assignment = (0..expr_stmt.child_count())
        .filter_map(|i| expr_stmt.child(i))
        .find(|n| n.kind() == "assignment")?;
    let left = assignment.child_by_field_name("left")?;
    if left.kind() != "identifier" {
        return None;
    }
    let right = assignment.child_by_field_name("right")?;
    if right.kind() != "lambda" {
        return None;
    }
    let name = left.utf8_text(src).ok()?.to_string();
    Some(SymbolRef {
        name,
        kind: SymbolKind::Lambda,
        line: expr_stmt.start_position().row + 1,
    })
}

fn py_dotted_names_under(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "dotted_name" => {
                    if let Ok(text) = child.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
                "aliased_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
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

/// Recognize Flask/FastAPI-style decorators -- mirrors
/// `languages/python.rs`'s `route_from_decorated` byte-for-byte.
fn py_route_from_decorated(decorated_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    for i in 0..decorated_node.child_count() {
        let child = decorated_node.child(i)?;
        if child.kind() != "decorator" {
            continue;
        }
        let call = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.kind() == "call")?;
        let function = call.child_by_field_name("function")?;
        let function_text = function.utf8_text(src).unwrap_or("");
        let method_word = function_text.rsplit('.').next().unwrap_or("");
        let method = if method_word.eq_ignore_ascii_case("route") {
            "GET".to_string()
        } else if PY_HTTP_METHODS
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(method_word))
        {
            method_word.to_uppercase()
        } else {
            continue;
        };
        let path = call
            .child_by_field_name("arguments")
            .and_then(|args| {
                (0..args.child_count())
                    .filter_map(|k| args.child(k))
                    .find(|n| n.kind() == "string")
            })
            .and_then(|n| n.utf8_text(src).ok())
            .map(|raw| raw.trim_matches(|c| c == '"' || c == '\'').to_string())
            .unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        return Some(RouteRef {
            method,
            path,
            line: decorated_node.start_position().row + 1,
        });
    }
    None
}

/// Everything Python's flat `LangSpec` arrays cannot express: class
/// base-class INHERITS (with a scoped DEFINES body walk, matching
/// `class_definition`'s bespoke arm), `decorated_definition` route
/// detection + DECORATES (falling through so the wrapped
/// function/class definition underneath is still visited generically),
/// grouped/aliased import-path extraction, and named-lambda-binding
/// [`SymbolKind::Lambda`] -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook.
fn py_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                for base in py_base_class_names(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: base,
                        line,
                    });
                }
                py_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "decorated_definition" => {
            if let Some(route) = py_route_from_decorated(node, src) {
                out.routes.push(route);
            }
            for (target, decorator) in py_decorators_on(node, src) {
                out.decorates.push(crate::parsers::DecoratesRef {
                    target_name: target,
                    decorator_name: decorator,
                    line: node.start_position().row + 1,
                });
            }
            // Not fully claimed: the wrapped `function_definition`/
            // `class_definition` child underneath still needs the
            // generic walk to visit it (same as
            // `languages/python.rs`'s own `decorated_definition` arm,
            // which never `return`s early and falls through to the
            // shared `walk_children` call at the bottom of its `walk`).
            false
        }
        "import_statement" => {
            for path in py_dotted_names_under(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "import_from_statement" => {
            if let Some(module_node) = node.child_by_field_name("module_name") {
                if let Ok(module) = module_node.utf8_text(src) {
                    out.imports.push(ImportRef {
                        module_path: module.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            true
        }
        "expression_statement" => {
            if let Some(binding) = py_named_lambda_binding(node, src) {
                out.symbols.push(binding);
            }
            false
        }
        _ => false,
    }
}

/// `class_definition`'s DEFINES-scoped body walk -- same rationale as
/// [`ts_walk_scoped_body`]/[`rust_walk_impl_body`]: `class_definition`
/// is already claimed by [`py_quirk`] before the generic class-shape
/// fallback (which would otherwise perform this same recursive call)
/// ever runs, so this quirk re-implements that one call directly,
/// mirrors `languages/python.rs`'s `class_definition` arm's
/// `walk_children(node, src, out, Some(name.as_str()), fn_scope)`.
fn py_walk_scoped_body(class_node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::python();
    let quirks = python_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        // `true` unconditionally, same rationale as `parse_python`'s
        // own top-level call: Python's test-name convention is never
        // filename-gated, so a nested scoped-body walk (a method
        // inside a class) must apply the exact same always-on gate the
        // outer walk already uses -- otherwise a `test_*`-named method
        // would incorrectly classify as Method instead of Test.
        is_test_file: true,
    };
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// Python's [`Quirks`] row: class base-class INHERITS + DEFINES-scoped
/// body walk, `@decorator` route detection/DECORATES (Flask/FastAPI-
/// style), grouped/aliased import paths, `test_*`/`test`-prefix
/// test-name convention, and named-lambda-binding
/// [`SymbolKind::Lambda`].
pub fn python_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(py_quirk)),
        is_test_name: py_is_test_name,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse Python source through the generic engine (this wave's Python
/// zero-regression proof against `languages::python::parse`).
/// `is_test_file: true` unconditionally (unlike Go's filename-gated
/// `_test.go` convention): Python's own bespoke `is_test_name` check
/// applies at every scope, gated only by the name itself, never by
/// which file it is in -- matches `languages/python.rs`'s
/// `function_definition` arm, which calls `is_test_name` regardless of
/// `rel_path`.
pub fn parse_python(source: &str) -> ParsedFile {
    let spec = LangSpec::python();
    let quirks = python_quirks();
    let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, true)
}

/// Spring MVC mapping annotations recognized as routes -- same list
/// `languages/java.rs`'s `MAPPING_ANNOTATIONS` const duplicates.
const JAVA_MAPPING_ANNOTATIONS: &[(&str, &str)] = &[
    ("GetMapping", "GET"),
    ("PostMapping", "POST"),
    ("PutMapping", "PUT"),
    ("PatchMapping", "PATCH"),
    ("DeleteMapping", "DELETE"),
];

/// `package a.b.c;` -- mirrors `languages/java.rs`'s `package_name`
/// byte-for-byte.
fn java_package_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(child.kind(), "scoped_identifier" | "identifier") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `class Sub extends Base` -- mirrors `languages/java.rs`'s
/// `superclass_name` byte-for-byte.
fn java_superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    let superclass = class_node.child_by_field_name("superclass")?;
    let type_node = (0..superclass.child_count())
        .filter_map(|i| superclass.child(i))
        .find(|n| n.is_named())?;
    type_node.utf8_text(src).ok().map(str::to_string)
}

/// `class C implements I1, I2` / `enum E implements I1, I2` -- mirrors
/// `languages/java.rs`'s `super_interfaces` byte-for-byte.
fn java_super_interfaces(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(interfaces) = node.child_by_field_name("interfaces") else {
        return out;
    };
    let Some(type_list) = (0..interfaces.child_count())
        .filter_map(|i| interfaces.child(i))
        .find(|n| n.kind() == "type_list")
    else {
        return out;
    };
    for i in 0..type_list.child_count() {
        if let Some(child) = type_list.child(i) {
            if child.is_named() {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// `interface Sub extends Base1, Base2` -- mirrors
/// `languages/java.rs`'s `extends_interfaces` byte-for-byte.
fn java_extends_interfaces(interface_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..interface_node.child_count() {
        let Some(child) = interface_node.child(i) else {
            continue;
        };
        if child.kind() != "extends_interfaces" {
            continue;
        }
        let Some(type_list) = (0..child.child_count())
            .filter_map(|i| child.child(i))
            .find(|n| n.kind() == "type_list")
        else {
            continue;
        };
        for j in 0..type_list.child_count() {
            if let Some(entry) = type_list.child(j) {
                if entry.is_named() {
                    if let Ok(text) = entry.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Annotations (`@Foo`, `@Foo(...)`) attached to a declaration via its
/// `modifiers` child -- mirrors `languages/java.rs`'s `annotations`
/// byte-for-byte.
fn java_annotations(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "modifiers" {
            continue;
        }
        for j in 0..child.child_count() {
            let Some(modifier) = child.child(j) else {
                continue;
            };
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

fn java_modifier_texts(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "modifiers" {
            continue;
        }
        for j in 0..child.child_count() {
            if let Some(modifier) = child.child(j) {
                if !modifier.is_named() {
                    if let Ok(text) = modifier.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// `int A = 1, B = 2;` -- mirrors `languages/java.rs`'s `field_names`
/// byte-for-byte.
fn java_field_names(field_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..field_node.child_count() {
        let Some(child) = field_node.child(i) else {
            continue;
        };
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

/// Parameter and return types on a `method_declaration`'s signature --
/// mirrors `languages/java.rs`'s `signature_type_refs` byte-for-byte.
fn java_signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if param.kind() == "formal_parameter" {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
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
/// mirrors `languages/java.rs`'s `import_path` byte-for-byte.
fn java_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut parts = Vec::new();
    for i in 0..node.child_count() {
        let child = node.child(i)?;
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

/// `obj.method(...)` / `method(...)` -- mirrors `languages/java.rs`'s
/// `method_invocation_callee` byte-for-byte.
fn java_method_invocation_callee(node: Node<'_>, src: &[u8]) -> String {
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

/// For a `method_invocation` with an `object` field, the receiver's own
/// text plus a cheap syntactic hint -- mirrors `languages/java.rs`'s
/// `receiver_of_call` byte-for-byte.
fn java_receiver_of_call(
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

/// Each argument expression's own source text, in written order --
/// mirrors `languages/java.rs`'s `call_arg_texts` byte-for-byte.
fn java_call_arg_texts(invocation_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = invocation_node.child_by_field_name("arguments") else {
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

/// `@GetMapping("/path")` / `@PostMapping(path = "/path")` -- mirrors
/// `languages/java.rs`'s `route_from_mapping` byte-for-byte.
fn java_route_from_mapping(
    decorator_name: &str,
    method_node: Node<'_>,
    src: &[u8],
) -> Option<RouteRef> {
    let http_method = JAVA_MAPPING_ANNOTATIONS
        .iter()
        .find(|(name, _)| *name == decorator_name)
        .map(|(_, method)| method.to_string())?;
    let path = java_mapping_path_argument(decorator_name, method_node, src).unwrap_or_default();
    Some(RouteRef {
        method: http_method,
        path,
        line: method_node.start_position().row + 1,
    })
}

fn java_mapping_path_argument(
    decorator_name: &str,
    method_node: Node<'_>,
    src: &[u8],
) -> Option<String> {
    for i in 0..method_node.child_count() {
        let child = method_node.child(i)?;
        if child.kind() != "modifiers" {
            continue;
        }
        for j in 0..child.child_count() {
            let annotation = child.child(j)?;
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
            for k in 0..args.child_count() {
                let arg = args.child(k)?;
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

/// Everything Java's flat `LangSpec` arrays cannot express: dotted
/// package-name Module symbol, class/interface/enum heritage
/// (INHERITS/IMPLEMENTS) + annotations + DEFINES-scoped body walk,
/// `static final` field-only Constant + DEFINES, and dotted/wildcard
/// import-path reconstruction -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. Method-level extras
/// (annotations/route/type-refs/`@Test`) are a separate
/// [`Quirks::on_method_defined`] hook (see [`java_on_method_defined`]),
/// and callee reconstruction is a separate [`Quirks::call_override`]
/// hook (see [`java_call_override`]), since both need data the generic
/// walker's own func/method and call branches already consume before
/// `on_unmatched_node` would ever see those node kinds.
fn java_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "package_declaration" => {
            if let Some(name) = java_package_name(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(super_name) = java_superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                for interface_name in java_super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                for decorator in java_annotations(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator,
                        line,
                    });
                }
                java_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "interface_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for extended in java_extends_interfaces(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: extended,
                        line,
                    });
                }
                for decorator in java_annotations(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator,
                        line,
                    });
                }
                java_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "enum_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
                    line,
                });
                for interface_name in java_super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                java_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "field_declaration" => {
            let modifiers = java_modifier_texts(node, src);
            let is_constant = modifiers.contains(&"static".to_string())
                && modifiers.contains(&"final".to_string());
            if is_constant {
                let line = node.start_position().row + 1;
                for name in java_field_names(node, src) {
                    out.symbols.push(SymbolRef {
                        name: name.clone(),
                        kind: SymbolKind::Constant,
                        line,
                    });
                    // `enclosing` is not available in this quirk's
                    // signature; DEFINES for a constant field is
                    // instead emitted by the caller (see
                    // `java_walk_scoped_body`'s post-pass) since a
                    // `field_declaration` only ever appears directly
                    // inside a class/interface/enum body this quirk's
                    // own scoped-body walk already knows the name of.
                }
            }
            false
        }
        "import_declaration" => {
            if let Some(path) = java_import_path(node, src) {
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

/// `class_declaration`/`interface_declaration`/`enum_declaration`'s
/// DEFINES-scoped body walk -- mirrors `languages/java.rs`'s own
/// `walk_children(node, src, out, Some(name.as_str()), fn_scope)`
/// exactly: every descendant (not just immediate children -- Java's
/// grammar nests a class's members one level deeper inside a
/// `class_body` node) is visited through the fully generic [`walk`],
/// so nested classes/methods/calls/fields all recurse correctly
/// on their own. The `static final` field DEFINES edge
/// [`java_quirk`]'s own `field_declaration` arm cannot emit (its hook
/// signature carries no `enclosing`) is added by a separate shallow
/// recursive search that stops at any nested class/interface/enum
/// boundary -- a field declared inside a *nested* type must DEFINES to
/// that nested type, not this one, exactly like the bespoke walk's own
/// recursion naturally achieves by re-scoping `enclosing` at each
/// nested class boundary.
fn java_walk_scoped_body(
    class_node: Node<'_>,
    src: &[u8],
    name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::java();
    let quirks = java_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
    if let Some(container) = name {
        java_emit_constant_field_defines(class_node, src, container, out);
    }
}

/// Depth-first search for `field_declaration` descendants of
/// `class_node` belonging directly to it (not to any nested
/// class/interface/enum inside it), emitting a DEFINES edge for each
/// `static final` one -- see [`java_walk_scoped_body`]'s doc for why
/// this is a separate pass from the fully generic recursion above.
fn java_emit_constant_field_defines(
    class_node: Node<'_>,
    src: &[u8],
    container: &str,
    out: &mut ParsedFile,
) {
    for i in 0..class_node.child_count() {
        let Some(child) = class_node.child(i) else {
            continue;
        };
        match child.kind() {
            "field_declaration" => {
                let modifiers = java_modifier_texts(child, src);
                let is_constant = modifiers.contains(&"static".to_string())
                    && modifiers.contains(&"final".to_string());
                if is_constant {
                    let line = child.start_position().row + 1;
                    for field_name in java_field_names(child, src) {
                        out.defines.push(DefinesRef {
                            container_name: container.to_string(),
                            member_name: field_name,
                            line,
                        });
                    }
                }
            }
            "class_declaration" | "interface_declaration" | "enum_declaration" => {
                // A nested type's own fields belong to *it*, not to
                // `container` -- `java_walk_scoped_body`'s own
                // recursive `walk` call already handles this nested
                // node's DEFINES correctly when it reaches this same
                // quirk again with the nested type's own name, so this
                // search must not descend into it a second time.
            }
            _ => java_emit_constant_field_defines(child, src, container, out),
        }
    }
}

/// Annotations-as-DECORATES + route-from-mapping + signature TYPE_REF
/// for a just-recorded Java method symbol, plus `@Test`-annotation
/// reclassification -- wired as [`Quirks::on_method_defined`], mirrors
/// `languages/java.rs`'s `method_declaration` arm.
fn java_on_method_defined(
    node: Node<'_>,
    name: &str,
    line: usize,
    src: &[u8],
    out: &mut ParsedFile,
) {
    if node.kind() != "method_declaration" {
        return;
    }
    let decorators = java_annotations(node, src);
    if decorators.iter().any(|d| d == "Test") {
        if let Some(last) = out.symbols.last_mut() {
            if last.name == name && last.line == line {
                last.kind = SymbolKind::Test;
            }
        }
    }
    for decorator in &decorators {
        out.decorates.push(crate::parsers::DecoratesRef {
            target_name: name.to_string(),
            decorator_name: decorator.clone(),
            line,
        });
        if let Some(route) = java_route_from_mapping(decorator, node, src) {
            out.routes.push(route);
        }
    }
    for type_ref in java_signature_type_refs(node, src) {
        out.type_refs.push(crate::parsers::TypeRefRef {
            from_name: name.to_string(),
            type_name: type_ref,
            line,
        });
    }
}

/// Full `method_invocation` callee reconstruction (`object.name` when a
/// receiver is present, bare `name` otherwise) -- wired as
/// [`Quirks::call_override`] since Java's grammar splits the receiver
/// and method name into two separate fields rather than one field
/// holding the whole written callee expression.
fn java_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "method_invocation" {
        return false;
    }
    let callee = java_method_invocation_callee(node, src);
    let (receiver_text, receiver_hint) = java_receiver_of_call(node, src);
    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text,
        receiver_hint,
        arg_texts: java_call_arg_texts(node, src),
    });
    true
}

/// Java's [`Quirks`] row: dotted package Module symbol, class/interface/
/// enum heritage + annotations + DEFINES-scoped body walk, `static
/// final` field Constant + DEFINES, dotted/wildcard import paths,
/// method annotations/route/type-refs/`@Test` reclassification, and
/// full receiver-qualified callee reconstruction.
pub fn java_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(java_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: Some(Box::new(java_on_method_defined)),
        call_override: Some(Box::new(java_call_override)),
    }
}

/// Parse Java source through the generic engine (this wave's Java
/// zero-regression proof against `languages::java::parse`).
pub fn parse_java(source: &str) -> ParsedFile {
    let spec = LangSpec::java();
    let quirks = java_quirks();
    let language: tree_sitter::Language = tree_sitter_java::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// A `function_definition` node's name -- mirrors `languages/c.rs`'s
/// `function_name` byte-for-byte.
fn c_function_name(function_node: Node<'_>, src: &[u8]) -> Option<String> {
    let declarator = function_node.child_by_field_name("declarator")?;
    c_innermost_declarator_identifier(declarator, src)
}

fn c_innermost_declarator_identifier(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" => node.utf8_text(src).ok().map(str::to_string),
        "function_declarator" | "pointer_declarator" | "parenthesized_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|inner| c_innermost_declarator_identifier(inner, src)),
        _ => None,
    }
}

/// Field names inside a `struct`/`union` body -- mirrors
/// `languages/c.rs`'s `struct_field_names` byte-for-byte.
fn c_struct_field_names(body: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..body.child_count() {
        let Some(child) = body.child(i) else { continue };
        if child.kind() != "field_declaration" {
            continue;
        }
        if let Some(declarator) = child.child_by_field_name("declarator") {
            if let Some(name) = c_innermost_declarator_identifier(declarator, src) {
                out.push(name);
            }
        }
    }
    out
}

/// `typedef struct {...} Foo;` / `typedef int MyInt;` -- mirrors
/// `languages/c.rs`'s `typedef_alias_names` byte-for-byte.
fn c_typedef_alias_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "typedef" | "struct_specifier" | "union_specifier" | "enum_specifier" | ";" => {
                continue;
            }
            "type_identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
            _ => {
                if let Some(name) = c_innermost_declarator_identifier(child, src) {
                    out.push(name);
                }
            }
        }
    }
    out
}

/// A top-level (or struct/union-scoped) `declaration` node -- mirrors
/// `languages/c.rs`'s `top_level_declaration_symbols` byte-for-byte.
fn c_top_level_declaration_symbols(
    node: Node<'_>,
    src: &[u8],
    inside_struct: bool,
) -> Vec<SymbolRef> {
    if inside_struct {
        return Vec::new();
    }
    let is_const = c_declaration_has_const(node, src);
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
            continue;
        }
        if let Some(name) = c_innermost_declarator_identifier(declarator, src) {
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

fn c_declaration_has_const(node: Node<'_>, src: &[u8]) -> bool {
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

/// `#include "path"` / `#include <path>` -- mirrors `languages/c.rs`'s
/// `include_path` byte-for-byte.
fn c_include_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = node.child_by_field_name("path")?;
    let raw = path_node.utf8_text(src).ok()?;
    Some(
        raw.trim_matches(|c| c == '"' || c == '<' || c == '>')
            .to_string(),
    )
}

/// C's name-convention test heuristic -- mirrors `languages/c.rs`'s
/// `is_test_function` byte-for-byte.
fn c_is_test_function(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test_") || lower.ends_with("_test")
}

/// Everything C's flat `LangSpec` arrays cannot express (see
/// `LangSpec::c()`'s own doc comment for why): declarator-unwrapping
/// function names, struct/enum/typedef symbol + struct-field DEFINES,
/// `#define`-value-macro constants, and `const`-vs-plain top-level
/// declaration Constant/Variable splitting -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. `#define`/global-declaration
/// test-file-wide reclassification (`is_test_file`) is handled by
/// [`parse_c`]'s own post-pass, mirroring `languages/c.rs::parse`'s own
/// post-walk loop (its `is_test_file` gate has no natural per-node hook
/// point in this walker's design, exactly like the bespoke function's
/// own choice to do it as a separate pass after the walk completes).
fn c_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = c_function_name(node, src) {
                let line = node.start_position().row + 1;
                let kind = if c_is_test_function(&name) {
                    SymbolKind::Test
                } else {
                    SymbolKind::Function
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    c_walk_function_body(body, src, name.as_str(), line, out);
                }
            }
            true
        }
        "struct_specifier" => {
            // Fully claimed (`true`): `struct_specifier` is also one of
            // `LangSpec::c()`'s `class_types`, so this quirk is called
            // once from the class-shape branch's own early
            // `on_unmatched_node` check -- returning `false` here would
            // fall through to that branch's `name_field`-keyed fallback
            // (neutralized/no-op per `LangSpec::c()`'s doc), which
            // itself falls through past the whole class-shape block to
            // the walker's *final* catch-all `on_unmatched_node`,
            // invoking this same quirk a *second* time for the same
            // node. `true` prevents that, so this arm performs its own
            // recursion directly (mirrors `languages/c.rs`'s own
            // `struct_specifier` arm's `walk_children(node, src, out,
            // Some(name.as_str()), fn_scope)` call). Recursion happens
            // *unconditionally* (not just in the named case): an
            // anonymous struct (`struct { int x; } p;`) has no `name`
            // field for `child_text` to find, so no Struct symbol/
            // DEFINES is pushed, but `languages/c.rs`'s own arm still
            // falls through to its own bottom-of-`walk` recursion in
            // that case (its `if let Some(name) = ...` body -- the only
            // place it `return`s early -- simply never runs), so a
            // nested type inside an anonymous struct must still be
            // found.
            let mut container = None;
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Struct,
                    line,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    for field_name in c_struct_field_names(body, src) {
                        out.defines.push(DefinesRef {
                            container_name: name.clone(),
                            member_name: field_name,
                            line,
                        });
                    }
                }
                container = Some(name);
            }
            c_walk_scoped(node, src, container.as_deref(), out);
            true
        }
        "enum_specifier" => {
            // Fully claimed for the same double-invocation reason as
            // `struct_specifier` above (`enum_specifier` is one of
            // `LangSpec::c()`'s `enum_types`) -- `languages/c.rs`'s own
            // `enum_specifier` arm never calls `walk_children` itself
            // and never `return`s early, instead falling through to
            // the shared bottom-of-`walk` call with the *unchanged*
            // `enclosing`/`fn_scope` its caller already had, so this
            // arm's own recursion must use the *outer* scope this
            // quirk was invoked under, not re-scope to the enum's own
            // name -- see [`c_walk_unscoped`].
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
            c_walk_unscoped(node, src, out);
            true
        }
        "type_definition" => {
            // Fully claimed, same rationale/outer-scope recursion as
            // `enum_specifier` above (`type_definition` is one of
            // `LangSpec::c()`'s `alias_types`, and
            // `languages/c.rs`'s own `type_definition` arm also never
            // recurses itself, falling through to the shared bottom
            // call unchanged).
            for alias_name in c_typedef_alias_names(node, src) {
                out.symbols.push(SymbolRef {
                    name: alias_name,
                    kind: SymbolKind::TypeAlias,
                    line: node.start_position().row + 1,
                });
            }
            c_walk_unscoped(node, src, out);
            true
        }
        "declaration" => {
            // Not one of `class_types`/`enum_types`/`alias_types` --
            // reached only via the walker's single final catch-all, so
            // `false` (matching `languages/c.rs`'s own `declaration`
            // arm, which also never recurses itself) is safe here
            // without a double-invocation risk.
            for symbol in c_top_level_declaration_symbols(node, src, false) {
                out.symbols.push(symbol);
            }
            false
        }
        // No `"field_declaration"` arm at all (matches
        // `languages/c.rs`'s own `match`, which also has none): the
        // struct-field DEFINES edge is already fully handled by the
        // enclosing `struct_specifier` arm above (which reads its own
        // `body` node directly via `c_struct_field_names`), so a bare
        // `field_declaration` node just needs plain recursion into its
        // children -- e.g. a nested `enum_specifier`/`struct_specifier`
        // inside a field's type specifier (`enum Status {...} status;`
        // as a struct member) must still be discovered. Falling to the
        // `_ => false` catch-all below achieves exactly that (the
        // generic engine's own field-DEFINES branch is a no-op here
        // regardless, per `LangSpec::c()`'s bogus-`name_field` doc, so
        // there is no double-processing risk either way).
        "preproc_def" if node.child_by_field_name("value").is_some() => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Constant,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "preproc_include" => {
            if let Some(path) = c_include_path(node, src) {
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

/// A `function_definition`'s body: every call inside it needs
/// `fn_scope` set to this function's own name/line -- mirrors
/// `languages/c.rs`'s own `function_definition` arm's
/// `walk_children(body, src, out, enclosing, FnScope { name: Some(...),
/// line: Some(...) })` call. `enclosing` is hardcoded `None` (not
/// threaded through from the quirk's caller, whose
/// [`Quirks::on_unmatched_node`] signature carries no `enclosing`
/// parameter): valid C syntax never nests a `function_definition`
/// inside a `struct`/`union` body (C has no methods), so `enclosing` is
/// always `None` at the one call site (`c_quirk`'s own
/// `"function_definition"` arm) that ever invokes this. Not one of
/// `LangSpec::c()`'s own func/method arrays' generic body-walk (that
/// generic path is never reached for C -- `c_quirk` claims
/// `function_definition` before it would run), so this quirk
/// re-implements that one recursive call directly, then hands each
/// child straight back to [`walk`] so every nested call/struct/
/// declaration inside the function body still goes through the fully
/// generic path.
fn c_walk_function_body(body: Node<'_>, src: &[u8], name: &str, line: usize, out: &mut ParsedFile) {
    let spec = LangSpec::c();
    let quirks = c_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name: Some(name),
        line: Some(line),
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, None, fn_scope);
        }
    }
}

/// `struct_specifier`'s own scoped recursion (`enclosing` re-scoped to
/// the struct's own name) -- mirrors `languages/c.rs`'s own
/// `struct_specifier` arm's `walk_children(node, src, out,
/// Some(name.as_str()), fn_scope)` call. Recurses over `node`'s
/// children directly (not `node` itself), same as `walk_children`'s
/// own contract.
fn c_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::c();
    let quirks = c_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, FnScope::default());
        }
    }
}

/// `enum_specifier`/`type_definition`'s own recursion under the
/// *outer* `enclosing`/`fn_scope` -- mirrors `languages/c.rs`'s own
/// `enum_specifier`/`type_definition` arms, which never call
/// `walk_children` themselves and never `return` early, instead
/// falling through to the shared bottom-of-`walk` call with whatever
/// `enclosing`/`fn_scope` their caller already had. Since
/// [`Quirks::on_unmatched_node`]'s signature carries neither, and a
/// C `enum`/`typedef` can only ever appear at file scope or directly
/// inside a `struct`/`union` body (never inside a function, where
/// `fn_scope` would matter) in valid C, `None`/[`FnScope::default`]
/// is exactly the value the bespoke walk's fall-through would have
/// used in every case this quirk is ever invoked for.
fn c_walk_unscoped(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let spec = LangSpec::c();
    let quirks = c_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, None, FnScope::default());
        }
    }
}

/// C's [`Quirks`] row: everything (see `LangSpec::c()`'s doc comment
/// for why C cannot use the generic engine's own name-field-based
/// fallback at all).
pub fn c_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(c_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse C source through the generic engine (this wave's C
/// zero-regression proof against `languages::c::parse`). `is_test_file`
/// mirrors `languages/c.rs::parse`'s own post-walk reclassification
/// pass exactly (every [`SymbolKind::Function`] becomes
/// [`SymbolKind::Test`] when `true`, applied here as the identical
/// post-pass rather than a per-node hook, since the bespoke function
/// itself chose a post-pass over inline gating).
pub fn parse_c(source: &str, is_test_file: bool) -> ParsedFile {
    let spec = LangSpec::c();
    let quirks = c_quirks();
    let language: tree_sitter::Language = tree_sitter_c::LANGUAGE.into();
    let mut out = parse_with_spec(source, &language, &spec, &quirks, false);
    if is_test_file {
        for symbol in &mut out.symbols {
            if symbol.kind == SymbolKind::Function {
                symbol.kind = SymbolKind::Test;
            }
        }
    }
    out
}

/// Declarator name + out-of-line scope -- mirrors `languages/cpp.rs`'s
/// `declarator_name_and_scope` byte-for-byte.
fn cpp_declarator_name_and_scope(node: Node<'_>, src: &[u8]) -> Option<(String, Option<String>)> {
    match node.kind() {
        "function_declarator" | "pointer_declarator" | "reference_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|inner| cpp_declarator_name_and_scope(inner, src)),
        "qualified_identifier" => {
            let name_node = node.child_by_field_name("name")?;
            let name = name_node.utf8_text(src).ok()?.to_string();
            let scope = node
                .child_by_field_name("scope")
                .and_then(|s| s.utf8_text(src).ok())
                .map(str::to_string);
            Some((name, scope))
        }
        "identifier" | "field_identifier" | "destructor_name" | "operator_name" => {
            node.utf8_text(src).ok().map(|s| (s.to_string(), None))
        }
        _ => None,
    }
}

fn cpp_innermost_declarator_identifier(node: Node<'_>, src: &[u8]) -> Option<String> {
    cpp_declarator_name_and_scope(node, src).map(|(name, _)| name)
}

/// `typedef struct {...} Foo;` -- mirrors `languages/cpp.rs`'s
/// `typedef_alias_names` byte-for-byte.
fn cpp_typedef_alias_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "typedef" | "struct_specifier" | "union_specifier" | "enum_specifier"
            | "class_specifier" | ";" => {
                continue;
            }
            "type_identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
            _ => {
                if let Some(name) = cpp_innermost_declarator_identifier(child, src) {
                    out.push(name);
                }
            }
        }
    }
    out
}

/// Mirrors `languages/cpp.rs`'s `top_level_declaration_symbols`
/// byte-for-byte.
fn cpp_top_level_declaration_symbols(
    node: Node<'_>,
    src: &[u8],
    inside_class_or_struct: bool,
) -> Vec<SymbolRef> {
    if inside_class_or_struct {
        return Vec::new();
    }
    let is_const = cpp_declaration_has_const(node, src);
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
            continue;
        }
        if let Some(name) = cpp_innermost_declarator_identifier(declarator, src) {
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

fn cpp_declaration_has_const(node: Node<'_>, src: &[u8]) -> bool {
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

/// `class Sub : public Base1, private Base2` -- mirrors
/// `languages/cpp.rs`'s `base_class_names` byte-for-byte.
fn cpp_base_class_names(class_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..class_node.child_count() {
        let Some(child) = class_node.child(i) else {
            continue;
        };
        if child.kind() != "base_class_clause" {
            continue;
        }
        for j in 0..child.child_count() {
            let Some(entry) = child.child(j) else {
                continue;
            };
            if matches!(
                entry.kind(),
                "type_identifier" | "qualified_identifier" | "template_type"
            ) {
                if let Ok(text) = entry.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// Field names inside a class/struct body -- mirrors
/// `languages/cpp.rs`'s `field_names` byte-for-byte.
fn cpp_field_names(body: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..body.child_count() {
        let Some(child) = body.child(i) else { continue };
        if child.kind() != "field_declaration" {
            continue;
        }
        if let Some(declarator) = child.child_by_field_name("declarator") {
            if declarator.kind() == "function_declarator" {
                continue;
            }
            if let Some(name) = cpp_innermost_declarator_identifier(declarator, src) {
                out.push(name);
            }
        }
    }
    out
}

/// Best-effort "interface" heuristic -- mirrors `languages/cpp.rs`'s
/// `is_abstract_class_body` byte-for-byte.
fn cpp_is_abstract_class_body(body: Node<'_>, src: &[u8]) -> bool {
    let mut saw_virtual_member = false;
    for i in 0..body.child_count() {
        let Some(child) = body.child(i) else { continue };
        if child.kind() != "field_declaration" {
            continue;
        }
        let field_children: Vec<Node<'_>> = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .collect();
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

/// `auto f = [](...) { ... };` on a `declaration` node -- mirrors
/// `languages/cpp.rs`'s `named_lambda_binding` byte-for-byte.
fn cpp_named_lambda_binding(declaration_node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    for j in 0..declaration_node.child_count() {
        let inner = declaration_node.child(j)?;
        if inner.kind() != "init_declarator" {
            continue;
        }
        let declarator = inner.child_by_field_name("declarator")?;
        let value = inner.child_by_field_name("value")?;
        if value.kind() == "lambda_expression" {
            let name = cpp_innermost_declarator_identifier(declarator, src)?;
            return Some(SymbolRef {
                name,
                kind: SymbolKind::Lambda,
                line: declaration_node.start_position().row + 1,
            });
        }
    }
    None
}

/// `f = [](...) { ... };` -- mirrors `languages/cpp.rs`'s
/// `named_lambda_binding_from_assignment` byte-for-byte.
fn cpp_named_lambda_binding_from_assignment(
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
        let name = left.utf8_text(src).ok()?.to_string();
        return Some(SymbolRef {
            name,
            kind: SymbolKind::Lambda,
            line: expression_statement_node.start_position().row + 1,
        });
    }
    None
}

fn cpp_include_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = node.child_by_field_name("path")?;
    let raw = path_node.utf8_text(src).ok()?;
    Some(
        raw.trim_matches(|c| c == '"' || c == '<' || c == '>')
            .to_string(),
    )
}

/// gtest-family name-convention fallback -- mirrors
/// `languages/cpp.rs`'s `is_test_function` byte-for-byte.
fn cpp_is_test_function(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("test_") || lower.ends_with("_test")
}

/// Recognize a gtest `TEST(Suite, Name)`-family invocation from a
/// `function_definition`'s own `declarator` field -- mirrors
/// `languages/cpp.rs`'s `gtest_macro_test_name` byte-for-byte (used both
/// from the `function_definition` arm and, with a `call_expression`
/// node's own `function` field as `declarator`, from the call-override
/// hook -- the bespoke code calls the identical helper from both of its
/// own two call sites for the same reason).
fn cpp_gtest_macro_test_name(callee: &str, declarator: Node<'_>, src: &[u8]) -> Option<String> {
    if !matches!(callee, "TEST" | "TEST_F" | "TEST_P") {
        return None;
    }
    let params = declarator.child_by_field_name("parameters")?;
    let idents: Vec<&str> = (0..params.child_count())
        .filter_map(|i| params.child(i))
        .filter_map(|n| match n.kind() {
            "identifier" | "type_identifier" => n.utf8_text(src).ok(),
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
    Some(format!("{}.{}", idents[0], idents[1]))
}

/// For a `field_expression`-shaped callee, the receiver text plus a
/// cheap syntactic hint -- mirrors `languages/cpp.rs`'s
/// `receiver_of_call` byte-for-byte.
fn cpp_receiver_of_call(
    function_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
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
    (Some(text.to_string()), Some(hint))
}

/// Each argument expression's own source text -- mirrors
/// `languages/cpp.rs`'s `call_arg_texts` byte-for-byte.
fn cpp_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// `class`/`struct` declaration handling -- mirrors
/// `languages/cpp.rs`'s `handle_class_or_struct` byte-for-byte (Class/
/// Interface classification, INHERITS, field DEFINES, and a
/// scoped recursion for member methods).
fn cpp_handle_class_or_struct(
    node: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    out: &mut ParsedFile,
) {
    let Some(name) = child_text(node, "name", src) else {
        cpp_walk_scoped(node, src, enclosing, out);
        return;
    };
    let line = node.start_position().row + 1;
    let body = node.child_by_field_name("body");

    let kind = if body
        .map(|b| cpp_is_abstract_class_body(b, src))
        .unwrap_or(false)
    {
        SymbolKind::Interface
    } else {
        SymbolKind::Class
    };
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind,
        line,
    });

    for base_name in cpp_base_class_names(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: name.clone(),
            super_name: base_name,
            line,
        });
    }

    if let Some(body) = body {
        for field_name in cpp_field_names(body, src) {
            out.defines.push(DefinesRef {
                container_name: name.clone(),
                member_name: field_name,
                line,
            });
        }
    }

    cpp_walk_scoped(node, src, Some(name.as_str()), out);
}

/// `function_definition`: free function, in-class method, or
/// out-of-line `Class::method(...) { ... }` -- mirrors
/// `languages/cpp.rs`'s `handle_function_definition` byte-for-byte.
fn cpp_handle_function_definition(
    node: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    out: &mut ParsedFile,
) {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        cpp_walk_scoped(node, src, enclosing, out);
        return;
    };
    let Some((name, out_of_line_scope)) = cpp_declarator_name_and_scope(declarator, src) else {
        cpp_walk_scoped(node, src, enclosing, out);
        return;
    };
    let line = node.start_position().row + 1;

    if let Some(test_name) = cpp_gtest_macro_test_name(&name, declarator, src) {
        out.symbols.push(SymbolRef {
            name: test_name,
            kind: SymbolKind::Test,
            line,
        });
        cpp_walk_scoped(node, src, enclosing, out);
        return;
    }

    let container = out_of_line_scope.as_deref().or(enclosing);
    let kind = if cpp_is_test_function(&name) {
        SymbolKind::Test
    } else if container.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind,
        line,
    });
    if let Some(container) = container {
        out.defines.push(DefinesRef {
            container_name: container.to_string(),
            member_name: name.clone(),
            line,
        });
    }
    // The body recurses with the *lexical* `enclosing` unchanged (an
    // out-of-line method's body is not lexically inside `Class`) but
    // `fn_scope` updated to this function/method -- matches the
    // bespoke arm's own `walk_children(node, src, out, enclosing,
    // FnScope { name: Some(...), line: Some(...) })` call exactly.
    let spec = LangSpec::cpp();
    let quirks = cpp_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name: Some(name.as_str()),
        line: Some(line),
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, fn_scope);
        }
    }
}

/// Generic per-child recursion under `enclosing` with a fresh
/// [`FnScope::default`] -- the shared "just recurse, same as
/// `walk_children`" helper every C++ quirk arm below that does not need
/// a special `fn_scope` uses.
fn cpp_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::cpp();
    let quirks = cpp_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, FnScope::default());
        }
    }
}

/// Everything C++'s flat `LangSpec` arrays cannot express (see
/// `LangSpec::cpp()`'s own doc comment) -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. Every arm returns `true`
/// (fully claimed): every one of these node kinds is also a member of
/// one of `LangSpec::cpp()`'s own `func_types`/`method_types`/
/// `class_types`/`enum_types`/`alias_types` arrays (or, for
/// `namespace_definition`/`declaration`/`preproc_def`/
/// `preproc_include`/`expression_statement`, is reached only via the
/// walker's single final catch-all with no earlier quirk-consulting
/// branch to double-invoke from), so unlike a couple of
/// [`c_quirk`]'s arms, none of these ever need the "return `false`,
/// let the *outer* fallback still run" pattern -- each arm performs its
/// own complete recursion internally instead.
fn cpp_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_definition" => {
            // `enclosing` (not hardcoded `None`): an in-class method
            // definition (`draw()`/`~Drawable()` written directly
            // inside a `class`/`struct` body, as opposed to an
            // out-of-line `Class::method` definition) has no
            // `qualified_identifier`-shaped declarator for
            // `cpp_declarator_name_and_scope` to read a container name
            // from -- its container is purely lexical, exactly the
            // `enclosing` this hook is now given, threaded all the way
            // down from `cpp_walk_scoped`'s own recursive `walk` calls.
            cpp_handle_function_definition(node, src, enclosing, out);
            true
        }
        "class_specifier" | "struct_specifier" => {
            cpp_handle_class_or_struct(node, src, enclosing, out);
            true
        }
        "namespace_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
                cpp_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                cpp_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "enum_specifier" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
            // `enclosing` (not `None`): `languages/cpp.rs`'s own
            // `enum_specifier` arm never recurses itself, falling
            // through to the shared bottom-of-`walk` call with the
            // *unchanged* `enclosing` its caller already had (same
            // rationale as `c_quirk`'s identical `enum_specifier` arm).
            cpp_walk_scoped(node, src, enclosing, out);
            true
        }
        "type_definition" => {
            for alias_name in cpp_typedef_alias_names(node, src) {
                out.symbols.push(SymbolRef {
                    name: alias_name,
                    kind: SymbolKind::TypeAlias,
                    line: node.start_position().row + 1,
                });
            }
            // `enclosing` (not `None`), same rationale as
            // `enum_specifier` above.
            cpp_walk_scoped(node, src, enclosing, out);
            true
        }
        "alias_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::TypeAlias,
                    line: node.start_position().row + 1,
                });
            }
            // `enclosing` (not `None`), same rationale as
            // `enum_specifier`/`type_definition` above.
            cpp_walk_scoped(node, src, enclosing, out);
            true
        }
        "declaration" => {
            if let Some(binding) = cpp_named_lambda_binding(node, src) {
                out.symbols.push(binding);
            } else {
                // `enclosing.is_some()` (not hardcoded `false`):
                // `languages/cpp.rs`'s own `declaration` arm passes
                // `enclosing.is_some()` through to
                // `top_level_declaration_symbols`'s
                // `inside_class_or_struct` gate -- a `declaration`
                // node reached while inside a class/struct body (this
                // grammar version parses some member declarations that
                // way) must be skipped entirely there, not
                // double-counted as a top-level global.
                for symbol in cpp_top_level_declaration_symbols(node, src, enclosing.is_some()) {
                    out.symbols.push(symbol);
                }
            }
            false
        }
        "preproc_def" if node.child_by_field_name("value").is_some() => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Constant,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "preproc_include" => {
            if let Some(path) = cpp_include_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "expression_statement" => {
            if let Some(binding) = cpp_named_lambda_binding_from_assignment(node, src) {
                out.symbols.push(binding);
            }
            false
        }
        _ => false,
    }
}

/// C++'s `call_expression` full override -- wired as
/// [`Quirks::call_override`], fully claiming the call (`true`) rather
/// than falling back to the shared generic reconstruction, for two
/// independent reasons: (1) the gtest `TEST(Suite, Name)`-family
/// side-effect symbol (`languages/cpp.rs`'s own `call_expression` arm
/// checks this on every call, not just ones that turn out to be gtest
/// macros), and (2) the receiver hint's C++-specific `field_expression`
/// shape, whose object sub-node lives in a field named `"argument"` --
/// not `"value"`, which is what Rust's *unrelated* grammar node that
/// happens to share the same kind name `field_expression` calls it
/// (the shared [`receiver_of_call`] only knows Rust's mapping, so
/// C++'s own [`cpp_receiver_of_call`] is used here instead of relying
/// on that shared helper).
fn cpp_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let Ok(callee) = function.utf8_text(src) else {
        return false;
    };
    let callee = callee.to_string();
    if let Some(test_name) = cpp_gtest_macro_test_name(&callee, node, src) {
        out.symbols.push(SymbolRef {
            name: test_name,
            kind: SymbolKind::Test,
            line: node.start_position().row + 1,
        });
    }
    let (receiver_text, receiver_hint) = cpp_receiver_of_call(function, src);
    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text,
        receiver_hint,
        arg_texts: cpp_call_arg_texts(node, src),
    });
    true
}

/// C++'s [`Quirks`] row: everything (see `LangSpec::cpp()`'s doc
/// comment for why).
pub fn cpp_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(cpp_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(cpp_call_override)),
    }
}

/// HTTP methods recognized in `[Http*]` attributes and `Map*`
/// minimal-API calls -- same list `languages/csharp.rs`'s
/// `HTTP_METHODS` const duplicates.
const CSHARP_HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// Every base-list entry's identifier text -- mirrors
/// `languages/csharp.rs`'s `base_list_names` byte-for-byte.
fn csharp_base_list_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(base_list) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == "base_list")
    else {
        return out;
    };
    for i in 0..base_list.child_count() {
        let Some(entry) = base_list.child(i) else {
            continue;
        };
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

/// Split a `class`/`struct` declaration's base list into INHERITS +
/// IMPLEMENTS -- mirrors `languages/csharp.rs`'s `emit_base_list_edges`
/// byte-for-byte.
fn csharp_emit_base_list_edges(
    node: Node<'_>,
    src: &[u8],
    type_name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let names = csharp_base_list_names(node, src);
    let mut iter = names.into_iter();
    let Some(first) = iter.next() else {
        return;
    };
    if csharp_looks_like_interface_name(&first) {
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

/// C# interface-naming convention -- mirrors `languages/csharp.rs`'s
/// `looks_like_interface_name` byte-for-byte.
fn csharp_looks_like_interface_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('I')) && matches!(chars.next(), Some(c) if c.is_uppercase())
}

/// `[Attribute]`/`[Attribute(...)]` lists directly preceding a
/// declaration -- mirrors `languages/csharp.rs`'s `attribute_names`
/// byte-for-byte.
fn csharp_attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(candidate) = node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(attr) = candidate.child(j) {
                if attr.kind() == "attribute" {
                    if let Some(name_node) = attr.child_by_field_name("name") {
                        if let Ok(text) = name_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

fn csharp_is_test_attribute(attribute_name: &str) -> bool {
    matches!(
        attribute_name.rsplit('.').next().unwrap_or(attribute_name),
        "Test" | "Fact" | "TestMethod" | "Theory"
    )
}

/// One route per matching `[Http*("...")]`/`[Route("...")]` attribute
/// on a method -- mirrors `languages/csharp.rs`'s
/// `routes_from_attributes` byte-for-byte.
fn csharp_routes_from_attributes(method_node: Node<'_>, src: &[u8], line: usize) -> Vec<RouteRef> {
    let mut out = Vec::new();
    for i in 0..method_node.child_count() {
        let Some(candidate) = method_node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(attr) = candidate.child(j) {
                if attr.kind() == "attribute" {
                    if let Some(route) = csharp_route_from_attribute(attr, src, line) {
                        out.push(route);
                    }
                }
            }
        }
    }
    out
}

fn csharp_route_from_attribute(attr: Node<'_>, src: &[u8], line: usize) -> Option<RouteRef> {
    let name_node = attr.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?;
    let method = if name.eq_ignore_ascii_case("Route") {
        String::new()
    } else if let Some(stripped) = name.strip_prefix("Http") {
        let candidate = stripped.to_lowercase();
        if !CSHARP_HTTP_METHODS.contains(&candidate.as_str()) {
            return None;
        }
        candidate.to_uppercase()
    } else {
        return None;
    };
    let path = csharp_attribute_first_string_arg(attr, src).unwrap_or_default();
    Some(RouteRef { method, path, line })
}

/// Mirrors `languages/csharp.rs`'s `attribute_first_string_arg`
/// byte-for-byte.
fn csharp_attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
    let args = (0..attr.child_count())
        .filter_map(|i| attr.child(i))
        .find(|c| c.kind() == "attribute_argument_list")?;
    for i in 0..args.child_count() {
        let arg = args.child(i)?;
        let literal = if arg.kind() == "string_literal" {
            Some(arg)
        } else {
            (0..arg.child_count())
                .filter_map(|i| arg.child(i))
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

/// Minimal-API endpoint calls (`app.MapGet("/path", handler)`) --
/// mirrors `languages/csharp.rs`'s `route_from_map_call` byte-for-byte,
/// wired as [`Quirks::route_from_call`].
fn csharp_route_from_map_call(callee: &str, call_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    let last_segment = callee.rsplit('.').next()?;
    let method = last_segment.strip_prefix("Map")?.to_lowercase();
    if !CSHARP_HTTP_METHODS.contains(&method.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
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

/// Mirrors `languages/csharp.rs`'s `using_directive_path`
/// byte-for-byte.
fn csharp_using_directive_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    if let Some(alias) = node.child_by_field_name("name") {
        return alias.utf8_text(src).ok().map(str::to_string);
    }
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

/// Parameter and return types on a method's signature -- mirrors
/// `languages/csharp.rs`'s `signature_type_refs` byte-for-byte.
fn csharp_signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if param.kind() == "parameter" {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
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

/// Mirrors `languages/csharp.rs`'s `is_const_or_static_readonly`
/// byte-for-byte.
fn csharp_is_const_or_static_readonly(node: Node<'_>, src: &[u8]) -> bool {
    let mut has_const = false;
    let mut has_static = false;
    let mut has_readonly = false;
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
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

/// Mirrors `languages/csharp.rs`'s `field_declarator_names`
/// byte-for-byte.
fn csharp_field_declarator_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(decl) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == "variable_declaration")
    else {
        return out;
    };
    for i in 0..decl.child_count() {
        if let Some(child) = decl.child(i) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(text) = name_node.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Everything C#'s flat `LangSpec` arrays cannot express: class/struct
/// base-list INHERITS+IMPLEMENTS split + attribute DECORATES +
/// DEFINES-scoped body walk, interface base-list all-INHERITS + DEFINES
/// (no scoped body walk -- matches `languages/csharp.rs`'s own
/// `interface_declaration` arm, which never re-scopes `enclosing` to
/// its own name), enum DEFINES (same no-rescoping), namespace Module
/// symbol (also no re-scoping -- `languages/csharp.rs`'s own
/// `namespace_declaration` arm has no scoped recursion either, unlike
/// TS's/C++'s namespace handling), `local_function_statement` as
/// [`SymbolKind::Lambda`] (never Function/Method), `const`/`static
/// readonly`-gated field Constant + DEFINES, and custom `using`-path
/// extraction -- wired as this wave's [`Quirks::on_unmatched_node`]
/// hook. Method-level extras (attribute test-check/type-refs/
/// decorates/routes, expression-bodied-member fallback) are a separate
/// [`Quirks::on_method_defined`] hook (see [`csharp_on_method_defined`]).
fn csharp_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" | "struct_declaration" => {
            // Fully claimed (`true`): both kinds are members of
            // `LangSpec::csharp()`'s own `class_types`, so this quirk
            // is invoked once from the class-shape branch's own early
            // check -- returning `false` would fall through to that
            // branch's `name_field`-keyed fallback, which (unlike C/
            // C++) is *not* neutralized here (C#'s `name_field` really
            // is `"name"`), so it would double-push a second, wrongly-
            // classified (always `SymbolKind::Class`, never `Struct`,
            // and missing every one of the edges/DEFINES below)
            // symbol for the same node. `true` prevents that.
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
                csharp_emit_base_list_edges(node, src, &name, line, out);
                for decorator_name in csharp_attribute_names(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
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
                csharp_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                csharp_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "interface_declaration" => {
            // Fully claimed for the same double-invocation reason as
            // `class_declaration` above. No scoped body walk (matches
            // the bespoke arm exactly).
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for base_name in csharp_base_list_names(node, src) {
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
            csharp_walk_scoped(node, src, enclosing, out);
            true
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
            csharp_walk_scoped(node, src, enclosing, out);
            true
        }
        "namespace_declaration" | "file_scoped_namespace_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
            // `enclosing` unchanged (not re-scoped to the namespace's
            // own name): matches `languages/csharp.rs`'s own
            // `namespace_declaration` arm exactly, which has no scoped
            // recursion of its own.
            csharp_walk_scoped(node, src, enclosing, out);
            true
        }
        "field_declaration" if csharp_is_const_or_static_readonly(node, src) => {
            for name in csharp_field_declarator_names(node, src) {
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
            false
        }
        "local_function_statement" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Lambda,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "using_directive" => {
            if let Some(path) = csharp_using_directive_path(node, src) {
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

/// Generic per-child recursion under `enclosing` -- the shared "just
/// recurse, same as `walk_children`" helper `csharp_quirk`'s arms use.
fn csharp_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::csharp();
    let quirks = csharp_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, FnScope::default());
        }
    }
}

/// Attribute test-check/TYPE_REF/DECORATES/routes for a just-recorded
/// C# method symbol -- wired as [`Quirks::on_method_defined`], mirrors
/// `languages/csharp.rs`'s `method_declaration` arm. Note: the
/// expression-bodied-member fallback (`int F() => x.Y();`, no `body`
/// field) is already handled correctly by the *generic* engine's own
/// func/method branch, which recurses into `spec.body_field`
/// (`"body"`) only `if let Some(body) = ...` -- when absent, it simply
/// does not recurse further there, same as the bespoke arm's own
/// `else` branch falling back to `walk_children(node, ...)` over the
/// whole node: neither call ever finds a body-less method's `=>`
/// expression sibling specially, so this is observably identical
/// without needing its own hook logic.
fn csharp_on_method_defined(
    node: Node<'_>,
    name: &str,
    line: usize,
    src: &[u8],
    out: &mut ParsedFile,
) {
    if node.kind() != "method_declaration" {
        return;
    }
    let attributes = csharp_attribute_names(node, src);
    if attributes.iter().any(|a| csharp_is_test_attribute(a)) {
        if let Some(last) = out.symbols.last_mut() {
            if last.name == name && last.line == line {
                last.kind = SymbolKind::Test;
            }
        }
    }
    for type_ref in csharp_signature_type_refs(node, src) {
        out.type_refs.push(crate::parsers::TypeRefRef {
            from_name: name.to_string(),
            type_name: type_ref,
            line,
        });
    }
    for decorator_name in &attributes {
        out.decorates.push(crate::parsers::DecoratesRef {
            target_name: name.to_string(),
            decorator_name: decorator_name.clone(),
            line,
        });
    }
    for route in csharp_routes_from_attributes(node, src, line) {
        out.routes.push(route);
    }
}

/// C#'s [`Quirks`] row: class/struct base-list INHERITS+IMPLEMENTS +
/// attribute DECORATES + DEFINES-scoped body walk, interface/enum/
/// namespace DEFINES (no re-scoping), local-function Lambda,
/// const/static-readonly field Constant + DEFINES, custom `using`-path
/// extraction, method attribute test-check/TYPE_REF/DECORATES/routes,
/// and minimal-API `Map*` call routes.
pub fn csharp_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(csharp_quirk)),
        is_test_name: |_| false,
        route_from_call: Some(Box::new(csharp_route_from_map_call)),
        on_method_defined: Some(Box::new(csharp_on_method_defined)),
        call_override: None,
    }
}

/// Parse C# source through the generic engine (this wave's C#
/// zero-regression proof against `languages::csharp::parse`).
pub fn parse_csharp(source: &str) -> ParsedFile {
    let spec = LangSpec::csharp();
    let quirks = csharp_quirks();
    let language: tree_sitter::Language = tree_sitter_c_sharp::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// HTTP methods recognized in `Route::get(...)`-style static calls --
/// same list `languages/php.rs`'s `HTTP_METHODS` const duplicates.
const PHP_HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

/// Every `name` entry inside a `base_clause`/`class_interface_clause`
/// child matching `keyword` -- mirrors `languages/php.rs`'s
/// `heritage_names` byte-for-byte.
fn php_heritage_names(node: Node<'_>, src: &[u8], keyword: &str) -> Vec<String> {
    let mut out = Vec::new();
    let clause_kind = if keyword == "extends" {
        "base_clause"
    } else {
        "class_interface_clause"
    };
    let Some(clause) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == clause_kind)
    else {
        return out;
    };
    for i in 0..clause.child_count() {
        if let Some(entry) = clause.child(i) {
            if entry.kind() == "name" || entry.kind() == "qualified_name" {
                if let Ok(text) = entry.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// A `class_declaration`'s heritage edges, reporting whether the base
/// class is `TestCase` -- mirrors `languages/php.rs`'s
/// `emit_class_heritage_edges` byte-for-byte.
fn php_emit_class_heritage_edges(
    node: Node<'_>,
    src: &[u8],
    type_name: &str,
    line: usize,
    out: &mut ParsedFile,
) -> bool {
    let mut extends_test_case = false;
    for base_name in php_heritage_names(node, src, "extends") {
        if base_name.rsplit('\\').next() == Some("TestCase") {
            extends_test_case = true;
        }
        out.inherits.push(InheritsRef {
            sub_name: type_name.to_string(),
            super_name: base_name,
            line,
        });
    }
    for iface_name in php_heritage_names(node, src, "implements") {
        out.implements.push(ImplementsRef {
            type_name: type_name.to_string(),
            trait_name: iface_name,
            line,
        });
    }
    extends_test_case
}

/// PHP 8 `#[Attribute]`/`#[Attribute(...)]` groups directly preceding
/// a declaration -- mirrors `languages/php.rs`'s `attribute_names`
/// byte-for-byte.
fn php_attribute_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(candidate) = node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(group) = candidate.child(j) {
                if group.kind() != "attribute_group" {
                    continue;
                }
                for k in 0..group.child_count() {
                    if let Some(attr) = group.child(k) {
                        if attr.kind() == "attribute" {
                            if let Some(name) = php_attribute_name_text(attr, src) {
                                out.push(name);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// Mirrors `languages/php.rs`'s `attribute_name_text` byte-for-byte.
fn php_attribute_name_text(attr: Node<'_>, src: &[u8]) -> Option<String> {
    (0..attr.child_count())
        .filter_map(|i| attr.child(i))
        .find(|c| matches!(c.kind(), "name" | "qualified_name"))
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

fn php_is_test_attribute(attribute_name: &str) -> bool {
    matches!(
        attribute_name.rsplit('\\').next().unwrap_or(attribute_name),
        "Test"
    )
}

/// Symfony-style `#[Route("/path")]` attributes on a method -- mirrors
/// `languages/php.rs`'s `routes_from_attributes` byte-for-byte.
fn php_routes_from_attributes(method_node: Node<'_>, src: &[u8], line: usize) -> Vec<RouteRef> {
    let mut out = Vec::new();
    for i in 0..method_node.child_count() {
        let Some(candidate) = method_node.child(i) else {
            continue;
        };
        if candidate.kind() != "attribute_list" {
            continue;
        }
        for j in 0..candidate.child_count() {
            if let Some(group) = candidate.child(j) {
                if group.kind() != "attribute_group" {
                    continue;
                }
                for k in 0..group.child_count() {
                    if let Some(attr) = group.child(k) {
                        if attr.kind() == "attribute" {
                            if let Some(route) = php_route_from_symfony_attribute(attr, src, line) {
                                out.push(route);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn php_route_from_symfony_attribute(attr: Node<'_>, src: &[u8], line: usize) -> Option<RouteRef> {
    let name = php_attribute_name_text(attr, src)?;
    if name.rsplit('\\').next().unwrap_or(&name) != "Route" {
        return None;
    }
    let path = php_attribute_first_string_arg(attr, src).unwrap_or_default();
    Some(RouteRef {
        method: String::new(),
        path,
        line,
    })
}

fn php_attribute_first_string_arg(attr: Node<'_>, src: &[u8]) -> Option<String> {
    let args = (0..attr.child_count())
        .filter_map(|i| attr.child(i))
        .find(|c| c.kind() == "arguments")?;
    for i in 0..args.child_count() {
        let arg = args.child(i)?;
        if arg.kind() != "argument" {
            continue;
        }
        for j in 0..arg.child_count() {
            if let Some(candidate) = arg.child(j) {
                if matches!(candidate.kind(), "string" | "encapsed_string") {
                    if let Ok(text) = candidate.utf8_text(src) {
                        return Some(php_strip_string_literal(text));
                    }
                }
            }
        }
    }
    None
}

/// Laravel-style `Route::get('/path', ...)` static calls -- mirrors
/// `languages/php.rs`'s `route_from_scoped_call` byte-for-byte.
fn php_route_from_scoped_call(
    call_node: Node<'_>,
    name_node: Node<'_>,
    src: &[u8],
) -> Option<RouteRef> {
    let scope = call_node.child_by_field_name("scope")?;
    let scope_text = scope.utf8_text(src).ok()?;
    if scope_text.rsplit('\\').next().unwrap_or(scope_text) != "Route" {
        return None;
    }
    let method_name = name_node.utf8_text(src).ok()?.to_lowercase();
    if !PHP_HTTP_METHODS.contains(&method_name.as_str()) {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .filter(|n| n.kind() == "argument")
        .find_map(|arg| {
            (0..arg.child_count())
                .filter_map(|i| arg.child(i))
                .find(|c| matches!(c.kind(), "string" | "encapsed_string"))
        })?;
    let raw = first_string.utf8_text(src).ok()?;
    Some(RouteRef {
        method: method_name.to_uppercase(),
        path: php_strip_string_literal(raw),
        line: call_node.start_position().row + 1,
    })
}

fn php_qualify_scoped_call(call_node: Node<'_>, name_node: Node<'_>, src: &[u8]) -> String {
    let name = name_node.utf8_text(src).unwrap_or("");
    match call_node
        .child_by_field_name("scope")
        .and_then(|s| s.utf8_text(src).ok())
    {
        Some(scope) => format!("{scope}::{name}"),
        None => name.to_string(),
    }
}

fn php_strip_string_literal(text: &str) -> String {
    text.trim_matches(|c| c == '"' || c == '\'').to_string()
}

/// Mirrors `languages/php.rs`'s `namespace_use_paths` byte-for-byte.
fn php_namespace_use_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "namespace_use_clause" => {
                if let Some(path) = php_namespace_use_clause_path(child, src) {
                    out.push(path);
                }
            }
            "namespace_use_group" => {
                let prefix = child
                    .child_by_field_name("prefix")
                    .and_then(|n| n.utf8_text(src).ok())
                    .unwrap_or("");
                for j in 0..child.child_count() {
                    if let Some(clause) = child.child(j) {
                        if clause.kind() == "namespace_use_clause" {
                            if let Some(tail) = php_namespace_use_clause_path(clause, src) {
                                out.push(if prefix.is_empty() {
                                    tail
                                } else {
                                    format!("{prefix}\\{tail}")
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if out.is_empty() {
        if let Ok(text) = node.utf8_text(src) {
            let trimmed = text
                .trim_start_matches("use")
                .trim()
                .trim_end_matches(';')
                .trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn php_namespace_use_clause_path(clause: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..clause.child_count() {
        let child = clause.child(i)?;
        if matches!(child.kind(), "qualified_name" | "name") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `require`/`require_once`/`include`/`include_once` expressions --
/// mirrors `languages/php.rs`'s `require_include_import` byte-for-byte.
fn php_require_include_import(node: Node<'_>, src: &[u8]) -> Option<ImportRef> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(
            child.kind(),
            "require_expression"
                | "require_once_expression"
                | "include_expression"
                | "include_once_expression"
        ) {
            for j in 0..child.child_count() {
                if let Some(target) = child.child(j) {
                    if matches!(target.kind(), "string" | "encapsed_string") {
                        if let Ok(text) = target.utf8_text(src) {
                            return Some(ImportRef {
                                module_path: php_strip_string_literal(text),
                                line: node.start_position().row + 1,
                            });
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parameter and return types on a method/function's signature --
/// mirrors `languages/php.rs`'s `signature_type_refs` byte-for-byte.
fn php_signature_type_refs(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            if let Some(param) = params.child(i) {
                if matches!(
                    param.kind(),
                    "simple_parameter" | "property_promotion_parameter"
                ) {
                    if let Some(type_node) = param.child_by_field_name("type") {
                        if let Ok(text) = type_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(return_type) = node.child_by_field_name("return_type") {
        if let Ok(text) = return_type.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// `$f = function($x) { ... };` / `$g = fn($x) => ...;` -- mirrors
/// `languages/php.rs`'s `named_closure_binding` byte-for-byte.
fn php_named_closure_binding(node: Node<'_>, src: &[u8]) -> Option<SymbolRef> {
    let left = node.child_by_field_name("left")?;
    if left.kind() != "variable_name" {
        return None;
    }
    let right = node.child_by_field_name("right")?;
    if !matches!(right.kind(), "anonymous_function" | "arrow_function") {
        return None;
    }
    let name = left
        .utf8_text(src)
        .ok()?
        .trim_start_matches('$')
        .to_string();
    Some(SymbolRef {
        name,
        kind: SymbolKind::Lambda,
        line: node.start_position().row + 1,
    })
}

/// `define("NAME", value)` -- mirrors `languages/php.rs`'s
/// `constant_from_define_call` byte-for-byte.
fn php_constant_from_define_call(
    callee: &str,
    call_node: Node<'_>,
    src: &[u8],
) -> Option<SymbolRef> {
    if callee != "define" {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_arg = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "argument")?;
    let literal = (0..first_arg.child_count())
        .filter_map(|i| first_arg.child(i))
        .find(|n| matches!(n.kind(), "string" | "encapsed_string"))?;
    let text = literal.utf8_text(src).ok()?;
    let name = php_strip_string_literal(text);
    if name.is_empty() {
        return None;
    }
    Some(SymbolRef {
        name,
        kind: SymbolKind::Constant,
        line: call_node.start_position().row + 1,
    })
}

/// Each argument expression's own source text -- mirrors
/// `languages/php.rs`'s `call_arg_texts` byte-for-byte.
fn php_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// The receiver of a `member_call_expression`/
/// `nullsafe_member_call_expression`, plus a cheap syntactic hint --
/// mirrors `languages/php.rs`'s `receiver_of_member_call`
/// byte-for-byte.
fn php_receiver_of_member_call(
    call_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    let Some(receiver) = call_node.child_by_field_name("object") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "$this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "object_creation_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "variable_name" {
        ReceiverHint::Identifier
    } else if matches!(
        receiver.kind(),
        "string" | "encapsed_string" | "integer" | "float"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// PHPUnit "extends TestCase" test-detection: walk up from a method
/// node to its lexically-enclosing `class_declaration` (if any) and
/// check whether *that* class's own heritage names `TestCase` --
/// computed on demand from tree structure alone (via `Node::parent()`)
/// rather than threaded state, since [`Quirks::on_method_defined`]'s
/// signature carries no extra per-language scope field the way
/// `languages/php.rs`'s own `WalkScope::enclosing_extends_test_case`
/// does. Observably identical: a method's enclosing class in valid PHP
/// is always its nearest `class_declaration` ancestor, so re-deriving
/// it this way finds the exact same class the bespoke walk was already
/// tracking.
fn php_enclosing_extends_test_case(method_node: Node<'_>, src: &[u8]) -> bool {
    let mut current = method_node.parent();
    while let Some(node) = current {
        if node.kind() == "class_declaration" {
            return php_heritage_names(node, src, "extends")
                .iter()
                .any(|base_name| base_name.rsplit('\\').next() == Some("TestCase"));
        }
        current = node.parent();
    }
    false
}

/// Everything PHP's flat `LangSpec` arrays cannot express: class/trait/
/// interface declarations (heritage, attributes, DEFINES-scoped body
/// walk), namespace Module symbol, class constants (`const NAME = ..`),
/// `use`-path imports, `require`/`include` imports, named-closure-
/// binding Lambda, and all four call-shaped node kinds (each with its
/// own field-name shape, receiver logic, and route detection) -- wired
/// as this wave's [`Quirks::on_unmatched_node`] hook. Method-level
/// extras (attribute test-check/type-refs/decorates/routes, PHPUnit
/// "extends TestCase"/`test`-name-inside-class detection) are a
/// separate [`Quirks::on_method_defined`] hook (see
/// [`php_on_method_defined`]).
fn php_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                php_emit_class_heritage_edges(node, src, &name, line, out);
                for decorator_name in php_attribute_names(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
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
                php_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                php_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "trait_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line: node.start_position().row + 1,
                });
                php_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                php_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "interface_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                for base_name in php_heritage_names(node, src, "extends") {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: base_name,
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
                php_walk_scoped(node, src, Some(name.as_str()), out);
            } else {
                php_walk_scoped(node, src, enclosing, out);
            }
            true
        }
        "namespace_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(text) = name_node.utf8_text(src) {
                    out.symbols.push(SymbolRef {
                        name: text.to_string(),
                        kind: SymbolKind::Module,
                        line: node.start_position().row + 1,
                    });
                }
            }
            false
        }
        "const_declaration" => {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "const_element" {
                        let name_node = (0..child.child_count())
                            .filter_map(|j| child.child(j))
                            .find(|c| c.kind() == "name");
                        if let Some(name_node) = name_node {
                            if let Ok(text) = name_node.utf8_text(src) {
                                let line = node.start_position().row + 1;
                                out.symbols.push(SymbolRef {
                                    name: text.to_string(),
                                    kind: SymbolKind::Constant,
                                    line,
                                });
                                if let Some(container) = enclosing {
                                    out.defines.push(DefinesRef {
                                        container_name: container.to_string(),
                                        member_name: text.to_string(),
                                        line,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            false
        }
        "namespace_use_declaration" => {
            for path in php_namespace_use_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "expression_statement" => {
            if let Some(import) = php_require_include_import(node, src) {
                out.imports.push(import);
            }
            false
        }
        // No arms for `function_call_expression`/
        // `member_call_expression`/`nullsafe_member_call_expression`/
        // `scoped_call_expression` here: all four are members of
        // `LangSpec::php()`'s own `call_types`, so `walk`'s call
        // branch invokes [`php_call_override`] for them *before* this
        // `on_unmatched_node` catch-all would ever run -- see
        // `LangSpec::php()`'s own doc comment for why `call_override`
        // (which receives the caller's `fn_scope`) is used instead of
        // handling these here (this hook's signature carries no
        // `fn_scope`, only `enclosing`).
        "assignment_expression" => {
            if let Some(binding) = php_named_closure_binding(node, src) {
                out.symbols.push(binding);
            }
            false
        }
        _ => false,
    }
}

/// Generic per-child recursion under `enclosing` -- the shared "just
/// recurse, same as `walk_children`" helper `php_quirk`'s
/// class/interface/trait arms use. `fn_scope` resets to
/// [`FnScope::default`] rather than threading through whatever the
/// caller had: a class/interface/trait *declaration* itself is not
/// call-shaped, so the only way this would ever observably differ from
/// the bespoke walk is a call appearing directly inside a nested
/// class's field initializer while that whole class declaration is
/// itself lexically inside another function's body (PHP permits
/// declaring a class inside a function) -- an edge case no existing
/// fixture/test exercises either way.
fn php_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::php();
    let quirks = php_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, FnScope::default());
        }
    }
}

/// Full override for all four of PHP's call-shaped node kinds -- wired
/// as [`Quirks::call_override`] (not `on_unmatched_node`) specifically
/// so `from_symbol`/`from_symbol_line` can be populated from the
/// caller's actual `fn_scope` (`on_unmatched_node`'s signature carries
/// no such parameter, only `enclosing`), matching
/// `languages/php.rs`'s own four call-shape arms byte-for-byte.
fn php_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "function_call_expression" => {
            let Some(function) = node.child_by_field_name("function") else {
                return false;
            };
            let callee = function.utf8_text(src).unwrap_or("").to_string();
            out.calls.push(CallRef {
                callee: callee.clone(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: php_call_arg_texts(node, src),
            });
            if let Some(constant) = php_constant_from_define_call(&callee, node, src) {
                out.symbols.push(constant);
            }
            true
        }
        "member_call_expression" | "nullsafe_member_call_expression" => {
            let Some(name) = node.child_by_field_name("name") else {
                return false;
            };
            let Ok(text) = name.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = php_receiver_of_member_call(node, src);
            out.calls.push(CallRef {
                callee: text.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: php_call_arg_texts(node, src),
            });
            true
        }
        "scoped_call_expression" => {
            let Some(name) = node.child_by_field_name("name") else {
                return false;
            };
            let callee = php_qualify_scoped_call(node, name, src);
            let receiver_hint = node
                .child_by_field_name("scope")
                .and_then(|s| s.utf8_text(src).ok())
                .map(|scope| {
                    if scope.rsplit('.').next() == Some("new") || scope == "new" {
                        ReceiverHint::NewExpression
                    } else {
                        ReceiverHint::Other
                    }
                });
            out.calls.push(CallRef {
                callee,
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: node
                    .child_by_field_name("scope")
                    .and_then(|s| s.utf8_text(src).ok())
                    .map(str::to_string),
                receiver_hint,
                arg_texts: php_call_arg_texts(node, src),
            });
            if let Some(route) = php_route_from_scoped_call(node, name, src) {
                out.routes.push(route);
            }
            true
        }
        _ => false,
    }
}

/// Attribute test-check/TYPE_REF/DECORATES/routes plus PHPUnit
/// "extends TestCase"/`test`-name-inside-class reclassification for a
/// just-recorded PHP method/function symbol -- wired as
/// [`Quirks::on_method_defined`], mirrors `languages/php.rs`'s
/// `method_declaration`/`function_definition` arm.
fn php_on_method_defined(
    node: Node<'_>,
    name: &str,
    line: usize,
    src: &[u8],
    out: &mut ParsedFile,
) {
    if !matches!(node.kind(), "method_declaration" | "function_definition") {
        return;
    }
    let attributes = php_attribute_names(node, src);
    let enclosing_is_method = node.kind() == "method_declaration";
    let is_test = php_enclosing_extends_test_case(node, src)
        || attributes.iter().any(|a| php_is_test_attribute(a))
        || (enclosing_is_method && name.to_lowercase().starts_with("test"));
    if is_test {
        if let Some(last) = out.symbols.last_mut() {
            if last.name == name && last.line == line {
                last.kind = SymbolKind::Test;
            }
        }
    }
    for type_ref in php_signature_type_refs(node, src) {
        out.type_refs.push(crate::parsers::TypeRefRef {
            from_name: name.to_string(),
            type_name: type_ref,
            line,
        });
    }
    for decorator_name in &attributes {
        out.decorates.push(crate::parsers::DecoratesRef {
            target_name: name.to_string(),
            decorator_name: decorator_name.clone(),
            line,
        });
    }
    for route in php_routes_from_attributes(node, src, line) {
        out.routes.push(route);
    }
}

/// PHP's [`Quirks`] row: everything (see `LangSpec::php()`'s doc
/// comment for why `call_types` is empty and every call shape is
/// claimed directly by [`php_quirk`]).
pub fn php_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(php_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: Some(Box::new(php_on_method_defined)),
        call_override: Some(Box::new(php_call_override)),
    }
}

/// Parse PHP source through the generic engine (this wave's PHP
/// zero-regression proof against `languages::php::parse`).
pub fn parse_php(source: &str) -> ParsedFile {
    let spec = LangSpec::php();
    let quirks = php_quirks();
    let language: tree_sitter::Language = tree_sitter_php::LANGUAGE_PHP.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// Parse C++ source through the generic engine (this wave's C++
/// zero-regression proof against `languages::cpp::parse`).
/// `is_test_file` mirrors `languages/cpp.rs::parse`'s own post-walk
/// reclassification pass exactly (every Function/Method becomes Test
/// when `true`).
pub fn parse_cpp(source: &str, is_test_file: bool) -> ParsedFile {
    let spec = LangSpec::cpp();
    let quirks = cpp_quirks();
    let language: tree_sitter::Language = tree_sitter_cpp::LANGUAGE.into();
    let mut out = parse_with_spec(source, &language, &spec, &quirks, false);
    if is_test_file {
        for symbol in &mut out.symbols {
            if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
                symbol.kind = SymbolKind::Test;
            }
        }
    }
    out
}

/// Kotlin's `secondary_constructor`/`class_declaration`/
/// `object_declaration`/`companion_object`/`function_declaration`/
/// `anonymous_function` bodies all live as an UNFIELDED child
/// (`function_body`/`class_body`) rather than a `"body"` field --
/// [`LangSpec::kotlin`]'s own doc comment records the full grammar-shape
/// finding. This helper finds that child by kind, mirroring
/// `child_by_field_name`'s contract (a `Option<Node>`) for the two kinds
/// the generic engine's own `func_types`/`method_types` walk needs.
fn kotlin_unfielded_body<'a>(node: Node<'a>, body_kind: &str) -> Option<Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|child| child.kind() == body_kind)
}

/// Kotlin's `function_declaration`/`anonymous_function`/
/// `secondary_constructor` symbol + DEFINES-scoped body walk -- mirrors
/// the generic engine's own func/method branch (`walk`'s
/// `spec.func_types.contains(&kind) || spec.method_types.contains(&kind)`
/// arm) but reimplemented directly because Kotlin's body is an
/// unfielded child (see [`kotlin_unfielded_body`]), which
/// [`LangSpec::body_field`]'s generic `child_by_field_name` lookup
/// cannot find. `enclosing` is threaded through explicitly (unlike most
/// of this file's other `on_unmatched_node` hooks, whose signature
/// carries `enclosing` too) so a method nested inside a
/// `class_declaration`/`object_declaration`/`companion_object` body
/// still gets its DEFINES edge, matching every other language's
/// class-body-method convention.
fn kotlin_function_like(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let line = node.start_position().row + 1;
    // `secondary_constructor` has no name at all in this grammar (a
    // Kotlin constructor is unnamed, unlike every other Kotlin
    // function-shaped node) -- record it as a nameless Method the same
    // way `c_quirk`'s anonymous-struct case has no name to push either,
    // then still walk its body for nested calls.
    let name = if node.kind() == "secondary_constructor" {
        None
    } else {
        child_text(node, "name", src)
    };
    if let Some(name) = &name {
        let kind = if enclosing.is_some() {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };
        out.symbols.push(SymbolRef {
            name: name.clone(),
            kind,
            line,
        });
        if let Some(container) = enclosing {
            out.defines.push(DefinesRef {
                container_name: container.to_string(),
                member_name: name.clone(),
                line,
            });
        }
    } else if node.kind() == "secondary_constructor" {
        out.symbols.push(SymbolRef {
            name: String::new(),
            kind: SymbolKind::Method,
            line,
        });
    }
    if let Some(body) = kotlin_unfielded_body(node, "function_body") {
        kotlin_walk_scoped(
            body,
            src,
            enclosing,
            FnScope {
                name: name.as_deref(),
                line: Some(line),
            },
            out,
        );
    }
    true
}

/// Kotlin's `class_declaration`/`object_declaration`/
/// `companion_object` symbol + DEFINES-scoped body walk -- same
/// unfielded-body rationale as [`kotlin_function_like`].
/// `declaration_kind`/`class_body` are direct unfielded children here
/// too (this grammar's `class_declaration` has a real `name` field, see
/// [`LangSpec::kotlin`]'s doc comment, so only the body lookup needs
/// this treatment).
fn kotlin_class_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = child_text(node, "name", src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Class,
        line,
    });
    for super_name in kotlin_delegation_bases(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: name.clone(),
            super_name,
            line,
        });
    }
    if let Some(body) = kotlin_unfielded_body(node, "class_body") {
        kotlin_walk_scoped(body, src, Some(name.as_str()), FnScope::default(), out);
    }
    true
}

/// `class Sub: Base(), IFace` -- Kotlin heritage lives in a
/// `delegation_specifiers` wrapper child of the class/object node,
/// itself holding one `delegation_specifier` per base, each wrapping
/// either a bare `user_type` (interface-only heritage) or a
/// `constructor_invocation` wrapping the `user_type` (superclass
/// heritage, called with constructor args) -- verified directly
/// against `tree-sitter-kotlin-ng`'s own real parse tree with a
/// standalone debug harness against this exact crate version (`class
/// Widget : Base() {}` parses as `delegation_specifiers` >
/// `delegation_specifier` > `constructor_invocation` > `user_type` >
/// `identifier`), not assumed from the baseline's own C source alone.
/// The baseline's own `extract_kotlin_bases`
/// (`codebase-memory-mcp/internal/cbm/extract_defs.c` :2210-2247)
/// iterates `delegation_specifier` as a DIRECT child of the class node
/// with no `delegation_specifiers` wrapper level at all -- either an
/// older/different tree-sitter-kotlin grammar revision genuinely
/// lacked this wrapper, or the baseline's own walk has a latent bug
/// there; moot either way, since this function targets OUR actual
/// grammar dependency's real, empirically-confirmed shape rather than a
/// byte-for-byte port of C against a grammar revision we do not
/// ourselves vendor. Once past the `delegation_specifiers` wrapper,
/// descends through an optional `constructor_invocation` layer exactly
/// as the baseline describes, then takes `user_type`'s own first named
/// child (the `identifier`, stripped of any generic-argument tail by
/// only ever reading that first child rather than the whole
/// `user_type` text).
fn kotlin_delegation_bases(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(specifiers) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "delegation_specifiers")
    else {
        return out;
    };
    for i in 0..specifiers.child_count() {
        let Some(child) = specifiers.child(i) else {
            continue;
        };
        if child.kind() != "delegation_specifier" {
            continue;
        }
        let Some(mut user_type) = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.is_named())
        else {
            continue;
        };
        if user_type.kind() == "constructor_invocation" {
            let Some(inner) = (0..user_type.child_count())
                .filter_map(|j| user_type.child(j))
                .find(|n| n.is_named())
            else {
                continue;
            };
            user_type = inner;
        }
        let name_node = if user_type.kind() == "user_type" {
            (0..user_type.child_count())
                .filter_map(|j| user_type.child(j))
                .find(|n| n.is_named())
                .unwrap_or(user_type)
        } else {
            user_type
        };
        if let Ok(text) = name_node.utf8_text(src) {
            if !text.is_empty() {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// Recurse into a Kotlin function/class body under a freshly-scoped
/// `enclosing`/`fn_scope`, mirroring every other quirk's
/// `*_walk_scoped_body` helper in this file (`ts_walk_scoped_body`/
/// `py_walk_scoped_body`/`rust_walk_impl_body`) -- needed because
/// Kotlin's body is an unfielded child the generic engine's own
/// body-walk (keyed off [`LangSpec::body_field`]) can never reach (see
/// [`kotlin_unfielded_body`]).
fn kotlin_walk_scoped(
    body: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::kotlin();
    let quirks = kotlin_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, enclosing, fn_scope);
        }
    }
}

/// `type Sub = Base` -- Kotlin's `type_alias` names its own alias via a
/// field literally called `"type"` (not `"name"`, unlike every other
/// language's alias-shaped node in this file) -- see
/// [`LangSpec::kotlin`]'s own doc comment.
fn kotlin_type_alias(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    if let Some(name) = child_text(node, "type", src) {
        out.symbols.push(SymbolRef {
            name,
            kind: SymbolKind::TypeAlias,
            line: node.start_position().row + 1,
        });
    }
    true
}

/// `import a.b.C` / `import a.b.*` -- Kotlin's `import` node has no
/// fields at all (see [`LangSpec::kotlin`]'s doc comment); the path is
/// either a `qualified_identifier` or a bare `identifier` unfielded
/// child.
fn kotlin_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| matches!(n.kind(), "qualified_identifier" | "identifier"))?;
    path_node.utf8_text(src).ok().map(str::to_string)
}

/// Everything Kotlin's flat [`LangSpec`] arrays cannot express directly
/// (unfielded bodies, `type_alias`'s `"type"`-named field, unfielded
/// import paths) -- wired as this wave's [`Quirks::on_unmatched_node`]
/// hook. Fully claims every one of [`LangSpec::kotlin`]'s `func_types`/
/// `method_types`/`class_types`/`alias_types` node kinds (same posture
/// as [`c_quirk`]/[`cpp_quirk`]/[`php_quirk`] -- see
/// [`LangSpec::kotlin`]'s own doc comment for why the generic engine's
/// field-name-keyed fallbacks could never reach them).
fn kotlin_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_declaration" | "anonymous_function" | "secondary_constructor" => {
            kotlin_function_like(node, enclosing, src, out)
        }
        "class_declaration" | "object_declaration" | "companion_object" => {
            kotlin_class_like(node, src, out)
        }
        "type_alias" => kotlin_type_alias(node, src, out),
        "import" => {
            if let Some(path) = kotlin_import_path(node, src) {
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

/// Kotlin's `call_expression`/`navigation_expression` have NO fields at
/// all (see [`LangSpec::kotlin`]'s own doc comment) -- wired as this
/// wave's [`Quirks::call_override`] hook, mirroring
/// [`php_call_override`]'s posture of fully claiming every `call_types`
/// entry directly rather than relying on the generic engine's own
/// single-field `call_function_field` reconstruction.
fn kotlin_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "call_expression" => {
            // The callee is the call's own first named child (an
            // `expression`-shaped node -- a bare `identifier`, or a
            // `navigation_expression` for `a.b(...)`); `value_arguments`
            // is an unfielded sibling. Reading the whole callee
            // sub-node's own text (rather than resolving down to a bare
            // name) matches every other receiver-qualified callee
            // convention already used in this file (e.g. Go's
            // `w.Draw` full-text callee).
            let Some(callee_node) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.is_named())
            else {
                return false;
            };
            let Ok(callee) = callee_node.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = if callee_node.kind() == "navigation_expression" {
                kotlin_navigation_receiver(callee_node, src)
            } else {
                (None, None)
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: kotlin_value_arguments_texts(node, src),
            });
            true
        }
        "navigation_expression" => {
            // A bare navigation (`a.b`, no call suffix) -- still one of
            // `LangSpec::kotlin`'s own `call_types` per the baseline's
            // `kotlin_call_types` array (a property read modeled the
            // same as a call, the baseline's own choice, not this
            // wave's). Only reached when NOT already claimed as a
            // `call_expression`'s own callee sub-node above (a
            // `navigation_expression` wrapped by a `call_expression`
            // parent is never independently walked to as a top-level
            // node by the generic engine, since `call_expression`
            // returns `true`/fully-claimed before generic recursion
            // would ever reach its own children as siblings).
            let Ok(callee) = node.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = kotlin_navigation_receiver(node, src);
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: Vec::new(),
            });
            true
        }
        _ => false,
    }
}

/// `a.b`'s receiver (`a`) plus a syntactic hint, mirroring this file's
/// shared [`receiver_of_call`] convention for every other language's
/// method-call-shaped callee -- Kotlin's `navigation_expression` has no
/// fields (its `expression`/`identifier` children are unfielded, see
/// [`LangSpec::kotlin`]'s doc comment), so this reads the first named
/// child directly instead of `child_by_field_name`.
fn kotlin_navigation_receiver(
    navigation_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    let Some(receiver) = (0..navigation_node.child_count())
        .filter_map(|i| navigation_node.child(i))
        .find(|n| n.is_named())
    else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "call_expression" && text.rsplit('(').next() != Some(text) {
        // A call as the navigation's own receiver (`Widget().draw()`)
        // -- Kotlin's idiomatic constructor call has no dedicated `new`
        // keyword/node kind (a constructor invocation is written
        // exactly like an ordinary call), so this only recognizes the
        // shape (a nested call), not a naming convention, unlike Go's
        // `NewXxx`/Rust's `Xxx::new` heuristics elsewhere in this file.
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" || receiver.kind() == "simple_identifier" {
        ReceiverHint::Identifier
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// A `call_expression`'s `value_arguments` unfielded child, each
/// argument's own source text in written order -- mirrors this file's
/// shared [`call_arg_texts`] convention (which needs a *field* name and
/// so cannot be reused directly for Kotlin's unfielded shape).
fn kotlin_value_arguments_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = (0..call_node.child_count())
        .filter_map(|i| call_node.child(i))
        .find(|n| n.kind() == "value_arguments")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..args.child_count() {
        if let Some(child) = args.child(i) {
            if child.is_named() {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// Kotlin's [`Quirks`] row: unfielded function/class bodies,
/// `type_alias`'s `"type"`-named field, unfielded import paths,
/// delegation-specifier INHERITS (mirrors the baseline's
/// `extract_kotlin_bases`), and full `call_expression`/
/// `navigation_expression` claiming (neither has any fields at all).
/// No `route_from_call`: the baseline's own Kotlin route signal
/// (Ktor) is a library-NAME match against a call's resolved import
/// (`service_patterns.c`'s `route_reg_libraries` table,
/// `"ktor.server"`/`"ktor.routing"`), not a callee-string/decorator
/// convention like Go's `net/http/mux`-style or TS's Express/NestJS
/// conventions this file's other `route_from_call`/`route_from_decorator`
/// hooks already implement -- this generic engine has no
/// import-context-aware route mechanism wired for ANY language yet (a
/// G3-scope rich-tier pass per the language-parity plan's own framing),
/// so Kotlin routes are correctly left unwired here rather than
/// approximated with a callee-string heuristic the baseline itself
/// does not use for this language.
pub fn kotlin_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(kotlin_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(kotlin_call_override)),
    }
}

/// Parse Kotlin source through the generic engine. Grammar:
/// `tree-sitter-kotlin-ng` (crates.io), packaging the
/// `tree-sitter-grammars/tree-sitter-kotlin` grammar -- see this
/// crate's `Cargo.toml` for why (ABI-stable via `tree-sitter-language`,
/// unlike the older non-"-ng" `tree-sitter-kotlin` crate).
pub fn parse_kotlin(source: &str) -> ParsedFile {
    let spec = LangSpec::kotlin();
    let quirks = kotlin_quirks();
    let language: tree_sitter::Language = tree_sitter_kotlin_ng::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// Swift's `class_declaration` node covers `class`/`struct`/`enum`/
/// `actor`/`extension` ALL as the one node kind, distinguished only by
/// its own `declaration_kind` field's text -- see [`LangSpec::swift`]'s
/// doc comment for the full grammar-shape finding (this grammar has no
/// `struct_declaration`/`enum_declaration` node kind at all, unlike
/// what the baseline's own `swift_class_types` array names would
/// suggest). Reads that field to recover the more specific
/// [`SymbolKind::Struct`]/[`SymbolKind::Enum`] the same way
/// [`LangSpec::rust`]'s own `struct_item` needs a dedicated
/// classification the generic class-shape fallback's default
/// (`SymbolKind::Class`) cannot express -- an `extension` (no
/// dedicated `SymbolKind` variant exists for it) stays `Class`, same as
/// a plain `class`/`actor`.
fn swift_class_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = child_text(node, "name", src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    let declaration_kind = child_text(node, "declaration_kind", src);
    let kind = match declaration_kind.as_deref() {
        Some("struct") => SymbolKind::Struct,
        Some("enum") => SymbolKind::Enum,
        _ => SymbolKind::Class,
    };
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind,
        line,
    });
    for super_name in swift_inheritance_bases(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: name.clone(),
            super_name,
            line,
        });
    }
    let body_kind = if declaration_kind.as_deref() == Some("enum") {
        "enum_class_body"
    } else {
        "class_body"
    };
    if let Some(body) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == body_kind)
    {
        swift_walk_scoped(body, src, Some(name.as_str()), out);
    }
    true
}

/// `protocol Sub: Base1, Base2` -- Swift's `protocol_declaration` has a
/// real `name`/`body` field pair (unlike `class_declaration`, see
/// [`LangSpec::swift`]'s doc comment), but is still claimed fully here
/// (rather than left to the generic class-shape fallback) purely for
/// symmetry with [`swift_class_like`]'s INHERITS handling -- Swift
/// protocol/superclass heritage (`inheritance_specifier` children) uses
/// the exact same shape for both node kinds, and a protocol's `extends`
/// -equivalent list is semantically Interface-INHERITS the same way
/// [`LangSpec::typescript`]'s own `interface_declaration` arm records
/// it (extends-only, no DEFINES-scoped body walk for member
/// signatures).
fn swift_protocol_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = child_text(node, "name", src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Interface,
        line,
    });
    for super_name in swift_inheritance_bases(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: name.clone(),
            super_name,
            line,
        });
    }
    true
}

/// `class Sub: Base, IFace` / `protocol Sub: Base1, Base2` -- Swift
/// heritage is a list of `inheritance_specifier` unfielded children
/// (no `constructor_invocation`-style wrapping the way Kotlin's
/// `delegation_specifier` needs, per [`kotlin_delegation_bases`] --
/// each `inheritance_specifier` wraps its base type directly).
fn swift_inheritance_bases(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "inheritance_specifier" {
            continue;
        }
        let Some(type_node) = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.is_named())
        else {
            continue;
        };
        if let Ok(text) = type_node.utf8_text(src) {
            if !text.is_empty() {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// Recurse into a Swift class/protocol body under a freshly-scoped
/// `enclosing`, mirroring every other quirk's `*_walk_scoped`/
/// `*_walk_scoped_body` helper in this file -- needed because
/// `swift_class_like`'s body lookup is keyed off a computed
/// `body_kind` (`class_body`/`enum_class_body`), not
/// [`LangSpec::body_field`] (Swift's `class_declaration` DOES have a
/// real `"body"` field per [`LangSpec::swift`]'s own doc comment, so
/// this helper's existence is purely about needing the
/// `declaration_kind`-aware body-kind choice `swift_class_like`
/// already computed, not a second unfielded-child workaround).
fn swift_walk_scoped(body: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::swift();
    let quirks = swift_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, enclosing, FnScope::default());
        }
    }
}

/// `let x = ...` / `var y: Int` -- Swift's `property_declaration` has
/// no `name` field of its own (see [`LangSpec::swift`]'s doc comment);
/// the name lives two levels down, at `pattern.bound_identifier`. Only
/// pushes a DEFINES edge (matching [`LangSpec::swift`]'s `field_types`
/// contract -- this row's generic field-shape branch would otherwise
/// do exactly that, were `child_text` able to read the name directly).
fn swift_property_defines(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) {
    let Some(container) = enclosing else { return };
    let Some(pattern) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "pattern")
    else {
        return;
    };
    let Some(name_node) = pattern.child_by_field_name("bound_identifier") else {
        return;
    };
    let Ok(name) = name_node.utf8_text(src) else {
        return;
    };
    out.defines.push(DefinesRef {
        container_name: container.to_string(),
        member_name: name.to_string(),
        line: node.start_position().row + 1,
    });
}

/// `import Foundation` / `import struct Foo.Bar` -- Swift's
/// `import_declaration` has no fields at all (see [`LangSpec::swift`]'s
/// doc comment); the path is an unfielded `identifier` child
/// (`modifiers`, when present, precedes it for a scoped import like
/// `import struct Foo.Bar` and is skipped).
fn swift_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let path_node = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "identifier")?;
    path_node.utf8_text(src).ok().map(str::to_string)
}

/// Everything Swift's flat [`LangSpec`] arrays cannot express directly
/// (the `class_declaration` struct/enum/class/actor/extension split,
/// `property_declaration`'s two-levels-deep name, unfielded import
/// paths) -- wired as this wave's [`Quirks::on_unmatched_node`] hook.
/// Fully claims [`LangSpec::swift`]'s `class_types` (both
/// `class_declaration` and `protocol_declaration`) the same "everything
/// claimed" posture as [`c_quirk`]/[`cpp_quirk`]/[`kotlin_quirk`], for
/// the reasons each of their own doc comments and
/// [`LangSpec::swift`]'s doc comment record.
fn swift_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" => swift_class_like(node, src, out),
        "protocol_declaration" => swift_protocol_like(node, src, out),
        "property_declaration" => {
            swift_property_defines(node, enclosing, src, out);
            false
        }
        "import_declaration" | "import" => {
            if let Some(path) = swift_import_path(node, src) {
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

/// Swift's `call_expression`/`constructor_expression`/
/// `macro_invocation`/`navigation_expression` all have shapes the
/// generic engine's single-field `call_function_field`/
/// `call_arguments_field` reconstruction cannot express uniformly (see
/// [`LangSpec::swift`]'s doc comment) -- wired as this wave's
/// [`Quirks::call_override`] hook, same "every call shape claimed
/// directly" posture as [`php_call_override`]/[`kotlin_call_override`].
fn swift_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "call_expression" => {
            let Some(callee_node) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.is_named())
            else {
                return false;
            };
            let Ok(callee) = callee_node.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = if callee_node.kind() == "navigation_expression" {
                swift_navigation_receiver(callee_node, src)
            } else {
                (None, None)
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: swift_call_suffix_arg_texts(node, "call_suffix", src),
            });
            true
        }
        "constructor_expression" => {
            let Some(type_node) = node.child_by_field_name("constructed_type") else {
                return false;
            };
            let Ok(callee) = type_node.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: Some(ReceiverHint::NewExpression),
                arg_texts: swift_call_suffix_arg_texts(node, "constructor_suffix", src),
            });
            true
        }
        "macro_invocation" => {
            let Some(name_node) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.kind() == "simple_identifier")
            else {
                return false;
            };
            let Ok(callee) = name_node.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: swift_call_suffix_arg_texts(node, "call_suffix", src),
            });
            true
        }
        "navigation_expression" => {
            let Ok(callee) = node.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = swift_navigation_receiver(node, src);
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: Vec::new(),
            });
            true
        }
        _ => false,
    }
}

/// `a.b`'s receiver (`a`, the `target` field) plus a syntactic hint --
/// Swift's `navigation_expression` DOES have real fields
/// (`target`/`suffix`, see [`LangSpec::swift`]'s doc comment), unlike
/// Kotlin's identically-named but unfielded node kind
/// ([`kotlin_navigation_receiver`]).
fn swift_navigation_receiver(
    navigation_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    let Some(receiver) = navigation_node.child_by_field_name("target") else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if matches!(text, "self" | "Self") {
        ReceiverHint::SelfOrThis
    } else if matches!(
        receiver.kind(),
        "call_expression" | "constructor_expression"
    ) {
        ReceiverHint::NewExpression
    } else if matches!(receiver.kind(), "identifier" | "simple_identifier") {
        ReceiverHint::Identifier
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// A `call_expression`/`constructor_expression`/`macro_invocation`'s
/// own `suffix_kind`-named unfielded child (`call_suffix`/
/// `constructor_suffix`), which itself wraps an unfielded
/// `value_arguments` child -- each argument's own source text in
/// written order, mirroring this file's shared [`call_arg_texts`]
/// convention (field-name-keyed, so unusable directly for Swift's
/// unfielded shape).
fn swift_call_suffix_arg_texts(call_node: Node<'_>, suffix_kind: &str, src: &[u8]) -> Vec<String> {
    let Some(suffix) = (0..call_node.child_count())
        .filter_map(|i| call_node.child(i))
        .find(|n| n.kind() == suffix_kind)
    else {
        return Vec::new();
    };
    let Some(args) = (0..suffix.child_count())
        .filter_map(|i| suffix.child(i))
        .find(|n| n.kind() == "value_arguments")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..args.child_count() {
        if let Some(child) = args.child(i) {
            if child.is_named() {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// Swift's [`Quirks`] row: the `class_declaration` struct/enum/class/
/// actor/extension split (via `declaration_kind`) with INHERITS,
/// protocol extends-only INHERITS, `property_declaration`'s
/// two-levels-deep name DEFINES, unfielded import paths, and full
/// `call_expression`/`constructor_expression`/`macro_invocation`/
/// `navigation_expression` claiming. No `route_from_call`: Swift's
/// real-world route frameworks (Vapor, ...) are not in the baseline's
/// own `service_patterns.c` route-library table at all (only Kotlin's
/// Ktor is, among this wave's three languages) -- the baseline itself
/// extracts no Swift routes, so this generic-engine row correctly
/// matches that by leaving `route_from_call` unwired rather than
/// inventing a heuristic the baseline has no equivalent for.
pub fn swift_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(swift_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(swift_call_override)),
    }
}

/// Parse Swift source through the generic engine. Grammar:
/// `tree-sitter-swift` (crates.io, `alex-pinkus/tree-sitter-swift`
/// upstream) -- the same lineage the baseline's own vendored
/// `internal/cbm/vendored/grammars/swift/` copy carries per its
/// `LICENSE` copyright line.
pub fn parse_swift(source: &str) -> ParsedFile {
    let spec = LangSpec::swift();
    let quirks = swift_quirks();
    let language: tree_sitter::Language = tree_sitter_swift::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// `import "x.sol";` / `import "x.sol" as X;` / `import {A, B} from
/// "x.sol";` -- the `source` field holds the quoted path in every shape
/// (`node-types.json`'s `import_directive` fields).
fn solidity_import_directive_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let source = node.child_by_field_name("source")?;
    let raw = source.utf8_text(src).ok()?;
    Some(raw.trim_matches('"').to_string())
}

/// `using SafeMath for uint256;` -- the library name being brought into
/// scope (`SafeMath`) is NOT `using_directive`'s own `source` field (a
/// real parse tree dump shows `source` holds `uint256`, the type being
/// extended -- the `for` clause's target, not the library) -- it is
/// instead a bare (non-field) `type_alias`-kind child (confusingly the
/// same node-kind name the initial `node-types.json`-only reading of
/// this grammar mistook for a top-level type-alias declaration; see
/// [`LangSpec::solidity`]'s own doc comment for the full correction).
/// `using SafeMath for uint256 global;`'s bare `using_alias` shape
/// (`using {a, b} for uint256;`) is not handled here -- best-effort,
/// same posture as every other import-path helper in this file.
fn solidity_using_directive_library_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.kind() == "type_alias" {
            for j in 0..child.child_count() {
                let grandchild = child.child(j)?;
                if grandchild.kind() == "identifier" {
                    return grandchild.utf8_text(src).ok().map(str::to_string);
                }
            }
        }
    }
    None
}

/// Everything [`LangSpec::solidity`]'s flat arrays cannot express:
/// `import_directive`/`using_directive` IMPORTS-path extraction (the
/// generic engine's own default import handling is recursion-only, by
/// design -- see [`walk`]'s `import_types` branch doc comment -- so a
/// language wanting real IMPORTS edges out of it supplies exactly this
/// kind of quirk, same as every other language row that has one).
/// Wired as this row's [`Quirks::on_unmatched_node`] hook.
fn solidity_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "import_directive" => {
            if let Some(path) = solidity_import_directive_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "using_directive" => {
            if let Some(path) = solidity_using_directive_library_name(node, src) {
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

/// A `call_expression`'s `function` field is always wrapped in one
/// `expression` node (verified via a real parse tree dump), so this
/// unwraps exactly that one layer before matching on the underlying
/// node kind -- `identifier` (`helper()`), `member_expression`
/// (`h.register()`, with an `object` field receiver), or
/// `new_expression` (`new Helper(...)`, whose own text already includes
/// the `new` keyword, matched here as a plain unqualified callee since
/// a constructor call has no method-call-shaped receiver of its own).
fn solidity_call_callee_and_receiver(
    call_node: Node<'_>,
    src: &[u8],
) -> Option<(String, Option<String>, Option<ReceiverHint>)> {
    let wrapped = call_node.child_by_field_name("function")?;
    let unwrapped = if wrapped.kind() == "expression" && wrapped.named_child_count() == 1 {
        wrapped.named_child(0)?
    } else {
        wrapped
    };
    let callee = unwrapped.utf8_text(src).ok()?.to_string();
    if unwrapped.kind() != "member_expression" {
        return Some((callee, None, None));
    }
    let Some(receiver) = unwrapped.child_by_field_name("object") else {
        return Some((callee, None, None));
    };
    let Ok(receiver_text) = receiver.utf8_text(src) else {
        return Some((callee, None, None));
    };
    let hint = if receiver_text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "new_expression" {
        ReceiverHint::NewExpression
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else {
        ReceiverHint::Other
    };
    Some((callee, Some(receiver_text.to_string()), Some(hint)))
}

/// `call_expression`'s argument list is exposed as bare `call_argument`
/// children in `node-types.json`, not a named field.
fn solidity_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..call_node.child_count() {
        let Some(child) = call_node.child(i) else {
            continue;
        };
        if child.kind() != "call_argument" {
            continue;
        }
        if let Ok(text) = child.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// Full `call_expression` reconstruction (callee, receiver, receiver
/// hint, argument texts) -- wired as [`Quirks::call_override`] since
/// Solidity's `function` field's extra `expression` wrapper layer and
/// unnamed `call_argument` children cannot be expressed by the generic
/// engine's own single-field callee/single-field-arguments assumptions
/// (see [`LangSpec::solidity`]'s own doc comment for the wrapper
/// finding).
fn solidity_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let Some((callee, receiver_text, receiver_hint)) = solidity_call_callee_and_receiver(node, src)
    else {
        return false;
    };
    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text,
        receiver_hint,
        arg_texts: solidity_call_arg_texts(node, src),
    });
    true
}

/// Solidity's [`Quirks`] row: `import_directive`/`using_directive`
/// IMPORTS-path extraction, and full `call_expression` reconstruction
/// (the `function` field's extra `expression` wrapper layer, plus
/// unnamed `call_argument` children, neither of which the generic
/// engine's own flat-field call handling can read directly). Every
/// other construct (contracts/interfaces/libraries/structs/enums,
/// functions/modifiers/constructors/fallback-receive, type aliases via
/// `user_defined_type_definition`'s own `name` field, state-variable/
/// struct-member fields, branches) is fully covered by
/// [`LangSpec::solidity`]'s own flat arrays with no quirk needed.
pub fn solidity_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(solidity_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(solidity_call_override)),
    }
}

/// Parse Solidity source through the generic engine. Solidity has no
/// pre-existing bespoke `languages/*.rs` extractor (this crate's first
/// language onboarded directly onto the G1/G1b generic engine with no
/// prior behavior to reproduce) -- correctness here rests entirely on
/// direct verification against the `tree-sitter-solidity` crate's own
/// `node-types.json` plus a real parse tree dump (see
/// [`LangSpec::solidity`]'s doc comment) and the C baseline's
/// `solidity_*_types` arrays
/// (`codebase-memory-mcp/internal/cbm/lang_specs.c:990-1012`), not
/// byte-for-byte comparison against an oracle.
pub fn parse_solidity(source: &str) -> ParsedFile {
    let spec = LangSpec::solidity();
    let quirks = solidity_quirks();
    let language: tree_sitter::Language = tree_sitter_solidity::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// GDScript's `extends_statement`/`class_name_statement` base-path text
/// (a bare `string` or `type` child -- `node-types.json` shows
/// `extends_statement` has no fields at all).
fn gdscript_extends_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(child.kind(), "string" | "type" | "identifier") {
            let text = child.utf8_text(src).ok()?;
            return Some(text.trim_matches('"').to_string());
        }
    }
    None
}

/// `class_name_statement`'s own `extends` field (a nested
/// `extends_statement`, per `node-types.json`) -- distinct from the
/// bare `extends_statement` node kind itself, which
/// [`gdscript_extends_path`] handles directly.
fn gdscript_class_name_extends_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let extends = node.child_by_field_name("extends")?;
    gdscript_extends_path(extends, src)
}

/// `@onready` / `@export(...)` -- the annotation name is a bare
/// `identifier` child (`node-types.json`'s `annotation` has no `name`
/// field), same "callee-shaped node, no field for the identifying part"
/// situation Rust's `attribute_item`/TS's `decorator` handling already
/// mirror via their own dedicated name helpers.
fn gdscript_annotation_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.kind() == "identifier" {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// Everything [`LangSpec::gdscript`]'s flat arrays cannot express:
/// `extends_statement` IMPORTS-path extraction (bare children, no
/// field); `class_name_statement`'s OWN dual role as both a
/// [`SymbolKind::Class`] symbol (`LangSpec::gdscript`'s `class_types`)
/// AND an IMPORTS-path source for its own `extends` field
/// (`LangSpec::gdscript`'s `import_types`) -- [`walk`] checks
/// `import_types` before `class_types` with an early `return`, so this
/// one node kind's Class symbol must be emitted here rather than via
/// the generic class-shape fallback that array would otherwise trigger
/// (see [`LangSpec::gdscript`]'s own doc comment); and `annotation`
/// DECORATES (bare `identifier` child, no field) -- wired as this row's
/// [`Quirks::on_unmatched_node`] hook. `annotation` additionally still
/// needs generic recursion into its own `arguments` field afterward
/// (an annotation's argument expressions can themselves contain calls,
/// e.g. `@export(some_default())`), so this arm returns `false` rather
/// than claiming the node fully.
fn gdscript_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "extends_statement" => {
            if let Some(path) = gdscript_extends_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "class_name_statement" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Class,
                    line: node.start_position().row + 1,
                });
            }
            if let Some(path) = gdscript_class_name_extends_path(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        "annotation" => {
            if let Some(name) = gdscript_annotation_name(node, src) {
                // An `annotation` decorates whatever sibling statement
                // follows it, not `enclosing` itself (`enclosing` is the
                // lexically containing class, same as every other
                // language's DECORATES target is the specific member,
                // not the class it lives in) -- best-effort target is
                // the next named sibling's own name field, mirroring
                // TS's `ts_preceding_decorators`'s "decorator precedes
                // its target" convention but walked forward instead of
                // backward, since GDScript's grammar attaches
                // `annotations` as a *preceding* child of the annotated
                // statement rather than the reverse.
                if let Some(target) = gdscript_annotation_target_name(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
                        target_name: target,
                        decorator_name: name,
                        line: node.start_position().row + 1,
                    });
                }
            }
            false
        }
        _ => false,
    }
}

/// The name of the declaration an `annotation` node immediately
/// precedes -- an `annotation` is itself wrapped in an `annotations`
/// aux node that is a child of the function/variable/signal node it
/// decorates (per `node-types.json`'s `function_definition`/
/// `variable_statement`/etc. `children` list), so the target is this
/// node's own parent's parent, read back for a `name` field.
fn gdscript_annotation_target_name(annotation_node: Node<'_>, src: &[u8]) -> Option<String> {
    let annotations_wrapper = annotation_node.parent()?;
    let target = annotations_wrapper.parent()?;
    child_text(target, "name", src)
}

/// GDScript's three call-shaped node kinds (`call`, `attribute_call`,
/// `base_call`) each expose their callee as a bare, non-field child
/// (`node-types.json` confirms none of the three has a field for it) --
/// full reconstruction wired as [`Quirks::call_override`], same
/// rationale as [`java_call_override`]/`php_call_override`.
fn gdscript_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let callee = match node.kind() {
        "call" => gdscript_call_callee(node, src),
        "attribute_call" => gdscript_attribute_call_callee(node, src),
        "base_call" => gdscript_base_call_callee(node, src),
        _ => return false,
    };
    let Some(callee) = callee else {
        return false;
    };
    let (receiver_text, receiver_hint) = gdscript_receiver_of_call(node, src);
    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text,
        receiver_hint,
        arg_texts: gdscript_call_arg_texts(node, src),
    });
    true
}

/// `foo(...)` / `SomeClass.new(...)` -- `call`'s callee is its one
/// non-`arguments` named child (`node-types.json`: `call`'s `children`
/// list is exactly one `_primary_expression`).
fn gdscript_call_callee(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.kind() == "arguments" {
            continue;
        }
        if child.is_named() {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `.foo(...)` (no receiver at all -- the "call the base
/// implementation" idiom, GDScript's `base_call` grammar rule: `seq(".",
/// $.identifier, field("arguments", $.arguments))`, verified via a real
/// parse tree dump) -- the called method's own name is its one
/// `identifier` child.
fn gdscript_base_call_callee(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.kind() == "identifier" {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `x.foo(...)` -- `attribute_call`'s callee name is its one
/// `identifier` child; the receiver (`x`) lives on the *parent*
/// `attribute`-shaped node one level up in this grammar (mirrors how
/// Go's `selector_expression`/TS's `member_expression` each nest a
/// receiver field one level above their own call-shaped node, except
/// GDScript's `attribute_call` has no receiver of its own at all --
/// only the parent attribute-access node does), so this reconstructs
/// just the bare method name here; [`gdscript_receiver_of_call`]
/// recovers the qualifying receiver text separately for
/// [`CallRef::receiver_text`].
fn gdscript_attribute_call_callee(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.kind() == "identifier" {
            let name = child.utf8_text(src).ok()?;
            if let Some(receiver) = gdscript_attribute_call_receiver_text(node, src) {
                return Some(format!("{receiver}.{name}"));
            }
            return Some(name.to_string());
        }
    }
    None
}

/// The receiver expression text for an `attribute_call`, if this call
/// node is itself a child of an enclosing `attribute` node one level up
/// -- GDScript's grammar models `a.b.foo()` as one `attribute` node
/// whose children are the receiver path segments (`a`, `b`) followed by
/// this `attribute_call` node, with no field of its own identifying
/// where the receiver ends and the call begins (`node-types.json`'s
/// `attribute` has `"fields": {}`). Sliced by byte range (this node's
/// own start byte, exclusive) rather than concatenating sibling text
/// with inserted `.` separators, so the original source's own
/// punctuation/whitespace between segments is preserved byte-for-byte
/// -- correct for any number of leading segments, unlike returning just
/// the first or last named sibling would be.
fn gdscript_attribute_call_receiver_text(node: Node<'_>, src: &[u8]) -> Option<String> {
    let parent = node.parent()?;
    if parent.kind() != "attribute" {
        return None;
    }
    let start = parent.start_byte();
    let end = node.start_byte();
    if end <= start {
        return None;
    }
    let raw = std::str::from_utf8(&src[start..end]).ok()?;
    let trimmed = raw.trim_end_matches('.').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn gdscript_receiver_of_call(node: Node<'_>, src: &[u8]) -> (Option<String>, Option<ReceiverHint>) {
    if node.kind() == "base_call" {
        // `.foo(...)` -- genuinely no receiver expression is written in
        // source at all (unlike `super.foo()`, which is an ordinary
        // `attribute_call` with a real `super` identifier receiver, see
        // [`LangSpec::gdscript`]'s own doc comment) -- `None`/`None`
        // here is the same "nothing was written" representation
        // [`CallRef::receiver_text`]'s own doc comment already uses for
        // a plain unqualified call, rather than inventing receiver text
        // the source never actually contains.
        return (None, None);
    }
    if node.kind() != "attribute_call" {
        return (None, None);
    }
    let Some(text) = gdscript_attribute_call_receiver_text(node, src) else {
        return (None, None);
    };
    // `super.foo()` is a perfectly ordinary `attribute_call` with
    // `super` as its identifier receiver (see this function's own
    // `base_call` doc comment above for the contrast with the real
    // bare-dot `base_call` syntax) -- `super`, like `self`, is treated
    // as a self-referential receiver rather than falling through to the
    // ordinary-identifier/PascalCase-constructor heuristics below it.
    let hint = if text == "self" || text == "super" {
        ReceiverHint::SelfOrThis
    } else if text.chars().next().is_some_and(|c| c.is_uppercase()) {
        ReceiverHint::NewExpression
    } else {
        ReceiverHint::Identifier
    };
    (Some(text), Some(hint))
}

fn gdscript_call_arg_texts(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = node.child_by_field_name("arguments") else {
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

/// GDScript's [`Quirks`] row: `extends_statement`/`class_name_statement`
/// IMPORTS-path extraction, `annotation` DECORATES, and full call-callee
/// reconstruction for all three call-shaped node kinds (`call`/
/// `attribute_call`/`base_call`), none of which expose their callee
/// through a named field the generic engine's flat `call_function_field`
/// could read.
pub fn gdscript_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(gdscript_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(gdscript_call_override)),
    }
}

/// Parse GDScript source through the generic engine. Same "no
/// pre-existing bespoke extractor, correctness verified directly
/// against the grammar crate's own `node-types.json`" posture as
/// [`parse_solidity`] -- see [`LangSpec::gdscript`]'s doc comment.
pub fn parse_gdscript(source: &str) -> ParsedFile {
    let spec = LangSpec::gdscript();
    let quirks = gdscript_quirks();
    let language: tree_sitter::Language = tree_sitter_gdscript::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// `class Sub extends Base` -- Dart's `superclass` field wraps a
/// `superclass` node whose own `type` field wraps the `type` node
/// holding the base-class text directly (verified against
/// `tree-sitter-dart`'s `node-types.json`: `class_declaration`'s
/// `superclass` field -> `superclass` node -> `type` field -> `type`
/// node).
fn dart_superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    let superclass = class_node.child_by_field_name("superclass")?;
    let type_node = superclass.child_by_field_name("type")?;
    type_node.utf8_text(src).ok().map(str::to_string)
}

/// `class Sub implements I1, I2` -- Dart's `interfaces` field wraps an
/// `interfaces` node whose named children are `type` nodes directly (no
/// intermediate wrapper the way [`java_super_interfaces`]'s `type_list`
/// needs).
fn dart_interfaces(class_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(interfaces) = class_node.child_by_field_name("interfaces") else {
        return out;
    };
    for i in 0..interfaces.child_count() {
        if let Some(child) = interfaces.child(i) {
            if child.is_named() {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// A `function_declaration`/`method_declaration` node's own name --
/// both wrap their name-bearing node one level deeper under a
/// `signature` field (`function_signature` directly for
/// `function_declaration`; `method_signature` -> nested
/// `function_signature` for `method_declaration`, since
/// `method_signature` itself carries no fields of its own at all --
/// `node-types.json`: `"fields": {}` -- mirroring the baseline's own
/// dedicated `resolve_dart_method_name` special case one level further
/// in).
fn dart_declaration_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let signature = node.child_by_field_name("signature")?;
    match signature.kind() {
        "function_signature" => child_text(signature, "name", src),
        "method_signature" => {
            for i in 0..signature.child_count() {
                let child = signature.child(i)?;
                if child.kind() == "function_signature" {
                    return child_text(child, "name", src);
                }
            }
            None
        }
        _ => None,
    }
}

/// A Dart `import`/`export` directive's URI string, quote-stripped --
/// the `uri` field wraps either a `configurable_uri` node (conditional
/// imports, `import 'x.dart' if (dart.library.io) 'y.dart';` -- nests a
/// further `uri` child) or a bare `uri` node directly; both bottom out
/// at a `string_literal`, read here by its own outer text rather than
/// descending into its `string_literal_single_quotes`/
/// `string_literal_double_quotes` children, since the surrounding quote
/// characters are trimmed the same way regardless of which quote style
/// was written.
fn dart_import_specification_uri(spec: Node<'_>, src: &[u8]) -> Option<String> {
    let uri_wrapper = spec.child_by_field_name("uri")?;
    let uri_node = if uri_wrapper.kind() == "configurable_uri" {
        (0..uri_wrapper.child_count())
            .filter_map(|i| uri_wrapper.child(i))
            .find(|n| n.kind() == "uri")?
    } else {
        uri_wrapper
    };
    let raw = uri_node.utf8_text(src).ok()?;
    Some(raw.trim_matches(|c| c == '\'' || c == '"').to_string())
}

/// Every URI declared by one `import_or_export` node -- its own
/// children are exactly one of `library_import`/`library_export`
/// (`node-types.json`: `"multiple": false, "required": true`), each of
/// which wraps one `import_specification` (import) or is itself the URI
/// holder (export, whose own `uri` field is read the same way
/// [`dart_import_specification_uri`] reads an `import_specification`'s).
fn dart_import_paths(import_or_export: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..import_or_export.child_count() {
        let Some(wrapper) = import_or_export.child(i) else {
            continue;
        };
        match wrapper.kind() {
            "library_import" => {
                for j in 0..wrapper.child_count() {
                    if let Some(spec) = wrapper.child(j) {
                        if spec.kind() == "import_specification" {
                            if let Some(path) = dart_import_specification_uri(spec, src) {
                                out.push(path);
                            }
                        }
                    }
                }
            }
            "library_export" => {
                if let Some(path) = dart_import_specification_uri(wrapper, src) {
                    out.push(path);
                }
            }
            _ => {}
        }
    }
    out
}

/// Everything Dart's flat `LangSpec` arrays cannot express: class
/// heritage (INHERITS/IMPLEMENTS) + DEFINES-scoped body walk,
/// `function_declaration`/`method_declaration`'s name resolution +
/// correctly `fn_scope`-scoped body walk (see [`LangSpec::dart`]'s own
/// doc comment for why these two -- not the inner `function_signature`/
/// `method_signature` nodes baseline's own arrays target -- must be the
/// ones this engine's func/method handling is keyed on), and grouped
/// import/export URI extraction -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. `enum_declaration`/`type_alias`/
/// `mixin_declaration` are left to the generic class-shape fallback
/// (all three carry a direct `name` field per `node-types.json`,
/// confirmed by a real parse), except `type_alias` specifically, which
/// has NO `name` field at all (its name is a bare `type_identifier`
/// child, not a fielded one) and is claimed here too.
fn dart_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(super_name) = dart_superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                for interface_name in dart_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                dart_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "function_declaration" | "method_declaration" => {
            // `LangSpec::dart()`'s own `func_types`/`method_types` point
            // at these two node kinds precisely so this quirk -- not the
            // generic engine's own func/method branch -- is what runs:
            // the name lives one (`function_declaration`) or two
            // (`method_declaration`) levels deeper under this node's own
            // `signature` field, but the `body` field (needed to set
            // `fn_scope` correctly for every call inside it) lives
            // directly on THIS node -- see [`dart_declaration_name`]'s
            // own doc comment for the full field-split rationale.
            let Some(name) = dart_declaration_name(node, src) else {
                return true;
            };
            let line = node.start_position().row + 1;
            let kind = if node.kind() == "method_declaration" {
                SymbolKind::Method
            } else {
                SymbolKind::Function
            };
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind,
                line,
            });
            if let Some(container) = _enclosing {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name.clone(),
                    line,
                });
            }
            if let Some(body) = node.child_by_field_name("body") {
                dart_walk_function_body(body, src, &name, line, out);
            }
            true
        }
        "type_alias" => {
            // No `name` field at all (`node-types.json`: `"fields":
            // {}`) -- name is the bare `type_identifier` child.
            if let Some(type_identifier) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.kind() == "type_identifier")
            {
                if let Ok(text) = type_identifier.utf8_text(src) {
                    out.symbols.push(SymbolRef {
                        name: text.to_string(),
                        kind: SymbolKind::TypeAlias,
                        line: node.start_position().row + 1,
                    });
                }
            }
            true
        }
        "import_or_export" => {
            for path in dart_import_paths(node, src) {
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

/// A `function_declaration`/`method_declaration`'s own `body` field:
/// every call inside it needs `fn_scope` set to this function/method's
/// own name/line -- mirrors [`c_walk_function_body`]'s identical
/// rationale/shape (this quirk's own signature carries no `enclosing`
/// either, but `enclosing` is irrelevant here regardless: a nested
/// declaration/class INSIDE a function body would need `enclosing =
/// None` at its own point of definition the same way every other
/// language's function-body walk already passes, since Dart has no
/// syntax for defining a class inside a function body that would ever
/// need a non-`None` `enclosing` here).
fn dart_walk_function_body(
    body: Node<'_>,
    src: &[u8],
    name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::dart();
    let quirks = dart_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name: Some(name),
        line: Some(line),
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, None, fn_scope);
        }
    }
}

/// `class_declaration`'s DEFINES-scoped body walk -- not one of
/// `LangSpec::dart()`'s own class/interface/enum/alias arrays' generic
/// fallback (that fallback never runs for `class_declaration`, since
/// [`dart_quirk`] claims it first) -- mirrors every other language's
/// identical `*_walk_scoped_body` helper (see [`ts_walk_scoped_body`]/
/// [`py_walk_scoped_body`]/[`java_walk_scoped_body`]).
fn dart_walk_scoped_body(
    class_node: Node<'_>,
    src: &[u8],
    name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::dart();
    let quirks = dart_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// Dart's [`Quirks`] row: class heritage (INHERITS/IMPLEMENTS) +
/// DEFINES-scoped body walk, `function_declaration`/
/// `method_declaration`'s nested-signature name resolution +
/// correctly `fn_scope`-scoped body walk, `type_alias`'s fieldless
/// name, and grouped import/export URI extraction.
pub fn dart_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(dart_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse Dart source through the generic engine. No pre-existing
/// bespoke `languages::dart` extractor to prove zero-regression
/// against (Dart has never had one in this crate) -- correctness is
/// instead verified directly against the `tree-sitter-dart` crate's
/// own `node-types.json` plus real parse trees (see [`LangSpec::dart`]'s
/// doc comment and `tests/unit_languages_dart.rs`).
pub fn parse_dart(source: &str) -> ParsedFile {
    let spec = LangSpec::dart();
    let quirks = dart_quirks();
    let language: tree_sitter::Language = tree_sitter_dart::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// `class Sub extends Base with Trait1 with Trait2` -- Scala's `extend`
/// field wraps an `extends_clause` whose repeated `type` field holds
/// every base/mixin in written order, interspersed with the literal
/// `with` keyword (itself oddly tagged with the same `"type"` field
/// name in `node-types.json`, but `is_named() == false`, filtered out
/// here the same way [`ts_collect_heritage_clause`]/every other
/// heritage helper in this file skips punctuation/keyword children).
/// Every base/mixin is recorded as an INHERITS edge (not split into a
/// single superclass plus separate IMPLEMENTS the way Java's strict
/// class-vs-interface model works): `extends Base with T1 with T2` is a
/// genuinely linear trait-mixin chain in Scala, not a distinct
/// one-superclass-plus-many-interfaces split, so there is no principled
/// way to single out exactly one of these names as "the" superclass.
fn scala_extends_bases(class_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(extends_clause) = class_node.child_by_field_name("extend") else {
        return out;
    };
    for i in 0..extends_clause.child_count() {
        if let Some(child) = extends_clause.child(i) {
            // `arguments` (constructor call args on the superclass,
            // `extends Base(1, 2)`) is a named node under this same
            // `"type"` field slot too -- not a base/mixin name, skipped
            // alongside the unnamed `with` keyword.
            if child.is_named() && child.kind() != "arguments" {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// A Scala `import_declaration`'s dotted path -- its own `path` field is
/// REPEATED (one `identifier`/`.`/`operator_identifier` child per
/// segment, `node-types.json`: `"multiple": true`), so
/// [`child_by_field_name`](Node::child_by_field_name) (which only ever
/// returns the first match) cannot read it directly; this iterates every
/// child instead, keeping only the ones actually tagged with the
/// `"path"` field name, and joins the named segments with `.` (the
/// unnamed `.` token children are skipped, since the join already
/// supplies the separator explicitly -- reusing the literal tokens
/// verbatim would double them up). A trailing `{Foo, Bar}`
/// `namespace_selectors`/wildcard `namespace_wildcard` is deliberately
/// NOT expanded into one import per selector (same "quoted-string
/// import-path extraction, not full destructuring" posture
/// [`ts_quirk`]'s `import_statement` arm already takes for a JS/TS named
/// import list) -- the base dotted path alone is still a correct,
/// useful IMPORTS edge.
fn scala_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut parts = Vec::new();
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if node.field_name_for_child(i as u32) == Some("path") && child.is_named() {
            if let Ok(text) = child.utf8_text(src) {
                parts.push(text.to_string());
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

/// Everything Scala's flat `LangSpec` arrays cannot express: class/trait
/// heritage (`extends ... with ...` as INHERITS) + DEFINES-scoped body
/// walk, and repeated-field import-path reconstruction -- wired as this
/// wave's [`Quirks::on_unmatched_node`] hook. `object_definition`/
/// `trait_definition`/`enum_definition`/`type_definition` are left to
/// the generic class-shape fallback entirely (every one carries its own
/// direct `name` field per `node-types.json`, confirmed by a real parse)
/// except for heritage, which only `class_definition`/`trait_definition`
/// (both carry an `extend` field) can even have -- `object_definition`
/// (a companion/singleton object) can ALSO carry an `extend` field
/// (`object Foo extends Base`), so it is claimed here too for the same
/// heritage reason, not because its own name resolution needs it.
fn scala_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_definition" | "trait_definition" | "object_definition" => {
            let Some(name) = child_text(node, "name", src) else {
                return false;
            };
            let line = node.start_position().row + 1;
            let kind = if node.kind() == "trait_definition" {
                SymbolKind::Interface
            } else {
                SymbolKind::Class
            };
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind,
                line,
            });
            for base in scala_extends_bases(node, src) {
                out.inherits.push(InheritsRef {
                    sub_name: name.clone(),
                    super_name: base,
                    line,
                });
            }
            scala_walk_scoped_body(node, src, Some(name.as_str()), out);
            true
        }
        "import_declaration" => {
            if let Some(path) = scala_import_path(node, src) {
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

/// `class_definition`/`trait_definition`/`object_definition`'s
/// DEFINES-scoped body walk -- not one of `LangSpec::scala()`'s own
/// class/interface/enum/alias arrays' generic fallback (that fallback
/// never runs for these three kinds, since [`scala_quirk`] claims them
/// first) -- mirrors every other language's identical
/// `*_walk_scoped_body` helper.
fn scala_walk_scoped_body(
    class_node: Node<'_>,
    src: &[u8],
    name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::scala();
    let quirks = scala_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// Scala's [`Quirks`] row: class/trait/object heritage
/// (`extends ... with ...` as INHERITS) + DEFINES-scoped body walk, and
/// repeated-field import-path reconstruction.
pub fn scala_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(scala_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse Scala source through the generic engine. No pre-existing
/// bespoke `languages::scala` extractor to prove zero-regression
/// against (Scala has never had one in this crate) -- correctness is
/// instead verified directly against the `tree-sitter-scala` crate's
/// own `node-types.json` plus real parse trees (see
/// [`LangSpec::scala`]'s doc comment and
/// `tests/unit_languages_scala.rs`).
pub fn parse_scala(source: &str) -> ParsedFile {
    let spec = LangSpec::scala();
    let quirks = scala_quirks();
    let language: tree_sitter::Language = tree_sitter_scala::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// `class Sub extends Base` -- Groovy's `superclass` field wraps a
/// `superclass` node whose single named child is the base-class type
/// directly (mirrors [`java_superclass_name`] byte-for-byte; this
/// grammar's class-declaration shape is Java's almost verbatim).
fn groovy_superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    let superclass = class_node.child_by_field_name("superclass")?;
    let type_node = (0..superclass.child_count())
        .filter_map(|i| superclass.child(i))
        .find(|n| n.is_named())?;
    type_node.utf8_text(src).ok().map(str::to_string)
}

/// `class C implements I1, I2` -- Groovy's `interfaces` field wraps a
/// `super_interfaces` node whose own child is a `type_list` holding the
/// interface names (mirrors [`java_super_interfaces`] byte-for-byte,
/// modulo the field's own node-kind name: Java's is also called
/// `interfaces` -> a bare `type_list` wrapper with no intermediate
/// node, whereas this grammar names the wrapper `super_interfaces`).
fn groovy_super_interfaces(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(interfaces) = node.child_by_field_name("interfaces") else {
        return out;
    };
    let Some(type_list) = (0..interfaces.child_count())
        .filter_map(|i| interfaces.child(i))
        .find(|n| n.kind() == "type_list")
    else {
        return out;
    };
    for i in 0..type_list.child_count() {
        if let Some(child) = type_list.child(i) {
            if child.is_named() {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
        }
    }
    out
}

/// `package a.b.c` -- Groovy's `package_declaration` has no `name`
/// field of its own; its single named child is a `scoped_identifier`
/// (multi-segment) or bare `identifier` (single-segment) whose own text
/// is already the fully dotted path (mirrors [`java_package_name`]
/// byte-for-byte).
fn groovy_package_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(child.kind(), "scoped_identifier" | "identifier") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `import a.b.C` -- same shape/rationale as [`groovy_package_name`]
/// (mirrors [`java_import_path`]'s dotted-name half; Groovy's
/// `import_declaration` has no `static`/wildcard-suffix complexity of
/// its own beyond an `asterisk` child for `import a.b.*`, which this
/// helper does not special-case since a plain `scoped_identifier`/
/// `identifier` match already covers the two real-world common cases
/// tested).
fn groovy_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if matches!(child.kind(), "scoped_identifier" | "identifier") {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// `obj.method(...)` / `method(...)` -- mirrors
/// [`java_method_invocation_callee`] byte-for-byte (identical
/// `name`/`object` field split).
fn groovy_method_invocation_callee(node: Node<'_>, src: &[u8]) -> String {
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

/// For a `method_invocation` with an `object` field, the receiver's own
/// text plus a cheap syntactic hint -- mirrors [`java_receiver_of_call`]
/// closely, adjusted for this grammar's own receiver node kinds
/// (`object_creation_expression` for `new Foo()`, confirmed present in
/// `node-types.json` unlike Java's differently-named equivalent).
fn groovy_receiver_of_call(
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
        "string_literal" | "decimal_integer_literal" | "true" | "false"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Each argument expression's own source text, in written order --
/// mirrors [`java_call_arg_texts`]/the generic engine's own
/// `call_arg_texts` byte-for-byte, duplicated here rather than reused
/// directly because [`groovy_call_override`] needs it under this row's
/// own `"arguments"`-field name (`method_invocation`'s field, verified
/// present in `node-types.json`) at a call site that does not have
/// convenient access to `LangSpec::call_arguments_field` (the free
/// function in this module is private to the generic engine's own
/// call-types branch, not exported for quirks to call directly).
fn groovy_call_arg_texts(invocation_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = invocation_node.child_by_field_name("arguments") else {
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

/// Full `method_invocation` callee reconstruction (`object.name` when a
/// receiver is present, bare `name` otherwise) -- wired as
/// [`Quirks::call_override`] since Groovy's grammar splits the receiver
/// and method name into two separate fields rather than one field
/// holding the whole written callee expression (mirrors
/// [`java_call_override`] byte-for-byte). `juxt_function_call` (the
/// parenthesis-less call idiom, `println "x"`) is NOT claimed here --
/// its own `name`/`args` field shape already matches the generic call
/// branch's single-field expectation closely enough (`name` field holds
/// the callee) that [`LangSpec::groovy`]'s `call_function_field`/
/// `call_arguments_field` would need to differ per call-shaped kind
/// either way, so both are routed through this same override for a
/// single, consistent call-handling code path.
fn groovy_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "method_invocation" => {
            let callee = groovy_method_invocation_callee(node, src);
            let (receiver_text, receiver_hint) = groovy_receiver_of_call(node, src);
            out.calls.push(CallRef {
                callee,
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: groovy_call_arg_texts(node, src),
            });
            true
        }
        "juxt_function_call" => {
            let Some(name_node) = node.child_by_field_name("name") else {
                return false;
            };
            let Ok(callee) = name_node.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: Vec::new(),
            });
            true
        }
        _ => false,
    }
}

/// Everything Groovy's flat `LangSpec` arrays cannot express: dotted
/// package-name Module symbol, class heritage (INHERITS/IMPLEMENTS) +
/// DEFINES-scoped body walk, and dotted/wildcard import-path
/// reconstruction -- wired as this wave's [`Quirks::on_unmatched_node`]
/// hook. `method_declaration` is left to the generic func/method
/// branch entirely (it carries its own direct `name` field, confirmed
/// present in `node-types.json` and by a real parse), and its receiver-
/// qualified callee is a separate [`Quirks::call_override`] hook (see
/// [`groovy_call_override`]), since neither needs anything this
/// `on_unmatched_node` hook's node-kind dispatch could add.
fn groovy_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "package_declaration" => {
            if let Some(name) = groovy_package_name(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(super_name) = groovy_superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                for interface_name in groovy_super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                groovy_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        "import_declaration" => {
            if let Some(path) = groovy_import_path(node, src) {
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

/// `class_declaration`'s DEFINES-scoped body walk -- not one of
/// `LangSpec::groovy()`'s own class/interface/enum/alias arrays'
/// generic fallback (that fallback never runs for `class_declaration`,
/// since [`groovy_quirk`] claims it first) -- mirrors every other
/// language's identical `*_walk_scoped_body` helper.
fn groovy_walk_scoped_body(
    class_node: Node<'_>,
    src: &[u8],
    name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::groovy();
    let quirks = groovy_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// Groovy's [`Quirks`] row: dotted package Module symbol, class heritage
/// (INHERITS/IMPLEMENTS) + DEFINES-scoped body walk, dotted/wildcard
/// import paths, and full receiver-qualified callee reconstruction for
/// both call-shaped node kinds.
pub fn groovy_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(groovy_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(groovy_call_override)),
    }
}

/// Parse Groovy source through the generic engine. No pre-existing
/// bespoke `languages::groovy` extractor to prove zero-regression
/// against (Groovy has never had one in this crate) -- correctness is
/// instead verified directly against the `tree-sitter-groovy` crate's
/// own `node-types.json` plus real parse trees (see
/// [`LangSpec::groovy`]'s doc comment and
/// `tests/unit_languages_groovy.rs`).
pub fn parse_groovy(source: &str) -> ParsedFile {
    let spec = LangSpec::groovy();
    let quirks = groovy_quirks();
    let language: tree_sitter::Language = tree_sitter_groovy::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Ruby
// =====================================================================

/// Ruby's `class Sub < Base`/`module`'s own scoped body walk -- mirrors
/// every other language's DEFINES-scoped body-walk helper
/// ([`ts_walk_scoped_body`]/[`py_walk_scoped_body`]/...): `class`/
/// `module` bodies are wrapped one level deeper in a `body_statement`
/// node (verified against a real parse), so this walks straight into
/// that wrapper's own children under the class/module's own name as
/// `enclosing`, rather than recursing into `body_statement` itself
/// generically (which would leave `enclosing` at whatever the caller
/// had, dropping every member DEFINES edge).
fn ruby_walk_scoped_body(node: Node<'_>, src: &[u8], name: &str, out: &mut ParsedFile) {
    let spec = LangSpec::ruby();
    let quirks = ruby_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, Some(name), FnScope::default());
        }
    }
}

/// The identifier text inside a Ruby `superclass` field wrapper node
/// (`< Animal` -- the field's own text includes the `<` token, so this
/// walks its children for the first named, non-`<` one instead of
/// reading the field's raw text directly). Verified against a real
/// parse: `superclass` wraps exactly `[<, constant]` (or `[<,
/// scope_resolution]` for a dotted `< Foo::Base`).
fn ruby_superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    let superclass = class_node.child_by_field_name("superclass")?;
    let named = (0..superclass.child_count())
        .filter_map(|i| superclass.child(i))
        .find(|n| n.is_named())?;
    named.utf8_text(src).ok().map(str::to_string)
}

/// `require 'x'` / `require_relative 'x'` -- mirrors
/// `internal/cbm/extract_imports.c`'s `ruby_require_method`/
/// `extract_ruby_require_arg` exactly: callee text is one of the two
/// recognized method names, first `string` argument (quotes stripped)
/// is the module path.
fn ruby_require_import(call_node: Node<'_>, method_text: &str, src: &[u8]) -> Option<ImportRef> {
    if method_text != "require" && method_text != "require_relative" {
        return None;
    }
    let args = call_node.child_by_field_name("arguments")?;
    let first_string = (0..args.child_count())
        .filter_map(|i| args.child(i))
        .find(|n| n.kind() == "string")?;
    let raw = first_string.utf8_text(src).ok()?;
    let path = raw.trim_matches(|c| c == '"' || c == '\'').to_string();
    if path.is_empty() {
        return None;
    }
    Some(ImportRef {
        module_path: path,
        line: call_node.start_position().row + 1,
    })
}

/// Each argument expression's own source text, in written order --
/// same shape as every other language's `call_arg_texts` helper, kept
/// as its own function since Ruby's `call` node needs it from
/// [`ruby_call_override`] (which cannot use the generic engine's own
/// `call_arg_texts` -- that helper is keyed by `LangSpec::call_types`'s
/// single flat `call_arguments_field`, which Ruby's `LangSpec` row
/// deliberately leaves as a placeholder since every one of its
/// `call_types` is fully claimed here instead).
fn ruby_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// Full `call`-node callee reconstruction (`receiver.method` when a
/// receiver is present, bare `method` otherwise), `require`/
/// `require_relative` IMPORTS detection, and the baseline's
/// `Widget.new(...)` constructor-callee redirect -- wired as
/// [`Quirks::call_override`] since Ruby's grammar splits the receiver
/// and method name into two separate fields (`receiver`/`method`) the
/// same way Java's `method_invocation` does, rather than one field
/// holding the whole written callee expression the way Go/Rust/TS/
/// Python/Zig's call-shaped nodes do.
fn ruby_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "call" {
        return false;
    }
    let Some(method_node) = node.child_by_field_name("method") else {
        return false;
    };
    let Ok(method_text) = method_node.utf8_text(src) else {
        return false;
    };
    let receiver = node.child_by_field_name("receiver");
    let receiver_text = receiver.and_then(|r| r.utf8_text(src).ok());

    // Baseline `internal/cbm/extract_calls.c` CBM_LANG_RUBY redirect:
    // `Widget.new(...)` on a `constant`-kind receiver with method text
    // exactly `"new"` records the callee as the receiver's own type
    // name (`"Widget"`), not literally `"new"` -- Ruby's constructor
    // body lives in `initialize`, so a bare `"new"` callee would never
    // resolve to anything a caller could look up, unlike every other
    // language's `new T()`/`T::new()` idiom.
    let callee = if method_text == "new" && receiver.is_some_and(|r| r.kind() == "constant") {
        receiver_text.unwrap_or(method_text).to_string()
    } else if let Some(receiver_text) = receiver_text {
        format!("{receiver_text}.{method_text}")
    } else {
        method_text.to_string()
    };

    let receiver_hint = receiver.map(|r| {
        if r.utf8_text(src) == Ok("self") {
            ReceiverHint::SelfOrThis
        } else if r.kind() == "constant" {
            ReceiverHint::NewExpression
        } else if r.kind() == "identifier" {
            ReceiverHint::Identifier
        } else if matches!(r.kind(), "string" | "integer" | "float" | "true" | "false") {
            ReceiverHint::Literal
        } else {
            ReceiverHint::Other
        }
    });

    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: receiver_text.map(str::to_string),
        receiver_hint,
        arg_texts: ruby_call_arg_texts(node, src),
    });

    if let Some(import) = ruby_require_import(node, method_text, src) {
        out.imports.push(import);
    }

    true
}

/// Everything Ruby's flat `LangSpec` arrays cannot express: `class`/
/// `module` INHERITS (`superclass` field only -- see [`LangSpec::ruby`]'s
/// own doc comment for why this crate does not build a richer walker
/// than the baseline's own generic-fallback-only depth) + DEFINES-scoped
/// body walk -- wired as this wave's [`Quirks::on_unmatched_node`] hook.
/// `require`/`require_relative` IMPORTS and full callee reconstruction
/// are a separate [`Quirks::call_override`] hook (see
/// [`ruby_call_override`]) since both need data the generic walker's
/// own call branch already consumes before `on_unmatched_node` would
/// ever see a `call`-kind node.
fn ruby_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(super_name) = ruby_superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                ruby_walk_scoped_body(node, src, name.as_str(), out);
            }
            true
        }
        "module" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                ruby_walk_scoped_body(node, src, name.as_str(), out);
            }
            true
        }
        _ => false,
    }
}

/// Ruby's [`Quirks`] row: `class`/`module` (INHERITS from `superclass`
/// field + DEFINES-scoped body walk), full `receiver.method` callee
/// reconstruction, `require`/`require_relative` IMPORTS, and the
/// baseline's `Widget.new(...)` constructor-callee redirect. No
/// `is_test_name`/`route_from_call`/`on_method_defined`: the baseline
/// gives Ruby no dedicated test-name convention, decorator wiring, or
/// route-registration-by-call-shape detection either (routes for
/// Rails/Sinatra are library-NAME matches over IMPORTS --
/// `internal/cbm/service_patterns.c`'s `route_reg_libraries` table --
/// which this crate's [`crate::code_graph`] layer, not this
/// per-language quirk row, is responsible for; the IMPORTS edges this
/// row's [`ruby_call_override`] already emits are exactly what that
/// layer needs).
pub fn ruby_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(ruby_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(ruby_call_override)),
    }
}

/// Parse Ruby source through the generic engine.
pub fn parse_ruby(source: &str) -> ParsedFile {
    let spec = LangSpec::ruby();
    let quirks = ruby_quirks();
    let language: tree_sitter::Language = tree_sitter_ruby::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Zig
// =====================================================================

/// A Zig `struct_declaration`/`enum_declaration`/`union_declaration`'s
/// own name: NOT a field on the node itself (it has none), but the
/// `identifier` child of its *parent* `variable_declaration` --
/// `const Foo = struct {...}` -- mirrors `internal/cbm/extract_defs.c`'s
/// `CBM_LANG_ZIG` case (:3731-3740) exactly.
fn zig_container_name(container_node: Node<'_>, src: &[u8]) -> Option<String> {
    let parent = container_node.parent()?;
    if parent.kind() != "variable_declaration" {
        return None;
    }
    (0..parent.child_count())
        .filter_map(|i| parent.child(i))
        .find(|n| n.kind() == "identifier")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// A Zig `test_declaration`'s own name: the quoted string literal's
/// `string_content` child (`test "some name" { ... }`) -- mirrors
/// `internal/cbm/extract_defs.c`'s `resolve_zig_test_name` exactly.
/// Returns `None` for the identifier-named form (`test name { ... }`,
/// no quotes) -- same scope the baseline's own helper has (its own
/// fallback returns a null node, i.e. "unnamed", for that shape too).
fn zig_test_name(test_node: Node<'_>, src: &[u8]) -> Option<String> {
    let string_node = (0..test_node.child_count())
        .filter_map(|i| test_node.child(i))
        .find(|n| n.kind() == "string")?;
    let content = (0..string_node.child_count())
        .filter_map(|i| string_node.child(i))
        .find(|n| n.kind() == "string_content")?;
    content.utf8_text(src).ok().map(str::to_string)
}

/// Every direct-child `container_field` of a struct/enum/union's own
/// body -- mirrors every other language's struct/class-field DEFINES
/// extraction, scoped to direct children only (a nested
/// struct/enum/union's own fields belong to *it*, not to this
/// container, same rationale as [`java_emit_constant_field_defines`]'s
/// nested-boundary stop).
fn zig_container_field_names(container_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..container_node.child_count() {
        let Some(child) = container_node.child(i) else {
            continue;
        };
        if child.kind() != "container_field" {
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

/// `struct_declaration`/`enum_declaration`/`union_declaration`'s own
/// scoped recursion once its name (if any) has been resolved via
/// [`zig_container_name`] -- these three kinds are not one of
/// `LangSpec::zig()`'s `func_types`/`method_types` (so the generic
/// walker's own DEFINES-scoped body-walk-on-symbol-push never runs for
/// them), and are fully claimed by [`zig_quirk`] before the generic
/// class-shape fallback would ever recurse into them either, so this
/// quirk re-implements that one recursive call directly.
fn zig_walk_container_body(node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::zig();
    let quirks = zig_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// Each argument expression's own source text -- same shape as every
/// other language's `call_arg_texts` helper, kept separate since
/// [`zig_call_override`] needs it for `builtin_function`'s own
/// positional `arguments` child (not a field-keyed lookup the way
/// `call_expression`'s `"arguments"` field already is).
fn zig_builtin_arg_texts(builtin_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = (0..builtin_node.child_count())
        .filter_map(|i| builtin_node.child(i))
        .find(|n| n.kind() == "arguments")
    else {
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

/// `@import("std")`/`@compileLog(...)`-style builtin-function calls,
/// which have no fields at all (positional `[builtin_identifier,
/// arguments]` children) -- wired as [`Quirks::call_override`] since
/// `builtin_function` cannot use the generic engine's own
/// `call_function_field`-keyed reconstruction. `call_expression`
/// (Zig's ordinary call node, which DOES have a working `"function"`
/// field) is intentionally not matched here -- it falls through to the
/// generic engine's own default single-field path unchanged.
fn zig_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "builtin_function" {
        return false;
    }
    let Some(builtin_id) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "builtin_identifier")
    else {
        return false;
    };
    let Ok(callee) = builtin_id.utf8_text(src) else {
        return false;
    };
    let arg_texts = zig_builtin_arg_texts(node, src);
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts: arg_texts.clone(),
    });
    if callee == "@import" {
        if let Some(first_arg) = arg_texts.first() {
            let path = first_arg.trim_matches('"').to_string();
            if !path.is_empty() {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
    }
    true
}

/// Everything Zig's flat `LangSpec` arrays cannot express:
/// struct/enum/union anonymous-type-expression naming (via the parent
/// `variable_declaration`) + field DEFINES + scoped body walk, and
/// `test "name" { ... }` string-literal naming -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. `@import`/other builtin-function
/// calls are a separate [`Quirks::call_override`] hook (see
/// [`zig_call_override`]) since `builtin_function` needs handling
/// before the generic walker's own call branch's field-keyed
/// reconstruction would silently find nothing and skip the call
/// entirely.
fn zig_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "struct_declaration" | "union_declaration" => {
            let name = zig_container_name(node, src);
            if let Some(name) = &name {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Struct,
                    line,
                });
                for field_name in zig_container_field_names(node, src) {
                    out.defines.push(DefinesRef {
                        container_name: name.clone(),
                        member_name: field_name,
                        line,
                    });
                }
            }
            zig_walk_container_body(node, src, name.as_deref(), out);
            true
        }
        "enum_declaration" => {
            let name = zig_container_name(node, src);
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
            zig_walk_container_body(node, src, name.as_deref(), out);
            true
        }
        "test_declaration" => {
            if let Some(name) = zig_test_name(node, src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Test,
                    line,
                });
            }
            // Not fully claimed: a test body can itself contain calls
            // this quirk does not walk itself -- fall through to
            // generic recursion (same rationale as Go's
            // `const_declaration`/`var_declaration` quirk arms).
            false
        }
        _ => false,
    }
}

/// Zig's [`Quirks`] row: struct/enum/union anonymous-container naming +
/// field DEFINES + scoped body walk, `test "name" { ... }` naming, and
/// `@import`/builtin-function call handling (incl. IMPORTS detection).
/// No `is_test_name`/`route_from_call`/`on_method_defined`: Zig test
/// naming is fully handled by the `test_declaration` quirk arm above
/// (there is no separate filename/annotation convention to gate a
/// *second* time), and the baseline wires Zig no route-registration
/// detection at all (`CBM_LANG_ZIG` never appears in
/// `internal/cbm/service_patterns.c`'s route-library tables).
pub fn zig_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(zig_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(zig_call_override)),
    }
}

/// Parse Zig source through the generic engine.
pub fn parse_zig(source: &str) -> ParsedFile {
    let spec = LangSpec::zig();
    let quirks = zig_quirks();
    let language: tree_sitter::Language = tree_sitter_zig::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Objective-C
// =====================================================================

/// Join every direct-child `identifier` node under `node` with `:`,
/// Objective-C's own multi-keyword selector syntax
/// (`setName:withAge:` for a two-keyword method/message,  bare `bark`
/// for a zero-argument one -- no trailing `:` when there was exactly
/// one identifier and no `method_parameter` sibling at all, since that
/// shape is the argument-less form). Shared by
/// [`objc_method_selector`] (declaration/definition node, direct
/// `identifier` children) and [`objc_message_selector`] (a
/// `message_expression`'s `"method"` field, which is itself already
/// the exact identifier-per-keyword list with no punctuation to filter
/// out -- both converge on this one joiner).
fn objc_join_selector_parts(parts: &[&str], has_parameters: bool) -> String {
    if has_parameters {
        parts.iter().map(|p| format!("{p}:")).collect()
    } else {
        parts.join("")
    }
}

/// A `method_definition`/`method_declaration` node's own full selector
/// -- every direct-child `identifier` (verified: a parameter's own
/// local-variable-name `identifier` lives one level deeper inside a
/// `method_parameter` sibling, so a direct-children-only scan never
/// picks it up), joined per [`objc_join_selector_parts`]. Returns
/// `None` if no `identifier` direct child exists (malformed/incomplete
/// source).
fn objc_method_selector(method_node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut parts = Vec::new();
    let mut has_parameters = false;
    for i in 0..method_node.child_count() {
        let Some(child) = method_node.child(i) else {
            continue;
        };
        match child.kind() {
            "identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    parts.push(text);
                }
            }
            "method_parameter" => has_parameters = true,
            _ => {}
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(objc_join_selector_parts(&parts, has_parameters))
}

/// A `message_expression`'s own full selector from its `"method"`
/// field (`multiple: true` -- every keyword part in written order,
/// verified: `["setName", "withAge"]` for `[self setName:@"x"
/// withAge:3]`), joined per [`objc_join_selector_parts`] the same way
/// as [`objc_method_selector`] so a call site's callee text and its
/// target definition's own recorded name agree byte-for-byte. Whether
/// the send has parameters is decided by the presence of a literal
/// `":"` direct-child token -- NOT `parts.len() > 1`, which a real
/// parse disproves: a single-keyword send that still takes an argument
/// (`[NSString stringWithFormat:@"%@", self]`) has exactly ONE `method`
/// part (`["stringWithFormat"]`), indistinguishable by count alone from
/// a true zero-argument send (`[self bark]`, also exactly one part) --
/// only the `":"` token's presence tells the two apart.
fn objc_message_selector(message_node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = message_node.walk();
    let parts: Vec<&str> = message_node
        .children_by_field_name("method", &mut cursor)
        .filter_map(|n| n.utf8_text(src).ok())
        .collect();
    if parts.is_empty() {
        return None;
    }
    let has_parameters = (0..message_node.child_count())
        .filter_map(|i| message_node.child(i))
        .any(|c| c.kind() == ":");
    Some(objc_join_selector_parts(&parts, has_parameters))
}

/// A `class_interface`/`class_implementation` node's own name: the
/// first direct-child `identifier` (verified: neither kind has a
/// `"name"` field -- the class name is purely positional, immediately
/// after the `@interface`/`@implementation` keyword token).
fn objc_class_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "identifier")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// `class_interface`/`class_implementation`'s scoped recursion under
/// the class's own name (once resolved via [`objc_class_name`]) --
/// neither kind is one of `LangSpec::objc()`'s `func_types`/
/// `method_types` (so the generic walker's own DEFINES-scoped
/// body-walk-on-symbol-push never applies), and both are fully claimed
/// by [`objc_quirk`] before the generic class-shape fallback's own
/// recursion would ever run, so this quirk re-implements that one
/// recursive call directly.
fn objc_walk_scoped(node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::objc();
    let quirks = objc_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// A `method_definition`'s body: every call inside it needs `fn_scope`
/// set to this method's own selector/line -- same rationale as
/// [`c_walk_function_body`] (`method_definition`/`method_declaration`
/// are not one of `LangSpec::objc()`'s func/method-array kinds the
/// generic walker's own DEFINES-scoped-body-on-symbol-push branch would
/// otherwise handle, since [`LangSpec::objc`]'s `name_field` is a
/// placeholder never consulted -- every one of these two kinds is fully
/// claimed by [`objc_quirk`] instead).
fn objc_walk_method_body(
    body: Node<'_>,
    src: &[u8],
    name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::objc();
    let quirks = objc_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name: Some(name),
        line: Some(line),
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, None, fn_scope);
        }
    }
}

/// Everything Objective-C's flat `LangSpec` arrays cannot express:
/// `class_interface`/`class_implementation` positional naming +
/// `superclass`-field INHERITS + DEFINES-scoped body walk,
/// `protocol_declaration` positional naming + scoped body walk (no
/// INHERITS -- a protocol's own heritage is a `protocol_reference_list`
/// child this crate does not walk, matching the "no idealized depth
/// beyond the baseline" instruction: the baseline's own
/// `extract_base_classes` dedicated-walker list has no Objective-C
/// entry either), `method_definition`/`method_declaration` full-selector
/// naming (reusing [`c_quirk`]'s plain `function_definition` handling
/// unchanged for that one C-family kind), and `struct_specifier`/
/// `enum_specifier`/`union_specifier`/`type_definition` delegated
/// verbatim to [`c_quirk`] (Objective-C's grammar gives these kinds the
/// identical field shapes C's own does, so there is no reason to
/// duplicate C's declarator-unwrapping logic here) -- wired as this
/// wave's [`Quirks::on_unmatched_node`] hook. Full selector-based
/// callee reconstruction for `message_expression` is a separate
/// [`Quirks::call_override`] hook (see [`objc_call_override`]).
fn objc_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_interface" => {
            let name = objc_class_name(node, src);
            if let Some(name) = &name {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line,
                });
                if let Some(super_name) = node
                    .child_by_field_name("superclass")
                    .and_then(|n| n.utf8_text(src).ok())
                {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: super_name.to_string(),
                        line,
                    });
                }
            }
            objc_walk_scoped(node, src, name.as_deref(), out);
            true
        }
        "class_implementation" => {
            let name = objc_class_name(node, src);
            if let Some(name) = &name {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(super_name) = node
                    .child_by_field_name("superclass")
                    .and_then(|n| n.utf8_text(src).ok())
                {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: super_name.to_string(),
                        line,
                    });
                }
            }
            objc_walk_scoped(node, src, name.as_deref(), out);
            true
        }
        "protocol_declaration" => {
            let name = objc_class_name(node, src);
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Interface,
                    line: node.start_position().row + 1,
                });
            }
            objc_walk_scoped(node, src, name.as_deref(), out);
            true
        }
        "method_definition" | "method_declaration" => {
            let Some(name) = objc_method_selector(node, src) else {
                return true;
            };
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Method,
                line,
            });
            if let Some(container) = enclosing {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name.clone(),
                    line,
                });
            }
            if let Some(body) = node.child_by_field_name("body").or_else(|| {
                (0..node.child_count())
                    .filter_map(|i| node.child(i))
                    .find(|n| n.kind() == "compound_statement")
            }) {
                objc_walk_method_body(body, src, name.as_str(), line, out);
            }
            true
        }
        // Plain C-family shapes: delegate to `c_quirk` unchanged
        // (Objective-C's grammar gives these kinds identical field
        // shapes to C's own).
        "function_definition"
        | "struct_specifier"
        | "enum_specifier"
        | "union_specifier"
        | "type_definition"
        | "declaration"
        | "preproc_def"
        | "preproc_include" => c_quirk(node, enclosing, src, out),
        _ => false,
    }
}

/// Each argument expression's own source text -- mirrors every other
/// language's `call_arg_texts` helper, kept separate for
/// [`objc_call_override`]'s own use since `message_expression`'s
/// argument expressions are its own non-`method`/non-`receiver`-field
/// named children (positional, not a single `"arguments"`-field list
/// the way `call_expression` has).
fn objc_message_arg_texts(message_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut cursor = message_node.walk();
    let method_nodes: Vec<_> = message_node
        .children_by_field_name("method", &mut cursor)
        .collect();
    let receiver = message_node.child_by_field_name("receiver");
    let mut out = Vec::new();
    for i in 0..message_node.child_count() {
        let Some(child) = message_node.child(i) else {
            continue;
        };
        if !child.is_named() {
            continue;
        }
        if Some(child) == receiver {
            continue;
        }
        if method_nodes.contains(&child) {
            continue;
        }
        if let Ok(text) = child.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// Full `message_expression` callee reconstruction
/// (`receiver method:withKeyword:` selector, joined per
/// [`objc_message_selector`]) -- wired as [`Quirks::call_override`]
/// since Objective-C's grammar splits the receiver and the (possibly
/// multi-part) selector into two separate fields rather than one field
/// holding the whole written callee expression. `call_expression`
/// (Objective-C's plain C-style call, which DOES have a working
/// `"function"` field) is intentionally not matched here -- it falls
/// through to the generic engine's own default single-field path
/// unchanged.
fn objc_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "message_expression" {
        return false;
    }
    let Some(callee) = objc_message_selector(node, src) else {
        return false;
    };
    let receiver = node.child_by_field_name("receiver");
    let receiver_text = receiver.and_then(|r| r.utf8_text(src).ok());
    let receiver_hint = receiver.map(|r| {
        if r.utf8_text(src) == Ok("self") || r.utf8_text(src) == Ok("super") {
            ReceiverHint::SelfOrThis
        } else if r.kind() == "identifier" {
            // A bare capitalized identifier receiver (`[NSString
            // stringWithFormat:...]`) is a class-method send, the
            // Objective-C equivalent of a `new T()`-shaped constructor
            // idiom -- lowercase-starting identifiers are ordinary
            // instance variables/locals.
            if r.utf8_text(src)
                .ok()
                .and_then(|t| t.chars().next())
                .is_some_and(|c| c.is_uppercase())
            {
                ReceiverHint::NewExpression
            } else {
                ReceiverHint::Identifier
            }
        } else {
            ReceiverHint::Other
        }
    });
    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: receiver_text.map(str::to_string),
        receiver_hint,
        arg_texts: objc_message_arg_texts(node, src),
    });
    true
}

/// Objective-C's [`Quirks`] row: `class_interface`/`class_implementation`/
/// `protocol_declaration` positional naming + `superclass`-field
/// INHERITS + DEFINES-scoped body walk, `method_definition`/
/// `method_declaration` full-selector naming + DEFINES + scoped call-body
/// walk, plain-C-family delegation to [`c_quirk`], and full
/// `message_expression` selector-based callee reconstruction. No
/// `is_test_name`/`route_from_call`/`on_method_defined`: the baseline
/// wires Objective-C no test-name convention, decorator/annotation
/// handling, or route-registration detection at all (`CBM_LANG_OBJC`
/// never appears in `internal/cbm/service_patterns.c`'s route-library
/// tables, and its own `lang_specs.c` row has `NULL` for both the
/// decorator-types and env-funcs columns).
pub fn objc_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(objc_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(objc_call_override)),
    }
}

/// Parse Objective-C source through the generic engine.
pub fn parse_objc(source: &str) -> ParsedFile {
    let spec = LangSpec::objc();
    let quirks = objc_quirks();
    let language: tree_sitter::Language = tree_sitter_objc::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}
