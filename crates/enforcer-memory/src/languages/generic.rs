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

// =====================================================================
// Bash
// =====================================================================

/// Every `command`'s own written argument text, in written order --
/// unlike [`call_arg_texts`] (which reads a single `arguments`-field
/// child node's own children), `command` has no such wrapper at all: a
/// real parse confirms every argument is instead a repeated `argument`
/// FIELD entry directly on the `command` node itself (see
/// [`LangSpec::bash`]'s own doc comment) -- walked here via
/// [`tree_sitter::Node::children_by_field_name`] rather than
/// [`tree_sitter::Node::child_by_field_name`] (which would only ever
/// return the first one).
fn bash_command_arg_texts(command_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut cursor = command_node.walk();
    command_node
        .children_by_field_name("argument", &mut cursor)
        .filter_map(|arg| arg.utf8_text(src).ok().map(str::to_string))
        .collect()
}

/// Bash's [`Quirks::call_override`]: every `command` node is recorded as
/// a CALLS edge (its own `name` field's `command_name` wrapper already
/// yields the correct callee text via plain `.utf8_text()`, see
/// [`LangSpec::bash`]'s own doc comment, so this does not need to
/// re-derive the callee itself) plus its own `argument`-field children
/// as `arg_texts` (see [`bash_command_arg_texts`]) -- neither of which
/// [`LangSpec::bash`]'s flat `call_arguments_field` mechanism could
/// express on its own. Additionally recognizes `source ./lib.sh`/
/// `. ./other.sh` (Bash's closest import-statement analog, matching the
/// baseline's own `bash_import_types`/`parse_generic_imports(ctx,
/// "command")` intent -- see [`LangSpec::bash`]'s own doc comment for why
/// a flat `import_types` array entry cannot express this without
/// colliding with this same node kind's own `call_types` membership) and
/// additionally pushes an IMPORTS edge in that case, using the first
/// `argument`-field child's own text as the module path.
fn bash_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "command" {
        return false;
    }
    let Some(name_node) = node.child_by_field_name("name") else {
        return false;
    };
    let Ok(callee) = name_node.utf8_text(src) else {
        return false;
    };
    let arg_texts = bash_command_arg_texts(node, src);
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts: arg_texts.clone(),
    });
    if matches!(callee, "source" | ".") {
        if let Some(path) = arg_texts.into_iter().next() {
            out.imports.push(ImportRef {
                module_path: path,
                line: node.start_position().row + 1,
            });
        }
    }
    true
}

/// Bash's [`Quirks`] row: full CALLS recording (own callee text +
/// `argument`-field `arg_texts`, see [`bash_call_override`]) doubling as
/// `source`/`.` IMPORTS detection. No `on_unmatched_node`/`is_test_name`/
/// `route_from_call`/`on_method_defined`: `function_definition` has a
/// real `name`/`body` field pair the generic engine's own func/method
/// branch already handles unaided (see [`LangSpec::bash`]'s own doc
/// comment), and the baseline gives Bash no test-name convention,
/// decorator syntax, or route-registration-by-call-shape detection
/// either (`CBM_LANG_BASH` never appears in
/// `internal/cbm/service_patterns.c`'s route-library tables, and its own
/// `lang_specs.c` row has `NULL` for both the decorator-types and
/// env-funcs columns).
pub fn bash_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: None,
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(bash_call_override)),
    }
}

/// Parse Bash source through the generic engine. No pre-existing bespoke
/// `languages::bash` extractor to prove zero-regression against (Bash
/// has never had one in this crate) -- correctness is instead verified
/// directly against the `tree-sitter-bash` crate's own `node-types.json`
/// plus real parse trees (see [`LangSpec::bash`]'s doc comment and
/// `tests/unit_languages_bash.rs`).
pub fn parse_bash(source: &str) -> ParsedFile {
    let spec = LangSpec::bash();
    let quirks = bash_quirks();
    let language: tree_sitter::Language = tree_sitter_bash::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Lua
// =====================================================================

/// A Lua `function_definition` (the ANONYMOUS function-literal form,
/// `local foo = function(a) ... end` / `foo = function(a) ... end`) has
/// no `name` field at all (see [`LangSpec::lua`]'s own doc comment) --
/// its name, when one exists, is the LEFT-hand side of the enclosing
/// `assignment_statement` this literal is the right-hand side of.
/// Mirrors the baseline's own dedicated `resolve_lua_func_name`
/// (`codebase-memory-mcp/internal/cbm/extract_defs.c`:207-232) exactly:
/// walk up through an intervening `expression_list` (the RHS wrapper for
/// `local foo = function() end`'s single expression) to the parent
/// `assignment_statement`, then read its own `variable_list` field's
/// first `variable` child (an `identifier` for the common case this
/// crate's own `child_text`-based reconstruction below already handles;
/// a `dot_index_expression`/`bracket_index_expression` target is
/// possible too but rarer for an anonymous-function assignment and left
/// unresolved here the same "no name recovered, no def emitted" way the
/// baseline's own walker leaves any parent shape it does not recognize).
fn lua_anonymous_function_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut parent = node.parent()?;
    if parent.kind() == "expression_list" {
        parent = parent.parent()?;
    }
    if parent.kind() != "assignment_statement" {
        return None;
    }
    let variables = parent.child_by_field_name("variables").or_else(|| {
        (0..parent.child_count())
            .filter_map(|i| parent.child(i))
            .find(|c| c.kind() == "variable_list")
    })?;
    let first_var = (0..variables.child_count()).find_map(|i| variables.child(i))?;
    first_var.utf8_text(src).ok().map(str::to_string)
}

/// Lua's [`Quirks::on_unmatched_node`]: the anonymous
/// `function_definition` literal's own name resolution (see
/// [`lua_anonymous_function_name`]) plus a correctly `fn_scope`-scoped
/// walk of its own `body` field -- [`LangSpec::lua`]'s `func_types`
/// deliberately excludes `function_definition` (it has no `name` field
/// the generic engine's own func/method branch could read), so this
/// quirk hook -- not that branch -- is what actually claims it.
fn lua_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    if node.kind() != "function_definition" {
        return false;
    }
    let line = node.start_position().row + 1;
    let Some(name) = lua_anonymous_function_name(node, src) else {
        // No recoverable name (e.g. an anonymous function passed
        // directly as a call argument, not assigned to a variable) --
        // still walk its body generically (with no `fn_scope`) so any
        // call/import nested inside it is still found, matching every
        // other language's "no name, still recurse" posture.
        walk_children(node, &ctx_for_lua(src), out, None, FnScope::default());
        return true;
    };
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Function,
        line,
    });
    if let Some(body) = node.child_by_field_name("body") {
        let fn_scope = FnScope {
            name: Some(name.as_str()),
            line: Some(line),
        };
        walk_children(body, &ctx_for_lua(src), out, None, fn_scope);
    }
    true
}

/// A throwaway [`Ctx`] for [`lua_quirk`]'s own re-entrant body walk --
/// mirrors every other language's identical `*_walk_scoped_body`/
/// `*_walk_function_body` helper pattern (e.g. [`dart_walk_function_body`])
/// of constructing a fresh spec/quirks pair rather than threading the
/// caller's own borrowed `Ctx` through (which the [`Quirks::on_unmatched_node`]
/// hook signature does not carry access to).
fn ctx_for_lua(src: &[u8]) -> Ctx<'_> {
    // `Box::leak` keeps a `'static`-lifetime-free `LangSpec`/`Quirks`
    // pair alive for exactly this call's own nested walk without
    // widening `Ctx`'s own lifetime parameter beyond `src`'s -- every
    // field `LangSpec`/`Quirks` need is itself `Copy`/boxed-fn-pointer
    // data with no borrow back into this function's stack, so leaking
    // one pair per anonymous-function-literal node (a bounded, rare
    // construct, not a hot loop) is a deliberate, small, one-directional
    // trade favoring this helper's own simplicity over pooling/caching.
    static SPEC: std::sync::OnceLock<LangSpec> = std::sync::OnceLock::new();
    let spec = SPEC.get_or_init(LangSpec::lua);
    let quirks: &'static Quirks = Box::leak(Box::new(lua_quirks()));
    Ctx {
        spec,
        src,
        quirks,
        is_test_file: false,
    }
}

/// Lua's [`Quirks::call_override`]: `function_call`'s own `name` field
/// can be `method_index_expression` (`w:draw()`, a `table`/`method`
/// two-field split the generic engine's single `call_function_field`
/// cannot reconstruct receiver info from on its own -- see
/// [`LangSpec::lua`]'s own doc comment) -- claimed here, mirroring
/// [`ruby_call_override`]'s identical `receiver`/`method`-field-split
/// posture. Every other `name`-field shape (`identifier`/`function_call`/
/// `parenthesized_expression`) returns `false` (not overridden), falling
/// through to the generic engine's own ordinary `call_function_field`
/// flat path unchanged. Also recognizes a bare `require(...)` callee and
/// pushes an IMPORTS edge from its first string-literal argument,
/// mirroring [`ruby_call_override`]'s identical `require`/
/// `require_relative` precedent and the baseline's own dedicated
/// `parse_lua_imports` (`codebase-memory-mcp/internal/cbm/
/// extract_imports.c`:749-810) intent.
fn lua_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "function_call" {
        return false;
    }
    let Some(name_node) = node.child_by_field_name("name") else {
        return false;
    };
    let arg_texts = call_arg_texts(node, "arguments", src);
    if name_node.kind() == "method_index_expression" {
        let Some(table) = name_node.child_by_field_name("table") else {
            return false;
        };
        let Some(method) = name_node.child_by_field_name("method") else {
            return false;
        };
        let (Ok(table_text), Ok(method_text)) = (table.utf8_text(src), method.utf8_text(src))
        else {
            return false;
        };
        let receiver_hint = if table_text == "self" {
            ReceiverHint::SelfOrThis
        } else if table.kind() == "identifier" {
            ReceiverHint::Identifier
        } else {
            ReceiverHint::Other
        };
        out.calls.push(CallRef {
            callee: format!("{table_text}:{method_text}"),
            line: node.start_position().row + 1,
            from_symbol: from_symbol.map(str::to_string),
            from_symbol_line,
            receiver_text: Some(table_text.to_string()),
            receiver_hint: Some(receiver_hint),
            arg_texts,
        });
        return true;
    }
    // Not a method-index call -- record the bare, unqualified callee (the
    // generic engine's own flat path would do the same thing for this
    // shape, but returning `false` here would also skip the
    // `require(...)` IMPORTS check below, which needs the callee text
    // regardless of which path recorded the call itself).
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
        arg_texts: arg_texts.clone(),
    });
    if callee == "require" {
        // `arg_texts`' own raw text keeps the surrounding quote
        // characters verbatim (`"\"json\""`, matching every other
        // language's `call_arg_texts`-derived `arg_texts` convention of
        // recording exactly what was written) -- the IMPORTS module
        // path needs those quotes stripped instead, mirroring
        // [`ruby_call_override`]'s own `ruby_require_import` precedent
        // exactly (same `trim_matches` call, same quote-character set).
        if let Some(raw) = arg_texts.into_iter().next() {
            let path = raw.trim_matches(|c| c == '"' || c == '\'').to_string();
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

/// Lua's [`Quirks`] row: anonymous `function_definition`-literal name
/// resolution + scoped body walk (see [`lua_quirk`]), full
/// `method_index_expression` receiver-qualified callee reconstruction,
/// and `require(...)` IMPORTS (see [`lua_call_override`]). No
/// `is_test_name`/`route_from_call`/`on_method_defined`: the baseline
/// gives Lua no test-name convention or route-registration-by-call-shape
/// detection either (`CBM_LANG_LUA` never appears in
/// `internal/cbm/service_patterns.c`'s route-library tables).
pub fn lua_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(lua_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(lua_call_override)),
    }
}

/// Parse Lua source through the generic engine. No pre-existing bespoke
/// `languages::lua` extractor to prove zero-regression against (Lua has
/// never had one in this crate) -- correctness is instead verified
/// directly against the `tree-sitter-lua` crate's own `node-types.json`
/// plus real parse trees (see [`LangSpec::lua`]'s doc comment and
/// `tests/unit_languages_lua.rs`).
pub fn parse_lua(source: &str) -> ParsedFile {
    let spec = LangSpec::lua();
    let quirks = lua_quirks();
    let language: tree_sitter::Language = tree_sitter_lua::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Elixir
// =====================================================================

/// An Elixir `call` node's own `arguments` field, tolerating the
/// baseline's own "no `arguments` field, take the second positional
/// child instead" fallback (`elixir_call_args`,
/// `codebase-memory-mcp/internal/cbm/extract_defs.c`:4350-4356) -- in
/// practice this crate's own grammar version always exposes a real
/// `arguments` field for every argument-bearing call shape seen during
/// this wave's own real-parse-tree verification, but the fallback is
/// kept for defensive parity with the baseline's own documented
/// uncertainty about that.
fn elixir_call_args(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("arguments")
        .or_else(|| node.child(1))
}

/// A `def`/`defp`/`defmacro` call's own function-name resolution --
/// mirrors the baseline's own `extract_elixir_func_def`
/// (`codebase-memory-mcp/internal/cbm/extract_defs.c`:4359-4392) exactly
/// for the two shapes it already recognizes (a bare `identifier` for a
/// zero-arg `def go do`, or a `call`'s own first child for `def bar(x)
/// do`) -- PLUS, as a deliberate additive improvement over the baseline
/// (see [`LangSpec::elixir`]'s own doc comment), one `binary_operator`
/// unwrap for a guard clause (`def bar(x) when x > 0 do`), taking its
/// `left` child (the `bar(x)` call) before applying the same two-shape
/// check to what remains.
fn elixir_def_name(first_arg: Node<'_>, src: &[u8]) -> Option<String> {
    let first_arg = if first_arg.kind() == "binary_operator"
        && first_arg
            .child_by_field_name("operator")
            .and_then(|op| op.utf8_text(src).ok())
            == Some("when")
    {
        first_arg.child_by_field_name("left")?
    } else {
        first_arg
    };
    match first_arg.kind() {
        "call" => first_arg
            .child(0)
            .and_then(|head| head.utf8_text(src).ok())
            .map(str::to_string),
        "identifier" => first_arg.utf8_text(src).ok().map(str::to_string),
        _ => None,
    }
}

/// Push a `def`/`defp`/`defmacro` call's own Function symbol -- mirrors
/// the baseline's `extract_elixir_func_def`'s own `CBMDefinition` shape
/// (name + start/end line; `is_exported` is a `def`/`defmacro` vs `defp`
/// distinction this crate's own [`SymbolRef`] has no field for, so it is
/// not carried here, matching every OTHER language row's own "exported"-
/// ness likewise going unrepresented in this crate's schema).
fn elixir_push_def(call_node: Node<'_>, macro_name: &str, src: &[u8], out: &mut ParsedFile) {
    let Some(args) = elixir_call_args(call_node) else {
        return;
    };
    let Some(first_arg) = args.child(0) else {
        return;
    };
    let Some(name) = elixir_def_name(first_arg, src) else {
        return;
    };
    let line = call_node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Function,
        line,
    });
    let is_macro_or_def_or_defp = macro_name == "def" || macro_name == "defmacro";
    let _ = is_macro_or_def_or_defp; // documented above; no exported-ness field to set.
    if let Some(do_block) = call_node.child_by_field_name("do_block").or_else(|| {
        (0..call_node.child_count())
            .filter_map(|i| call_node.child(i))
            .find(|c| c.kind() == "do_block")
    }) {
        let fn_scope = FnScope {
            name: Some(name.as_str()),
            line: Some(line),
        };
        walk_children(do_block, &ctx_for_elixir(src), out, None, fn_scope);
    }
}

/// Push a `defmodule` call's own Class symbol (matching the baseline's
/// own `emit_elixir_module_class`'s `label = "Class"`, NOT `"Module"` --
/// see [`LangSpec::elixir`]'s own doc comment) plus a DEFINES-scoped walk
/// of its own `do_block`.
fn elixir_push_module(call_node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let Some(args) = elixir_call_args(call_node) else {
        return;
    };
    let Some(name_node) = args.child(0) else {
        return;
    };
    let Ok(name) = name_node.utf8_text(src) else {
        return;
    };
    let line = call_node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.to_string(),
        kind: SymbolKind::Class,
        line,
    });
    if let Some(do_block) = (0..call_node.child_count())
        .filter_map(|i| call_node.child(i))
        .find(|c| c.kind() == "do_block")
    {
        walk_children(
            do_block,
            &ctx_for_elixir(src),
            out,
            Some(name),
            FnScope::default(),
        );
    }
}

/// Push an `alias`/`import`/`use`/`require` call's own IMPORTS edge --
/// see [`LangSpec::elixir`]'s own doc comment for why this crate reaches
/// every one of these regardless of nesting depth (the generic engine's
/// own recursive walk already visits every `call` node in the tree,
/// unlike the baseline's non-recursive root-children-only scan).
fn elixir_push_import(call_node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let Some(args) = elixir_call_args(call_node) else {
        return;
    };
    let Some(first) = args.child(0) else {
        return;
    };
    let Ok(path) = first.utf8_text(src) else {
        return;
    };
    out.imports.push(ImportRef {
        module_path: path.to_string(),
        line: call_node.start_position().row + 1,
    });
}

/// Elixir's [`Quirks::on_unmatched_node`]: every `call` node is
/// disambiguated by its own `target` field's text -- `defmodule`
///   (Class symbol + scoped body walk), `def`/`defp`/`defmacro` (Function
///   symbol, incl. the guard-clause unwrap -- see [`elixir_def_name`] --
///   plus a `fn_scope`-scoped body walk), `alias`/`import`/`use`/`require`
///   (IMPORTS), or -- for anything else -- returns `false` so the node
///   falls through to [`elixir_call_override`] as an ordinary call.
///   Mirrors the baseline's own `extract_elixir_call`'s
///   `strcmp(macro, "def")`-style dispatch
///   (`codebase-memory-mcp/internal/cbm/extract_defs.c`:4425-4463) except
///   reached via this crate's own recursive walk rather than a bespoke
///   explicit stack (see [`LangSpec::elixir`]'s own doc comment for why
///   this also reaches nested/non-root-child import calls the baseline's
///   own non-recursive scan misses).
fn elixir_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "call" {
        return false;
    }
    let Some(target) = node.child_by_field_name("target") else {
        return false;
    };
    let Ok(macro_name) = target.utf8_text(src) else {
        return false;
    };
    match macro_name {
        "defmodule" => {
            elixir_push_module(node, src, out);
            true
        }
        "def" | "defp" | "defmacro" => {
            elixir_push_def(node, macro_name, src, out);
            true
        }
        "alias" | "import" | "use" | "require" => {
            elixir_push_import(node, src, out);
            // Still recorded as an ordinary call too (matching the
            // baseline's own `extract_scripting_callee`, which never
            // excludes these macro names from CALLS just because
            // `extract_imports.c` ALSO wants them -- the two passes are
            // independent there, and this crate's own unified walk keeps
            // that same independence by falling through to the ordinary
            // call-override path rather than returning `true` here).
            false
        }
        _ => false,
    }
}

/// A throwaway [`Ctx`] for [`elixir_push_def`]/[`elixir_push_module`]'s
/// own re-entrant `do_block` walks -- see [`ctx_for_lua`]'s own doc
/// comment for the identical `Box::leak`-based rationale (this function
/// is its Elixir-specific twin).
fn ctx_for_elixir(src: &[u8]) -> Ctx<'_> {
    static SPEC: std::sync::OnceLock<LangSpec> = std::sync::OnceLock::new();
    let spec = SPEC.get_or_init(LangSpec::elixir);
    let quirks: &'static Quirks = Box::leak(Box::new(elixir_quirks()));
    Ctx {
        spec,
        src,
        quirks,
        is_test_file: false,
    }
}

/// Elixir's [`Quirks::call_override`]: an ordinary call's own callee
/// text -- mirrors the baseline's `extract_scripting_callee`'s
/// `CBM_LANG_ELIXIR` branch exactly
/// (`codebase-memory-mcp/internal/cbm/extract_calls.c`:433-440): a
/// `call`'s own first child must be an `identifier` (a bare `helper(x)`)
/// or a `dot` (a qualified `Enum.map(...)`, whose own `.utf8_text()`
/// already spans the full dotted text, e.g. `"Enum.map"`) -- anything
/// else (a `binary_operator`, e.g.) resolves no callee at all and the
/// call is silently not recorded, matching the baseline's own `return
/// NULL` for that case. [`LangSpec::elixir`]'s `func_types`/
/// `class_types`/`call_types` arrays are ALL `["call"]`, so
/// [`elixir_quirk`] (run first, as [`Quirks::on_unmatched_node`]) claims
/// `defmodule`/`def`-family/`alias`-family calls before this hook would
/// ever see them for those macro names specifically -- this hook only
/// ever actually receives an ORDINARY call (or an `alias`/`import`/
/// `use`/`require` call, which `elixir_quirk` deliberately returns
/// `false` for after pushing its own IMPORTS edge, see
/// [`elixir_quirk`]'s own doc comment, so it is ALSO recorded as a call
/// here).
fn elixir_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "call" {
        return false;
    }
    let Some(first) = node.child(0) else {
        return false;
    };
    if !matches!(first.kind(), "identifier" | "dot") {
        return false;
    }
    let Ok(callee) = first.utf8_text(src) else {
        return false;
    };
    let arg_texts = elixir_call_args(node)
        .map(|args| {
            (0..args.child_count())
                .filter_map(|i| args.child(i))
                .filter(|c| c.is_named())
                .filter_map(|c| c.utf8_text(src).ok().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts,
    });
    true
}

/// Elixir's [`Quirks`] row: `defmodule`/`def`-family/`alias`-family
/// macro-call disambiguation (see [`elixir_quirk`]) + ordinary-call
/// recording (see [`elixir_call_override`]). No `is_test_name`/
/// `route_from_call`/`on_method_defined`: this crate has no
/// ExUnit-test-name convention wired, and Elixir's real route-
/// registration signal is Phoenix's `Phoenix.Router` library-NAME match
/// over IMPORTS (`codebase-memory-mcp/internal/cbm/
/// service_patterns.c`:374, `route_reg_libraries`) -- a
/// [`crate::code_graph`]-layer concern over this row's own
/// [`elixir_push_import`]-emitted IMPORTS edges, not a per-language
/// `route_from_call` quirk, matching every G2.1 scripting-language row's
/// identical `route_from_call: None` posture (see e.g. [`ruby_quirks`]'s
/// own doc comment).
pub fn elixir_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(elixir_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(elixir_call_override)),
    }
}

/// Parse Elixir source through the generic engine. No pre-existing
/// bespoke `languages::elixir` extractor to prove zero-regression
/// against (Elixir has never had one in this crate) -- correctness is
/// instead verified directly against the `tree-sitter-elixir` crate's
/// own `node-types.json` plus real parse trees (see [`LangSpec::elixir`]'s
/// doc comment and `tests/unit_languages_elixir.rs`).
pub fn parse_elixir(source: &str) -> ParsedFile {
    let spec = LangSpec::elixir();
    let quirks = elixir_quirks();
    let language: tree_sitter::Language = tree_sitter_elixir::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Haskell
// =====================================================================

/// Recreates a [`Ctx`] for Haskell and recurses into `node`'s own
/// children -- same "no single `body_field`, re-walk this node's whole
/// remaining structure generically" rationale as [`kotlin_walk_scoped`],
/// needed here because a `function`/`bind` node's actual body lives
/// inside one or more `match` children (each itself wrapping an
/// `expression` via its own `expression` field, for a guarded/
/// multi-equation definition) rather than behind one flat `"body"`
/// field this file's generic func/method branch could read directly --
/// see [`LangSpec::haskell`]'s own doc comment.
fn haskell_walk_scoped(node: Node<'_>, src: &[u8], fn_scope: FnScope<'_>, out: &mut ParsedFile) {
    let spec = LangSpec::haskell();
    let quirks = haskell_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, None, fn_scope);
        }
    }
}

/// `function`/`bind`'s own symbol + scoped body walk -- fully claimed
/// (not the generic engine's own func/method branch) purely because of
/// the `body_field` gap [`haskell_walk_scoped`]'s own doc comment
/// explains; the name itself DOES come from the ordinary `"name"` field
/// (`function`/`bind` both have one, confirmed via `node-types.json` --
/// see [`LangSpec::haskell`]'s own doc comment), so this only
/// re-implements the body-walk half of the generic branch, not the name
/// resolution. Haskell has no class/impl-body nesting concept the way
/// Rust/TS/Python's `enclosing` convention models (every `function`/
/// `bind` is module-scope, `class`/`instance` bodies hold `signature`/
/// `function` declarations that are themselves still ordinary top-level-
/// shaped definitions, not "methods" in the DEFINES sense other
/// languages use) -- `enclosing` is consequently always `None` for this
/// symbol/DEFINES pair, matching Python's identical no-DEFINES-edge
/// posture for its own module-level functions.
fn haskell_function_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = child_text(node, "name", src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Function,
        line,
    });
    haskell_walk_scoped(
        node,
        src,
        FnScope {
            name: Some(name.as_str()),
            line: Some(line),
        },
        out,
    );
    true
}

/// `import Data.List (sort)` / `import qualified Data.Map as Map` --
/// Haskell's `import` node has a real `"module"` field (confirmed via
/// `node-types.json`), read directly rather than through the generic
/// engine's own field-fallback chain (which only ever fires for
/// `LangSpec::class_types`-shaped nodes, not `import_types`).
fn haskell_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let module = node.child_by_field_name("module")?;
    module.utf8_text(src).ok().map(str::to_string)
}

/// `class`/`data_type`/`newtype`/`instance`'s own Class symbol --
/// EVERY one of these four genuinely has a real, working `"name"` field
/// (confirmed via `node-types.json`), so this reads it directly the same
/// way [`haskell_function_like`] reads `function`/`bind`'s own `"name"`
/// field directly rather than through `spec.name_field`: both
/// `func_types` AND `class_types` share that one `LangSpec::name_field`
/// slot, but only the func/method half needs it turned into a
/// placeholder (to let `on_unmatched_node` run before the generic
/// engine's own func/method branch would otherwise short-circuit first
/// -- see [`LangSpec::haskell`]'s own doc comment for the full
/// explanation), which means this class-shape half can no longer rely
/// on that shared field being real either, despite never having needed
/// a quirk for its OWN sake -- caught by this row's own
/// `tests/unit_languages_haskell.rs`'s own `extracts_data_type_as_class`/
/// `extracts_newtype_as_class`/`extracts_class_and_instance_as_class`
/// tests regressing back to failing once the func/method fix above
/// landed (both halves read the identical field, so protecting one
/// silently broke the other), not by inspection. Recurses into the
/// node's own children generically afterward (`walk_children`, matching
/// the generic engine's own class-shape branch's identical
/// `walk_children(node, ctx, out, Some(name.as_str()), fn_scope)` call
/// this function stands in for) so a `class`/`instance` body's own
/// `signature`/`function` declarations are still visited with `enclosing`
/// set to this symbol's own name.
fn haskell_class_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = child_text(node, "name", src) else {
        return true;
    };
    out.symbols.push(SymbolRef {
        name,
        kind: SymbolKind::Class,
        line: node.start_position().row + 1,
    });
    haskell_walk_scoped(node, src, FnScope::default(), out);
    true
}

/// Everything Haskell's flat [`LangSpec`] arrays cannot express: the
/// `function`/`bind` scoped-body-walk gap [`haskell_walk_scoped`]'s own
/// doc comment explains, `class`/`data_type`/`newtype`/`instance`'s own
/// `"name"`-field read (see [`haskell_class_like`]'s own doc comment for
/// why these need an explicit arm too despite each having a genuinely
/// working field the generic engine's own fallback COULD read, if only
/// `LangSpec::name_field` were not shared with the func/method half),
/// and `import`'s `"module"`-field path -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook.
fn haskell_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "function" | "bind" => haskell_function_like(node, src, out),
        "class" | "data_type" | "newtype" | "instance" => haskell_class_like(node, src, out),
        "import" => {
            if let Some(path) = haskell_import_path(node, src) {
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

/// Curried-application head recovery for `apply` (`f a b` nests as
/// `apply(function=apply(function=f,argument=a),argument=b)`, confirmed
/// via a real parse-tree dump) -- mirrors the baseline's own shared
/// `extract_fp_callee` (`internal/cbm/extract_calls.c`:324-344, used for
/// `CBM_LANG_HASKELL`/`CBM_LANG_OCAML`/`CBM_LANG_PURESCRIPT` alike)
/// exactly: descend via the `function` field until it is no longer
/// itself an `apply`/`application_expression`, then read whatever is
/// there directly (an identifier-like leaf, or -- unlike the baseline's
/// own fixed accepted-kind allowlist -- ANY other expression shape,
/// e.g. a parenthesized lambda `(\x -> x + 1) 3`, whose own written text
/// is still a reasonable callee string even though it is not a bare
/// name). `apply_kind` is threaded through explicitly (rather than
/// hardcoding one grammar's own node-kind string) so this one function
/// serves both Haskell's `apply` and [`ocaml_call_override`]'s
/// `application_expression` callers without duplicating the recursion.
fn fp_call_head<'a>(node: Node<'a>, apply_kind: &str) -> Node<'a> {
    let mut current = node;
    loop {
        let Some(function) = current.child_by_field_name("function") else {
            return current;
        };
        if function.kind() == apply_kind {
            current = function;
            continue;
        }
        return function;
    }
}

/// Every curried argument of an `apply` chain, in written
/// (leftmost-first) order -- e.g. `f a b` yields `["a", "b"]`, recovered
/// by walking the SAME `function`-field spine [`fp_call_head`] descends,
/// collecting each level's own `argument` field on the way back up
/// (implemented as a recursive collect-then-append rather than a
/// reversal, since the recursion already visits levels outermost-first
/// and this file's `arg_texts` convention is "written order", i.e.
/// innermost/leftmost first). Haskell-only, NOT shared with
/// [`ocaml_call_override`]'s own `application_expression` arm despite
/// [`fp_call_head`] itself being shared by both: this grammar's `apply`
/// genuinely nests one node per curried argument (`argument` field
/// `multiple: false`), but OCaml's `application_expression` is a FLAT
/// single node with a `multiple: true` `"argument"` field instead (see
/// [`ocaml_application_arg_texts`]'s own doc comment for the real
/// grammar-shape divergence this caught) -- `apply_kind` is kept as an
/// explicit parameter anyway (mirroring [`fp_call_head`]'s own
/// parameterization) purely so a future third curried-nesting-shaped
/// language never needs to duplicate this recursion, not because OCaml
/// itself still uses it.
fn fp_call_arg_texts(node: Node<'_>, apply_kind: &str, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    fp_collect_args(node, apply_kind, src, &mut out);
    out
}

fn fp_collect_args(node: Node<'_>, apply_kind: &str, src: &[u8], out: &mut Vec<String>) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    if function.kind() == apply_kind {
        fp_collect_args(function, apply_kind, src, out);
    }
    if let Some(argument) = node.child_by_field_name("argument") {
        if let Ok(text) = argument.utf8_text(src) {
            out.push(text.to_string());
        }
    }
}

/// `apply`/`infix` callee reconstruction -- wired as this wave's
/// [`Quirks::call_override`] hook since neither shape fits the generic
/// engine's single `function`/`arguments`-field-pair assumption (see
/// [`LangSpec::haskell`]'s own doc comment for the field-shape details).
/// `infix`'s callee is its own `operator` field's text, mirroring the
/// baseline's `extract_fp_callee` `infix`/`infix_expression` arm
/// exactly (`internal/cbm/extract_calls.c`:345-354).
fn haskell_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "apply" => {
            let head = fp_call_head(node, "apply");
            let Ok(callee) = head.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: fp_call_arg_texts(node, "apply", src),
            });
            true
        }
        "infix" => {
            let Some(operator) = node.child_by_field_name("operator") else {
                return false;
            };
            let Ok(callee) = operator.utf8_text(src) else {
                return false;
            };
            let left = node.child_by_field_name("left_operand");
            let right = node.child_by_field_name("right_operand");
            let arg_texts = [left, right]
                .into_iter()
                .flatten()
                .filter_map(|n| n.utf8_text(src).ok().map(str::to_string))
                .collect();
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts,
            });
            true
        }
        _ => false,
    }
}

/// Haskell's [`Quirks`] row: `function`/`bind` scoped-body-walk gap +
/// `import`'s `"module"`-field path + `apply`/`infix` callee
/// reconstruction. No `is_test_name`/`route_from_call`/
/// `on_method_defined`: the baseline wires Haskell no test-name
/// convention, DEFINES-container-from-elsewhere method convention, or
/// route-registration detection at all (`CBM_LANG_HASKELL` never
/// appears in `internal/cbm/service_patterns.c`'s route-library tables).
pub fn haskell_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(haskell_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(haskell_call_override)),
    }
}

/// Parse Haskell source through the generic engine.
pub fn parse_haskell(source: &str) -> ParsedFile {
    let spec = LangSpec::haskell();
    let quirks = haskell_quirks();
    let language: tree_sitter::Language = tree_sitter_haskell::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// OCaml
// =====================================================================

/// Recreates a [`Ctx`] for OCaml and recurses into `node`'s own
/// children -- same rationale as [`haskell_walk_scoped`]/
/// [`kotlin_walk_scoped`]: every one of `LangSpec::ocaml`'s
/// `func_types`/`class_types` node kinds needs its name recovered via a
/// child-kind search (never a flat field, see [`LangSpec::ocaml`]'s own
/// doc comment) and none has a single `"body"` field either, so this
/// re-walks the node's own remaining children generically once its name
/// (if any) has been resolved and pushed.
fn ocaml_walk_scoped(
    node: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::ocaml();
    let quirks = ocaml_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, fn_scope);
        }
    }
}

/// The first direct child of `node` whose own `.kind()` is `child_kind`
/// -- OCaml's `value_definition`/`class_definition`/`module_definition`/
/// `exception_definition` all name themselves via one specific,
/// unfielded child rather than a `name`/`pattern`/... field on the
/// outer node itself (see [`LangSpec::ocaml`]'s own doc comment), so
/// every one of this file's OCaml naming helpers below is built on this
/// one shared child-kind search.
fn ocaml_find_child<'a>(node: Node<'a>, child_kind: &str) -> Option<Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == child_kind)
}

/// `let square x = x * x` -- `value_definition`'s own name lives on its
/// `let_binding` child's `"pattern"` field, mirroring the baseline's own
/// dedicated `resolve_ocaml_func_name`
/// (`internal/cbm/extract_defs.c`:253-262) exactly (find the
/// `let_binding` child, then that binding's own `pattern` field).
fn ocaml_value_definition_name<'a>(node: Node<'a>, src: &[u8]) -> Option<(String, Node<'a>)> {
    let binding = ocaml_find_child(node, "let_binding")?;
    let pattern = binding.child_by_field_name("pattern")?;
    let text = pattern.utf8_text(src).ok()?.to_string();
    Some((text, binding))
}

/// `value_definition`'s own symbol + scoped body walk (the `let_binding`
/// child itself, since that is what carries the actual bound
/// expression/parameters -- see [`ocaml_value_definition_name`]).
fn ocaml_value_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some((name, binding)) = ocaml_value_definition_name(node, src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Function,
        line,
    });
    ocaml_walk_scoped(
        binding,
        src,
        None,
        FnScope {
            name: Some(name.as_str()),
            line: Some(line),
        },
        out,
    );
    true
}

/// `Circle of float` (a data constructor) / `exception BadShape` (an
/// exception, itself a bare `constructor_declaration`) -- named by its
/// own unfielded `constructor_name` child (see [`LangSpec::ocaml`]'s own
/// doc comment: this is a real gap the baseline's own resolver never
/// closes either, corrected here per the "more complete than baseline
/// when baseline has a real gap" precedent). Recorded as a
/// [`SymbolKind::Function`] (OCaml has no dedicated "data constructor"
/// `SymbolKind` of its own, and a constructor genuinely is a callable --
/// `Circle 2.0` constructs a value the same syntactic way a function
/// application does, confirmed via [`LangSpec::ocaml`]'s own
/// `application_expression` `constructor`-headed case in this file's
/// probe output) rather than [`SymbolKind::Class`] (already used for
/// the surrounding `type_definition`'s own alias-shaped symbol, one
/// level up, when `type_definition` is classified via
/// [`LangSpec::alias_types`]'s ordinary flat-field path -- a data
/// constructor is a MEMBER of that type, not the type itself). No
/// DEFINES edge to the enclosing type: unlike a class-body method,
/// OCaml's own `type shape = Circle of float | Rectangle of float *
/// float` variant list has no natural "container name" the way a
/// class's own name is -- `type_binding`'s name is read via the
/// generic engine's own [`LangSpec::alias_types`] fallback (a sibling
/// path, not `enclosing`-threaded down into this node), so there is no
/// container name available here to attach a DEFINES edge to without
/// re-deriving it from the parent chain -- left for a future G3 pass
/// rather than guessed at.
fn ocaml_constructor_declaration(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    if let Some(name_node) = ocaml_find_child(node, "constructor_name") {
        if let Ok(name) = name_node.utf8_text(src) {
            out.symbols.push(SymbolRef {
                name: name.to_string(),
                kind: SymbolKind::Function,
                line: node.start_position().row + 1,
            });
        }
    }
    // Not fully claimed: a record-shaped constructor
    // (`Circle of { radius : float }`) nests a `record_declaration`
    // child worth recursing into generically (its own `field_declaration`
    // children carry no DEFINES target here for the same "no natural
    // container name available" reason this function's own doc comment
    // gives, but they are harmless to visit -- there is simply nothing
    // this quirk itself pushes for them).
    false
}

/// `method draw = print_string "draw"` -- named by its own unfielded
/// `method_name` child (see [`LangSpec::ocaml`]'s own doc comment for
/// why this is a real gap the baseline's own resolver never closes
/// either). `enclosing` is always `None` here: `method_definition` only
/// ever appears nested inside an `object_expression` body
/// (`class widget = object ... end`), which is itself a `class_binding`
/// child rather than a [`LangSpec::class_types`]-classified node in its
/// own right (`ocaml_class_definition` below walks straight through
/// `object_expression` generically without setting `enclosing`,
/// mirroring how Rust's own `struct_item` arm never sets `enclosing`
/// when recursing into a struct body either -- see [`LangSpec::rust`]'s
/// own `field_types` doc comment for that precedent), so there is no
/// container name available to attach a DEFINES edge to.
fn ocaml_method_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name_node) = ocaml_find_child(node, "method_name") else {
        return true;
    };
    let Ok(name) = name_node.utf8_text(src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.to_string(),
        kind: SymbolKind::Method,
        line,
    });
    ocaml_walk_scoped(
        node,
        src,
        None,
        FnScope {
            name: Some(name),
            line: Some(line),
        },
        out,
    );
    true
}

/// `class widget = object ... end` -- named by `class_binding`'s own
/// unfielded `class_name` child (two levels of unwrap below
/// `class_definition` itself, see [`LangSpec::ocaml`]'s own doc
/// comment).
fn ocaml_class_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(binding) = ocaml_find_child(node, "class_binding") else {
        return true;
    };
    let Some(name_node) = ocaml_find_child(binding, "class_name") else {
        return true;
    };
    let Ok(name) = name_node.utf8_text(src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.to_string(),
        kind: SymbolKind::Class,
        line,
    });
    ocaml_walk_scoped(binding, src, Some(name), FnScope::default(), out);
    true
}

/// `module Helper = struct ... end` -- named by `module_binding`'s own
/// unfielded `module_name` child (same two-level unwrap shape as
/// [`ocaml_class_definition`]'s own `class_binding`/`class_name`, see
/// [`LangSpec::ocaml`]'s own doc comment). Recorded as
/// [`SymbolKind::Module`] (not [`SymbolKind::Class`], despite living in
/// this row's own `class_types` array for lack of a better generic-array
/// home -- OCaml's `module` really is a namespace/module container, the
/// same construct [`LangSpec::module_types`] models for every other
/// language row in this file, so the symbol kind should say so even
/// though [`LangSpec::ocaml`]'s own `module_types` stays empty for the
/// dead-baseline-data reason its doc comment gives).
fn ocaml_module_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(binding) = ocaml_find_child(node, "module_binding") else {
        return true;
    };
    let Some(name_node) = ocaml_find_child(binding, "module_name") else {
        return true;
    };
    let Ok(name) = name_node.utf8_text(src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.to_string(),
        kind: SymbolKind::Module,
        line,
    });
    ocaml_walk_scoped(binding, src, Some(name), FnScope::default(), out);
    true
}

/// `exception BadShape` / `exception BadShape of string` -- a bare
/// `constructor_declaration` child, reusing
/// [`ocaml_constructor_declaration`]'s own `constructor_name` search
/// directly (see [`LangSpec::ocaml`]'s own doc comment: confirmed via
/// `node-types.json` that `exception_definition.children` includes
/// `constructor_declaration`).
fn ocaml_exception_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(decl) = ocaml_find_child(node, "constructor_declaration") else {
        return true;
    };
    let Some(name_node) = ocaml_find_child(decl, "constructor_name") else {
        return true;
    };
    if let Ok(name) = name_node.utf8_text(src) {
        out.symbols.push(SymbolRef {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line: node.start_position().row + 1,
        });
    }
    true
}

/// `type shape = Circle of float | Rectangle of float * float` --
/// UNLIKE every other node kind this quirk claims, `type_definition`'s
/// own `type_binding` child DOES have a real, working `"name"` field
/// (see [`LangSpec::ocaml`]'s own doc comment) -- this helper exists
/// purely so the generic engine's own [`LangSpec::alias_types`]-shaped
/// fallback (`walk`'s `spec.alias_types.contains(&kind)` arm, which
/// reads `spec.name_field` off `type_definition` ITSELF, not its
/// `type_binding` child) still works: `type_definition` has no `"name"`
/// field of its own at all, only its child does, so this is wired as an
/// `on_unmatched_node` claim the same as every other OCaml node kind in
/// this file, just to perform the one level of descent the generic
/// engine's own field lookup cannot.
fn ocaml_type_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(binding) = ocaml_find_child(node, "type_binding") else {
        return true;
    };
    let Some(name_node) = binding.child_by_field_name("name") else {
        return true;
    };
    let Ok(name) = name_node.utf8_text(src) else {
        return true;
    };
    out.symbols.push(SymbolRef {
        name: name.to_string(),
        kind: SymbolKind::TypeAlias,
        line: node.start_position().row + 1,
    });
    ocaml_walk_scoped(binding, src, Some(name), FnScope::default(), out);
    true
}

/// `open Printf` -- `open_module`'s own `"module"` field (confirmed via
/// `node-types.json`), read directly rather than through the generic
/// engine's own field-fallback chain (which only ever fires for
/// [`LangSpec::class_types`]-shaped nodes, not `import_types`) --
/// mirrors the baseline's own `try_generic_path_fields`'s `"module"`-
/// field case (`internal/cbm/extract_imports.c`:902-917) for the
/// dedicated `parse_generic_imports(ctx, "open_module")` dispatch
/// exactly (`internal/cbm/extract_imports.c`:2809). See
/// [`LangSpec::ocaml`]'s own doc comment for why `include_module` is
/// deliberately left unhandled.
fn ocaml_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let module = node.child_by_field_name("module")?;
    module.utf8_text(src).ok().map(str::to_string)
}

/// Everything OCaml's flat [`LangSpec`] arrays cannot express: every one
/// of `func_types`/`class_types`/`alias_types`'s own child-kind-search
/// naming (see this file's `ocaml_value_definition`/
/// `ocaml_constructor_declaration`/`ocaml_method_definition`/
/// `ocaml_class_definition`/`ocaml_module_definition`/
/// `ocaml_exception_definition`/`ocaml_type_definition`, one arm each)
/// plus `open_module`'s own `"module"`-field path -- wired as this
/// wave's [`Quirks::on_unmatched_node`] hook, fully claiming every one
/// of [`LangSpec::ocaml`]'s own `func_types`/`class_types`/
/// `alias_types`/`import_types` node kinds (same posture as
/// [`kotlin_quirk`]/[`c_quirk`] -- see [`LangSpec::ocaml`]'s own doc
/// comment for why the generic engine's field-name-keyed fallbacks
/// could never reach any of them directly).
fn ocaml_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "value_definition" => ocaml_value_definition(node, src, out),
        "constructor_declaration" => ocaml_constructor_declaration(node, src, out),
        "method_definition" => ocaml_method_definition(node, src, out),
        "class_definition" => ocaml_class_definition(node, src, out),
        "module_definition" => ocaml_module_definition(node, src, out),
        "exception_definition" => ocaml_exception_definition(node, src, out),
        "type_definition" => ocaml_type_definition(node, src, out),
        "open_module" => {
            if let Some(path) = ocaml_import_path(node, src) {
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

/// `x.foo`/`Module.value`-shaped receiver text plus a cheap syntactic
/// hint for [`ocaml_call_override`]'s `method_invocation` arm --
/// `method_invocation.object` is a `_simple_expression`, whose own
/// written text is a reasonable receiver string regardless of its exact
/// sub-shape (mirrors this file's shared [`receiver_of_call`]
/// convention for every other language's method-call-shaped callee).
fn ocaml_method_invocation_receiver(
    object: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    let Ok(text) = object.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "self" {
        ReceiverHint::SelfOrThis
    } else if object.kind() == "value_path" {
        ReceiverHint::Identifier
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Every argument of an `application_expression`, in written order --
/// e.g. `combine 1 2` yields `["1", "2"]`. UNLIKE Haskell's `apply`
/// (whose own `argument` field is singular, `multiple: false`, forcing
/// a curried nested-node chain -- see [`fp_call_head`]/
/// [`fp_collect_args`]'s own doc comments), this grammar's
/// `application_expression` has ONE node with a `multiple: true`
/// `"argument"` field: `combine 1 2` is a SINGLE node (`{function:
/// value_path "combine"}, {argument: number "1"}, {argument: number
/// "2"}` all direct children of the one node), confirmed via a real
/// parse-tree dump after [`fp_call_arg_texts`] (designed around
/// Haskell's curried-nesting shape) silently dropped every argument
/// past the first for this language -- caught by this row's own
/// `tests/unit_languages_ocaml.rs::extracts_multi_arg_curried_application_callee`
/// failing during this wave's own verification, not by inspection (a
/// real grammar-shape divergence between two languages sharing the
/// same "curried function application" surface syntax, exactly the
/// kind of drift this worker's brief warned functional languages are
/// especially likely to have). [`tree_sitter::Node::child_by_field_name`]
/// only ever returns the FIRST matching child for a `multiple: true`
/// field, so this reads every direct child instead, filtering by
/// [`tree_sitter::Node::field_name_for_child`] rather than
/// [`tree_sitter::Node::children_by_field_name`] (which needs a
/// [`tree_sitter::TreeCursor`] this file's other helpers do not
/// otherwise thread through) -- same "iterate `0..child_count()`,
/// check each child's own field/kind directly" convention as
/// [`ocaml_find_child`]/[`zig_container_field_names`] elsewhere in this
/// file.
fn ocaml_application_arg_texts(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        if node.field_name_for_child(i as u32) != Some("argument") {
            continue;
        }
        let Some(child) = node.child(i) else { continue };
        if let Ok(text) = child.utf8_text(src) {
            out.push(text.to_string());
        }
    }
    out
}

/// `application_expression`/`infix_expression`/`method_invocation`/
/// `module_application`/`new_expression` callee reconstruction -- wired
/// as this wave's [`Quirks::call_override`] hook since none fits the
/// generic engine's single `function`/`arguments`-field-pair assumption
/// (see [`LangSpec::ocaml`]'s own doc comment for the field-shape
/// details of each). `application_expression`'s own CALLEE (not its
/// arguments, see [`ocaml_application_arg_texts`]'s own doc comment for
/// why those need a dedicated helper) and `infix_expression` mirror the
/// baseline's own shared `extract_fp_callee` exactly (via
/// [`fp_call_head`], the same helper [`haskell_call_override`] uses,
/// parameterized by this grammar's own `"application_expression"`
/// node-kind string); `method_invocation`/`module_application`/
/// `new_expression` have no baseline-shared helper to mirror (the
/// baseline's own `extract_fp_callee` never matches any of the three)
/// and are handled directly.
fn ocaml_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "application_expression" => {
            // NOT `fp_call_arg_texts` (Haskell's own curried-NESTING
            // shape, `argument` field `multiple: false`) -- a real
            // parse-tree dump of `combine 1 2` proved this grammar's
            // `application_expression` is instead FLAT: one node with a
            // single `"function"` field plus `multiple: true` sibling
            // `"argument"`-field children (`{function: value_path
            // "combine"}, {argument: number "1"}, {argument: number
            // "2"}` all as direct children of the SAME node, no nested
            // `application_expression` at all) -- see
            // `ocaml_application_arg_texts`'s own doc comment. Caught by
            // this row's own
            // `tests/unit_languages_ocaml.rs::extracts_multi_arg_curried_application_callee`
            // failing during this wave's own verification (silently
            // dropped every argument past the first), not by inspection
            // -- [`fp_call_head`] itself is unaffected (it only follows
            // one level when `function` is not itself an
            // `application_expression`, which correctly matches this
            // flat shape's own single, non-recursive `function` field).
            let head = fp_call_head(node, "application_expression");
            let Ok(callee) = head.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: ocaml_application_arg_texts(node, src),
            });
            true
        }
        "infix_expression" => {
            let Some(operator) = node.child_by_field_name("operator") else {
                return false;
            };
            let Ok(callee) = operator.utf8_text(src) else {
                return false;
            };
            let left = node.child_by_field_name("left");
            let right = node.child_by_field_name("right");
            let arg_texts = [left, right]
                .into_iter()
                .flatten()
                .filter_map(|n| n.utf8_text(src).ok().map(str::to_string))
                .collect();
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts,
            });
            true
        }
        "method_invocation" => {
            let Some(method) = node.child_by_field_name("method") else {
                return false;
            };
            let Ok(callee) = method.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = node
                .child_by_field_name("object")
                .map(|object| ocaml_method_invocation_receiver(object, src))
                .unwrap_or((None, None));
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
        "module_application" => {
            let Some(functor) = node.child_by_field_name("functor") else {
                return false;
            };
            let Ok(callee) = functor.utf8_text(src) else {
                return false;
            };
            let arg_texts = node
                .child_by_field_name("argument")
                .and_then(|a| a.utf8_text(src).ok())
                .map(|t| vec![t.to_string()])
                .unwrap_or_default();
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts,
            });
            true
        }
        "new_expression" => {
            let Some(class_path) = ocaml_find_child(node, "class_path") else {
                return false;
            };
            let Ok(callee) = class_path.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: Some(ReceiverHint::NewExpression),
                arg_texts: Vec::new(),
            });
            true
        }
        _ => false,
    }
}

/// OCaml's [`Quirks`] row: every `func_types`/`class_types`/
/// `alias_types` node kind's own child-kind-search naming + scoped body
/// walk, `open_module`'s own `"module"`-field import path, and
/// `application_expression`/`infix_expression`/`method_invocation`/
/// `module_application`/`new_expression` callee reconstruction. No
/// `is_test_name`/`route_from_call`/`on_method_defined`: the baseline
/// wires OCaml no test-name convention, DEFINES-container-from-elsewhere
/// method convention, or route-registration detection at all
/// (`CBM_LANG_OCAML` never appears in
/// `internal/cbm/service_patterns.c`'s route-library tables).
pub fn ocaml_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(ocaml_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(ocaml_call_override)),
    }
}

/// Parse OCaml source through the generic engine -- covers both `.ml`
/// and `.mli` files (see [`LangSpec::ocaml`]'s own doc comment for why
/// this crate, like the baseline, uses the one implementation grammar
/// for both rather than binding `tree-sitter-ocaml`'s separate
/// `LANGUAGE_OCAML_INTERFACE` entry point too).
pub fn parse_ocaml(source: &str) -> ParsedFile {
    let spec = LangSpec::ocaml();
    let quirks = ocaml_quirks();
    let language: tree_sitter::Language = tree_sitter_ocaml::LANGUAGE_OCAML.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Erlang
// =====================================================================

/// `-type shape() :: {circle, float()} | ...` -- `type_alias`'s own
/// `"name"` field points at an intermediate `type_name` wrapper node
/// (`shape()`, including the parenthesized arity/params suffix, e.g.
/// `shape(A, B)` for a parameterized type), not a bare identifier; the
/// real type name text is that wrapper's OWN nested `"name"` field (an
/// `atom` node) -- see [`LangSpec::erlang`]'s own doc comment.
fn erlang_type_alias(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(wrapper) = node.child_by_field_name("name") else {
        return true;
    };
    let Some(atom) = wrapper.child_by_field_name("name") else {
        return true;
    };
    let Ok(name) = atom.utf8_text(src) else {
        return true;
    };
    out.symbols.push(SymbolRef {
        name: name.to_string(),
        kind: SymbolKind::TypeAlias,
        line: node.start_position().row + 1,
    });
    true
}

/// `-import(lists, [sort/1, reverse/1]).` -- `import_attribute`'s own
/// `"module"` field (an `atom`, confirmed via `node-types.json`) is the
/// real dependency-edge text -- see [`LangSpec::erlang`]'s own doc
/// comment for why this row deliberately does NOT reproduce the
/// baseline's own literal `parse_generic_imports(ctx, "module_attribute")`
/// dispatch (a self-referential "this file imports its own name" choice
/// with no useful dependency-edge meaning), instead extracting the
/// semantically real one via this direct field read.
fn erlang_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let module = node.child_by_field_name("module")?;
    module.utf8_text(src).ok().map(str::to_string)
}

/// Everything Erlang's flat [`LangSpec`] arrays cannot express:
/// `type_alias`'s own two-level `"name"`-field unwrap +
/// `import_attribute`'s own `"module"`-field path -- wired as this
/// wave's [`Quirks::on_unmatched_node`] hook. `function_clause`
/// (`LangSpec::erlang`'s own `func_types`) is deliberately NOT matched
/// here: it has real, working `"name"`/`"body"` fields (confirmed via
/// `node-types.json`), so the generic engine's own func/method branch
/// (`walk`'s `spec.func_types.contains(&kind)` arm) already reaches it
/// correctly with no quirk needed at all -- see [`LangSpec::erlang`]'s
/// own doc comment.
fn erlang_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "type_alias" => erlang_type_alias(node, src, out),
        "import_attribute" => {
            if let Some(path) = erlang_import_path(node, src) {
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

/// `call` callee reconstruction -- wired as this wave's
/// [`Quirks::call_override`] hook since `call`'s own `"args"` field
/// points at an intermediate `expr_args` wrapper node (see
/// [`LangSpec::erlang`]'s own doc comment), not a flat argument list the
/// way this file's shared [`call_arg_texts`] helper's single-field
/// convention assumes for languages whose `LangSpec::call_arguments_field`
/// points directly at the argument list itself -- `expr_args`'s own
/// children (parens/commas plus each argument expression) are exactly
/// the shape [`call_arg_texts`] already knows how to skip past, so this
/// reuses it unchanged, just called on `call` (whose `"args"` field
/// happens to point at that intermediate wrapper rather than the list
/// directly the way, say, Rust/Go/TS's own `"arguments"` field does).
/// Mirrors the baseline's own dedicated `extract_erlang_callee`
/// (`internal/cbm/extract_calls.c`:487-492) exactly: a `call` node's
/// callee is simply its own `"expr"` field's text, with no special
/// handling for a `remote`-wrapped qualified call (`io:format(...)`) --
/// see [`LangSpec::erlang`]'s own doc comment for why this row matches
/// that real (qualifier-dropping) baseline depth rather than
/// reconstructing the full `io:format` qualified name.
fn erlang_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "call" {
        return false;
    }
    let Some(expr) = node.child_by_field_name("expr") else {
        return false;
    };
    let Ok(callee) = expr.utf8_text(src) else {
        return false;
    };
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts: call_arg_texts(node, "args", src),
    });
    true
}

/// Erlang's [`Quirks`] row: `type_alias`'s own two-level `"name"`-field
/// unwrap + `import_attribute`'s own `"module"`-field path + `call`
/// callee reconstruction. No `is_test_name`/`route_from_call`/
/// `on_method_defined`: the baseline wires Erlang no test-name
/// convention, DEFINES-container-from-elsewhere method convention, or
/// route-registration detection at all (`CBM_LANG_ERLANG` never appears
/// in `internal/cbm/service_patterns.c`'s route-library tables --
/// Elixir's own `Phoenix.Router` entry there is Elixir-specific, a
/// distinct baseline language this crate does not onboard in this
/// wave).
pub fn erlang_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(erlang_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(erlang_call_override)),
    }
}

/// Parse Erlang source through the generic engine.
pub fn parse_erlang(source: &str) -> ParsedFile {
    let spec = LangSpec::erlang();
    let quirks = erlang_quirks();
    let language: tree_sitter::Language = tree_sitter_erlang::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// Parse CUDA source through the generic engine. G2.2b: reuses
/// [`LangSpec::cpp`]/[`cpp_quirks`] verbatim (see [`LangSpec::cuda`]'s own
/// doc comment for the full grammar-superset finding this relies on) --
/// this function is `parse_cpp` with only the grammar entry point and
/// `LangSpec` row swapped, since every node kind/field
/// [`cpp_quirk`]/[`cpp_call_override`] read is confirmed byte-for-byte
/// identical in this grammar. `is_test_file` mirrors [`parse_cpp`]'s own
/// post-walk Function/Method -> Test reclassification pass exactly (same
/// `tests/`/`_test.cu`-suffix convention as every other C-family
/// language in this crate, wired via [`crate::parsers::is_c_family_test_path`]
/// at the [`crate::parsers::parse_file`] call site).
pub fn parse_cuda(source: &str, is_test_file: bool) -> ParsedFile {
    let spec = LangSpec::cuda();
    let quirks = cpp_quirks();
    let language: tree_sitter::Language = tree_sitter_cuda::LANGUAGE.into();
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

/// D's `import_declaration`/`class_declaration`/... own name-bearing
/// child, found by KIND rather than field (this grammar's own
/// `node-types.json` gives EVERY node kind `"fields": {}` -- see
/// [`LangSpec::d`]'s own doc comment for the full finding) -- the
/// direct-children-only (not recursive-descendant) scan mirrors the
/// baseline's own `cbm_find_child_by_kind` exactly (`internal/cbm/
/// extract_defs.c`'s D-specific branches at :3548/:3660 both use it,
/// never a recursive descendant search).
fn d_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|child| child.kind() == kind)
}

/// A `function_declaration`'s own name -- a plain `identifier` direct
/// child (mirrors the baseline's own `cbm_resolve_func_name`'s
/// `CBM_LANG_DLANG` branch, `internal/cbm/extract_defs.c`:711-720:
/// "the name is a plain `identifier` child"). Returns `None` for
/// `constructor`/`destructor` (D's `this(...)`/`~this()` special-method
/// syntax has no identifier child at all -- see [`LangSpec::d`]'s own
/// doc comment), which [`d_function_like`] records as a nameless
/// [`SymbolKind::Method`] rather than treating as "not found, skip
/// entirely".
fn d_function_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    if node.kind() == "constructor" || node.kind() == "destructor" {
        return None;
    }
    d_child_by_kind(node, "identifier")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// `class Dog : Animal, Serializable` -- every `base_class` direct
/// child holds one heritage name (mirrors the baseline's own dedicated
/// D `extract_base_classes` walker exactly, `internal/cbm/
/// extract_defs.c`:2446-2473: "class_declaration lists one `base_class`
/// child per base").
fn d_base_class_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|child| child.kind() == "base_class")
        .filter_map(|child| child.utf8_text(src).ok().map(str::to_string))
        .collect()
}

/// `module myapp.widgets;` -- the dotted name lives on a `module_fqn`
/// direct child of `module_declaration` (`module` keyword, `module_fqn`,
/// `;`).
fn d_module_fqn_text(node: Node<'_>, src: &[u8]) -> Option<String> {
    d_child_by_kind(node, "module_fqn")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// `import std.stdio;` / `import std.algorithm : map, filter;` -- the
/// module path lives on an `imported` direct child wrapping a
/// `module_fqn` (mirrors the baseline's own `parse_dlang_imports`
/// exactly, `internal/cbm/extract_imports.c`:2361-2381: find the first
/// `module_fqn` descendant of the `import_declaration`, read its text).
/// Selective-import `import_bind` siblings (`: map, filter`) are not
/// additionally chased, matching the baseline's own identical scope.
fn d_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let imported = d_child_by_kind(node, "imported")?;
    d_child_by_kind(imported, "module_fqn")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// `int x;` inside an `aggregate_body` -- `variable_declaration`'s own
/// name lives on a `declarator` direct child wrapping a bare
/// `identifier` (confirmed in a real parse tree: `variable_declaration {
/// type, declarator { identifier "x" }, ; }`).
fn d_variable_declarator_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let declarator = d_child_by_kind(node, "declarator")?;
    d_child_by_kind(declarator, "identifier")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// `function_declaration`/`constructor`/`destructor`'s own symbol +
/// DEFINES-scoped body walk -- mirrors the generic engine's own
/// func/method branch but reimplemented directly because this
/// grammar's body is an UNFIELDED `function_body` child (see
/// [`LangSpec::d`]'s own doc comment), which [`LangSpec::body_field`]'s
/// generic `child_by_field_name` lookup can never find.
fn d_function_like(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let line = node.start_position().row + 1;
    let name = d_function_name(node, src);
    let kind = if enclosing.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };
    if let Some(name) = &name {
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
    } else {
        // `constructor`/`destructor`: no name to recover -- still record
        // a nameless Method (mirrors `kotlin_function_like`'s
        // `secondary_constructor` case, also unnamed in ITS grammar) so
        // the body is still walked for nested calls, and still gets a
        // DEFINES edge from the enclosing class/struct if any.
        out.symbols.push(SymbolRef {
            name: String::new(),
            kind: SymbolKind::Method,
            line,
        });
    }
    if let Some(body) = d_child_by_kind(node, "function_body") {
        d_walk_scoped(
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

/// `class_declaration`/`struct_declaration`/`union_declaration`/
/// `interface_declaration`'s own symbol + heritage + DEFINES-scoped body
/// walk -- same unfielded-body rationale as [`d_function_like`].
fn d_class_like(node: Node<'_>, kind: SymbolKind, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = d_child_by_kind(node, "identifier").and_then(|n| n.utf8_text(src).ok()) else {
        return true;
    };
    let name = name.to_string();
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind,
        line,
    });
    for super_name in d_base_class_names(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: name.clone(),
            super_name,
            line,
        });
    }
    if let Some(body) = d_child_by_kind(node, "aggregate_body") {
        d_walk_scoped(body, src, Some(name.as_str()), FnScope::default(), out);
    }
    true
}

/// `module_declaration`'s own Module symbol -- `module myapp.widgets;`
/// pushes one [`SymbolKind::Module`] named after the FULL dotted
/// `module_fqn` text (`"myapp.widgets"`), not just its last segment
/// (matches every other language row's `module_types`-driven convention
/// of using the whole written name, e.g. C++'s `namespace geometry {}`).
fn d_module_declaration(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    if let Some(name) = d_module_fqn_text(node, src) {
        out.symbols.push(SymbolRef {
            name,
            kind: SymbolKind::Module,
            line: node.start_position().row + 1,
        });
    }
    // Not `true`: `module_declaration` has no children worth
    // recursing into beyond its own `module_fqn` (already consumed
    // above) and the `module`/`;` keyword tokens -- but returning
    // `false` here is harmless (ordinary recursion finds nothing new)
    // and keeps this arm consistent with every other "record, then let
    // the walker's own generic recursion continue" quirk arm in this
    // file that has no scoped-body concern of its own.
    false
}

/// D's `aggregate_body`/`function_body`'s own scoped recursion
/// (`enclosing`/`fn_scope` threaded through explicitly) -- mirrors every
/// other quirk's `*_walk_scoped`/`*_walk_scoped_body` helper in this file
/// (`kotlin_walk_scoped`/`ts_walk_scoped_body`/`py_walk_scoped_body`),
/// needed because D's body is an unfielded child the generic engine's
/// own body-walk (keyed off [`LangSpec::body_field`]) can never reach.
fn d_walk_scoped(
    body: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::d();
    let quirks = d_quirks();
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

/// Everything D's flat [`LangSpec`] arrays cannot express (no tree-sitter
/// fields anywhere in this grammar at all -- see [`LangSpec::d`]'s own
/// doc comment) -- wired as this wave's [`Quirks::on_unmatched_node`]
/// hook. Fully claims every one of [`LangSpec::d`]'s `func_types`/
/// `method_types`/`class_types`/`field_types`/`import_types` node kinds
/// (same posture as [`c_quirk`]/[`cpp_quirk`]/[`kotlin_quirk`]).
fn d_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_declaration" | "constructor" | "destructor" => {
            d_function_like(node, enclosing, src, out)
        }
        "class_declaration" => d_class_like(node, SymbolKind::Class, src, out),
        "struct_declaration" => d_class_like(node, SymbolKind::Struct, src, out),
        "union_declaration" => d_class_like(node, SymbolKind::Class, src, out),
        "interface_declaration" => d_class_like(node, SymbolKind::Interface, src, out),
        "enum_declaration" => {
            if let Some(name) =
                d_child_by_kind(node, "identifier").and_then(|n| n.utf8_text(src).ok())
            {
                out.symbols.push(SymbolRef {
                    name: name.to_string(),
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
            // Not fully claimed: an enum's own members (`enum_member`
            // children) carry no further nested defs/calls worth a
            // dedicated scoped walk (matches every other language row's
            // enum handling, e.g. `LangSpec::rust`'s plain `enum_item`
            // flat-array path, which also does not scope into the enum
            // body specially) -- ordinary recursion suffices.
            false
        }
        "variable_declaration" => {
            if let (Some(container), Some(name)) =
                (enclosing, d_variable_declarator_name(node, src))
            {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name,
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "module_declaration" => d_module_declaration(node, src, out),
        "import_declaration" => {
            if let Some(path) = d_import_path(node, src) {
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

/// D's `call_expression` has NO fields at all (see [`LangSpec::d`]'s own
/// doc comment) -- wired as this wave's [`Quirks::call_override`] hook,
/// mirroring [`kotlin_call_override`]'s posture of fully claiming the
/// one `call_types` entry directly rather than relying on the generic
/// engine's own single-field `call_function_field` reconstruction. The
/// callee is always the node's own FIRST child (an `identifier` for a
/// plain call, `super` for a super-constructor call, a `type` node for a
/// receiver-qualified member call, or a `new_expression` for a
/// constructor call); the argument list is a `named_arguments` sibling
/// (NOT a field named `"arguments"`).
fn d_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(callee_node) = node.child(0) else {
        return false;
    };
    let Ok(callee) = callee_node.utf8_text(src) else {
        return false;
    };
    let arg_texts = d_child_by_kind(node, "named_arguments")
        .map(|args| {
            (0..args.child_count())
                .filter_map(|i| args.child(i))
                .filter(|c| c.is_named())
                .filter_map(|c| c.utf8_text(src).ok().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts,
    });
    true
}

/// D's [`Quirks`] row: unfielded function/class bodies, `identifier`-
/// child-based naming (no fields anywhere in this grammar), `base_class`
/// heritage (mirrors the baseline's own dedicated D
/// `extract_base_classes` walker), `module_fqn`-based module/import
/// paths, and full `call_expression` claiming (no fields at all). No
/// `route_from_call`: this crate has no D web-framework route
/// convention wired (the baseline's own `service_patterns.c` route table
/// has no D-specific library-name entry either), matching every G2.1/
/// G2.2 scripting/systems-language row's identical `route_from_call:
/// None` posture for a language with no route signal of its own (see
/// e.g. [`elixir_quirks`]'s own doc comment for an analogous language
/// missing this signal for the identical "the baseline itself has
/// nothing here" reason).
pub fn d_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(d_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(d_call_override)),
    }
}

/// Parse D source through the generic engine. Grammar: `tree-sitter-d`
/// (crates.io, `gdamore/tree-sitter-d`) -- no pre-existing bespoke
/// `languages::d` extractor to prove zero-regression against (D has
/// never had one in this crate); correctness is verified directly
/// against this crate's own `node-types.json` plus real parse trees (see
/// [`LangSpec::d`]'s doc comment and `tests/unit_languages_d.rs`).
pub fn parse_d(source: &str) -> ParsedFile {
    let spec = LangSpec::d();
    let quirks = d_quirks();
    let language: tree_sitter::Language = tree_sitter_d::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// PowerShell's `script_block`/`class_method_definition`/... own body,
/// found by KIND (this grammar gives `function_statement`/
/// `class_method_definition` no fields at all -- see
/// [`LangSpec::powershell`]'s own doc comment) -- unlike D, a direct
/// child search is not enough here: the real body content is TWO
/// unfielded wrapper levels deep (`script_block > script_block_body >
/// statement_list`, confirmed in a real parse tree), so this walks down
/// through both wrapper kinds by name rather than a single-level
/// by-kind child scan.
fn ps_function_body<'a>(node: Node<'a>) -> Option<Node<'a>> {
    let script_block = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|child| child.kind() == "script_block")?;
    (0..script_block.child_count())
        .filter_map(|i| script_block.child(i))
        .find(|child| child.kind() == "script_block_body")
}

/// `function_statement`'s own name -- a `function_name` direct child
/// (mirrors the baseline's own `cbm_resolve_func_name`'s dedicated
/// `CBM_LANG_POWERSHELL` branch, `internal/cbm/extract_defs.c`:702-707
/// exactly: "the name is a `function_name` child node").
fn ps_function_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|child| child.kind() == "function_name")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// `class_statement`/`enum_statement`'s own name -- the FIRST
/// `simple_name` direct child (mirrors the baseline's own dedicated
/// `CBM_LANG_POWERSHELL` name-resolution branches,
/// `internal/cbm/extract_defs.c`:3553-3555/:3666-3668 exactly: "the name
/// is the FIRST `simple_name` child").
fn ps_first_simple_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|child| child.kind() == "simple_name")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// `class Dog : Animal` -- every `simple_name` child AFTER the first `:`
/// token is a base name, stopping at the class body's own opening `{`
/// (mirrors the baseline's own dedicated PowerShell
/// `extract_base_classes` walker exactly, `internal/cbm/
/// extract_defs.c`:2477-2510: "collect every `simple_name` that appears
/// after the first `:` token, stop at `{`"). The class's OWN name (the
/// first `simple_name`, before the `:`) is correctly excluded since the
/// `seen_colon` gate below only starts collecting after the colon.
fn ps_class_base_names(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen_colon = false;
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            ":" => seen_colon = true,
            "{" => break,
            "simple_name" if seen_colon => {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

/// `class_method_definition`'s own name -- its FIRST `simple_name`
/// direct child (this kind's own `simple_name` child -- the method's own
/// identifier -- always precedes its `class_method_parameter_list`/
/// `script_block` siblings in a real parse tree, mirroring the same
/// "first matching child wins" convention [`ps_first_simple_name`]
/// already uses for `class_statement`/`enum_statement`).
fn ps_method_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    ps_first_simple_name(node, src)
}

/// `function_statement`'s own symbol + DEFINES-scoped body walk --
/// mirrors the generic engine's own func/method branch but reimplemented
/// directly because this grammar's body is TWO unfielded wrapper levels
/// deep (see [`ps_function_body`]).
fn ps_function_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let line = node.start_position().row + 1;
    let name = ps_function_name(node, src);
    if let Some(name) = &name {
        out.symbols.push(SymbolRef {
            name: name.clone(),
            kind: SymbolKind::Function,
            line,
        });
    }
    if let Some(body) = ps_function_body(node) {
        ps_walk_scoped(
            body,
            src,
            None,
            FnScope {
                name: name.as_deref(),
                line: Some(line),
            },
            out,
        );
    }
    true
}

/// `class_method_definition`'s own symbol + DEFINES edge + body walk --
/// same unfielded-body rationale as [`ps_function_like`], but always a
/// [`SymbolKind::Method`] (a class-body method, never a free function)
/// with `enclosing` set to the containing class's own name.
fn ps_method_like(node: Node<'_>, enclosing: &str, src: &[u8], out: &mut ParsedFile) {
    let line = node.start_position().row + 1;
    let name = ps_method_name(node, src);
    if let Some(name) = &name {
        out.symbols.push(SymbolRef {
            name: name.clone(),
            kind: SymbolKind::Method,
            line,
        });
        out.defines.push(DefinesRef {
            container_name: enclosing.to_string(),
            member_name: name.clone(),
            line,
        });
    }
    if let Some(body) = ps_function_body(node) {
        ps_walk_scoped(
            body,
            src,
            Some(enclosing),
            FnScope {
                name: name.as_deref(),
                line: Some(line),
            },
            out,
        );
    }
}

/// `class_statement`'s own symbol + heritage + class-body walk (dispatch
/// to [`ps_method_like`] for each `class_method_definition` child; a
/// `class_property_definition` child is deliberately NOT turned into a
/// DEFINES edge, matching the baseline's own real absent depth here --
/// see [`LangSpec::powershell`]'s own doc comment).
fn ps_class_like(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = ps_first_simple_name(node, src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: SymbolKind::Class,
        line,
    });
    for super_name in ps_class_base_names(node, src) {
        out.inherits.push(InheritsRef {
            sub_name: name.clone(),
            super_name,
            line,
        });
    }
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() == "class_method_definition" {
            ps_method_like(child, name.as_str(), src, out);
        }
    }
    true
}

/// PowerShell's `script_block_body`/`statement_list`'s own scoped
/// recursion (`enclosing`/`fn_scope` threaded through explicitly) --
/// mirrors every other quirk's `*_walk_scoped`/`*_walk_scoped_body`
/// helper in this file, needed because PowerShell's body is an unfielded
/// wrapper chain the generic engine's own body-walk (keyed off
/// [`LangSpec::body_field`]) can never reach.
fn ps_walk_scoped(
    body: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::powershell();
    let quirks = ps_quirks();
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

/// Everything PowerShell's flat [`LangSpec`] arrays cannot express
/// (unfielded function/class-method bodies, by-kind-child-based naming,
/// `class_statement` heritage) -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. Fully claims
/// [`LangSpec::powershell`]'s `func_types`/`method_types`/`class_types`/
/// `enum_types` node kinds (same posture as [`c_quirk`]/[`d_quirk`]).
fn ps_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_statement" => ps_function_like(node, src, out),
        "class_statement" => ps_class_like(node, src, out),
        "enum_statement" => {
            if let Some(name) = ps_first_simple_name(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Enum,
                    line: node.start_position().row + 1,
                });
            }
            // Not fully claimed: an enum's own `enum_member` children
            // carry no further nested defs/calls worth a dedicated
            // scoped walk (matches every other language row's plain
            // enum handling) -- ordinary recursion suffices.
            false
        }
        _ => false,
    }
}

/// The LAST `generic_token` descendant of `node` whose own start byte is
/// strictly after `skip_before`, excluding the `module`/`namespace`/
/// `assembly` keyword tokens themselves -- mirrors the baseline's own
/// `parse_powershell_imports` exactly (`internal/cbm/
/// extract_imports.c`:1587-1618: "find the last `generic_token` anywhere
/// under the command", skipping those three specific keyword tokens).
/// A `command`'s own `command_elements` field is the natural place to
/// scan from (excludes the leading `command_name` -- the `"using"` token
/// itself never collides with a real keyword/path token this way), but
/// this walks `command_elements`' full subtree (not just its direct
/// children) since a nested `sub_expression`/`variable` could in
/// principle wrap a `generic_token` one level deeper -- matching the
/// baseline's own recursive stack-based descendant scan rather than a
/// direct-children-only one.
fn ps_last_generic_token_text<'a>(node: Node<'a>, src: &'a [u8]) -> Option<&'a str> {
    let mut best: Option<Node<'a>> = None;
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "generic_token" {
            if let Ok(text) = current.utf8_text(src) {
                if !matches!(text, "module" | "namespace" | "assembly") {
                    let better = best.is_none_or(|prev| current.start_byte() > prev.start_byte());
                    if better {
                        best = Some(current);
                    }
                }
            }
        }
        for i in 0..current.child_count() {
            if let Some(child) = current.child(i) {
                stack.push(child);
            }
        }
    }
    best.and_then(|n| n.utf8_text(src).ok())
}

/// `command`'s own callee + args, plus `using namespace ...`/`using
/// module ...` IMPORTS detection -- wired as this wave's
/// [`Quirks::call_override`] hook (this kind's own `command_name` FIELD
/// is real and usable directly, unlike the baseline's own manual
/// named-child scan -- see [`LangSpec::powershell`]'s own doc comment --
/// but its arguments are bare `command_elements` children, not a
/// parenthesized field, so the whole call still needs a full override
/// rather than the generic engine's flat `call_arguments_field`
/// mechanism). `invokation_expression` (`$d.Speak()`) is claimed too,
/// since neither kind has a usable single flat field pair.
fn ps_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "command" => {
            let Some(command_name) = node.child_by_field_name("command_name") else {
                return false;
            };
            let Ok(callee) = command_name.utf8_text(src) else {
                return false;
            };
            if callee == "using" {
                if let Some(elements) = node.child_by_field_name("command_elements") {
                    if let Some(path) = ps_last_generic_token_text(elements, src) {
                        out.imports.push(ImportRef {
                            module_path: path.to_string(),
                            line: node.start_position().row + 1,
                        });
                    }
                }
            }
            let arg_texts = node
                .child_by_field_name("command_elements")
                .map(|elements| {
                    (0..elements.child_count())
                        .filter_map(|i| elements.child(i))
                        .filter(|c| c.is_named() && c.kind() != "command_argument_sep")
                        .filter_map(|c| c.utf8_text(src).ok().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts,
            });
            true
        }
        "invokation_expression" => {
            let Some(receiver) = node.child(0) else {
                return false;
            };
            let Some(member_name) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|child| child.kind() == "member_name")
            else {
                return false;
            };
            let Ok(callee) = member_name.utf8_text(src) else {
                return false;
            };
            let Ok(receiver_text) = receiver.utf8_text(src) else {
                return false;
            };
            let hint = if receiver_text == "$this" {
                ReceiverHint::SelfOrThis
            } else if receiver.kind() == "variable" {
                ReceiverHint::Identifier
            } else {
                ReceiverHint::Other
            };
            let arg_texts = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|child| child.kind() == "argument_list")
                .map(|args| {
                    (0..args.child_count())
                        .filter_map(|i| args.child(i))
                        .filter(|c| c.is_named())
                        .filter_map(|c| c.utf8_text(src).ok().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: Some(receiver_text.to_string()),
                receiver_hint: Some(hint),
                arg_texts,
            });
            true
        }
        _ => false,
    }
}

/// PowerShell's [`Quirks`] row: unfielded function/class-method bodies,
/// by-kind-child-based naming (no `name` field anywhere in this
/// grammar's own function/class/enum node kinds), `class_statement`
/// heritage (mirrors the baseline's own dedicated PowerShell
/// `extract_base_classes` walker), and full `command`/
/// `invokation_expression` call claiming plus `using`-directive IMPORTS
/// detection (mirrors the baseline's own `parse_powershell_imports`). No
/// `route_from_call`: this crate has no PowerShell web-framework route
/// convention wired (the baseline's own `service_patterns.c` route table
/// has no PowerShell-specific library-name entry either), same posture
/// as [`d_quirks`]'s own doc comment for an analogous language missing
/// this signal for the identical "the baseline itself has nothing here"
/// reason.
pub fn ps_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(ps_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(ps_call_override)),
    }
}

/// Parse PowerShell source through the generic engine. Grammar:
/// `tree-sitter-powershell` (crates.io, `airbus-cert/
/// tree-sitter-powershell`) -- no pre-existing bespoke
/// `languages::powershell` extractor to prove zero-regression against
/// (PowerShell has never had one in this crate); correctness is verified
/// directly against this crate's own `node-types.json` plus real parse
/// trees (see [`LangSpec::powershell`]'s doc comment and
/// `tests/unit_languages_powershell.rs`).
pub fn parse_powershell(source: &str) -> ParsedFile {
    let spec = LangSpec::powershell();
    let quirks = ps_quirks();
    let language: tree_sitter::Language = tree_sitter_powershell::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// F#
// =====================================================================

/// Join a `long_identifier`'s (or a bare `identifier`'s) own dot-joined
/// segments into one written-as-source string -- shared by every F#
/// name-resolution site that reads a `long_identifier`
/// (`named_module`/`namespace`'s own `name` field, `import_decl`'s
/// positional child, `class_inherits_decl`'s base-type child when it is
/// a dotted path, and `type_name`'s own `type_name` field when it holds
/// a `long_identifier` rather than a bare `identifier`) -- same
/// "join a grammar's own unfielded identifier-list wrapper" pattern as
/// [`objc_join_selector_parts`], specialized to F#'s `.`-separator
/// convention instead of Objective-C's `:`.
fn fsharp_long_identifier_text(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "op_identifier" => node.utf8_text(src).ok().map(str::to_string),
        "long_identifier" => {
            let mut parts = Vec::new();
            for i in 0..node.child_count() {
                let Some(child) = node.child(i) else {
                    continue;
                };
                if child.kind() == "identifier" {
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
        _ => None,
    }
}

/// A `function_declaration_left`/`value_declaration_left` node's own
/// name: neither has a `name` FIELD (both `{"fields":{}}` in this
/// grammar's own `node-types.json`) -- the name is whichever direct
/// child is an `identifier`/`op_identifier`/`active_pattern`, scanned in
/// written order (mirrors [`objc_method_selector`]'s identical
/// "direct-children-only scan for the unfielded name" shape). A
/// `value_declaration_left` binding a destructuring pattern rather than
/// a bare name (`let (a, b) = ...`) has no `identifier` direct child at
/// all -- returns `None`, same "silently produce no symbol for an
/// unnamed/pattern binding" posture every other quirk in this file
/// already has for an analogous case (e.g. [`zig_quirk`]'s anonymous
/// active-pattern skip).
fn fsharp_declaration_left_name(left_node: Node<'_>, src: &[u8]) -> Option<String> {
    for i in 0..left_node.child_count() {
        let Some(child) = left_node.child(i) else {
            continue;
        };
        match child.kind() {
            "identifier" | "op_identifier" => {
                return child.utf8_text(src).ok().map(str::to_string);
            }
            "active_pattern" => {
                // `(|Even|Odd|)`-style active-pattern definitions: take
                // the node's own full written text (`|Even|Odd|`,
                // including the delimiting pipes) as its name -- there is
                // no bare-identifier sub-name to prefer, and this at
                // least gives a stable, non-empty symbol name rather
                // than silently dropping every active-pattern def.
                return child.utf8_text(src).ok().map(str::to_string);
            }
            _ => {}
        }
    }
    None
}

/// A `function_or_value_defn`'s inner `function_declaration_left`/
/// `value_declaration_left` child, if any (a `let rec`/`and`-chained
/// defn can wrap several via `_function_or_value_defns`, but this crate
/// only resolves the single, common `let name ... = body` shape --
/// matches this row's own "baseline's real depth, not idealized" bar
/// for `member_defn`/nested-OOP scope).
fn fsharp_declaration_left(defn_node: Node<'_>) -> Option<Node<'_>> {
    for i in 0..defn_node.child_count() {
        let child = defn_node.child(i)?;
        if matches!(
            child.kind(),
            "function_declaration_left" | "value_declaration_left"
        ) {
            return Some(child);
        }
    }
    None
}

/// An `anon_type_defn`/`record_type_defn`/`union_type_defn`/
/// `enum_type_defn`/`type_abbrev_defn`'s own name: find its positional
/// `type_name` child, then read THAT node's own `type_name` field (this
/// grammar reuses the literal string `"type_name"` as both a node kind
/// AND that same node kind's own field name -- see
/// [`crate::languages::spec::LangSpec::fsharp`]'s doc comment) -- the
/// field's value is either a bare `identifier` (common case) or a
/// `long_identifier` (a dotted/module-qualified type name), both
/// resolved via [`fsharp_long_identifier_text`].
fn fsharp_container_name(container_node: Node<'_>, src: &[u8]) -> Option<String> {
    let type_name_node = (0..container_node.child_count())
        .filter_map(|i| container_node.child(i))
        .find(|n| n.kind() == "type_name")?;
    let name_node = type_name_node.child_by_field_name("type_name")?;
    fsharp_long_identifier_text(name_node, src)
}

/// `inherit Base(...)`'s own base-type name -- mirrors
/// `internal/cbm/extract_defs.c`'s dedicated `CBM_LANG_FSHARP` branch of
/// `extract_base_classes` (:2429-2440) exactly: find the first
/// `class_inherits_decl` descendant, then its own `simple_type` child,
/// then take that node's own written text. `find_first_descendant`
/// mirrors the baseline's `find_first_descendant_by_kind` (a bounded-
/// depth DFS in the baseline; this crate has no existing shared
/// bounded-depth-DFS helper for the analogous case, so it recurses over
/// the WHOLE subtree instead -- a `record_type_defn`/`union_type_defn`+
/// body is never deep enough for this distinction to matter in practice,
/// and this crate's engine has no other bounded-depth-limited helper
/// anywhere else to match against for precedent either way).
fn fsharp_find_descendant<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        if let Some(found) = fsharp_find_descendant(child, kind) {
            return Some(found);
        }
    }
    None
}

fn fsharp_inherits_base_name(container_node: Node<'_>, src: &[u8]) -> Option<String> {
    let inh = fsharp_find_descendant(container_node, "class_inherits_decl")?;
    let simple_type = (0..inh.child_count())
        .filter_map(|i| inh.child(i))
        .find(|n| n.kind() == "simple_type")?;
    simple_type.utf8_text(src).ok().map(str::to_string)
}

/// `anon_type_defn`/`record_type_defn`/`union_type_defn`/
/// `enum_type_defn`/`type_abbrev_defn`'s own scoped recursion once its
/// name (if any) has been resolved via [`fsharp_container_name`] -- none
/// of these five kinds is one of `LangSpec::fsharp()`'s `func_types`/
/// `method_types` (so the generic walker's own DEFINES-scoped
/// body-walk-on-symbol-push never runs for them), and all five are fully
/// claimed by [`fsharp_quirk`] before the generic class-shape fallback's
/// own recursion would ever run, so this quirk re-implements that one
/// recursive call directly -- same shape as [`zig_walk_container_body`]/
/// [`objc_walk_scoped`].
fn fsharp_walk_container_body(
    node: Node<'_>,
    src: &[u8],
    name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::fsharp();
    let quirks = fsharp_quirks();
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

/// A `function_or_value_defn`'s own `body` field: every call inside it
/// needs `fn_scope` set to this defn's own name/line -- same rationale
/// as [`c_walk_function_body`]/[`objc_walk_method_body`]
/// (`function_or_value_defn` is not one of `LangSpec::fsharp()`'s
/// func/method arrays' the generic engine's own body-walk would
/// otherwise handle, since `LangSpec::fsharp()`'s `body_field` is a
/// placeholder never consulted -- `fsharp_quirk` claims the whole node
/// instead). Walks `body` DIRECTLY (a single [`walk`] call on the node
/// itself), NOT its children -- a real, test-caught bug this function
/// originally had iterated `body.child_count()` instead, copying the
/// "walk a block/compound-statement wrapper's own children" shape every
/// OTHER language's analogous helper correctly uses for THEIR grammars
/// (C's `compound_statement`, Objective-C's `compound_statement`, both
/// always real multi-statement block wrappers) -- but F#'s
/// `function_or_value_defn` `body` field is NOT always a block wrapper:
/// a single-expression let-binding (`let draw x = helper x`, no further
/// statements) has its `body` field point DIRECTLY AT the one
/// expression itself (`application_expression`, verified via a real
/// parse tree dump), so iterating that node's own children treated
/// `helper`/`x` (the call's own callee/argument sub-nodes) as if they
/// were independent top-level statements, silently walking past the
/// `application_expression` node itself (and therefore never invoking
/// [`fsharp_call_override`] on it at all) -- caught by
/// `tests/unit_languages_fsharp.rs::extracts_simple_application_call`
/// failing (no callee found) despite an earlier, larger fixture
/// happening to still pass (that fixture's own body was a
/// `sequential_expression` wrapping MULTIPLE statements, whose own
/// children genuinely are the top-level expressions -- the multi-
/// statement case accidentally worked, masking the single-expression
/// case's real breakage). `walk` itself already recurses into
/// `application_expression`'s own children generically after
/// `fsharp_call_override` runs (see the shared `walk` function's own
/// `call_types` branch), so calling `walk` on `body` once here is both
/// necessary and sufficient for either body shape.
fn fsharp_walk_defn_body(
    body: Node<'_>,
    src: &[u8],
    name: Option<&str>,
    line: usize,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::fsharp();
    let quirks = fsharp_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name,
        line: Some(line),
    };
    walk(body, &ctx, out, None, fn_scope);
}

/// Everything F#'s flat `LangSpec` arrays cannot express:
/// `function_or_value_defn` signature-node/body-field split (naming via
/// the inner `function_declaration_left`/`value_declaration_left`,
/// walking the OUTER node's own `body` field with `fn_scope` correctly
/// set), `anon_type_defn`/`record_type_defn`/`union_type_defn`/
/// `enum_type_defn`/`type_abbrev_defn` positional `type_name` naming +
/// `inherit Base(...)` INHERITS + DEFINES-scoped body walk,
/// `named_module`/`namespace` dot-joined naming, and `import_decl`
/// positional `long_identifier` IMPORTS -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. `application_expression`'s full
/// callee-head reconstruction is a separate [`Quirks::call_override`]
/// hook (see [`fsharp_call_override`]).
fn fsharp_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "function_or_value_defn" => {
            let Some(left) = fsharp_declaration_left(node) else {
                return true;
            };
            let Some(name) = fsharp_declaration_left_name(left, src) else {
                return true;
            };
            let line = node.start_position().row + 1;
            let kind = if left.kind() == "function_declaration_left" {
                SymbolKind::Function
            } else {
                SymbolKind::Variable
            };
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind,
                line,
            });
            if let Some(body) = node.child_by_field_name("body") {
                fsharp_walk_defn_body(body, src, Some(name.as_str()), line, out);
            }
            true
        }
        "anon_type_defn" | "record_type_defn" | "union_type_defn" | "enum_type_defn"
        | "type_abbrev_defn" => {
            let name = fsharp_container_name(node, src);
            if let Some(name) = &name {
                let line = node.start_position().row + 1;
                let kind = if node.kind() == "enum_type_defn" {
                    SymbolKind::Enum
                } else if node.kind() == "type_abbrev_defn" {
                    SymbolKind::TypeAlias
                } else {
                    SymbolKind::Class
                };
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                if let Some(base_name) = fsharp_inherits_base_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: base_name,
                        line,
                    });
                }
            }
            fsharp_walk_container_body(node, src, name.as_deref(), out);
            true
        }
        "named_module" | "namespace" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| fsharp_long_identifier_text(n, src));
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
            // Not fully claimed for recursion purposes: neither kind
            // needs its own re-scoped walk the way a class/type-defn
            // container does (F# module members are not "DEFINES"-linked
            // the way a class's own methods/fields are in this crate's
            // model -- matches [`Self::fsharp`]'s own doc comment on why
            // `module_defn`'s nested case is left unhandled too) --
            // falling through to the walker's own generic recursion
            // (`false`) still visits every nested declaration/call inside
            // correctly, just without a Module-named `enclosing` scope.
            false
        }
        "import_decl" => {
            let path = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| matches!(n.kind(), "long_identifier" | "identifier"))
                .and_then(|n| fsharp_long_identifier_text(n, src));
            if let Some(path) = path {
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

/// `application_expression`'s full callee reconstruction -- wired as
/// [`Quirks::call_override`] since this grammar's curried-application
/// shape has no single `call_function_field` the generic engine's own
/// default reconstruction could read (see
/// [`crate::languages::spec::LangSpec::fsharp`]'s own doc comment for the
/// full rationale). Mirrors `internal/cbm/extract_calls.c`'s own
/// `extract_fsharp_callee` (:514-523) exactly: only ever inspects this
/// node's own first named child, recognizing it as a callee ONLY when
/// its kind is `long_identifier_or_op`/`long_identifier`/`identifier` --
/// any other head shape (in particular, a NESTED `application_expression`
/// head from a genuinely curried multi-argument call) returns `false`
/// (no CALLS edge recorded), matching that baseline function's own real,
/// narrow depth rather than improving on it.
fn fsharp_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "application_expression" {
        return false;
    }
    let Some(head) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.is_named())
    else {
        return false;
    };
    if !matches!(
        head.kind(),
        "long_identifier_or_op" | "long_identifier" | "identifier"
    ) {
        return false;
    }
    let Ok(callee) = head.utf8_text(src) else {
        return false;
    };
    // The application's own second positional child is its single
    // argument -- mirrors this grammar's own `_high_prec_app`/
    // `_low_prec_app` two-slot shape (`[callee, arg]`); a genuinely
    // multi-argument curried call is out of scope for the SAME reason
    // the callee-head check above already is (see this function's own
    // doc comment). `unit` (a literal `f ()` zero-argument call's second
    // slot, confirmed via a real parse tree dump: `helper()` yields
    // `application_expression[long_identifier_or_op "helper", unit
    // "()"]` with the `unit` node itself having no children of its own)
    // is excluded from `arg_texts` so a true zero-argument call gets an
    // EMPTY arg list, matching every other language's own "empty parens
    // means empty arg_texts" convention (e.g. Rust/Go/TS's own
    // `call_arguments_field`-keyed reads naturally yield `[]` for `f()`)
    // rather than a synthetic `"()"` literal-text entry.
    let arg_texts: Vec<String> = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|n| n.is_named() && *n != head && n.kind() != "unit")
        .filter_map(|n| n.utf8_text(src).ok().map(str::to_string))
        .collect();
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts,
    });
    true
}

/// F#'s [`Quirks`] row: `function_or_value_defn` signature/body-split
/// naming, type-defn positional naming + `inherit` INHERITS + DEFINES-
/// scoped body walk, module/namespace dot-joined naming, `import_decl`
/// IMPORTS, and `application_expression` callee-head reconstruction. No
/// `is_test_name`/`route_from_call`/`on_method_defined`: the baseline
/// wires F# no test-name convention or route-registration detection at
/// all (`CBM_LANG_FSHARP` never appears in
/// `internal/cbm/service_patterns.c`'s route-library tables, and its own
/// `lang_specs.c` row has no dedicated test-name column populated
/// either), and F# has no method-vs-DEFINES-container distinction this
/// row's naming quirk does not already fully resolve inline (there is
/// no receiver-clause-style method binding the way Go's is).
pub fn fsharp_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(fsharp_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(fsharp_call_override)),
    }
}

/// Parse F# source through the generic engine.
pub fn parse_fsharp(source: &str) -> ParsedFile {
    let spec = LangSpec::fsharp();
    let quirks = fsharp_quirks();
    let language: tree_sitter::Language = tree_sitter_fsharp::LANGUAGE_FSHARP.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Gleam
// =====================================================================

/// A `type_definition`/`type_alias` node's own name: neither has a
/// `name` FIELD of its own (both `{"fields":{}}` in this grammar's own
/// `node-types.json`) -- each carries a positional `type_name` child
/// instead, whose OWN `name` field (a `type_identifier`/
/// `remote_type_identifier` leaf) is the actual written name.
fn gleam_type_name(container_node: Node<'_>, src: &[u8]) -> Option<String> {
    let type_name_node = (0..container_node.child_count())
        .filter_map(|i| container_node.child(i))
        .find(|n| n.kind() == "type_name")?;
    type_name_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Everything Gleam's flat `LangSpec` arrays cannot express:
/// `type_definition`/`type_alias` positional `type_name` naming (both
/// have no `name` field of their own -- see
/// [`crate::languages::spec::LangSpec::gleam`]'s own doc comment) and
/// `import`'s own `module`-field text as the IMPORTS path -- wired as
/// this wave's [`Quirks::on_unmatched_node`] hook. `function_call` needs
/// NO override at all (this grammar's own `function`/`arguments` fields
/// already match the generic engine's default single-field
/// reconstruction, see [`LangSpec::gleam`]'s own doc comment), so this
/// row's [`Quirks::call_override`] stays `None`, unlike every other
/// language this wave/G2.1 onboarded.
fn gleam_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "type_definition" | "type_alias" => {
            if let Some(name) = gleam_type_name(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: if node.kind() == "type_alias" {
                        SymbolKind::TypeAlias
                    } else {
                        SymbolKind::Class
                    },
                    line: node.start_position().row + 1,
                });
            }
            // Not fully claimed: a `type_definition`'s own
            // `data_constructors` may themselves nest further
            // expressions (default values, ...) this quirk does not walk
            // itself -- fall through to generic recursion (same
            // rationale as [`zig_quirk`]'s `test_declaration` arm).
            false
        }
        "import" => {
            if let Some(path) = node
                .child_by_field_name("module")
                .and_then(|n| n.utf8_text(src).ok())
            {
                out.imports.push(ImportRef {
                    module_path: path.to_string(),
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        _ => false,
    }
}

/// Gleam's [`Quirks`] row: `type_definition`/`type_alias` positional
/// naming + `import`'s `module`-field IMPORTS. No `call_override`
/// (`function_call` needs none -- see [`gleam_quirk`]'s own doc
/// comment), and no `is_test_name`/`route_from_call`/`on_method_defined`:
/// the baseline wires Gleam no test-name convention or route-
/// registration detection at all (`CBM_LANG_GLEAM` never appears in
/// `internal/cbm/service_patterns.c`'s route-library tables), and Gleam
/// has no class/method-DEFINES-container distinction at all (no classes
/// exist in the language -- see [`LangSpec::gleam`]'s own doc comment on
/// the exhaustive extends/implements/inherit/protocol/trait/interface
/// search finding none).
pub fn gleam_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(gleam_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse Gleam source through the generic engine.
pub fn parse_gleam(source: &str) -> ParsedFile {
    let spec = LangSpec::gleam();
    let quirks = gleam_quirks();
    let language: tree_sitter::Language = tree_sitter_gleam::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// GLSL
// =====================================================================

/// Parse GLSL source through the generic engine. GLSL is a C-like shader
/// language whose own baseline `lang_specs.c` row reuses C's node-type
/// arrays verbatim (see [`crate::languages::spec::LangSpec::glsl`]'s own
/// doc comment) -- this function mirrors that reuse exactly, delegating
/// straight to [`c_quirks`] with only the grammar binding left
/// unchanged from [`parse_c`] (both share the SAME `tree_sitter_c`
/// crate dependency; no new grammar needed at all). `is_test_file` is
/// hardcoded `false` (unlike [`parse_c`]'s own parameter): GLSL shader
/// source has no test-file naming convention in the baseline (GLSL never
/// appears in any file-suffix-driven test-detection table there), so
/// there is nothing analogous to thread through from a caller.
pub fn parse_glsl(source: &str) -> ParsedFile {
    let spec = LangSpec::glsl();
    let quirks = c_quirks();
    let language: tree_sitter::Language = tree_sitter_c::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Ada
// =====================================================================

/// Read a name off a nested `procedure_specification`/
/// `function_specification` child's own `name` field -- the shared shape
/// [`ada_quirk`]'s `subprogram_body`/`subprogram_declaration`/
/// `expression_function_declaration` arms all need (see
/// [`LangSpec::ada`]'s own doc comment for why: none of the three has a
/// `name` field on itself, only on this nested child).
fn ada_specification_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let spec = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| {
            matches!(
                n.kind(),
                "procedure_specification" | "function_specification"
            )
        })?;
    child_text(spec, "name", src)
}

/// The base type named by a `full_type_declaration`'s nested
/// `derived_type_definition` child's own `subtype_mark` field (`type
/// Derived is new Base with record ... end record;` / `type Alias is new
/// Integer;`) -- see [`LangSpec::ada`]'s own doc comment's INHERITS
/// bullet for the verification behind this.
fn ada_derived_base_name(full_type_decl: Node<'_>, src: &[u8]) -> Option<String> {
    let derived = (0..full_type_decl.child_count())
        .filter_map(|i| full_type_decl.child(i))
        .find(|n| n.kind() == "derived_type_definition")?;
    child_text(derived, "subtype_mark", src)
}

/// `with Ada.Text_IO;` / `use Ada.Text_IO;` -- mirrors this crate's own
/// `internal/cbm/extract_imports.c` `parse_ada_imports` exactly: every
/// named child of kind `identifier`/`selected_component`/`name` is its
/// own dotted import path (a `with`/`use` clause may name several
/// packages separated by commas, each its own such child).
fn ada_import_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if matches!(child.kind(), "identifier" | "selected_component" | "name") {
            if let Ok(text) = child.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// `Name : String (1 .. 10);` inside a record's `component_list` -- the
/// component's own name is its first positional `identifier` child (no
/// `name` field on this node kind at all, verified) -- shared by
/// [`ada_quirk`]'s `full_type_declaration` arm (walks its own
/// `component_declaration` descendants for DEFINES) since a record
/// component only ever DEFINES to the enclosing TYPE, not the enclosing
/// package (unlike every subprogram/type at package scope, which
/// [`ada_walk_scoped_body`]'s ordinary generic recursion already scopes
/// correctly).
fn ada_component_declaration_names(record_body: Node<'_>, src: &[u8]) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut stack = vec![record_body];
    while let Some(node) = stack.pop() {
        if node.kind() == "component_declaration" {
            if let Some(first_named) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.is_named() && n.kind() == "identifier")
            {
                if let Ok(text) = first_named.utf8_text(src) {
                    out.push((text.to_string(), node.start_position().row + 1));
                }
            }
            continue;
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }
    out
}

/// `package_declaration`/`package_body`'s DEFINES-scoped body walk --
/// same "recurse into every child through the fully generic [`walk`], so
/// nested subprograms/types/calls all fall out correctly on their own"
/// pattern as [`java_walk_scoped_body`]/[`ruby_walk_scoped_body`].
fn ada_walk_scoped_body(node: Node<'_>, src: &[u8], name: &str, out: &mut ParsedFile) {
    let spec = LangSpec::ada();
    let quirks = ada_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, Some(name), FnScope::default());
        }
    }
}

/// Everything Ada's flat [`LangSpec`] arrays cannot express: subprogram/
/// entry naming off a nested specification node or a distinct field
/// name, `full_type_declaration`'s positional (fieldless) name plus its
/// `derived_type_definition` INHERITS + `component_declaration` DEFINES,
/// `package_declaration`/`package_body`'s scoped body walk, and
/// `with_clause`/`use_clause` IMPORTS -- wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. [`ada_specification_name`]'s
/// result is used for BOTH a def-shaped node's own symbol (Function --
/// Ada's grammar has no separate class/impl-nesting for subprograms
/// declared inside a package the way Rust/Java's `method_types` split
/// works, so every one is recorded as [`crate::parsers::SymbolKind::Function`]
/// rather than attempting a Method distinction this grammar's own
/// nesting does not support cleanly) AND its own DEFINES edge to the
/// enclosing package (when `enclosing` is `Some`, exactly mirroring the
/// generic engine's own func/method branch's identical DEFINES-push
/// convention it cannot reach here since these four kinds are absent
/// from `func_types`/`method_types`).
fn ada_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "subprogram_body" | "subprogram_declaration" | "expression_function_declaration" => {
            let Some(name) = ada_specification_name(node, src) else {
                return false;
            };
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Function,
                line,
            });
            if let Some(container) = enclosing {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name.clone(),
                    line,
                });
            }
            // Walk every child generically, scoped to this subprogram's
            // own name -- including the `procedure_specification`/
            // `function_specification` child already consumed for the
            // name above (a parameter default/return-type expression
            // could itself contain a call; letting the generic recursion
            // reach it for free is simpler than special-casing it away).
            let spec = LangSpec::ada();
            let quirks = ada_quirks();
            let ctx = Ctx {
                spec: &spec,
                src,
                quirks: &quirks,
                is_test_file: false,
            };
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    walk(
                        child,
                        &ctx,
                        out,
                        enclosing,
                        FnScope {
                            name: Some(name.as_str()),
                            line: Some(line),
                        },
                    );
                }
            }
            true
        }
        "entry_declaration" => {
            let Some(name) = child_text(node, "entry_name", src) else {
                return false;
            };
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Function,
                line,
            });
            if let Some(container) = enclosing {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name,
                    line,
                });
            }
            true
        }
        "full_type_declaration" => {
            // No `name` field at all -- the type's own name is the first
            // positional `identifier` child (right after the `type`
            // keyword token, before `is`), verified via a real parse.
            let Some(name) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.is_named() && n.kind() == "identifier")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_string)
            else {
                return false;
            };
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Class,
                line,
            });
            if let Some(container) = enclosing {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name.clone(),
                    line,
                });
            }
            if let Some(super_name) = ada_derived_base_name(node, src) {
                out.inherits.push(InheritsRef {
                    sub_name: name.clone(),
                    super_name,
                    line,
                });
            }
            for (field_name, field_line) in ada_component_declaration_names(node, src) {
                out.defines.push(DefinesRef {
                    container_name: name.clone(),
                    member_name: field_name,
                    line: field_line,
                });
            }
            true
        }
        "package_declaration" | "package_body" => {
            let Some(name) = child_text(node, "name", src) else {
                return false;
            };
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Class,
                line: node.start_position().row + 1,
            });
            ada_walk_scoped_body(node, src, name.as_str(), out);
            true
        }
        "with_clause" | "use_clause" => {
            let line = node.start_position().row + 1;
            for path in ada_import_paths(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line,
                });
            }
            true
        }
        _ => false,
    }
}

/// Ada's [`Quirks`] row: subprogram/entry naming + DEFINES,
/// `full_type_declaration` positional naming + INHERITS (from a nested
/// `derived_type_definition`'s `subtype_mark` field) + component DEFINES,
/// package-scoped body walk, and `with`/`use` clause IMPORTS. No
/// `is_test_name`/`route_from_call`/`on_method_defined`/`call_override`:
/// this crate's own baseline gives Ada no dedicated test-name convention
/// or route-registration-by-call-shape entry, `function_call`/
/// `procedure_call_statement` both carry a real, working `name` field so
/// the generic engine's own single-field callee reconstruction already
/// produces the correct callee text with no override needed (matching
/// `internal/cbm/extract_calls.c`'s own `extract_ada_callee`, which reads
/// the identical field), and every def-shaped node this crate records is
/// a plain [`crate::parsers::SymbolKind::Function`] with no
/// receiver-clause-style DEFINES-container-outside-syntactic-nesting
/// need the way Go's methods have.
pub fn ada_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(ada_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse Ada source through the generic engine.
pub fn parse_ada(source: &str) -> ParsedFile {
    let spec = LangSpec::ada();
    let quirks = ada_quirks();
    let language: tree_sitter::Language = tree_sitter_ada::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Apex
// =====================================================================

/// `class Sub extends Base` -- direct, non-generic-fallback port of
/// [`java_superclass_name`] (identical `superclass` field shape,
/// verified against this grammar's own `grammar.js`:
/// `superclass: ($) => seq(ci("extends"), $._type)`).
fn apex_superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    let superclass = class_node.child_by_field_name("superclass")?;
    let type_node = (0..superclass.child_count())
        .filter_map(|i| superclass.child(i))
        .find(|n| n.is_named())?;
    type_node.utf8_text(src).ok().map(str::to_string)
}

/// `class C implements I1, I2` / `enum E implements I1, I2` -- direct
/// port of [`java_super_interfaces`] (identical `interfaces` field
/// wrapping a `type_list`, verified).
fn apex_super_interfaces(node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// `interface Sub extends Base1, Base2` -- direct port of
/// [`java_extends_interfaces`] (identical `extends_interfaces` child
/// wrapping a `type_list`, verified).
fn apex_extends_interfaces(interface_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// Annotations (`@RestResource(...)`, `@AuraEnabled`) attached via a
/// `modifiers` child -- simplified single-kind port of
/// [`java_annotations`] (this grammar uses one unified `annotation` kind
/// for both the bare and argument-bearing forms, unlike Java's
/// `marker_annotation`/`annotation` split, verified).
fn apex_annotations(node: Node<'_>, src: &[u8]) -> Vec<String> {
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
            if modifier.kind() == "annotation" {
                if let Some(name_node) = modifier.child_by_field_name("name") {
                    if let Ok(text) = name_node.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
            }
        }
    }
    out
}

/// `int a = 1, b = 2;` -- direct port of [`java_field_names`] (identical
/// `variable_declarator` shape, verified).
fn apex_field_names(field_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// `obj.method(...)` / `method(...)` -- direct port of
/// [`java_method_invocation_callee`] (identical `object`/`name` field
/// shape, verified).
fn apex_method_invocation_callee(node: Node<'_>, src: &[u8]) -> String {
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
/// text plus a cheap syntactic hint -- direct port of
/// [`java_receiver_of_call`], with Apex's own literal node-kind names
/// (`string_literal`/no separate integer-literal-kind split the way
/// Java's numeric-base variants have -- verified this grammar uses one
/// unified numeric literal kind rather than Java's four).
fn apex_receiver_of_call(
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
        "string_literal" | "int" | "double" | "true" | "false" | "null_literal"
    ) {
        ReceiverHint::Literal
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// Each argument expression's own source text, in written order --
/// direct port of [`java_call_arg_texts`].
fn apex_call_arg_texts(invocation_node: Node<'_>, src: &[u8]) -> Vec<String> {
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

/// `class_declaration`/`interface_declaration`/`enum_declaration`'s
/// DEFINES-scoped body walk -- same pattern as [`java_walk_scoped_body`]/
/// [`ruby_walk_scoped_body`], plus a shallow field-DEFINES pass (Apex
/// records a DEFINES edge for EVERY field unconditionally, unlike Java's
/// `static final`-gated constant-only pass -- see [`LangSpec::apex`]'s
/// own doc comment for why this row deliberately does not replicate
/// Java's exact gate).
fn apex_walk_scoped_body(node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::apex();
    let quirks = apex_quirks();
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
    if let Some(container) = name {
        apex_emit_field_defines(node, src, container, out);
    }
}

/// Depth-first search for `field_declaration` descendants of `node`
/// belonging directly to it (not to any nested class/interface/enum
/// inside it) -- same "stop at a nested type boundary" shape as
/// [`java_emit_constant_field_defines`], but unconditional (every field,
/// not just constant-shaped ones).
fn apex_emit_field_defines(node: Node<'_>, src: &[u8], container: &str, out: &mut ParsedFile) {
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        match child.kind() {
            "field_declaration" => {
                let line = child.start_position().row + 1;
                for field_name in apex_field_names(child, src) {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: field_name,
                        line,
                    });
                }
            }
            "class_declaration" | "interface_declaration" | "enum_declaration" => {
                // A nested type's own fields belong to *it*, not to
                // `container` -- already correctly DEFINES-scoped when
                // `apex_walk_scoped_body`'s own recursive `walk` reaches
                // this nested node's own quirk arm.
            }
            _ => apex_emit_field_defines(child, src, container, out),
        }
    }
}

/// Everything Apex's flat [`LangSpec`] arrays cannot express: class/
/// interface/enum heritage + annotations + DEFINES-scoped body walk --
/// mirrors [`java_quirk`] closely, since the field shapes are identical.
/// `trigger_declaration` needs NO arm here despite its own `object`/
/// `events` fields having no flat-array equivalent: it is already listed
/// in [`LangSpec::apex`]'s own `func_types` with a working `name` field,
/// so `walk`'s own func/method branch claims it (recording it as a
/// plain [`crate::parsers::SymbolKind::Function`] and walking its `body`
/// field with the correct `fn_scope`) before this hook would ever see it
/// -- `object`/`events` are simply left unextracted (no fixture-driving
/// need this wave), same "left out rather than guessed at" posture as
/// several other languages' omitted narrow constructs this wave
/// documents elsewhere.
fn apex_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "class_declaration" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(super_name) = apex_superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                for interface_name in apex_super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                for decorator in apex_annotations(node, src) {
                    out.decorates.push(crate::parsers::DecoratesRef {
                        target_name: name.clone(),
                        decorator_name: decorator,
                        line,
                    });
                }
                apex_walk_scoped_body(node, src, Some(name.as_str()), out);
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
                for extended in apex_extends_interfaces(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name: extended,
                        line,
                    });
                }
                apex_walk_scoped_body(node, src, Some(name.as_str()), out);
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
                for interface_name in apex_super_interfaces(node, src) {
                    out.implements.push(ImplementsRef {
                        type_name: name.clone(),
                        trait_name: interface_name,
                        line,
                    });
                }
                apex_walk_scoped_body(node, src, Some(name.as_str()), out);
            }
            true
        }
        _ => false,
    }
}

/// Full `method_invocation` callee reconstruction -- direct port of
/// [`java_call_override`] (identical field shapes throughout).
fn apex_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "method_invocation" {
        return false;
    }
    let callee = apex_method_invocation_callee(node, src);
    let (receiver_text, receiver_hint) = apex_receiver_of_call(node, src);
    out.calls.push(CallRef {
        callee,
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text,
        receiver_hint,
        arg_texts: apex_call_arg_texts(node, src),
    });
    true
}

/// Apex's [`Quirks`] row: class/interface/enum heritage + annotations +
/// DEFINES-scoped body walk (a direct port of [`java_quirks`]'s own
/// logic, since the underlying grammar shapes are identical), plus full
/// receiver-qualified `method_invocation` callee reconstruction. No
/// `is_test_name`/`route_from_call`: this crate's own baseline gives Apex
/// no dedicated test-name convention, and `internal/cbm/
/// service_patterns.c`'s `route_reg_libraries` table has zero
/// Apex/Salesforce entries at all (a genuine baseline gap, matched here
/// rather than invented -- see [`LangSpec::apex`]'s own doc comment).
pub fn apex_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(apex_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(apex_call_override)),
    }
}

/// Parse Apex source through the generic engine.
pub fn parse_apex(source: &str) -> ParsedFile {
    let spec = LangSpec::apex();
    let quirks = apex_quirks();
    let language: tree_sitter::Language = tree_sitter_sfapex::apex::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Crystal
// =====================================================================

/// The identifier text inside a Crystal `class`/`struct` `superclass`
/// field -- unlike Ruby's identically-named field (which wraps the base
/// type together with its own leading `<` token, needing an unwrap),
/// this grammar's `superclass` field value is ALREADY just the bare base
/// type node directly (verified: `class_def`'s own `grammar.js` rule is
/// `optional(seq('<', field('superclass', choice($.constant,
/// $.generic_instance_type))))` -- the `<` token is a SIBLING, not a
/// wrapper, of the field value).
fn crystal_superclass_name(class_node: Node<'_>, src: &[u8]) -> Option<String> {
    child_text(class_node, "superclass", src)
}

/// `require "json"` -- mirrors this crate's own `internal/cbm/
/// extract_imports.c` `parse_crystal_imports` exactly: any `"require"`-
/// kind node's own `string` child (a bare positional child, no field)
/// is the import path, quotes stripped.
fn crystal_require_import(node: Node<'_>, src: &[u8]) -> Option<ImportRef> {
    let string_node = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "string")?;
    let raw = string_node.utf8_text(src).ok()?;
    let path = raw.trim_matches('"').to_string();
    if path.is_empty() {
        return None;
    }
    Some(ImportRef {
        module_path: path,
        line: node.start_position().row + 1,
    })
}

/// `class_def`/`struct_def`/`module_def`/`enum_def`/`annotation_def`'s
/// DEFINES-scoped body walk -- same "recurse into every child through
/// the fully generic [`walk`]" pattern as [`ruby_walk_scoped_body`],
/// operating on the node's own `body` field directly (present, though
/// optional, on every one of these five kinds -- absent for an empty
/// `class Foo; end`, in which case there is nothing to walk).
fn crystal_walk_scoped_body(node: Node<'_>, src: &[u8], name: &str, out: &mut ParsedFile) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let spec = LangSpec::crystal();
    let quirks = crystal_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            walk(child, &ctx, out, Some(name), FnScope::default());
        }
    }
}

/// Everything Crystal's flat [`LangSpec`] arrays cannot express:
/// `class_def`/`struct_def` INHERITS (from their own `superclass` field)
/// + DEFINES-scoped body walk for all five `class_types` kinds,
///   `instance_var`/`class_var` DEFINES (each node's own text, sigil
///   included, is its name -- no field to read), and `require` IMPORTS --
///   wired as this wave's [`Quirks::on_unmatched_node`] hook. `call`
///   IMPORTS/callee reconstruction is a separate [`Quirks::call_override`]
///   hook (see [`crystal_call_override`]) since the generic walker's own
///   call branch already consumes a `call`-kind node before
///   `on_unmatched_node` would ever see it -- `require` is NOT `call`-
///   shaped in this grammar (unlike Ruby's), so it is handled here instead.
fn crystal_quirk(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "class_def" | "struct_def" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    line,
                });
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: name.clone(),
                        line,
                    });
                }
                if let Some(super_name) = crystal_superclass_name(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line,
                    });
                }
                crystal_walk_scoped_body(node, src, name.as_str(), out);
            }
            true
        }
        "module_def" | "enum_def" | "annotation_def" => {
            if let Some(name) = child_text(node, "name", src) {
                let line = node.start_position().row + 1;
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: if node.kind() == "enum_def" {
                        SymbolKind::Enum
                    } else {
                        SymbolKind::Class
                    },
                    line,
                });
                if let Some(container) = enclosing {
                    out.defines.push(DefinesRef {
                        container_name: container.to_string(),
                        member_name: name.clone(),
                        line,
                    });
                }
                crystal_walk_scoped_body(node, src, name.as_str(), out);
            }
            true
        }
        "instance_var" | "class_var" => {
            if let (Some(container), Ok(name)) = (enclosing, node.utf8_text(src)) {
                out.defines.push(DefinesRef {
                    container_name: container.to_string(),
                    member_name: name.to_string(),
                    line: node.start_position().row + 1,
                });
            }
            false
        }
        "require" => {
            if let Some(import) = crystal_require_import(node, src) {
                out.imports.push(import);
            }
            true
        }
        _ => false,
    }
}

/// Each argument expression's own source text, in written order -- same
/// shape as [`ruby_call_arg_texts`] (identical `arguments` field name in
/// this grammar too, verified).
fn crystal_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
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
/// receiver is present, bare `method` otherwise) -- direct port of
/// [`ruby_call_override`]'s logic, WITHOUT Ruby's own `Widget.new(...)`
/// constructor-callee redirect (Crystal's own constructor convention is
/// `def initialize`, called via the SAME `Widget.new(...)` idiom as
/// Ruby, but this wave found no dedicated baseline
/// `internal/cbm/extract_calls.c` Crystal case to confirm the identical
/// redirect is warranted here rather than assumed -- left as a plain
/// `"new"` callee, matching every other language's default behavior,
/// rather than inventing a Crystal-specific redirect the baseline itself
/// never documents).
fn crystal_call_override(
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

    let callee = if let Some(receiver_text) = receiver_text {
        format!("{receiver_text}.{method_text}")
    } else {
        method_text.to_string()
    };

    let receiver_hint = receiver.map(|r| {
        if r.utf8_text(src) == Ok("self") {
            ReceiverHint::SelfOrThis
        } else if r.kind() == "constant" {
            ReceiverHint::NewExpression
        } else if matches!(r.kind(), "identifier" | "instance_var" | "class_var") {
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
        arg_texts: crystal_call_arg_texts(node, src),
    });

    true
}

/// Crystal's [`Quirks`] row: `class_def`/`struct_def`/`module_def`/
/// `enum_def`/`annotation_def` DEFINES-scoped body walk +
/// `class_def`/`struct_def` INHERITS (from their own `superclass`
/// field), `instance_var`/`class_var` DEFINES (each node's own sigil-
/// prefixed text is its name), `require` IMPORTS, and full
/// `receiver.method` callee reconstruction. No
/// `is_test_name`/`route_from_call`/`on_method_defined`: this crate's own
/// baseline gives Crystal no dedicated test-name convention or
/// route-registration-by-call-shape entry either (Crystal web frameworks
/// like Kemal/Lucky are absent from `internal/cbm/service_patterns.c`'s
/// `route_reg_libraries` table -- matches the identical Apex/Salesforce
/// finding, a real baseline gap left unfilled per this wave's own
/// "match, don't invent" mandate).
pub fn crystal_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(crystal_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(crystal_call_override)),
    }
}

/// Parse Crystal source through the generic engine.
pub fn parse_crystal(source: &str) -> ParsedFile {
    let spec = LangSpec::crystal();
    let quirks = crystal_quirks();
    let language: tree_sitter::Language = tree_sitter_crystal::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// R
// =====================================================================

/// An R `function_definition`'s own name: NOT a field on the node itself
/// (its `name` field always resolves to the literal `function` keyword
/// token, confirmed by a real parse dump -- R functions are anonymous
/// expressions), but the enclosing `binary_operator`'s `lhs` (`helper <-
/// function(x, y) {...}`) or `left` field, falling back to the parent's
/// first named child if neither field is present -- mirrors
/// `internal/cbm/extract_defs.c`'s `resolve_r_func_name` (:529-545)
/// exactly.
fn r_func_name(func_node: Node<'_>, src: &[u8]) -> Option<String> {
    let parent = func_node.parent()?;
    if parent.kind() != "binary_operator" {
        return None;
    }
    let lhs = parent
        .child_by_field_name("lhs")
        .or_else(|| parent.child_by_field_name("left"))
        .or_else(|| (0..parent.named_child_count()).find_map(|i| parent.named_child(i)))?;
    lhs.utf8_text(src).ok().map(str::to_string)
}

/// Recurses into a `function_definition`'s own `body` field with
/// `fn_scope` set to the (parent-resolved, see [`r_func_name`]) function
/// name/line -- builds a fresh local [`Ctx`] from [`LangSpec::r`]/
/// [`r_quirks`] rather than threading the caller's own `ctx` through
/// (this row's [`Quirks::on_unmatched_node`] hook signature carries
/// neither), same pattern as every other fully-quirk-claimed func/method
/// node kind (see [`dart_walk_function_body`]).
fn r_walk_function_body(
    func_node: Node<'_>,
    src: &[u8],
    name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let Some(body) = func_node.child_by_field_name("body") else {
        return;
    };
    let spec = LangSpec::r();
    let quirks = r_quirks();
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

/// R's `function_definition` naming quirk (see [`r_func_name`]'s own doc
/// comment) plus `library`/`require`/`requireNamespace`/`loadNamespace`/
/// `source`/`box::use` IMPORTS detection off an ordinary `call` node --
/// wired as this row's [`Quirks::on_unmatched_node`] hook. Mirrors
/// `internal/cbm/extract_imports.c`'s `r_collect_imports`/`parse_r_imports`
/// (:838-897) for the single-argument import-call shape (this Tier-2
/// scope does not special-case `box::use`'s own N-argument fan-out into N
/// separate imports -- see [`LangSpec::r`]'s own doc comment).
fn r_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    // `on_unmatched_node` also runs for a plain `call` node the generic
    // engine's own call branch already recorded a CallRef for (this
    // module's `walk` calls it unconditionally after that branch, not only
    // for genuinely-unmatched kinds) -- import detection piggybacks on
    // that second pass rather than claiming the node (`false`, so the
    // generic engine's own recursion into this node's children still
    // happens exactly once, from the call branch's own fallthrough).
    if node.kind() == "call" {
        if let Some(import) = r_import_from_call(node, src) {
            out.imports.push(import);
        }
        return false;
    }
    // `function_definition` is fully claimed here: resolve the real name
    // off the parent (see `r_func_name`), push the Function symbol, then
    // recurse into the body with the correct `fn_scope` -- the generic
    // engine's own func/method branch never reaches this node kind at all
    // for this row (`LangSpec::r`'s `func_types` IS `["function_definition"]`,
    // so `walk`'s own func/method branch tries it FIRST and only falls
    // through to `on_unmatched_node` when `child_text(node, spec.name_field,
    // ..)` -- reading the placeholder `"UNUSED_SEE_R_QUIRK"` field name --
    // returns `None`, which it always does since no real field by that
    // name exists).
    if node.kind() == "function_definition" {
        if let Some(name) = r_func_name(node, src) {
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Function,
                line,
            });
            r_walk_function_body(node, src, &name, line, out);
        }
        return true;
    }
    false
}

/// The first positional `argument`'s own `value` field text, quote-
/// stripped -- the module/path argument to `library`/`require`/
/// `requireNamespace`/`loadNamespace`/`source`/`box::use`.
fn r_first_arg_text(call_node: Node<'_>, src: &[u8]) -> Option<String> {
    let args = call_node.child_by_field_name("arguments")?;
    for i in 0..args.named_child_count() {
        let arg = args.named_child(i)?;
        if arg.kind() != "argument" {
            continue;
        }
        let value = arg.child_by_field_name("value")?;
        let text = value.utf8_text(src).ok()?;
        let stripped = text.trim_matches(|c| c == '"' || c == '\'');
        if !stripped.is_empty() {
            return Some(stripped.to_string());
        }
    }
    None
}

/// Recognizes `library(x)`/`require(x)`/`requireNamespace(x)`/
/// `loadNamespace(x)`/`source(x)` (a plain `identifier` callee) and
/// `box::use(pkg/mod)` (a `namespace_operator` callee whose `lhs`/`rhs`
/// text is exactly `"box"`/`"use"`) off an ordinary `call` node -- mirrors
/// `internal/cbm/extract_imports.c`'s `r_collect_imports` exactly (same
/// callee-name allowlist, same "first positional argument only" scope).
fn r_import_from_call(call_node: Node<'_>, src: &[u8]) -> Option<ImportRef> {
    let function = call_node.child_by_field_name("function")?;
    let line = call_node.start_position().row + 1;
    match function.kind() {
        "identifier" => {
            let name = function.utf8_text(src).ok()?;
            if matches!(
                name,
                "library" | "require" | "requireNamespace" | "loadNamespace" | "source"
            ) {
                let path = r_first_arg_text(call_node, src)?;
                return Some(ImportRef {
                    module_path: path,
                    line,
                });
            }
            None
        }
        "namespace_operator" => {
            let lhs = function.child_by_field_name("lhs")?.utf8_text(src).ok()?;
            let rhs = function.child_by_field_name("rhs")?.utf8_text(src).ok()?;
            if lhs == "box" && rhs == "use" {
                let path = r_first_arg_text(call_node, src)?;
                return Some(ImportRef {
                    module_path: path,
                    line,
                });
            }
            None
        }
        _ => None,
    }
}

/// R's [`Quirks`] row: `function_definition` naming (resolved off the
/// enclosing `binary_operator`, not any field on the node itself) plus
/// `library`/`require`/`requireNamespace`/`loadNamespace`/`source`/
/// `box::use` IMPORTS off an ordinary `call` node. No `route_from_call`/
/// `on_method_defined`/`is_test_name`: R has no method-receiver-clause
/// convention, decorator/attribute syntax, or file-level test-name
/// convention any bespoke extractor or baseline walker recognizes either.
/// `call_override` is deliberately `None`: `call`'s `function` field
/// already reconstructs correctly through the generic engine's own
/// default single-field path for every receiver shape this grammar has
/// (identifier/namespace_operator/extract_operator, confirmed by the real
/// parse dump -- see [`LangSpec::r`]'s own doc comment), so no override is
/// needed for the base callee-text case.
pub fn r_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(r_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse R source through the generic engine. Same "no pre-existing
/// bespoke extractor, correctness verified directly against the grammar
/// crate's own `node-types.json` plus a real parse-tree dump" posture as
/// [`parse_solidity`]/[`parse_gdscript`] -- see [`LangSpec::r`]'s doc
/// comment.
pub fn parse_r(source: &str) -> ParsedFile {
    let spec = LangSpec::r();
    let quirks = r_quirks();
    let language: tree_sitter::Language = tree_sitter_r::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Perl
// =====================================================================

/// A `function`/`method` wrapper field's own text already spans the real
/// callee/method name for every call-shaped kind this row claims --
/// confirmed by the real parse dump: `function_call_expression`'s
/// `function` field's single unfielded child is a `varname`-bearing leaf
/// for an ordinary call, or the literal builtin keyword token itself for
/// a builtin (`print`/`bless`/...); `func1op_call_expression`/
/// `func0op_call_expression`'s `function` field IS the literal, unnamed
/// builtin-keyword token (`shift`/`length`/`time`/...); `method_call_expression`'s
/// `method` field's own text is already the bare method name. No
/// unwrapping is needed in any case -- `.utf8_text()` on the field node
/// itself is correct.
fn perl_field_text<'a>(node: Node<'a>, field: &str, src: &'a [u8]) -> Option<&'a str> {
    node.child_by_field_name(field)?.utf8_text(src).ok()
}

/// Each argument expression's own source text, in written order.
/// Confirmed by the real parse dump, `function_call_expression`/
/// `ambiguous_function_call_expression`/`method_call_expression`'s own
/// `arguments` field has TWO different shapes depending on arity, not one
/// uniform flattened list: a call with two or more arguments (`helper(1,
/// 2)`, `bless $self, $class`) wraps them in one `list_expression` field
/// value whose OWN named children are the individual arguments, but a
/// call with EXACTLY ONE argument (`print($i)`, `print("pos")`) has the
/// `arguments` field point DIRECTLY at that single argument expression
/// with no `list_expression` wrapper at all -- both shapes are handled by
/// checking the field value's own `.kind()`. A zero-argument call
/// (`shift`, `Widget->new()`) simply has no `arguments` field to find,
/// correctly yielding an empty list. `func1op_call_expression`'s single
/// argument (if any, e.g. `length $x`) is a bare, unfielded child instead
/// (confirmed: no `arguments` field on this node kind at all) -- reached
/// via [`tree_sitter::Node::field_name_for_child`] rather than a `.kind()`
/// filter, since this node kind's OWN `function` field is the literal
/// builtin-keyword token itself (e.g. `.kind() == "shift"`, not
/// `"function"`), which a kind-string filter would fail to recognize as
/// the callee wrapper and incorrectly include as an argument.
fn perl_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    if let Some(arguments) = call_node.child_by_field_name("arguments") {
        if arguments.kind() == "list_expression" {
            return (0..arguments.named_child_count())
                .filter_map(|i| arguments.named_child(i))
                .filter_map(|n| n.utf8_text(src).ok().map(str::to_string))
                .collect();
        }
        return arguments
            .utf8_text(src)
            .ok()
            .map(|text| vec![text.to_string()])
            .unwrap_or_default();
    }
    // `func1op_call_expression`'s bare, unfielded single argument (if
    // any) -- every named child that is not this node's own `function`
    // field (the builtin-keyword token) is the argument.
    (0..call_node.child_count())
        .filter(|&i| call_node.child(i).is_some_and(|c| c.is_named()))
        .filter(|&i| call_node.field_name_for_child(i as u32) != Some("function"))
        .filter_map(|i| call_node.child(i))
        .filter_map(|n| n.utf8_text(src).ok().map(str::to_string))
        .collect()
}

/// Full callee reconstruction for all five of this row's `call_types`
/// node kinds, each of which needs a wrapper-field/two-field-split read
/// the generic engine's own single flat `call_function_field` cannot
/// express uniformly -- wired as [`Quirks::call_override`], same
/// "every call shape claimed here, generic engine's own reconstruction
/// never actually reached" posture as [`php_call_override`]/
/// [`ruby_call_override`].
fn perl_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let line = node.start_position().row + 1;
    match node.kind() {
        "function_call_expression" | "ambiguous_function_call_expression" => {
            let Some(callee) = perl_field_text(node, "function", src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: perl_call_arg_texts(node, src),
            });
            true
        }
        "func1op_call_expression" | "func0op_call_expression" => {
            let Some(callee) = perl_field_text(node, "function", src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: perl_call_arg_texts(node, src),
            });
            true
        }
        "method_call_expression" => {
            let Some(method) = perl_field_text(node, "method", src) else {
                return false;
            };
            let invocant = node.child_by_field_name("invocant");
            let receiver_text = invocant.and_then(|n| n.utf8_text(src).ok());
            let callee = match receiver_text {
                Some(receiver) => format!("{receiver}->{method}"),
                None => method.to_string(),
            };
            let receiver_hint = invocant.map(|r| {
                if r.utf8_text(src) == Ok("$self") {
                    ReceiverHint::SelfOrThis
                } else if r.kind() == "bareword" {
                    ReceiverHint::NewExpression
                } else if r.kind() == "scalar" {
                    ReceiverHint::Identifier
                } else {
                    ReceiverHint::Other
                }
            });
            out.calls.push(CallRef {
                callee,
                line,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: receiver_text.map(str::to_string),
                receiver_hint,
                arg_texts: perl_call_arg_texts(node, src),
            });
            true
        }
        _ => false,
    }
}

/// `use Foo::Bar;`/`use POSIX qw(...);` (a real `module` field, type
/// `package`, a fieldless leaf whose own text is the dotted module name --
/// confirmed unaffected by a trailing `qw(...)` import-list clause) and
/// `require Foo::Bar;` (a bareword child holding the full dotted path) --
/// wired as this row's [`Quirks::on_unmatched_node`] hook, mirroring the
/// intent of baseline's `perl_import_types = {"use_statement",
/// "require_statement", "require"}` while routing through this grammar's
/// real node shapes (see [`LangSpec::perl`]'s own doc comment for why
/// neither baseline node name is real here).
fn perl_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "use_statement" => {
            if let Some(module) = perl_field_text(node, "module", src) {
                if !module.is_empty() {
                    out.imports.push(ImportRef {
                        module_path: module.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            false
        }
        "require_expression" => {
            let path = (0..node.named_child_count())
                .filter_map(|i| node.named_child(i))
                .find(|n| n.kind() == "bareword")
                .and_then(|n| n.utf8_text(src).ok());
            if let Some(path) = path {
                if !path.is_empty() {
                    out.imports.push(ImportRef {
                        module_path: path.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            false
        }
        _ => false,
    }
}

/// Perl's [`Quirks`] row: `use_statement`/`require_expression` IMPORTS
/// plus full callee reconstruction for all five call-shaped node kinds
/// (`function_call_expression`/`ambiguous_function_call_expression`/
/// `func1op_call_expression`/`func0op_call_expression`/
/// `method_call_expression`). No `route_from_call`/`on_method_defined`/
/// `is_test_name`: the baseline gives Perl no dedicated route-registration-
/// by-call-shape detection, DEFINES-container-by-receiver-clause
/// convention, or file-level test-name convention either.
pub fn perl_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(perl_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(perl_call_override)),
    }
}

/// Parse Perl source through the generic engine, using the `ts-parser-perl`
/// crate (NOT the crates.io crate literally named `tree-sitter-perl`,
/// which has a real, unresolvable ABI conflict against this workspace's
/// `tree-sitter` core -- see [`LangSpec::perl`]'s own doc comment for the
/// full grammar-crate-choice rationale). Same "no pre-existing bespoke
/// extractor" posture as [`parse_solidity`]/[`parse_r`].
pub fn parse_perl(source: &str) -> ParsedFile {
    let spec = LangSpec::perl();
    let quirks = perl_quirks();
    let language: tree_sitter::Language = ts_parser_perl::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Clojure
// =====================================================================

/// The fixed def-form-head keyword table, restricted to Clojure's own
/// real subset of the baseline's shared Lisp-family `lisp_is_def_head`
/// table (`internal/cbm/extract_defs.c`:5995-6018) -- the baseline's own
/// array additionally lists several Scheme/Racket/Common-Lisp-only heads
/// (`define`, `define-syntax`, `define-values`, `define-syntax-rule`,
/// `define-struct`, `define-record-type`, `define/contract`, `struct`)
/// that are never real Clojure forms, so this table omits them rather
/// than accepting dead matches no real Clojure source would ever trigger.
const CLOJURE_DEF_HEADS: &[&str] = &[
    "defn",
    "defn-",
    "def",
    "defmacro",
    "defmulti",
    "defmethod",
    "defprotocol",
    "defrecord",
    "deftype",
    "definterface",
    "defonce",
];

/// The [`crate::parsers::SymbolKind`] a def-form's own head keyword
/// records -- mirrors the baseline's own `lisp_label` special-casing
/// (`internal/cbm/extract_defs.c`:6053-6060) exactly: `defrecord`/
/// `deftype` are product types ([`SymbolKind::Struct`]); `definterface`/
/// `defprotocol` are interface-shaped ([`SymbolKind::Interface`]);
/// everything else (`defn`/`defn-`/`def`/`defmacro`/`defmulti`/
/// `defmethod`/`defonce`) is [`SymbolKind::Function`].
fn clojure_def_symbol_kind(head: &str) -> SymbolKind {
    match head {
        "defrecord" | "deftype" => SymbolKind::Struct,
        "definterface" | "defprotocol" => SymbolKind::Interface,
        _ => SymbolKind::Function,
    }
}

/// A `list_lit`'s own head symbol -- its first NAMED child's `.utf8_text()`
/// (already the full written text, namespace-qualifying prefix included
/// if any -- `sym_lit`'s `namespace`/`delimiter`/`name` fields are three
/// adjacent children spanning the node's own byte range contiguously, so
/// no manual reconstruction is needed, confirmed by the real parse dump).
/// `None` for an empty list (`()`, a valid but headless form).
fn clojure_head_text<'a>(list_node: Node<'a>, src: &'a [u8]) -> Option<&'a str> {
    let head = (0..list_node.named_child_count()).find_map(|i| list_node.named_child(i))?;
    head.utf8_text(src).ok()
}

/// A def-form's own defined name -- the SECOND named child (index 1),
/// unwrapped one level further if that child is itself a nested
/// `list_lit` (the `(defn (foo args) ...)`-style shape the baseline's own
/// `extract_lisp_def` still handles even though idiomatic Clojure never
/// actually writes a `defn` this way) -- mirrors
/// `internal/cbm/extract_defs.c`'s `extract_lisp_def` (:6027-6072) exactly.
fn clojure_def_name<'a>(list_node: Node<'a>, src: &'a [u8]) -> Option<&'a str> {
    if list_node.named_child_count() < 2 {
        return None;
    }
    let target = list_node.named_child(1)?;
    let name_node = if target.kind() == "list_lit" && target.named_child_count() > 0 {
        target.named_child(0)?
    } else {
        target
    };
    let text = name_node.utf8_text(src).ok()?;
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// `(:require [some.ns :as alias] [other.ns] ...)`/`(:use ...)` clause
/// handling for an `ns` form -- each bracketed `vec_lit`'s own first named
/// child is the required namespace's `sym_lit` (confirmed by the real
/// parse dump: `[clojure.string :as str]`'s first named child is the bare
/// symbol `clojure.string`, followed by an optional `:as alias`/`:refer
/// [...]` clause this Tier-2 scope does not need to read). A bare
/// `sym_lit` clause (no brackets, `(:require some.ns)`) is accepted too,
/// same shape as the plain `(require 'some.ns)` form
/// [`clojure_plain_require_import`] handles.
fn clojure_ns_require_imports(ns_form: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let line = ns_form.start_position().row + 1;
    for i in 2..ns_form.named_child_count() {
        let Some(clause) = ns_form.named_child(i) else {
            continue;
        };
        if clause.kind() != "list_lit" {
            continue;
        }
        let Some(clause_head) = clojure_head_text(clause, src) else {
            continue;
        };
        if clause_head != ":require" && clause_head != ":use" {
            continue;
        }
        for j in 1..clause.named_child_count() {
            let Some(entry) = clause.named_child(j) else {
                continue;
            };
            let module = match entry.kind() {
                "vec_lit" => (0..entry.named_child_count())
                    .find_map(|k| entry.named_child(k))
                    .and_then(|n| n.utf8_text(src).ok()),
                "sym_lit" => entry.utf8_text(src).ok(),
                _ => None,
            };
            if let Some(module) = module {
                if !module.is_empty() {
                    out.imports.push(ImportRef {
                        module_path: module.to_string(),
                        line,
                    });
                }
            }
        }
    }
}

/// `(require 'some.ns)`/`(require 'some.ns 'other.ns)` -- each argument
/// after the head is a `quoting_lit` wrapping a bare `sym_lit` (confirmed
/// present by the real parse dump); a bare unquoted `sym_lit` argument is
/// accepted too for robustness against the rarer unquoted-symbol
/// convention some Clojure code uses.
fn clojure_plain_require_import(call_node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let line = call_node.start_position().row + 1;
    for i in 1..call_node.named_child_count() {
        let Some(arg) = call_node.named_child(i) else {
            continue;
        };
        let sym = match arg.kind() {
            "quoting_lit" => (0..arg.named_child_count()).find_map(|k| arg.named_child(k)),
            "sym_lit" => Some(arg),
            _ => None,
        };
        let Some(sym) = sym.filter(|n| n.kind() == "sym_lit") else {
            continue;
        };
        if let Ok(text) = sym.utf8_text(src) {
            if !text.is_empty() {
                out.imports.push(ImportRef {
                    module_path: text.to_string(),
                    line,
                });
            }
        }
    }
}

/// Recurses into a def-form's own remaining children (from index 2
/// onward -- past the head keyword at index 0 and the defined name at
/// index 1, mirroring [`clojure_def_name`]'s own indexing) with `fn_scope`
/// set to the newly-defined name/line, so a call lexically inside a
/// `defn`'s own body correctly records `from_symbol` as that `defn`'s own
/// name rather than whatever (or no) scope was in effect at the point the
/// def-form itself was reached. Builds a fresh local [`Ctx`] from
/// [`LangSpec::clojure`]/[`clojure_quirks`] rather than threading the
/// caller's own `ctx` through (this row's [`Quirks::on_unmatched_node`]
/// hook signature carries neither) -- same pattern as every other
/// fully-quirk-claimed def-shaped node kind (see
/// [`dart_walk_function_body`]/[`r_walk_function_body`]).
fn clojure_walk_def_body(
    def_node: Node<'_>,
    src: &[u8],
    name: &str,
    line: usize,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::clojure();
    let quirks = clojure_quirks();
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
    for i in 2..def_node.named_child_count() {
        if let Some(child) = def_node.named_child(i) {
            walk(child, &ctx, out, None, fn_scope);
        }
    }
}

/// Full `list_lit` handling: def-form recognition (mirrors
/// `internal/cbm/extract_defs.c`'s `extract_lisp_def` -- see
/// [`clojure_def_name`]/[`clojure_def_symbol_kind`]'s own doc comments)
/// plus `ns`/`require` IMPORTS (mirrors
/// `internal/cbm/extract_imports.c`'s `parse_lisp_imports`/
/// `lisp_process_list` -- see [`clojure_ns_require_imports`]/
/// [`clojure_plain_require_import`]'s own doc comments) -- wired as this
/// row's [`Quirks::on_unmatched_node`] hook. `ns`/`require`/a plain call
/// return `false` (never claim the subtree) so this module's own `walk`'s
/// OWN unchanged-scope recursion still descends into their children
/// generically (an `ns` form's own `:require` clauses, a plain call's own
/// nested argument expressions, ...); a recognized def-form instead
/// returns `true` and does its OWN scoped recursion via
/// [`clojure_walk_def_body`] -- claiming the subtree here specifically
/// (rather than falling through to the generic, unchanged-scope
/// recursion) is what gives every call lexically inside a `defn`'s own
/// body the correct `from_symbol` (see [`clojure_walk_def_body`]'s own
/// doc comment) -- caught by this worker's own standalone verification
/// harness (a `from_symbol: None` result for a call that should have
/// recorded its enclosing `defn`'s name), not by inspection.
fn clojure_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "list_lit" {
        return false;
    }
    let Some(head) = clojure_head_text(node, src) else {
        return false;
    };
    if head == "ns" {
        clojure_ns_require_imports(node, src, out);
        return false;
    }
    if head == "require" {
        clojure_plain_require_import(node, src, out);
        return false;
    }
    if CLOJURE_DEF_HEADS.contains(&head) {
        if let Some(name) = clojure_def_name(node, src) {
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.to_string(),
                kind: clojure_def_symbol_kind(head),
                line,
            });
            clojure_walk_def_body(node, src, name, line, out);
            return true;
        }
    }
    false
}

/// Every `list_lit`'s own head symbol is recorded as a call callee,
/// unconditionally -- mirrors `internal/cbm/extract_calls.c`'s
/// `extract_lisp_callee` (:497-510) exactly: a def-form's own head
/// keyword (e.g. `"defn"`) IS ALSO recorded as a call callee, by design,
/// not filtered out -- see [`LangSpec::clojure`]'s own doc comment for why
/// this matches the baseline's real (unfiltered) behavior rather than
/// "improving" on it. Wired as [`Quirks::call_override`] since `list_lit`'s
/// own arguments live under one `multiple: true` `"value"` field this
/// generic engine's single-callee-field-plus-separate-arguments-field
/// convention cannot enumerate (see [`LangSpec::clojure`]'s own doc
/// comment).
fn clojure_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    let Some(head) = clojure_head_text(node, src) else {
        return false;
    };
    let arg_texts = (1..node.named_child_count())
        .filter_map(|i| node.named_child(i))
        .filter_map(|n| n.utf8_text(src).ok().map(str::to_string))
        .collect();
    out.calls.push(CallRef {
        callee: head.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts,
    });
    true
}

/// Clojure's [`Quirks`] row: `list_lit` def-form recognition (Struct/
/// Interface/Function symbol kinds per the baseline's own `lisp_label`
/// convention) plus `ns`/`require` IMPORTS via `on_unmatched_node`, and
/// full unconditional head-symbol callee recording via `call_override`.
/// No `route_from_call`/`on_method_defined`/`is_test_name`: the baseline
/// gives Clojure no route-registration-by-call-shape detection, DEFINES-
/// container-by-receiver-clause convention (Clojure has no methods in the
/// OOP sense to attribute one), or file-level test-name convention
/// either.
pub fn clojure_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(clojure_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(clojure_call_override)),
    }
}

/// Parse Clojure source through the generic engine. Same "no pre-existing
/// bespoke extractor" posture as [`parse_solidity`]/[`parse_r`]/
/// [`parse_perl`].
pub fn parse_clojure(source: &str) -> ParsedFile {
    let spec = LangSpec::clojure();
    let quirks = clojure_quirks();
    let language: tree_sitter::Language = tree_sitter_clojure::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

/// Julia's `function_definition`/`struct_definition`/`abstract_definition`/
/// `primitive_definition`/`assignment`/`import_statement`/
/// `using_statement`/`export_statement` -- every one of them entirely
/// unfielded in this grammar (see [`LangSpec::julia`]'s own doc comment)
/// -- wired as this wave's [`Quirks::on_unmatched_node`] hook.
/// `call_expression`/`broadcast_call_expression` are a separate
/// [`Quirks::call_override`] hook (see [`julia_call_override`]) since the
/// generic walker's own call branch dispatches to `call_override` BEFORE
/// `on_unmatched_node` ever runs for a `call_types` node.
fn julia_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "function_definition" => {
            julia_walk_function_definition(node, src, out);
            true
        }
        "struct_definition" | "abstract_definition" | "primitive_definition" => {
            let kind = if node.kind() == "struct_definition" {
                SymbolKind::Struct
            } else {
                SymbolKind::Class
            };
            let name = julia_type_head_name(node, src);
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line: node.start_position().row + 1,
                });
                if let Some(super_name) = julia_type_head_supertype(node, src) {
                    out.inherits.push(InheritsRef {
                        sub_name: name.clone(),
                        super_name,
                        line: node.start_position().row + 1,
                    });
                }
            }
            julia_walk_scoped(node, src, name.as_deref(), out);
            true
        }
        // Julia's short-form function definition (`square(x) = x * x`):
        // an ordinary `assignment` whose LHS is call-shaped
        // (`call_expression`) -- see `LangSpec::julia`'s own doc comment
        // for why this gate (rather than listing `assignment` in
        // `func_types` directly) is needed, mirroring the baseline's OWN
        // source comment directly above `julia_func_types` in
        // `lang_specs.c` ("the resolver names it only when the LHS is a
        // call, so plain `x = 5` is not a def"). Fully claimed (`true`)
        // in BOTH the short-form-function and the plain-assignment case:
        // the LHS, if call-shaped, is itself one of `LangSpec::julia()`'s
        // OWN `call_types` (`call_expression`) -- letting generic
        // recursion continue into it unfiltered would record a spurious
        // self-referential CALLS edge (`square` "calling" `square`, its
        // own signature misread as an invocation) the same way
        // `julia_walk_function_definition`'s own doc comment already
        // warns about for the `function`-keyword form; this arm instead
        // walks the RHS ONLY (with `fn_scope` set to the resolved name,
        // so calls inside the function's real body are correctly
        // attributed -- e.g. `square(x) = helper(x)` records `helper`
        // called FROM `square`) via [`julia_walk_scoped_fn_body`],
        // skipping the LHS entirely rather than merely "harmlessly"
        // re-walking it. A plain, non-call-shaped LHS (`x = f()`) is
        // walked the identical way with `fn_scope` left at `None` (no
        // enclosing function was resolved), so `f()` is still found,
        // just correctly unscoped -- matching what unfiltered generic
        // recursion would have found for that RHS anyway, with the LHS
        // (a bare, uncallable `identifier` in this case) contributing
        // nothing either way.
        "assignment" => {
            let name = julia_short_form_function_name(node, src);
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Function,
                    line: node.start_position().row + 1,
                });
            }
            julia_walk_scoped_fn_body(node, src, name.as_deref(), out);
            true
        }
        "import_statement" | "using_statement" | "export_statement" => {
            for path in julia_import_paths(node, src) {
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

/// A `function_definition`'s own name: the `signature` child's own
/// callee identifier, reached through the (up to) two positional layers
/// a real signature can wrap it in -- confirmed via a real parse tree
/// dump of every combination this crate's test fixtures exercise:
/// - plain `function f(x)`: `signature` directly wraps a `call_expression`
///   (`f(x)`), whose own first child is the name `identifier`.
/// - return-type-annotated `function f(x)::T`: `signature` instead wraps
///   a `typed_expression` (`f(x)::T`), whose OWN first child is the
///   `call_expression` from the plain case -- one extra layer to unwrap.
fn julia_function_signature_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let signature = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "signature")?;
    let inner = (0..signature.child_count())
        .filter_map(|i| signature.child(i))
        .find(|n| n.is_named())?;
    let call = if inner.kind() == "call_expression" {
        inner
    } else {
        (0..inner.child_count())
            .filter_map(|i| inner.child(i))
            .find(|n| n.kind() == "call_expression")?
    };
    let callee = (0..call.child_count())
        .filter_map(|i| call.child(i))
        .find(|n| n.is_named())?;
    callee.utf8_text(src).ok().map(str::to_string)
}

/// `function_definition`'s own scoped recursion once its name (if any)
/// has been resolved via [`julia_function_signature_name`] -- Julia is
/// not one of `LangSpec::julia()`'s `class_types`/other DEFINES-container
/// shapes, and `function_definition` is fully claimed by [`julia_quirk`]
/// before the generic engine's own func/method branch would ever
/// classify it correctly (its `name_field`-keyed `child_text` lookup
/// always fails -- see `LangSpec::julia`'s own doc comment), so this
/// quirk re-implements the DEFINES-push-plus-scoped-body-walk sequence
/// directly, mirroring [`zig_walk_container_body`]'s identical rationale
/// and mechanics (including a nested `function_definition`, Julia's
/// closure idiom, correctly re-entering this same quirk arm one level
/// deeper via the shared `walk` re-invocation rather than any special
/// nested-function handling of its own).
fn julia_walk_function_definition(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let name = julia_function_signature_name(node, src);
    let line = node.start_position().row + 1;
    if let Some(name) = &name {
        out.symbols.push(SymbolRef {
            name: name.clone(),
            kind: SymbolKind::Function,
            line,
        });
    }
    let spec = LangSpec::julia();
    let quirks = julia_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name: name.as_deref(),
        line: Some(line),
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            // Skip the `signature` child itself: its own nested
            // `argument_list` holds only parameter-pattern identifiers
            // (never a real call/def), and re-walking it here would risk
            // a spurious self-referential CALLS edge from `f`'s own
            // `fn_scope` back to `f`'s own name (the signature's
            // `call_expression` looks exactly like an ordinary call
            // node). Matches `zig_walk_container_body`'s equally
            // deliberate exclusion of the node kind it already consumed
            // directly.
            if child.kind() == "signature" {
                continue;
            }
            walk(child, &ctx, out, None, fn_scope);
        }
    }
}

/// `struct_definition`/`abstract_definition`/`primitive_definition`'s
/// shared `type_head` child, present on every one of the three
/// (confirmed via a real parse tree dump; see `LangSpec::julia`'s own
/// doc comment) -- either a bare `identifier` (`struct Point`, no
/// supertype) or a `binary_expression` whose own two named children are
/// the type name and its supertype (`struct Dog <: Animal`), mirroring
/// `internal/cbm/extract_defs.c`'s `extract_julia_base_classes` exactly:
/// "find the `type_head` child; if its own first named child is a
/// `binary_expression`, the type's name is that binary expression's
/// FIRST named child" -- confirmed against a real parse tree that the
/// baseline's own `ts_node_named_child(inner, 0)` (the type name) and
/// `ts_node_named_child(inner, count - 1)` (the supertype, [`julia_type_head_supertype`]'s
/// own read) are exactly this grammar's `binary_expression`'s two
/// children, an `identifier` on each side of the `<:` operator token
/// (itself unnamed, so `count == 2` always for this specific binary
/// expression shape -- the baseline's own "last named child" phrasing
/// and this "second of exactly two" reading agree).
fn julia_type_head_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let type_head = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "type_head")?;
    let inner = (0..type_head.child_count())
        .filter_map(|i| type_head.child(i))
        .find(|n| n.is_named())?;
    if inner.kind() == "binary_expression" {
        let first_named = (0..inner.child_count())
            .filter_map(|i| inner.child(i))
            .find(|n| n.is_named())?;
        first_named.utf8_text(src).ok().map(str::to_string)
    } else {
        inner.utf8_text(src).ok().map(str::to_string)
    }
}

/// The supertype half of [`julia_type_head_name`]'s same `type_head`
/// `binary_expression` shape (`struct Dog <: Animal`'s `Animal`) -- see
/// that function's own doc comment for the full grammar-shape finding.
/// Returns `None` for a `type_head` with no supertype at all (a bare
/// `identifier`, e.g. `struct Point`), matching
/// `extract_julia_base_classes`'s own identical "no `binary_expression`
/// inner node -> no base classes" early return.
fn julia_type_head_supertype(node: Node<'_>, src: &[u8]) -> Option<String> {
    let type_head = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "type_head")?;
    let inner = (0..type_head.child_count())
        .filter_map(|i| type_head.child(i))
        .find(|n| n.is_named())?;
    if inner.kind() != "binary_expression" {
        return None;
    }
    let named: Vec<Node<'_>> = (0..inner.child_count())
        .filter_map(|i| inner.child(i))
        .filter(|n| n.is_named())
        .collect();
    named.last()?.utf8_text(src).ok().map(str::to_string)
}

/// `struct_definition`/`abstract_definition`/`primitive_definition`'s
/// own scoped recursion once its name (if any) has been resolved via
/// [`julia_type_head_name`] -- same "fully claimed by the quirk before
/// the generic engine's own class-shape branch would classify it
/// correctly, so the quirk re-implements the DEFINES-container-scoped
/// recursion directly" rationale as [`julia_walk_function_definition`]
/// (and [`zig_walk_container_body`]'s identical precedent) -- skips the
/// already-consumed `type_head` child for the same "avoid re-walking a
/// child this quirk already read directly" reason
/// [`julia_walk_function_definition`]'s own doc comment gives (a
/// `type_head`'s `binary_expression`/identifier holds no call/def/field
/// shape at all in practice, so skipping it costs nothing either way,
/// but is skipped anyway for clarity/consistency).
fn julia_walk_scoped(node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::julia();
    let quirks = julia_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "type_head" {
                continue;
            }
            walk(child, &ctx, out, name, FnScope::default());
        }
    }
}

/// An `assignment` node's own RHS-only scoped recursion (skips the LHS
/// entirely) -- see [`julia_quirk`]'s own `"assignment"` arm doc comment
/// for why the LHS must never be re-walked generically (it risks a
/// spurious self-call when call-shaped) while the RHS still must be, with
/// `fn_scope` set to `fn_name` when one was resolved (the short-form
/// function case) or left at [`FnScope::default`] otherwise (the plain-
/// assignment case, matching what unfiltered generic recursion would
/// have produced for that RHS anyway).
fn julia_walk_scoped_fn_body(
    node: Node<'_>,
    src: &[u8],
    fn_name: Option<&str>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::julia();
    let quirks = julia_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let line = node.start_position().row + 1;
    let fn_scope = FnScope {
        name: fn_name,
        line: fn_name.map(|_| line),
    };
    let mut skipped_lhs = false;
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if !skipped_lhs && child.is_named() {
            skipped_lhs = true;
            continue;
        }
        walk(child, &ctx, out, None, fn_scope);
    }
}

/// Julia's short-form function definition name (`square(x) = x * x`):
/// an `assignment` node's LHS is call-shaped -- returns the callee
/// identifier's text, mirroring the baseline's OWN documented gate (see
/// `LangSpec::julia`'s own doc comment for the exact baseline source
/// comment this mirrors). Returns `None` for a plain, non-call-shaped
/// assignment LHS (`x = 5`), which is correctly not a function
/// definition at all.
fn julia_short_form_function_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let lhs = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.is_named())?;
    if lhs.kind() != "call_expression" {
        return None;
    }
    let callee = (0..lhs.child_count())
        .filter_map(|i| lhs.child(i))
        .find(|n| n.is_named())?;
    callee.utf8_text(src).ok().map(str::to_string)
}

/// `import_statement`/`using_statement`/`export_statement`'s own
/// positional, unfielded module-path children (see `LangSpec::julia`'s
/// own doc comment) -- collects every `identifier`/`import_path`/
/// `scoped_identifier`/`selected_import` direct child's own text.
/// `export_statement` (`export draw, area`) is included even though it
/// exports LOCAL names rather than naming an external module -- matches
/// baseline's own `julia_import_types` array treating all three
/// uniformly as "import-shaped" for this crate's own IMPORTS-edge
/// purposes (mirrors [`Self::gdscript`]'s own `extends_statement`
/// "closest available analog, not a perfect semantic match" doc-comment
/// precedent), recorded as its own IMPORTS edge per name rather than
/// invented as a wholly separate EXPORTS edge kind this crate's
/// `ParsedFile` shape has no room for.
fn julia_import_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if !child.is_named() {
            continue;
        }
        match child.kind() {
            "identifier" | "scoped_identifier" => {
                if let Ok(text) = child.utf8_text(src) {
                    out.push(text.to_string());
                }
            }
            "import_path" | "selected_import" => {
                if let Ok(text) = child.utf8_text(src) {
                    // `selected_import` (`Base: show`) carries the whole
                    // `module: name` clause as its own text -- keep only
                    // the module half (matches the baseline's own
                    // `julia_import_types` treating this as one
                    // module-path unit, not a `":"`-split pair).
                    let module = text.split(':').next().unwrap_or(text).trim();
                    if !module.is_empty() {
                        out.push(module.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// `call_expression`/`broadcast_call_expression`, both entirely
/// unfielded in this grammar (see `LangSpec::julia`'s own doc comment) --
/// wired as [`Quirks::call_override`].
fn julia_call_override(
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
            // `field_expression` receiver detection (`obj.draw(...)`/
            // `Base.show(...)`) reuses `receiver_of_call`'s own shared
            // `"field_expression" => "value"` field-name dispatch
            // directly (this grammar's `field_expression` node shape
            // matches that shared helper's existing expectation exactly,
            // confirmed in `node-types.json`) -- a bare, non-dot-call
            // callee (`helper(1, 2)`, a plain `identifier`) correctly
            // yields `(None, None)` through that same shared helper's own
            // `_ => return (None, None)` fallback.
            let (receiver_text, receiver_hint) = receiver_of_call(callee_node, src);
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: julia_call_arg_texts(node, src),
            });
            true
        }
        "broadcast_call_expression" => {
            // Positional `[identifier, ".", argument_list]` children --
            // no wrapping `call_expression` at all (see `LangSpec::julia`'s
            // own doc comment). The written callee text is the bare
            // function name (`map`, for `map.(x, y)`) with no `.`
            // suffix -- matches how this crate's other languages record
            // a broadcast/vectorized-call idiom by its own plain callee
            // name, not a synthesized operator-inclusive string.
            let Some(callee_node) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.is_named())
            else {
                return false;
            };
            let Ok(callee) = callee_node.utf8_text(src) else {
                return false;
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts: julia_call_arg_texts(node, src),
            });
            true
        }
        _ => false,
    }
}

/// A `call_expression`/`broadcast_call_expression`'s own positional
/// `argument_list` sibling (see `LangSpec::julia`'s own doc comment) --
/// same shape as [`call_arg_texts`], but scanning for the node KIND
/// (`argument_list` has no field name to look up here) rather than a
/// field name.
fn julia_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let Some(args) = (0..call_node.child_count())
        .filter_map(|i| call_node.child(i))
        .find(|n| n.kind() == "argument_list")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..args.child_count() {
        if let Some(child) = args.child(i) {
            if !child.is_named() {
                continue;
            }
            if let Ok(text) = child.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

pub fn julia_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(julia_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(julia_call_override)),
    }
}

/// Parse Julia source through the generic engine. Grammar:
/// `tree-sitter-julia` (crates.io, official `tree-sitter` GitHub org
/// lineage). Julia has no pre-existing bespoke `languages::julia`
/// extractor -- correctness rests entirely on direct verification
/// against the crate's own `node-types.json`, a real parse tree dump,
/// and the C baseline's `julia_*` arrays plus its dedicated
/// `extract_julia_base_classes` walker (see `LangSpec::julia`'s own doc
/// comment), not byte-for-byte comparison against an oracle.
pub fn parse_julia(source: &str) -> ParsedFile {
    let spec = LangSpec::julia();
    let quirks = julia_quirks();
    let language: tree_sitter::Language = tree_sitter_julia::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}
/// Odin's `field` (struct-body field, including its `using`-prefixed
/// composition/embedding form) -- everything `LangSpec::odin`'s own flat
/// arrays cannot express (see that const's own doc comment). Wired as
/// this wave's [`Quirks::on_unmatched_node`] hook. `procedure_declaration`/
/// `struct_declaration`/`enum_declaration`/`union_declaration` are ALSO
/// fully claimed here even though [`LangSpec::odin`] lists them in
/// `func_types`/`class_types` too -- their own `name_field`-keyed
/// `child_text` lookup always fails first (this row's `name_field` is a
/// deliberate placeholder, see that const's own doc comment), so control
/// safely falls through to THIS hook's final catch-all invocation, same
/// "arrays exist to document the vocabulary, quirk claims the real
/// unfielded shape" posture as [`LangSpec::c`]/[`LangSpec::julia`].
/// `call_expression`/`selector_call_expression` are a separate
/// [`Quirks::call_override`] hook (see [`odin_call_override`]) since the
/// generic walker's own call branch dispatches to `call_override` BEFORE
/// `on_unmatched_node` ever runs for a `call_types` node -- BOTH need it:
/// `call_expression`'s own `"function"` callee field IS real and would
/// flow through the generic engine's default single-field path fine on
/// its own, but its arguments are exposed as a REPEATED `"argument"`
/// field (singular) rather than a single `"arguments"` wrapper field,
/// which the generic engine's shared `call_arg_texts` helper cannot read
/// correctly (see [`odin_call_arg_texts`]'s own doc comment) -- so both
/// kinds are fully claimed here rather than splitting "callee is fine,
/// only args need fixing" across the override boundary.
fn odin_quirk(node: Node<'_>, _enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "procedure_declaration" => {
            let name = odin_leading_identifier_name(node, src);
            let line = node.start_position().row + 1;
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind: SymbolKind::Function,
                    line,
                });
            }
            let Some(procedure) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.kind() == "procedure")
            else {
                return true;
            };
            // `procedure` has NO fields at all (confirmed:
            // `{"type":"procedure","fields":{}}`) -- `block` is one of
            // its positional children (alongside `parameters`/`type`/
            // `where_clause`/...), not a field, so it must be found by
            // kind rather than `child_by_field_name`.
            let Some(block) = (0..procedure.child_count())
                .filter_map(|i| procedure.child(i))
                .find(|n| n.kind() == "block")
            else {
                return true;
            };
            odin_walk_scoped(
                block,
                src,
                None,
                FnScope {
                    name: name.as_deref(),
                    line: Some(line),
                },
                out,
            );
            true
        }
        "struct_declaration" | "enum_declaration" | "union_declaration" => {
            let name = odin_leading_identifier_name(node, src);
            let line = node.start_position().row + 1;
            let kind = if node.kind() == "struct_declaration" {
                SymbolKind::Struct
            } else {
                SymbolKind::Class
            };
            if let Some(name) = &name {
                out.symbols.push(SymbolRef {
                    name: name.clone(),
                    kind,
                    line,
                });
                if node.kind() == "struct_declaration" {
                    for (member_name, embedded) in odin_struct_fields(node, src) {
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
            }
            // Fully claimed for naming/field-DEFINES/INHERITS above, but
            // still explicitly recurses (rather than a bare `true` with
            // no further walk): a struct/enum/union body has no ordinary
            // call/def-shaped content in idiomatic Odin (unlike e.g. a
            // Zig container, which this crate's own `zig_walk_container_body`
            // precedent already walks for the identical reason), but
            // recursing costs nothing and matches this crate's
            // established "never silently skip a subtree without a
            // concrete reason to" discipline rather than asserting a
            // negative (no default-value field initializer expressions
            // exist in this grammar) that a future language feature could
            // quietly invalidate.
            odin_walk_scoped(node, src, name.as_deref(), FnScope::default(), out);
            true
        }
        "import_declaration" => {
            if let Some(path) = odin_import_path(node, src) {
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

/// An `import_declaration`'s own module-path text: the `string` child's
/// `string_content` grandchild (`import "core:fmt"`, confirmed via a
/// real parse tree dump -- `node-types.json` shows `import_declaration`
/// has NO fields at all, so this must be found by kind rather than
/// `child_by_field_name`, same posture as [`odin_leading_identifier_name`]).
/// `foreign import lib "lib.a"` (this grammar's OTHER `import_declaration`
/// shape, confirmed via the same parse tree dump) additionally carries
/// an `"alias"`-tagged local name (`lib`) ahead of its own `string`
/// child -- this function still finds the `string` child directly by
/// kind regardless of that extra field, so both shapes resolve to their
/// own quoted path text correctly.
fn odin_import_path(node: Node<'_>, src: &[u8]) -> Option<String> {
    let string_node = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "string")?;
    let content = (0..string_node.child_count())
        .filter_map(|i| string_node.child(i))
        .find(|n| n.kind() == "string_content")?;
    content.utf8_text(src).ok().map(str::to_string)
}

/// A `procedure_declaration`/`struct_declaration`/`enum_declaration`/
/// `union_declaration`'s own name: the FIRST positional `identifier`
/// child (`add :: proc(...)`/`Dog :: struct {...}`, confirmed unfielded
/// via a real parse tree dump -- see `LangSpec::odin`'s own doc
/// comment). Every one of these four kinds shares this identical
/// leading-identifier-before-`::`-token shape.
fn odin_leading_identifier_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "identifier")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// A `struct_declaration`'s own direct-child `field` nodes (see
/// `LangSpec::odin`'s own doc comment for the shape -- entirely
/// unfielded: `identifier`, optionally preceded by a `using` keyword
/// token, then `:` and a `type`). Returns `(name, is_embedded)` per
/// field: for an ORDINARY field this is `(field_name, false)` (the
/// binding's own name, for a DEFINES edge); for a `using`-prefixed
/// composition field this is `(type_name, true)` -- the embedded TYPE's
/// own name (`Animal`, for `using animal: Animal`), not the local
/// binding name (`animal`), since an INHERITS edge must point at the
/// actual supertype-equivalent, not the field's own local identifier
/// (caught by this row's own hard test: a first-pass implementation used
/// the local binding name for both cases, silently recording `Dog
/// INHERITS animal` instead of `Dog INHERITS Animal`) -- directly
/// analogous to [`go_struct_fields`]'s identical `(member_name,
/// embedded)` pair for Go's own embedded-struct-field convention (see
/// `LangSpec::odin`'s own doc comment for the full finding).
fn odin_struct_fields(struct_node: Node<'_>, src: &[u8]) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for i in 0..struct_node.child_count() {
        let Some(field) = struct_node.child(i) else {
            continue;
        };
        if field.kind() != "field" {
            continue;
        }
        let mut is_embedded = false;
        let mut field_name = None;
        let mut type_name = None;
        for j in 0..field.child_count() {
            let Some(child) = field.child(j) else {
                continue;
            };
            match child.kind() {
                "using" => is_embedded = true,
                "identifier" if field_name.is_none() => {
                    field_name = child.utf8_text(src).ok().map(str::to_string);
                }
                "type" if type_name.is_none() => {
                    type_name = child.utf8_text(src).ok().map(str::to_string);
                }
                _ => {}
            }
        }
        if is_embedded {
            if let Some(name) = type_name {
                out.push((name, true));
            }
        } else if let Some(name) = field_name {
            out.push((name, false));
        }
    }
    out
}

/// A procedure's own `block` body, scoped recursion once its name (if
/// any) has been resolved via [`odin_leading_identifier_name`] --
/// `procedure_declaration` is fully claimed by [`odin_quirk`] before the
/// generic engine's own func/method branch would ever run its own
/// `body_field`-keyed lookup (which would fail regardless -- see
/// `LangSpec::odin`'s own doc comment), so this quirk re-implements the
/// DEFINES-push-plus-scoped-body-walk sequence directly, mirroring
/// [`zig_walk_container_body`]'s identical rationale.
fn odin_walk_scoped(
    node: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::odin();
    let quirks = odin_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, &ctx, out, enclosing, fn_scope);
        }
    }
}

/// Every direct-child node this grammar tags with the REPEATED
/// `"argument"` field (singular -- `node-types.json` confirms
/// `call_expression`'s own field is literally named `"argument"`, not a
/// single `"arguments"` field wrapping a list the way most of this
/// crate's other call-shaped nodes have) -- found via
/// [`Node::field_name_for_child`] rather than
/// [`Node::child_by_field_name`], which only ever returns the FIRST
/// match for a repeated field (this crate's shared [`call_arg_texts`]
/// helper assumes a single wrapper node to iterate, which this grammar
/// does not have for `call_expression` at all -- reusing it here would
/// have silently returned only the first argument, or none, rather than
/// every one).
fn odin_call_arg_texts(call_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..call_node.child_count() {
        if call_node.field_name_for_child(i as u32) != Some("argument") {
            continue;
        }
        if let Some(child) = call_node.child(i) {
            if let Ok(text) = child.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// `call_expression`/`selector_call_expression`, both wired as
/// [`Quirks::call_override`]:
/// - `call_expression` DOES have a real, working `"function"` field
///   (confirmed in `node-types.json`) -- the generic engine's own
///   default single-field callee reconstruction would already get the
///   callee text right; this override exists purely to fix up
///   [`odin_call_arg_texts`]'s own repeated-`"argument"`-field shape
///   (see that function's own doc comment), which the generic engine's
///   shared `call_arg_texts` helper cannot read correctly for this
///   grammar -- without this override every Odin call would silently
///   record zero/one argument regardless of how many were actually
///   written.
/// - `selector_call_expression` is Odin's pointer-dereference
///   method-call syntax (`p->bark()`) -- see `LangSpec::odin`'s own doc
///   comment for the full grammar-shape finding: this node's OWN
///   top-level `"function"` field holds the pointer identifier (`p`),
///   not the eventual method name; the real callee (`bark`) is one
///   level down inside its nested `call_expression` child, whose OWN
///   `"function"` field holds it directly.
fn odin_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "call_expression" => {
            let Some(callee_node) = node.child_by_field_name("function") else {
                return false;
            };
            let Ok(callee) = callee_node.utf8_text(src) else {
                return false;
            };
            let (receiver_text, receiver_hint) = receiver_of_call(callee_node, src);
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text,
                receiver_hint,
                arg_texts: odin_call_arg_texts(node, src),
            });
            true
        }
        "selector_call_expression" => {
            let Some(pointer) = node.child_by_field_name("function") else {
                return false;
            };
            let Ok(receiver_text) = pointer.utf8_text(src) else {
                return false;
            };
            let Some(inner_call) = (0..node.child_count())
                .filter_map(|i| node.child(i))
                .find(|n| n.kind() == "call_expression")
            else {
                return false;
            };
            let Some(callee_node) = inner_call.child_by_field_name("function") else {
                return false;
            };
            let Ok(callee) = callee_node.utf8_text(src) else {
                return false;
            };
            let receiver_hint = if receiver_text == "self" || receiver_text == "this" {
                ReceiverHint::SelfOrThis
            } else {
                ReceiverHint::Identifier
            };
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: Some(receiver_text.to_string()),
                receiver_hint: Some(receiver_hint),
                arg_texts: odin_call_arg_texts(inner_call, src),
            });
            true
        }
        _ => false,
    }
}

pub fn odin_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(odin_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(odin_call_override)),
    }
}

/// Parse Odin source through the generic engine. Grammar:
/// `tree-sitter-odin` (crates.io, `tree-sitter-grammars` GitHub org
/// lineage -- the same actively-maintained org this crate's Kotlin
/// grammar dependency already comes from). Odin has no pre-existing
/// bespoke `languages::odin` extractor -- correctness rests entirely on
/// direct verification against the crate's own `node-types.json`, a real
/// parse tree dump, and the C baseline's `odin_*` arrays (which give
/// Odin no dedicated `extract_base_classes` walker at all -- see
/// `LangSpec::odin`'s own doc comment), not byte-for-byte comparison
/// against an oracle.
pub fn parse_odin(source: &str) -> ParsedFile {
    let spec = LangSpec::odin();
    let quirks = odin_quirks();
    let language: tree_sitter::Language = tree_sitter_odin::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}
/// Pascal's `declType` (the real name-bearer for every `declClass`/
/// `declIntf`/`declHelper`/`declObject`/`declRecord` shape, see
/// `LangSpec::pascal`'s own doc comment), `defProc` (out-of-body
/// implementation wrapper), `unit`/`program` (module naming), and
/// `declUses` (multi-unit import list) -- everything `LangSpec::pascal`'s
/// own flat arrays cannot express. Wired as this wave's
/// [`Quirks::on_unmatched_node`] hook. `exprCall`/`exprDot` are a
/// separate [`Quirks::call_override`] hook (see [`pascal_call_override`])
/// since the generic walker's own call branch dispatches to
/// `call_override` BEFORE `on_unmatched_node` ever runs for a
/// `call_types` node. `declProc` (the ordinary, in-place forward
/// declaration/bodyless-signature case) needs NO quirk at all: its own
/// real `"name"` field flows through the generic engine's default
/// func/method branch unchanged -- only `defProc` (whose name lives on a
/// NESTED `declProc`, not itself) needs special handling here.
fn pascal_quirk(
    node: Node<'_>,
    _enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        // Not unconditionally `true`: `pascal_walk_decl_type` itself
        // returns whether `node` was one of the five class-shapes it
        // knows how to handle -- a plain type alias (`TId = Integer;`,
        // `type_node.kind()` none of `declClass`/`declIntf`/`declHelper`/
        // `declObject`/`declRecord`) falls through to `false`, so THIS
        // arm falls through to ordinary generic recursion for it too
        // (matches `pascal_walk_decl_type`'s own doc comment: "leaves it
        // entirely to ordinary generic recursion" is only true if this
        // caller actually lets that recursion happen).
        "declType" => pascal_walk_decl_type(node, src, out),
        "defProc" => {
            pascal_walk_def_proc(node, src, out);
            true
        }
        "unit" | "program" => {
            if let Some(name) = pascal_module_name(node, src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Module,
                    line: node.start_position().row + 1,
                });
            }
            // Not fully claimed (`false`): a `unit`/`program` node's own
            // children (`interface`/`implementation`/`declUses`/
            // `defProc`/`block`/...) still need ordinary generic
            // recursion to find every declaration/call inside them --
            // this arm only adds the ONE extra Module symbol the flat
            // `module_types` array cannot express for this grammar (see
            // `LangSpec::pascal`'s own doc comment: neither node has any
            // fields at all, so there is nothing further this specific
            // arm needs to claim).
            false
        }
        "declUses" => {
            for path in pascal_uses_paths(node, src) {
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

/// A `unit`/`program` node's own `moduleName` child (entirely
/// positional/unfielded, see `LangSpec::pascal`'s own doc comment) --
/// its own text spans the whole dotted name already (`identifier` alone,
/// or `identifier "." identifier` for a namespaced unit), so no further
/// unwrapping is needed beyond finding the child and reading its text.
fn pascal_module_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "moduleName")?
        .utf8_text(src)
        .ok()
        .map(str::to_string)
}

/// A `declUses` node's own `moduleName` children (see
/// `LangSpec::pascal`'s own doc comment) -- a single `uses A, B, C.D;`
/// clause holds one `moduleName` per comma-separated unit.
fn pascal_uses_paths(node: Node<'_>, src: &[u8]) -> Vec<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|n| n.kind() == "moduleName")
        .filter_map(|n| n.utf8_text(src).ok())
        .map(str::to_string)
        .collect()
}

/// A `declType`'s own `"name"` field (`TDog` in `TDog = class(...) ...
/// end`) plus its `"type"` field's inner `declClass`/`declIntf`/
/// `declHelper`/`declObject`/`declRecord` node -- the real name-bearer
/// for every one of `LangSpec::pascal`'s `class_types` (see that const's
/// own doc comment: none of those five kinds has a `"name"` field of its
/// own at all; the name lives one level UP, on this wrapping `declType`).
/// Pushes the Class/Interface symbol, any `declClass`-only INHERITS
/// edges (mirrors `internal/cbm/extract_defs.c`'s `extract_base_classes`
/// Pascal-specific `parent`-field walker exactly -- see
/// `LangSpec::pascal`'s own doc comment), and recurses into the class
/// body scoped to this name so nested `declField`/`declProp`/`declProc`
/// members are found via ordinary generic recursion (each already flows
/// through the generic engine's own field/func DEFINES-container logic
/// once `enclosing` is set here).
fn pascal_walk_decl_type(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let Ok(name_text) = name.utf8_text(src) else {
        return false;
    };
    let name_text = name_text.to_string();
    let Some(type_node) = node.child_by_field_name("type") else {
        return false;
    };
    let line = node.start_position().row + 1;
    let kind = match type_node.kind() {
        "declIntf" => SymbolKind::Interface,
        "declClass" | "declHelper" | "declObject" | "declRecord" => SymbolKind::Class,
        // A `declType` can also wrap an ordinary (non-class-shaped) type
        // expression (`TId = Integer;`, a type alias) -- not one of
        // `LangSpec::pascal`'s `class_types` at all, so this quirk leaves
        // it entirely to ordinary generic recursion (via this function's
        // own `false` return, which `pascal_quirk`'s `"declType"` arm
        // passes straight through) rather than guessing at a symbol kind
        // for a shape it was never meant to classify (the alias case is
        // out of this wave's own scope, see `LangSpec::pascal`'s own
        // `alias_types` being deliberately empty).
        _ => return false,
    };
    out.symbols.push(SymbolRef {
        name: name_text.clone(),
        kind,
        line,
    });
    if type_node.kind() == "declClass" {
        for base_name in pascal_class_parents(type_node, src) {
            out.inherits.push(InheritsRef {
                sub_name: name_text.clone(),
                super_name: base_name,
                line,
            });
        }
    }
    pascal_walk_scoped(type_node, src, Some(name_text.as_str()), out);
    true
}

/// A `declClass` node's own `"parent"`-tagged heritage children
/// (`class(TAnimal, IFoo)`) -- mirrors
/// `internal/cbm/extract_defs.c`'s `extract_base_classes` Pascal-specific
/// walker byte-for-byte (see `LangSpec::pascal`'s own doc comment): every
/// child whose OWN field name is literally `"parent"`, skipping the
/// unnamed `(`/`)`/`,` delimiter tokens the grammar ALSO tags `"parent"`
/// (`is_named()` is the exact filter the baseline's own
/// `ts_node_is_named` check uses).
fn pascal_class_parents(decl_class: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..decl_class.child_count() {
        if decl_class.field_name_for_child(i as u32) != Some("parent") {
            continue;
        }
        let Some(child) = decl_class.child(i) else {
            continue;
        };
        if !child.is_named() {
            continue;
        }
        if let Ok(text) = child.utf8_text(src) {
            if !text.is_empty() {
                out.push(text.to_string());
            }
        }
    }
    out
}

/// A `declClass`/`declIntf`/`declHelper`/`declObject`/`declRecord`'s own
/// scoped recursion once its name has been resolved via
/// [`pascal_walk_decl_type`] -- these five kinds are fully claimed by
/// [`pascal_quirk`]'s own `declType` arm (which calls this directly)
/// before the generic engine's own class-shape branch would ever
/// classify any of them correctly (their own `name_field`-keyed
/// `child_text` lookup always fails -- see `LangSpec::pascal`'s own doc
/// comment), so this quirk re-implements the DEFINES-container-scoped
/// recursion directly, mirroring [`zig_walk_container_body`]'s identical
/// rationale. Also reused by [`pascal_walk_def_proc`] for an out-of-line
/// method implementation's own body walk.
fn pascal_walk_scoped(node: Node<'_>, src: &[u8], enclosing: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::pascal();
    let quirks = pascal_quirks();
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

/// A `defProc` node's own `"header"` (a NESTED `declProc`) and `"body"`
/// fields (see `LangSpec::pascal`'s own doc comment for the full
/// grammar-shape finding: `defProc` itself has no `"name"` field at all,
/// only these two). Reads the header's own `"name"` field directly
/// (which can itself be a `genericDot` node for an out-of-line class-
/// method implementation, e.g. `TDog.Bark` -- `child_text`'s own
/// `.utf8_text()` read already returns that whole dotted text correctly,
/// no further unwrapping needed), pushes a Method symbol plus a DEFINES
/// edge to the owning class when the name is dotted (`TDog.Bark` ->
/// container `TDog`, member `Bark`, mirroring every other out-of-line
/// method-scoped language's own DEFINES convention) or a Function symbol
/// otherwise (a plain top-level `procedure P; begin ... end;`
/// implementation, no dotted qualifier at all), then walks the OUTER
/// node's own `"body"` field with the resolved (possibly dot-qualified)
/// name as `fn_scope` -- mirrors [`dart_walk_function_body`]'s identical
/// "read the nested signature's name, walk the outer node's own body"
/// mechanics.
fn pascal_walk_def_proc(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    let Some(header) = node.child_by_field_name("header") else {
        return;
    };
    let name = child_text(header, "name", src);
    let line = node.start_position().row + 1;
    if let Some(name) = &name {
        if let Some((container, member)) = name.rsplit_once('.') {
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Method,
                line,
            });
            out.defines.push(DefinesRef {
                container_name: container.to_string(),
                member_name: member.to_string(),
                line,
            });
        } else {
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Function,
                line,
            });
        }
    }
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let spec = LangSpec::pascal();
    let quirks = pascal_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let fn_scope = FnScope {
        name: name.as_deref(),
        line: Some(line),
    };
    walk(body, &ctx, out, None, fn_scope);
}

/// `exprCall`'s own `"entity"` (callee, NOT `"function"`) and `"args"`
/// (a WRAPPING `exprArgs` node, NOT a bare `"arguments"` field) --
/// neither of `LangSpec::pascal`'s own `call_function_field`/
/// `call_arguments_field` defaults match this grammar's real field
/// names, so this override reads both directly (see `LangSpec::pascal`'s
/// own doc comment). `exprDot` (a bare, PARENLESS method-style call
/// statement, `Obj.Draw;`) has an entirely different `"lhs"`/`"rhs"`
/// shape and is ALSO syntactically indistinguishable from an ordinary,
/// non-call field-read at the node-kind level alone -- recorded as a
/// call anyway (see `LangSpec::pascal`'s own doc comment for the full
/// "no type-checker, syntax-only, biased toward over- rather than
/// under-recording a CALLS edge" rationale), joining `"lhs"`'s own text
/// with `.` and `"rhs"`'s own text as the callee (`"Obj.Draw"`, matching
/// how a real `exprCall` with a dotted `exprDot` `"entity"` -- e.g.
/// `Exception.Create(...)` -- already records its own fully-qualified
/// callee text the same way, so a parenless and a parenthesized dotted
/// call agree on their own callee text shape).
fn pascal_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    match node.kind() {
        "exprCall" => {
            let Some(entity) = node.child_by_field_name("entity") else {
                return false;
            };
            let Ok(callee) = entity.utf8_text(src) else {
                return false;
            };
            let arg_texts = node
                .child_by_field_name("args")
                .map(|args| pascal_expr_args_texts(args, src))
                .unwrap_or_default();
            out.calls.push(CallRef {
                callee: callee.to_string(),
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: None,
                receiver_hint: None,
                arg_texts,
            });
            true
        }
        "exprDot" => {
            let Some(lhs) = node.child_by_field_name("lhs") else {
                return false;
            };
            let Some(rhs) = node.child_by_field_name("rhs") else {
                return false;
            };
            let (Ok(lhs_text), Ok(rhs_text)) = (lhs.utf8_text(src), rhs.utf8_text(src)) else {
                return false;
            };
            let callee = format!("{lhs_text}.{rhs_text}");
            // `inherited.MethodName` (Pascal's `super`-equivalent, calling
            // the parent class's overridden implementation) is its own
            // distinct, dedicated `"inherited"` node kind (confirmed in
            // `node-types.json`: `{"type":"inherited","fields":{}}`) --
            // checked by KIND rather than by text (`lhs_text == "self"`
            // is still a text check since a plain `self`-named receiver
            // has no dedicated node kind of its own the way `inherited`
            // does, matching every other language's identical `"self" |
            // "this"` text-comparison convention for the ordinary case).
            let receiver_hint = if lhs_text == "self" || lhs.kind() == "inherited" {
                Some(ReceiverHint::SelfOrThis)
            } else if lhs.kind() == "identifier" {
                Some(ReceiverHint::Identifier)
            } else {
                Some(ReceiverHint::Other)
            };
            out.calls.push(CallRef {
                callee,
                line: node.start_position().row + 1,
                from_symbol: from_symbol.map(str::to_string),
                from_symbol_line,
                receiver_text: Some(lhs_text.to_string()),
                receiver_hint,
                arg_texts: Vec::new(),
            });
            true
        }
        _ => false,
    }
}

/// An `exprArgs` node's own named-child argument expressions (see
/// `pascal_call_override`'s own doc comment: `exprCall`'s `"args"` field
/// is this WRAPPING node, not a bare list itself).
fn pascal_expr_args_texts(args_node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..args_node.child_count() {
        if let Some(child) = args_node.child(i) {
            if !child.is_named() {
                continue;
            }
            if let Ok(text) = child.utf8_text(src) {
                out.push(text.to_string());
            }
        }
    }
    out
}

pub fn pascal_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(pascal_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(pascal_call_override)),
    }
}

/// Parse Pascal source through the generic engine. Grammar:
/// `tree-sitter-pascal` (crates.io, `Isopod/tree-sitter-pascal` lineage --
/// the same distinctive `declProc`/`declClass`/`exprCall`/`kIf`-prefixed
/// node-kind vocabulary the C baseline's own `pascal_*` arrays already
/// use verbatim, confirming this crate binds the identical grammar the
/// baseline vendored). Pascal has no pre-existing bespoke
/// `languages::pascal` extractor -- correctness rests entirely on direct
/// verification against the crate's own `node-types.json`, a real parse
/// tree dump, and the C baseline's `pascal_*` arrays plus its dedicated
/// `extract_base_classes` Pascal-specific walker (see `LangSpec::pascal`'s
/// own doc comment), not byte-for-byte comparison against an oracle.
pub fn parse_pascal(source: &str) -> ParsedFile {
    let spec = LangSpec::pascal();
    let quirks = pascal_quirks();
    let language: tree_sitter::Language = tree_sitter_pascal::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// QML
// =====================================================================

/// `new Widget()`'s callee lives in a `constructor` field, not
/// `function` -- confirmed via a real parse-tree dump (see
/// [`LangSpec::qml`]'s own doc comment). `call_expression` itself has a
/// clean `function`/`arguments` field pair needing no override at all
/// (returning `false` here lets it fall through to the generic engine's
/// own default single-field reconstruction, per
/// [`LangSpec::qml`]'s `call_function_field: "function"`) -- only
/// `new_expression` is actually claimed.
fn qml_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "new_expression" {
        return false;
    }
    let Some(constructor) = node.child_by_field_name("constructor") else {
        return false;
    };
    let Ok(callee) = constructor.utf8_text(src) else {
        return false;
    };
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text: None,
        receiver_hint: None,
        arg_texts: call_arg_texts(node, "arguments", src),
    });
    true
}

/// `component Circle: Rectangle {...}` -- an inline-component
/// declaration whose own name lives on a `name` field (unlike its
/// sibling `class_declaration`/`abstract_class_declaration`/
/// `enum_declaration`, which the ordinary flat `name_field`/`body_field`
/// path already handles), but whose "body" is a nested
/// `ui_object_definition` under a `component` field, not a plain
/// `body`-field block -- recurses into that nested object's own
/// `ui_object_initializer` generically with `enclosing` set to the
/// component's name, so any property/signal/function inside it still
/// gets a DEFINES edge and a correctly-scoped walk. Also handles
/// `ui_import`/`import_statement`/`import` (IMPORTS path extraction --
/// none of the three has a single clean "whole path" field the way an
/// ordinary import scan could read) -- see [`LangSpec::qml`]'s own doc
/// comment for both findings. Falls through to [`ts_quirk`] for every
/// other node kind (heritage clauses, decorators, arrow/const-binding
/// [`SymbolKind::Lambda`], ... -- see [`qml_quirks`]'s own doc comment
/// for why reusing it directly is correct here, not merely convenient).
fn qml_quirk(node: Node<'_>, enclosing: Option<&str>, src: &[u8], out: &mut ParsedFile) -> bool {
    match node.kind() {
        "ui_inline_component" => {
            let Some(name) = child_text(node, "name", src) else {
                return true;
            };
            let line = node.start_position().row + 1;
            out.symbols.push(SymbolRef {
                name: name.clone(),
                kind: SymbolKind::Class,
                line,
            });
            if let Some(component) = node.child_by_field_name("component") {
                if let Some(initializer) = component.child_by_field_name("initializer") {
                    qml_walk_scoped_body(initializer, src, Some(name.as_str()), out);
                }
            }
            true
        }
        "ui_import" => {
            if let Some(source) = node.child_by_field_name("source") {
                if let Ok(path) = source.utf8_text(src) {
                    out.imports.push(ImportRef {
                        module_path: strip_quotes_str(path),
                        line: node.start_position().row + 1,
                    });
                }
            }
            true
        }
        "import_statement" | "import" => {
            if let Some(path) = first_named_child_text(node, src) {
                out.imports.push(ImportRef {
                    module_path: strip_quotes_str(&path),
                    line: node.start_position().row + 1,
                });
            }
            true
        }
        _ => ts_quirk(node, enclosing, src, out),
    }
}

/// Strip a single layer of matching leading/trailing `"`/`'` quotes, if
/// present -- `ui_import`'s `source` field's own text includes the
/// quote characters for the string-literal form (`"./helpers.js"`),
/// unlike its bare-identifier form (`QtQuick`, no quotes to strip).
fn strip_quotes_str(text: &str) -> String {
    let bytes = text.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return text[1..text.len() - 1].to_string();
        }
    }
    text.to_string()
}

/// [`ui_inline_component`]'s own nested `ui_object_definition`'s
/// `ui_object_initializer` body walk, scoped to the component's own
/// name -- mirrors [`ts_walk_scoped_body`]'s identical "resolve name
/// externally via the quirk, then re-enter the generic walk with
/// `enclosing` set" shape, reusing QML's own spec/quirks (not TS's)
/// since a nested property/signal/function inside the inline
/// component's body still needs QML's own `ui_property`/`ui_signal`
/// field-shape recognition, not just TS's.
fn qml_walk_scoped_body(node: Node<'_>, src: &[u8], name: Option<&str>, out: &mut ParsedFile) {
    let spec = LangSpec::qml();
    let quirks = qml_quirks();
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

/// QML's [`Quirks`] row: `ui_inline_component` naming + scoped body
/// walk, `ui_import`/`import_statement`/`import` IMPORTS-path
/// extraction (none has a single clean path field), and
/// `new_expression` call reconstruction (`constructor` field, not
/// `function`) alongside the ordinary `call_expression` path. Every
/// other construct -- `class_declaration`/`abstract_class_declaration`/
/// `enum_declaration`/`interface_declaration` (heritage clauses,
/// decorators -- identical field shapes to
/// [`Self::typescript`]/[`ts_quirk`], confirmed via the same parse-tree
/// dump), `function_declaration`/`method_definition`,
/// `ui_property`/`ui_signal`/`public_field_definition` fields, branches
/// -- is fully covered by [`LangSpec::qml`]'s own flat arrays with no
/// quirk needed, so [`ts_quirk`] is reused directly for the
/// class-shaped kinds rather than re-implemented (this means a
/// `class_declaration`'s own body walk goes through [`ts_walk_scoped_body`],
/// which hardcodes plain TS's spec/quirks rather than QML's own --
/// deliberately correct, not a gap: a QML `class {...}` body is an
/// embedded plain-JS class, and this grammar's own `class_body` node
/// kind cannot syntactically contain `ui_property`/`ui_signal`/
/// `ui_inline_component` at all -- those are only ever valid inside a
/// `ui_object_definition`'s own `ui_object_initializer`, confirmed via
/// the same parse-tree dump, so there is nothing QML-specific a nested
/// class body walk could ever need to recognize).
pub fn qml_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(qml_quirk)),
        is_test_name: ts_is_test_name,
        route_from_call: Some(Box::new(ts_route_from_call)),
        on_method_defined: Some(Box::new(ts_on_method_defined)),
        call_override: Some(Box::new(qml_call_override)),
    }
}

/// Parse QML source through the generic engine. Language-parity wave
/// G2.2e.
pub fn parse_qml(source: &str) -> ParsedFile {
    let spec = LangSpec::qml();
    let quirks = qml_quirks();
    let language: tree_sitter::Language = tree_sitter_qmljs::LANGUAGE.into();
    parse_with_spec(source, &language, &spec, &quirks, true)
}

// =====================================================================
// ReScript
// =====================================================================

/// `let add = (a, b) => {...}`'s `function` node has no `name` field of
/// its own -- the written name is the enclosing `let_binding`'s own
/// `pattern` field. Mirrors `internal/cbm/extract_defs.c`'s
/// `cbm_resolve_func_name` ReScript case exactly (see [`LangSpec::rescript`]'s
/// own doc comment): a `let_binding` whose `body` is anything other than
/// a `function` (a plain value binding, `let x = 42`) never reaches
/// this arm at all, since [`LangSpec::rescript`]'s own `func_types`
/// array only ever matches the `function` node kind itself.
fn rescript_function_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let parent = node.parent()?;
    if parent.kind() != "let_binding" {
        return None;
    }
    let pattern = parent.child_by_field_name("pattern")?;
    pattern.utf8_text(src).ok().map(str::to_string)
}

/// `function`'s own scoped body walk once its name has been resolved
/// via [`rescript_function_name`] -- mirrors
/// [`zig_walk_container_body`]'s identical "resolve name externally,
/// then re-enter the generic walk with `enclosing`/`fn_scope` set"
/// shape. Unlike a class/struct's scoped body (which sets `enclosing`
/// for member DEFINES edges), a function's own body sets `fn_scope`
/// instead (calls made directly inside it need `from_symbol` to record
/// this function's name) -- `enclosing` (the class/module a nested
/// symbol would DEFINE into) is threaded through unchanged from
/// whatever the caller already had, matching how [`walk`]'s own
/// generic func/method branch threads `enclosing` unchanged into its
/// own body walk.
fn rescript_walk_function_body(
    node: Node<'_>,
    src: &[u8],
    enclosing: Option<&str>,
    fn_scope: FnScope<'_>,
    out: &mut ParsedFile,
) {
    let spec = LangSpec::rescript();
    let quirks = rescript_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    walk_children(body, &ctx, out, enclosing, fn_scope);
}

/// `module Point = { ... }` / `type t = { ... }` -- neither
/// `module_declaration` nor `type_declaration` has a `name` field of
/// its own; the real name lives one level down, on their sole child's
/// own `name` field (`module_binding.name`/`type_binding.name`). Mirrors
/// `internal/cbm/extract_defs.c`'s dedicated `CBM_LANG_RESCRIPT` case
/// for `type_declaration` exactly (see [`LangSpec::rescript`]'s own doc
/// comment), extended identically for `module_declaration`/
/// `module_binding`. Recurses into the binding's own nested
/// definition/body afterward (`module_binding`'s `definition` field or
/// `type_binding`'s `body` field) so any call/nested-type inside it is
/// still visited, scoped to this symbol's own name.
fn rescript_class_quirk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let (wrapper_kind, kind_out) = match node.kind() {
        "module_declaration" => ("module_binding", SymbolKind::Class),
        "type_declaration" => ("type_binding", SymbolKind::TypeAlias),
        _ => return false,
    };
    let Some(binding) = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == wrapper_kind)
    else {
        return true;
    };
    let Some(name) = child_text(binding, "name", src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: kind_out,
        line,
    });
    let spec = LangSpec::rescript();
    let quirks = rescript_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    walk_children(binding, &ctx, out, Some(name.as_str()), FnScope::default());
    true
}

/// `open Belt` / `include MyModule` -- neither has a field of its own;
/// the module path is the sole positional named child
/// (`module_identifier`). Reuses [`first_named_child_text`], the same
/// helper [`walk`]'s own `module_types` branch already uses for an
/// analogous "the interesting name is just the first named child"
/// shape.
fn rescript_import_quirk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    if !matches!(node.kind(), "open_statement" | "include_statement") {
        return false;
    }
    if let Some(path) = first_named_child_text(node, src) {
        out.imports.push(ImportRef {
            module_path: path,
            line: node.start_position().row + 1,
        });
    }
    true
}

/// `@react.component`'s own name -- a preceding sibling of the
/// `let_declaration` it annotates (same shape
/// [`ts_preceding_decorators`]'s own `prev_sibling()` walk already
/// handles), but this grammar's own `decorator` node wraps a single
/// `decorator_identifier` child rather than TS's `call_expression`/
/// `identifier` split, so [`first_named_child_text`] reads its name
/// directly rather than [`ts_decorator_name`]'s TS-specific match.
fn rescript_preceding_decorators(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(candidate) = sibling {
        if candidate.kind() != "decorator" {
            break;
        }
        if let Some(name) = first_named_child_text(candidate, src) {
            out.push(name);
        }
        sibling = candidate.prev_sibling();
    }
    out.reverse();
    out
}

/// ReScript's [`Quirks`] row: `function`'s parent-`let_binding`-`pattern`
/// name resolution + scoped body walk (`func_types`/`method_types`'s
/// sole entry), `module_declaration`/`type_declaration`'s
/// child-binding name resolution (`class_types`), `open_statement`/
/// `include_statement` IMPORTS-path extraction, and `@decorator`
/// DECORATES edges on the annotated `let_declaration`. Every other
/// construct (`call_expression`'s ordinary `function`/`arguments`
/// field pair, `if_expression`/`switch_expression`/`try_expression`
/// branches) is fully covered by [`LangSpec::rescript`]'s own flat
/// arrays with no quirk needed.
fn rescript_quirk(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() == "function" {
        // Already claimed by `walk`'s own func/method branch if the
        // ordinary `name_field` lookup happened to succeed -- for
        // `function`, [`LangSpec::rescript`]'s own `name_field` is a
        // placeholder that never resolves (this grammar's `function`
        // node has no `name` field at all), so control always reaches
        // here for a well-formed `function` node.
        let Some(name) = rescript_function_name(node, src) else {
            return true;
        };
        let line = node.start_position().row + 1;
        // Always `Function`, never `Method`, even when `enclosing` is
        // set (a `function` nested inside a `module Foo = {...}`):
        // ReScript has no OOP method/instance concept at all -- a
        // module is a compile-time namespace, not an instantiable
        // type, so a function bound inside one is still, semantically,
        // just a function. Deliberate, not the generic engine's own
        // usual same-kind-in-both-arrays nesting-based Function-vs-
        // Method fallback (this quirk fully owns naming for `function`
        // already, so that shared convention never actually applies
        // here regardless).
        out.symbols.push(SymbolRef {
            name: name.clone(),
            kind: SymbolKind::Function,
            line,
        });
        if let Some(container) = enclosing {
            out.defines.push(DefinesRef {
                container_name: container.to_string(),
                member_name: name.clone(),
                line,
            });
        }
        rescript_walk_function_body(
            node,
            src,
            enclosing,
            FnScope {
                name: Some(&name),
                line: Some(line),
            },
            out,
        );
        // A decorator (`@react.component`) precedes the WHOLE
        // `let_declaration` (`function`'s grandparent: `function`'s
        // parent is `let_binding`, whose own parent is
        // `let_declaration`), not `function` itself or its immediate
        // `let_binding` parent -- confirmed via the real parse-tree
        // dump (`decorator` and `let_declaration` are siblings at
        // `source_file` level).
        if let Some(let_declaration) = node.parent().and_then(|let_binding| let_binding.parent()) {
            for decorator_name in rescript_preceding_decorators(let_declaration, src) {
                out.decorates.push(crate::parsers::DecoratesRef {
                    target_name: name.clone(),
                    decorator_name,
                    line,
                });
            }
        }
        return true;
    }
    if rescript_class_quirk(node, src, out) {
        return true;
    }
    rescript_import_quirk(node, src, out)
}

pub fn rescript_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(rescript_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: None,
    }
}

/// Parse ReScript source through the generic engine. Language-parity
/// wave G2.2e. Grammar sourced from the `arborium-rescript` crate (part
/// of the `bearcove/arborium` tree-sitter grammar bundle), NOT a
/// standalone `tree-sitter-rescript` -- crates.io has no such crate at
/// all (confirmed via a direct crates.io API search, not assumed).
/// `arborium-rescript` exports the same `tree-sitter-language::LanguageFn`
/// shape every other grammar dependency in this workspace already
/// uses, cross-checked node-kind-by-node-kind against
/// `internal/cbm/lang_specs.c`'s own `rescript_*` arrays (every single
/// one confirmed present, matching the exact same grammar shape the
/// baseline's own vendored `internal/cbm/vendored/grammars/rescript/`
/// copy -- author Victor Nakoryakov per that directory's own `LICENSE`
/// -- targets) via a real parse-tree dump, not `node-types.json`
/// inspection alone.
pub fn parse_rescript(source: &str) -> ParsedFile {
    let spec = LangSpec::rescript();
    let quirks = rescript_quirks();
    let language: tree_sitter::Language = arborium_rescript::language().into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}

// =====================================================================
// Squirrel
// =====================================================================

/// A `function_declaration`/`anonymous_function`/`lambda_expression`'s
/// own name -- the FIRST direct-child `identifier` (confirmed via a
/// real parse-tree dump: `function makeDog(name) {...}`'s own children
/// are literally `function`(keyword), `identifier`(name=`makeDog`),
/// `(`, `parameters`, `)`, `block` -- the `identifier` inside
/// `parameters` is one level deeper, so a direct-child-only scan never
/// confuses the two). Mirrors `internal/cbm/extract_defs.c`'s own
/// dedicated case exactly ("Cairo / D / Odin / Squirrel: the def node
/// has no `name` field; the name is a plain `identifier` child" --
/// :709-718). Returns `None` for `anonymous_function`/`lambda_expression`
/// (neither has a name-bearing `identifier` in this position at all --
/// confirmed via the dump: `anonymous_function`'s own direct children
/// start straight with `function`/`(`, no `identifier` in between), the
/// same "no name to push" outcome [`c_quirk`]'s anonymous-struct case
/// and [`kotlin_quirk`]'s `secondary_constructor` case already have.
fn squirrel_def_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "identifier")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// The LAST direct-child `block` of a `function_declaration`/
/// `anonymous_function`/`lambda_expression` -- this grammar's own
/// function-body node, positional rather than field-accessible
/// (confirmed absent from `node-types.json`'s own `"fields"` entry for
/// all three kinds). `lambda_expression`'s own body is a bare
/// expression, not a `block`, at all (`@(x) x + 1` has no braces) --
/// returns `None` for it, matching that its own construct has nothing
/// this generic engine's own DEFINES-scoped-body-walk concept could
/// usefully recurse into as a symbol/call-bearing block anyway (a
/// lambda's own single expression can still itself contain a call,
/// which the caller's own fallback `walk_children` recursion -- run
/// unconditionally by [`squirrel_quirk`] regardless of whether a
/// `block` was found -- already reaches with no special-casing needed
/// here).
fn squirrel_def_block(node: Node<'_>) -> Option<Node<'_>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .rfind(|n| n.kind() == "block")
}

/// `class Dog extends Animal {...}` -- heritage is a plain `identifier`
/// child directly following the literal `extends` KEYWORD TOKEN
/// (`"named": false` in `node-types.json` -- an unnamed token, so
/// [`Node::child`] rather than [`Node::named_child`] is required to
/// even see it at all). Mirrors `internal/cbm/extract_defs.c`'s own
/// dedicated `extract_base_classes` Squirrel walker (:2395-2421)
/// exactly: scan direct children for the `"extends"` token, then take
/// the very next `identifier` sibling as the one base-class name (this
/// grammar allows only single inheritance, confirmed via
/// `node-types.json`'s own `class_declaration` entry listing at most
/// one non-member-declaration/non-attribute-declaration identifier
/// position beyond the class's own name).
fn squirrel_base_class_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let mut seen_extends = false;
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() == "extends" {
            seen_extends = true;
            continue;
        }
        if seen_extends && child.kind() == "identifier" {
            return child.utf8_text(src).ok().map(str::to_string);
        }
    }
    None
}

/// A `class_declaration`'s own DEFINES-scoped member walk: this
/// grammar's `class_declaration` has NO `body`/`class_body` wrapper
/// node at all (confirmed via `internal/cbm/extract_defs.c`'s own
/// "Squirrel: class_declaration has no body field — member_declaration
/// nodes ... are direct children of the class" comment, :3936-3939,
/// and the same parse-tree dump) -- `member_declaration` nodes are
/// direct children of the class node itself, interspersed with the
/// class's own name `identifier` and (if present) the `extends`
/// keyword + base-class `identifier`. Each `member_declaration` in turn
/// wraps a single `function_declaration` for a named method (mirrors
/// `internal/cbm/extract_defs.c`'s own "Squirrel wraps each class
/// member in a member_declaration node; the method is the inner
/// function_declaration. Peek through the wrapper." comment,
/// :4192-4200) -- a `constructor(...) {...}` member, by contrast, has
/// no such inner `function_declaration` at all (confirmed via the same
/// dump), so it is correctly invisible here too, matching that same
/// baseline comment's own real (limited) reach (see [`LangSpec::squirrel`]'s
/// own doc comment for the full finding). A bare field-assignment
/// member (`name = "";`) is likewise correctly invisible (no
/// `function_declaration` inside its own `member_declaration` either,
/// and [`LangSpec::squirrel`]'s own `field_types` is empty by design).
fn squirrel_walk_class_members(node: Node<'_>, src: &[u8], class_name: &str, out: &mut ParsedFile) {
    let spec = LangSpec::squirrel();
    let quirks = squirrel_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else { continue };
        if child.kind() != "member_declaration" {
            continue;
        }
        let Some(inner) = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.kind() == "function_declaration")
        else {
            continue;
        };
        walk(inner, &ctx, out, Some(class_name), FnScope::default());
    }
}

/// `class_declaration`/`enum_declaration` -- neither has a `name`
/// field; the name is the class/enum's own first direct-child
/// `identifier` (immediately after the `class`/`enum` keyword, before
/// any `extends`/`{`). Mirrors `internal/cbm/extract_defs.c`'s own
/// dedicated case exactly ("CBM_LANG_SQUIRREL: class_declaration >
/// identifier", :3659, extended identically for `enum_declaration` per
/// the confirmed real parse-tree shape -- see [`LangSpec::squirrel`]'s
/// own doc comment). `class_declaration` additionally records an
/// INHERITS edge via [`squirrel_base_class_name`] and its own scoped
/// member walk via [`squirrel_walk_class_members`]; `enum_declaration`
/// needs neither (Squirrel enums have no heritage and
/// [`LangSpec::squirrel`]'s own `field_types` covers no enum-member
/// DEFINES edge either, matching the baseline's own equally minimal
/// depth for this construct).
fn squirrel_class_quirk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) -> bool {
    let kind_out = match node.kind() {
        "class_declaration" => SymbolKind::Class,
        "enum_declaration" => SymbolKind::Enum,
        _ => return false,
    };
    let Some(name) = squirrel_def_name(node, src) else {
        return true;
    };
    let line = node.start_position().row + 1;
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: kind_out,
        line,
    });
    if node.kind() == "class_declaration" {
        if let Some(super_name) = squirrel_base_class_name(node, src) {
            out.inherits.push(InheritsRef {
                sub_name: name.clone(),
                super_name,
                line,
            });
        }
        squirrel_walk_class_members(node, src, &name, out);
    }
    true
}

/// A `function_declaration`/`anonymous_function`/`lambda_expression`'s
/// own naming + scoped body walk -- neither has a `name`/`body` field
/// this file's generic `name_field`/`body_field` mechanism could read
/// directly (see [`squirrel_def_name`]/[`squirrel_def_block`]'s own doc
/// comments). Mirrors [`walk`]'s own generic func/method branch in
/// shape (push the Function/Method symbol, a DEFINES edge if
/// `enclosing` is set, then recurse into the body with `fn_scope` set)
/// but reads the name/body positionally rather than via
/// [`Node::child_by_field_name`].
fn squirrel_func_quirk(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if !matches!(
        node.kind(),
        "function_declaration" | "anonymous_function" | "lambda_expression"
    ) {
        return false;
    }
    let Some(name) = squirrel_def_name(node, src) else {
        // Anonymous (`anonymous_function`/`lambda_expression` with no
        // enclosing name to bind to at this position) -- nothing to
        // push, but still recurse into the body generically so any
        // call inside it is still visited (module-scoped, no
        // `fn_scope`, matching every other language's identical
        // "unnamed function literal" fallback, e.g.
        // `LangSpec::gdscript`'s own `lambda` doc note).
        return false;
    };
    let line = node.start_position().row + 1;
    let is_method = enclosing.is_some();
    out.symbols.push(SymbolRef {
        name: name.clone(),
        kind: if is_method {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        },
        line,
    });
    if let Some(container) = enclosing {
        out.defines.push(DefinesRef {
            container_name: container.to_string(),
            member_name: name.clone(),
            line,
        });
    }
    let spec = LangSpec::squirrel();
    let quirks = squirrel_quirks();
    let ctx = Ctx {
        spec: &spec,
        src,
        quirks: &quirks,
        is_test_file: false,
    };
    if let Some(block) = squirrel_def_block(node) {
        walk_children(
            block,
            &ctx,
            out,
            enclosing,
            FnScope {
                name: Some(name.as_str()),
                line: Some(line),
            },
        );
    }
    true
}

/// `h.register()`/`base.speak()`'s own `function` field is a
/// `deref_expression` node whose OWN children (`identifier`, `.`,
/// `identifier`) carry NO field names at all (confirmed via the
/// parse-tree dump -- unlike every node kind [`receiver_of_call`]'s own
/// `match function_node.kind()` already handles, all of which read a
/// NAMED field for their receiver) -- the receiver is instead the
/// FIRST direct child (positional), read here directly rather than
/// through that shared helper, which has no way to express a
/// field-less receiver shape. `.utf8_text()` still already includes
/// the full written `"h.register"`/`"base.speak"` text for the whole
/// `deref_expression` (used as-is for [`CallRef::callee`]) -- only the
/// separate RECEIVER text/hint needs this dedicated read.
fn squirrel_receiver_of_call(
    function_node: Node<'_>,
    src: &[u8],
) -> (Option<String>, Option<ReceiverHint>) {
    if function_node.kind() != "deref_expression" {
        return (None, None);
    }
    let Some(receiver) = function_node.child(0) else {
        return (None, None);
    };
    let Ok(text) = receiver.utf8_text(src) else {
        return (None, None);
    };
    let hint = if text == "this" {
        ReceiverHint::SelfOrThis
    } else if receiver.kind() == "identifier" {
        ReceiverHint::Identifier
    } else {
        ReceiverHint::Other
    };
    (Some(text.to_string()), Some(hint))
}

/// `call_expression`'s own callee/receiver reconstruction (this
/// grammar's clean `function` field) plus its own positional (not
/// field-accessible) `call_args` argument list -- mirrors
/// [`zig_builtin_arg_texts`]'s identical "scan children for the
/// unnamed arguments-holder kind, then take ITS OWN children" shape.
/// Receiver text/hint via [`squirrel_receiver_of_call`], NOT the
/// shared [`receiver_of_call`] helper -- see that function's own doc
/// comment for why (Squirrel's `deref_expression` has no named field
/// for its receiver at all, unlike every grammar shape that helper
/// already covers).
fn squirrel_call_override(
    node: Node<'_>,
    from_symbol: Option<&str>,
    from_symbol_line: Option<usize>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let Ok(callee) = function.utf8_text(src) else {
        return false;
    };
    let (receiver_text, receiver_hint) = squirrel_receiver_of_call(function, src);
    let arg_texts = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|n| n.kind() == "call_args")
        .map(|args| {
            (0..args.child_count())
                .filter_map(|i| args.child(i))
                .filter(|c| !matches!(c.kind(), "(" | ")" | ","))
                .filter_map(|c| c.utf8_text(src).ok())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    out.calls.push(CallRef {
        callee: callee.to_string(),
        line: node.start_position().row + 1,
        from_symbol: from_symbol.map(str::to_string),
        from_symbol_line,
        receiver_text,
        receiver_hint,
        arg_texts,
    });
    true
}

/// [`Quirks::on_unmatched_node`] entry point: tries
/// [`squirrel_func_quirk`], then [`squirrel_class_quirk`] -- named
/// (not an inline closure) to match every other language row in this
/// file's own `Box::new(named_fn)` convention.
fn squirrel_quirk(
    node: Node<'_>,
    enclosing: Option<&str>,
    src: &[u8],
    out: &mut ParsedFile,
) -> bool {
    squirrel_func_quirk(node, enclosing, src, out) || squirrel_class_quirk(node, src, out)
}

/// Squirrel's [`Quirks`] row: `function_declaration`/
/// `anonymous_function`/`lambda_expression` positional name/body
/// resolution (`func_types`/`method_types`), `class_declaration`/
/// `enum_declaration` positional name resolution plus `extends`-keyword
/// INHERITS detection and member-peeking DEFINES scan
/// (`class_types`), and `call_expression`'s positional `call_args`
/// argument-list reconstruction (its own callee field is clean and
/// needs no override, but this file's flat `call_arguments_field`
/// mechanism cannot read an unnamed child). No `import_types` handling
/// at all -- [`LangSpec::squirrel`]'s own `import_types` array is
/// empty by design (see its own doc comment for why porting the
/// baseline's `{"extends"}` verbatim would be actively wrong here, not
/// merely unused).
pub fn squirrel_quirks() -> Quirks {
    Quirks {
        on_unmatched_node: Some(Box::new(squirrel_quirk)),
        is_test_name: |_| false,
        route_from_call: None,
        on_method_defined: None,
        call_override: Some(Box::new(squirrel_call_override)),
    }
}

/// Parse Squirrel source through the generic engine. Language-parity
/// wave G2.2e.
pub fn parse_squirrel(source: &str) -> ParsedFile {
    let spec = LangSpec::squirrel();
    let quirks = squirrel_quirks();
    let language: tree_sitter::Language = tree_sitter_squirrel_local::language().into();
    parse_with_spec(source, &language, &spec, &quirks, false)
}
