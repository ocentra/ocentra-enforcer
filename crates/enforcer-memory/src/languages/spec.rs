//! Table-driven language specification, mirroring the C baseline's
//! `CBMLangSpec` (`codebase-memory-mcp/internal/cbm/lang_specs.c`):
//! one [`LangSpec`] row per language, holding tree-sitter node-*kind*
//! NAME arrays for the constructs the generic walker
//! ([`crate::languages::generic`]) needs to recognize -- functions,
//! classes/product-types, fields, module clauses, call expressions,
//! import statements, and branch/decision nodes (the latter shared
//! 1:1 with [`crate::complexity::NodeKindTable`], since "what counts
//! as a branch" is the same question for cyclomatic complexity as it
//! is for the generic walker's own bookkeeping).
//!
//! Adding a language to the generic engine means adding one
//! `LangSpec` row here (copy the node-kind names from the existing
//! bespoke extractor in `languages/*.rs` and/or the baseline's own
//! `lang_specs.c` array for that language) -- never touching
//! [`crate::languages::generic`]'s walk logic itself. A handful of
//! languages need small syntactic quirks the generic arrays cannot
//! express (Go's embedded-struct-field INHERITS signal, TS's
//! `class extends`/`implements` heritage clauses, ...): those are
//! handled by [`crate::languages::generic::quirk`], the seam mirroring
//! the baseline's per-language `if (lang == CBM_LANG_X)` branches in
//! `extract_defs.c`/`extract_calls.c`.
//!
//! G1/G1b scope note: every one of our existing 10 languages is now
//! fully routed through the generic walker (`generic::parse_<lang>`,
//! wired into [`crate::parsers::parse_file`]) -- Go was this wave's
//! original zero-regression proof; Rust, TypeScript/JavaScript,
//! Python, Java, C, C++, C#, and PHP followed the same pattern
//! (`tests/unit_lang_spec_engine.rs` proves each generic-path function
//! reproduces its bespoke counterpart byte-for-byte-equivalent output
//! on every existing scenario). Several rows below needed correcting
//! against the actual bespoke source during that migration -- a
//! node-kind array entry with no corresponding bespoke `match` arm
//! (invisible data, silently never dispatched until G1b actually
//! wired it up) is a real class of bug this crate now has direct
//! evidence for, not just a hypothetical; see e.g. [`LangSpec::java`],
//! [`LangSpec::csharp`], and [`LangSpec::php`]'s own doc comments for
//! the specific corrections each needed. [`LangSpec::c`] and
//! [`LangSpec::cpp`] additionally could not use this file's `name_field`
//! mechanism at all (see their own doc comments) -- every one of their
//! node kinds is instead claimed in full by the corresponding
//! `generic::c_quirk`/`generic::cpp_quirk`.

/// One language's tree-sitter node-*kind* vocabulary, addressed by
/// name (not grammar-specific enum) so the generic walker never needs
/// per-language Rust code to recognize "this node is a function".
/// Every field is a flat `&'static [&'static str]` of `.kind()`
/// strings, matched by exact string equality -- same convention as
/// [`crate::complexity::NodeKindTable`].
#[derive(Debug, Clone, Copy)]
pub struct LangSpec {
    /// Human-readable language name, for diagnostics only.
    pub name: &'static str,
    /// Free function definition node kinds (not a method inside a
    /// class/impl/struct body -- those are [`Self::method_types`]).
    pub func_types: &'static [&'static str],
    /// Method definition node kinds (a function nested inside a
    /// class/impl/struct/interface body). May overlap with
    /// [`Self::func_types`] for languages whose grammar uses one node
    /// kind for both (the generic walker tells them apart by nesting
    /// context, same as every bespoke extractor already does).
    pub method_types: &'static [&'static str],
    /// Class/struct/product-type declaration node kinds.
    pub class_types: &'static [&'static str],
    /// Interface/trait/protocol declaration node kinds.
    pub interface_types: &'static [&'static str],
    /// Enum declaration node kinds.
    pub enum_types: &'static [&'static str],
    /// Type-alias declaration node kinds.
    pub alias_types: &'static [&'static str],
    /// Field/property declaration node kinds inside a class/struct
    /// body (used for DEFINES edges).
    pub field_types: &'static [&'static str],
    /// Module/namespace/package clause node kinds.
    pub module_types: &'static [&'static str],
    /// Call-expression node kinds.
    pub call_types: &'static [&'static str],
    /// The field name on a call-expression node holding its callee.
    pub call_function_field: &'static str,
    /// The field name on a call-expression node holding its argument
    /// list.
    pub call_arguments_field: &'static str,
    /// Import/use/include statement node kinds.
    pub import_types: &'static [&'static str],
    /// Decision-point node kinds (mirrors
    /// [`crate::complexity::NodeKindTable::decision_points`] --
    /// intentionally the same data, kept alongside the rest of this
    /// language's vocabulary rather than forcing every caller to
    /// juggle two separate per-language tables for what is one
    /// underlying concept).
    pub branch_types: &'static [&'static str],
    /// Decorator/attribute/annotation node kinds (`@decorator` in
    /// Python/TS, `#[attribute]` in Rust). Empty for languages with no
    /// such syntax (e.g. Go).
    pub decorator_types: &'static [&'static str],
    /// The field name on a function/method/class definition node
    /// holding its own name.
    pub name_field: &'static str,
    /// The field name on a function/method definition node holding its
    /// body block.
    pub body_field: &'static str,
}

impl LangSpec {
    /// Go: smallest arrays of the 10 (this wave's migration proof --
    /// see `generic::parse_go`). Node kinds read off
    /// `languages/go.rs`'s bespoke walk plus
    /// `crate::complexity::NodeKindTable::go`.
    pub const fn go() -> Self {
        Self {
            name: "go",
            func_types: &["function_declaration"],
            method_types: &["method_declaration"],
            class_types: &["type_spec"], // struct_type is the type_spec's `type` field shape; see quirk hook.
            interface_types: &[], // interface_type is a type_spec `type` field shape too; see quirk hook.
            enum_types: &[],
            alias_types: &[],
            field_types: &["field_declaration"],
            module_types: &["package_clause"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_declaration"],
            branch_types: &[
                "if_statement",
                "expression_case",
                "type_case",
                "for_statement",
                "&&",
                "||",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Rust: node kinds read off `languages/rust.rs`.
    pub const fn rust() -> Self {
        Self {
            name: "rust",
            func_types: &["function_item"],
            method_types: &["function_item"],
            class_types: &["struct_item"],
            interface_types: &["trait_item"],
            enum_types: &["enum_item"],
            alias_types: &["type_item"],
            // Empty (not `["field_declaration"]`): `languages/rust.rs`'s
            // bespoke `struct_item` arm never sets `enclosing` when
            // recursing into a struct body, so it never emits a
            // struct-field DEFINES edge either -- matched here for
            // zero-regression rather than "improving" on the bespoke
            // behavior (G3's job, if ever wanted).
            field_types: &[],
            module_types: &["mod_item"],
            call_types: &["call_expression", "method_call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["use_declaration"],
            branch_types: &[
                "if_expression",
                "if_let_expression",
                "match_arm",
                "while_expression",
                "while_let_expression",
                "loop_expression",
                "for_expression",
                "&&",
                "||",
            ],
            decorator_types: &["attribute_item"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// TypeScript/JavaScript: node kinds read off
    /// `languages/typescript.rs`.
    pub const fn typescript() -> Self {
        Self {
            name: "typescript",
            func_types: &["function_declaration"],
            method_types: &["method_definition"],
            class_types: &["class_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &["type_alias_declaration"],
            // Empty (not `["public_field_definition"]`):
            // `languages/typescript.rs`'s bespoke walker has no
            // `"public_field_definition"` match arm at all -- it falls
            // through to the unmatched-node case and is never turned
            // into a DEFINES edge, so this stays empty for
            // zero-regression rather than adding a field DEFINES edge
            // the bespoke extractor never emitted.
            field_types: &[],
            module_types: &["module", "internal_module"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_statement"],
            branch_types: &[
                "if_statement",
                "switch_case",
                "while_statement",
                "do_statement",
                "for_statement",
                "for_in_statement",
                "catch_clause",
                "&&",
                "||",
                "ternary_expression",
            ],
            decorator_types: &["decorator"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Python: node kinds read off `languages/python.rs`.
    pub const fn python() -> Self {
        Self {
            name: "python",
            func_types: &["function_definition"],
            method_types: &["function_definition"],
            class_types: &["class_definition"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_statement", "import_from_statement"],
            branch_types: &[
                "if_statement",
                "elif_clause",
                "while_statement",
                "for_statement",
                "except_clause",
                "boolean_operator",
                "conditional_expression",
            ],
            decorator_types: &["decorator"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Java: node kinds read off `languages/java.rs`.
    pub const fn java() -> Self {
        Self {
            name: "java",
            func_types: &[],
            // NOT `constructor_declaration`:
            // `languages/java.rs`'s bespoke walk has no
            // `"constructor_declaration"` match arm at all -- a
            // constructor is invisible to the bespoke extractor today,
            // so including it here would add symbols the bespoke path
            // never emitted (a regression, not an improvement; G3's
            // job if ever wanted).
            method_types: &["method_declaration"],
            class_types: &["class_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &[],
            // Empty (not `["field_declaration"]`): only
            // `static final` fields become a DEFINES-carrying
            // `SymbolKind::Constant` in the bespoke walk (see its
            // `field_declaration` arm's `is_constant` gate) -- an
            // ordinary field is invisible. Handled fully by
            // [`crate::languages::generic::java_quirk`] instead of this
            // flat array, which cannot express the modifier gate.
            field_types: &[],
            module_types: &["package_declaration"],
            call_types: &["method_invocation"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            import_types: &["import_declaration"],
            branch_types: &[
                "if_statement",
                "switch_label",
                "while_statement",
                "do_statement",
                "for_statement",
                "catch_clause",
                "&&",
                "||",
                "ternary_expression",
            ],
            decorator_types: &["annotation", "marker_annotation"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// C: node kinds read off `languages/c.rs`. Every one of
    /// `func_types`/`class_types`/`enum_types`/`alias_types`/
    /// `field_types` is fully claimed by
    /// [`crate::languages::generic::c_quirk`] rather than dispatched
    /// through the generic engine's own `name_field`-based fallback --
    /// C's declarator-unwrapping (`int *foo(...)`), per-node-kind field
    /// name (`struct_specifier`/`enum_specifier`/`preproc_def` use
    /// `"name"`; `function_definition` uses `"declarator"`, which still
    /// needs unwrapping through pointer/parenthesized-declarator layers
    /// down to the bare identifier), and multi-name `type_definition`
    /// alias extraction cannot be expressed by a single flat
    /// `name_field`/`body_field` pair the way every other language row
    /// in this file can. These arrays exist so `LangSpec::c()` still
    /// documents which node kinds are "C's functions/classes/etc." at a
    /// glance and so `spec.call_types`/`branch_types`/`import_types`
    /// (which the generic engine *does* use directly and correctly)
    /// stay declared the normal way.
    pub const fn c() -> Self {
        Self {
            name: "c",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &["struct_specifier"],
            interface_types: &[],
            enum_types: &["enum_specifier"],
            alias_types: &["type_definition"],
            field_types: &["field_declaration"],
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["preproc_include"],
            branch_types: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "&&",
                "||",
            ],
            decorator_types: &[],
            // Never actually consulted: `c_quirk` claims (returns
            // `true` for) every node kind in `func_types`/
            // `class_types`/`enum_types`/`alias_types`/`field_types`
            // above before the generic engine's own `name_field`-keyed
            // fallback would ever run, so this value's only role is
            // "not a real field name, so a future maintainer never
            // mistakes it for one" -- see this const's own doc comment.
            name_field: "UNUSED_SEE_C_QUIRK",
            body_field: "body",
        }
    }

    /// C++: node kinds read off `languages/cpp.rs`. Same "every array
    /// fully claimed by the quirk hook, `name_field` never actually
    /// consulted" posture as [`Self::c`] and for the identical reasons
    /// (declarator unwrapping, out-of-line `Class::method` scoping, the
    /// abstract-class-as-Interface heuristic, and gtest macro detection
    /// on two different node kinds all need full custom logic no flat
    /// array/field-name pair could express) -- see
    /// [`crate::languages::generic::cpp_quirk`].
    pub const fn cpp() -> Self {
        Self {
            name: "cpp",
            func_types: &["function_definition"],
            method_types: &["function_definition"],
            class_types: &["class_specifier", "struct_specifier"],
            interface_types: &[],
            enum_types: &["enum_specifier"],
            alias_types: &["type_definition", "alias_declaration"],
            field_types: &["field_declaration"],
            module_types: &["namespace_definition"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["preproc_include"],
            branch_types: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "for_range_loop",
                "catch_clause",
                "&&",
                "||",
                "conditional_expression",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment and `LangSpec::c()`'s identical `name_field`
            // doc.
            name_field: "UNUSED_SEE_CPP_QUIRK",
            body_field: "body",
        }
    }

    /// C#: node kinds read off `languages/csharp.rs`. Several arrays
    /// differ from what a first glance at the bespoke source might
    /// suggest -- verified directly against `languages/csharp.rs`'s own
    /// `match` arms (see [`crate::languages::generic::csharp_quirk`]
    /// for the corresponding claims):
    /// - `func_types` is empty, NOT `["local_function_statement"]`: the
    ///   bespoke `"local_function_statement"` arm only ever pushes a
    ///   [`crate::parsers::SymbolKind::Lambda`] symbol, never Function/
    ///   Method -- there is no free-function node kind in C# at all.
    /// - `method_types` has no `"constructor_declaration"`: the
    ///   bespoke walk has no such arm (a constructor is invisible to
    ///   it today, same finding as Java's `LangSpec::java()`).
    /// - `field_types` is empty, NOT
    ///   `["field_declaration", "property_declaration"]`: the bespoke
    ///   `"field_declaration"` arm is match-guarded on
    ///   `is_const_or_static_readonly(...)` (an ordinary field is
    ///   invisible), and there is no `"property_declaration"` arm at
    ///   all.
    pub const fn csharp() -> Self {
        Self {
            name: "csharp",
            func_types: &[],
            method_types: &["method_declaration"],
            class_types: &["class_declaration", "struct_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &[],
            field_types: &[],
            module_types: &["namespace_declaration", "file_scoped_namespace_declaration"],
            call_types: &["invocation_expression"],
            call_function_field: "function",
            // `"arguments"`, NOT `"argument_list"`: verified against
            // `languages/csharp.rs`'s own `call_arg_texts`/
            // `route_from_map_call`, both of which read
            // `call_node.child_by_field_name("arguments")`.
            call_arguments_field: "arguments",
            import_types: &["using_directive"],
            branch_types: &[
                "if_statement",
                "switch_section",
                "switch_expression_arm",
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
                "catch_clause",
                "&&",
                "||",
                "conditional_expression",
            ],
            decorator_types: &["attribute"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// PHP: node kinds read off `languages/php.rs`. Several arrays
    /// differ from a first-glance guess -- verified directly against
    /// the bespoke source's own `match` arms (see
    /// [`crate::languages::generic::php_quirk`]/
    /// [`crate::languages::generic::php_call_override`] for the
    /// corresponding claims):
    /// - `class_types` includes `"trait_declaration"` (missing
    ///   before): the bespoke walk DOES have a `"trait_declaration"`
    ///   arm, tagging it [`crate::parsers::SymbolKind::Class`] same as
    ///   an ordinary class.
    /// - `enum_types` is empty, NOT `["enum_declaration"]`: the
    ///   bespoke walk has no `"enum_declaration"` arm at all (PHP
    ///   enums are invisible to it today).
    /// - `field_types` is empty, NOT `["property_declaration"]`: same
    ///   finding -- no `"property_declaration"` arm exists.
    /// - `call_types` has all *four* call-shaped node kinds the
    ///   bespoke walk actually handles (not just
    ///   `function_call_expression`/`member_call_expression` --
    ///   `nullsafe_member_call_expression`/`scoped_call_expression`
    ///   too), each with its own field-name shape the generic engine's
    ///   single `call_function_field` cannot express uniformly
    ///   (`function_call_expression`'s callee lives in a `"function"`
    ///   field; `member_call_expression`'s lives in a `"name"` field
    ///   with a *separate* `"object"` receiver field;
    ///   `scoped_call_expression`'s lives in a `"name"` field with a
    ///   separate `"scope"` field) -- every one is instead claimed in
    ///   full by [`crate::languages::generic::php_call_override`],
    ///   which (unlike `on_unmatched_node`) receives the caller's
    ///   `fn_scope` (`from_symbol`/`from_symbol_line`) directly, so
    ///   PHP's CALLS edges still get the correct enclosing-function
    ///   scope the bespoke walk threads through -- `call_function_field`
    ///   below is consequently never actually read (every one of
    ///   `call_types` is claimed before it would matter).
    pub const fn php() -> Self {
        Self {
            name: "php",
            func_types: &["function_definition"],
            method_types: &["method_declaration"],
            class_types: &["class_declaration", "trait_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["namespace_definition"],
            call_types: &[
                "function_call_expression",
                "member_call_expression",
                "nullsafe_member_call_expression",
                "scoped_call_expression",
            ],
            call_function_field: "UNUSED_SEE_PHP_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            import_types: &["namespace_use_declaration"],
            branch_types: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
                "catch_clause",
                "&&",
                "||",
                "conditional_expression",
            ],
            decorator_types: &["attribute_list"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Kotlin: node kinds copied from the baseline's
    /// `kotlin_func_types`/`kotlin_class_types`/`kotlin_module_types`/
    /// `kotlin_call_types`/`kotlin_import_types`/`kotlin_branch_types`/
    /// `kotlin_decorator_types` (`codebase-memory-mcp/internal/cbm/lang_specs.c`
    /// :437-450), verified directly against the `tree-sitter-grammars/
    /// tree-sitter-kotlin` grammar's own `src/node-types.json` (the
    /// upstream the `tree-sitter-kotlin-ng` crate packages, confirmed
    /// via its crates.io `repository` metadata field, and the same
    /// fwcd-authored lineage the baseline's own vendored
    /// `internal/cbm/vendored/grammars/kotlin/` copy carries per its
    /// `LICENSE` copyright line) -- G2.1a's grammar-shape ground truth,
    /// not guessed:
    /// - `alias_types` is `["type_alias"]` (baseline's `class_types`
    ///   array folds `type_alias` in alongside `class_declaration`/
    ///   `object_declaration`/`companion_object`; kept in its own
    ///   dedicated array here instead, matching every other language
    ///   row's `alias_types` convention, since `type_alias`'s own name
    ///   field is literally called `"type"` -- see [`generic::kotlin_quirk`]).
    /// - `call_types`/`call_function_field`/`call_arguments_field`,
    ///   `func_types`/`method_types`'s `body_field`, and `alias_types`'
    ///   `name_field` are ALL never actually consulted by the generic
    ///   engine's own field-name-keyed fallback paths -- same posture as
    ///   [`Self::c`]/[`Self::cpp`] and for an analogous reason:
    ///   `call_expression`/`navigation_expression` have NO named fields
    ///   at all in this grammar (the callee is an unfielded child, not a
    ///   `"function"` field), and `function_declaration`/
    ///   `class_declaration`/`object_declaration`/`companion_object`'s
    ///   own bodies (`function_body`/`class_body`) are likewise
    ///   unfielded children, not a `"body"` field --
    ///   [`crate::languages::generic::kotlin_quirk`] fully claims every
    ///   one of `func_types`/`method_types`/`class_types`/`alias_types`/
    ///   `call_types` before either fallback path would ever run.
    /// - `secondary_constructor` (a `method_types` entry, copied
    ///   verbatim from the baseline's `kotlin_func_types`) has no name
    ///   at all in this grammar (Kotlin constructors are unnamed) --
    ///   [`generic::kotlin_quirk`] records it as a nameless
    ///   [`crate::parsers::SymbolKind::Method`] the same way
    ///   [`generic::c_quirk`]'s anonymous-struct case has no name to
    ///   push either.
    pub const fn kotlin() -> Self {
        Self {
            name: "kotlin",
            func_types: &["function_declaration", "anonymous_function"],
            method_types: &["function_declaration", "secondary_constructor"],
            class_types: &[
                "class_declaration",
                "object_declaration",
                "companion_object",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &["type_alias"],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression", "navigation_expression"],
            call_function_field: "UNUSED_SEE_KOTLIN_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_KOTLIN_CALL_OVERRIDE",
            import_types: &["import"],
            branch_types: &[
                "if_expression",
                "for_statement",
                "while_statement",
                "when_expression",
                "when_entry",
                "try_expression",
                "catch_block",
            ],
            decorator_types: &["annotation"],
            name_field: "UNUSED_SEE_KOTLIN_QUIRK",
            body_field: "UNUSED_SEE_KOTLIN_QUIRK",
        }
    }

    /// Swift: node kinds copied from the baseline's
    /// `swift_func_types`/`swift_class_types`/`swift_field_types`/
    /// `swift_call_types`/`swift_import_types`/`swift_branch_types`/
    /// `swift_decorator_types` (`codebase-memory-mcp/internal/cbm/lang_specs.c`
    /// :556-569), verified directly against the actual
    /// `alex-pinkus/tree-sitter-swift` grammar's own generated
    /// `src/node-types.json` (fetched from the grammar's real GitHub
    /// repository via the GitHub contents API, not guessed) -- the same
    /// grammar lineage the baseline's own vendored
    /// `internal/cbm/vendored/grammars/swift/` copy carries per its
    /// `LICENSE` copyright line ("Copyright (c) 2021 alex-pinkus"):
    /// - `class_types` is `["class_declaration", "protocol_declaration"]`,
    ///   NOT the baseline's full `{"class_declaration",
    ///   "protocol_declaration", "struct_declaration", "enum_declaration"}`
    ///   -- this grammar has no `struct_declaration`/`enum_declaration`
    ///   node kind at all; `struct`/`enum`/`actor`/`extension` all
    ///   parse as the SAME `class_declaration` node, distinguished only
    ///   by its own `declaration_kind` field's text (`"class"`/
    ///   `"struct"`/`"enum"`/`"actor"`/`"extension"`) -- the baseline's
    ///   two extra array entries never match any real node this
    ///   grammar emits (dead data, same "invisible, never dispatched"
    ///   class of finding [`Self::java`]/[`Self::csharp`]/[`Self::php`]'s
    ///   own doc comments already record for other languages).
    ///   [`generic::swift_quirk`] reads `declaration_kind` directly to
    ///   recover `SymbolKind::Struct`/`SymbolKind::Enum` distinctly from
    ///   plain `SymbolKind::Class`.
    /// - No `throw_types`/`swift_throw_types` field exists on
    ///   [`LangSpec`] at all (only Rust/C++'s `Quirks` layer needs a
    ///   throw signal, wired ad hoc, not a flat array) -- noted here
    ///   only because the baseline's own `swift_throw_types =
    ///   {"throw_statement", NULL}` is METHODOLOGICALLY dead too: this
    ///   grammar has no `throw_statement` node kind at all (`throw` is
    ///   only ever `throw_keyword`/`throws`/`throws_clause`, a
    ///   function-signature effect marker, never a standalone
    ///   statement node) -- had our own `LangSpec` carried a
    ///   `throw_types` field, copying that baseline entry verbatim
    ///   would have been a second dead array, so this is flagged for
    ///   whichever future wave adds one.
    /// - `call_function_field`/`call_arguments_field`/`field_types`'
    ///   `name_field` are never actually consulted, same posture as
    ///   [`Self::c`]/[`Self::kotlin`]: `call_expression` has no fields
    ///   at all (callee is an unfielded child); `constructor_expression`
    ///   uses a `constructed_type` field instead of `function`;
    ///   `navigation_expression` uses `target`/`suffix` fields instead
    ///   of a single callee field; `macro_invocation` has its own shape
    ///   too -- [`generic::swift_quirk`]'s `call_override` hook fully
    ///   claims every one of `call_types` directly. Confirmed with a
    ///   standalone debug harness that `constructor_expression`'s real
    ///   trigger is narrower than "any `Type(...)` construction idiom"
    ///   -- a bare `Widget()` (no explicit type arguments) parses as an
    ///   ORDINARY `call_expression`, syntactically indistinguishable
    ///   from calling a function literally named `Widget` (Swift has no
    ///   dedicated `new`-keyword-equivalent for its common construction
    ///   idiom, so [`generic::swift_call_override`]'s `call_expression`
    ///   arm correctly records it with no [`crate::parsers::ReceiverHint::NewExpression`]
    ///   hint at all -- there is no syntactic signal here to hang one
    ///   on). `constructor_expression` only fires for a generic type
    ///   constructed with explicit type arguments (`Array<Int>()`),
    ///   which DOES get the `NewExpression` hint, via its own dedicated
    ///   match arm.
    ///   `property_declaration` (this row's `field_types`) has no
    ///   `name` field either -- its `name` field's own declared type is
    ///   `pattern` (a simple binding pattern, not a bare identifier
    ///   field the generic engine's `child_text` helper could read
    ///   directly), so [`generic::swift_quirk`] claims it too.
    pub const fn swift() -> Self {
        Self {
            name: "swift",
            func_types: &["function_declaration"],
            method_types: &["function_declaration"],
            class_types: &["class_declaration", "protocol_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["property_declaration"],
            module_types: &[],
            call_types: &[
                "call_expression",
                "constructor_expression",
                "macro_invocation",
                "navigation_expression",
            ],
            call_function_field: "UNUSED_SEE_SWIFT_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_SWIFT_CALL_OVERRIDE",
            import_types: &["import_declaration", "import"],
            branch_types: &[
                "if_statement",
                "guard_statement",
                "for_statement",
                "while_statement",
                "switch_statement",
            ],
            decorator_types: &["attribute"],
            name_field: "name",
            // NOT `"UNUSED_SEE_SWIFT_QUIRK"`: unlike `class_declaration`
            // (whose body lookup genuinely needs
            // [`generic::swift_class_like`]'s `declaration_kind`-aware
            // `class_body`/`enum_class_body` choice), `function_declaration`
            // has a real `"body"` field pointing at its `function_body`
            // child (confirmed against the grammar's actual parse tree
            // with a standalone debug harness, not just
            // `node-types.json`) -- the generic engine's own func/method
            // branch (`walk`'s `child_by_field_name(spec.body_field)`
            // call) needs this to be the real field name, or every
            // Swift function body is silently orphaned from the walk
            // (calls/nested symbols inside it never visited at all) --
            // found via an empty-`calls` test failure during this
            // wave's own verification, not guessed up front.
            body_field: "body",
        }
    }

    /// TSX (TypeScript-JSX): the baseline treats TSX as a DISTINCT
    /// `CBMLangSpec` row from plain TypeScript
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c` :1631-1636,
    /// `CBM_LANG_TSX`) even though every node-kind array it carries is
    /// copied VERBATIM from TypeScript's own row (`ts_func_types`/
    /// `ts_class_types`/`js_module_types`/`js_call_types`/
    /// `js_import_types`/`js_branch_types`/`ts_decorator_types`) --
    /// the only difference between the two baseline rows is which
    /// `ts_factory` function pointer they carry
    /// (`tree_sitter_typescript` vs `tree_sitter_tsx`). This row mirrors
    /// that exactly: every field below is identical to
    /// [`Self::typescript`]'s, because TSX's grammar is TypeScript's
    /// grammar plus JSX productions layered on top -- every construct
    /// this crate's TS extractor already recognizes
    /// (function/class/interface/enum/alias/call/import/branch/
    /// decorator) keeps the exact same node-kind name in the TSX
    /// grammar (verified: `tree-sitter-typescript` -- already this
    /// crate's TypeScript grammar dependency -- exports BOTH
    /// `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX` from the one crate, so
    /// no new grammar dependency is needed at all; see
    /// [`generic::parse_tsx`]). A distinct `LangSpec`/`Language::Tsx`
    /// row (rather than folding `.tsx` into `Language::TypeScript`, our
    /// prior behavior) exists purely so a caller can distinguish "this
    /// file is TSX" from "this file is plain TypeScript" the same way
    /// the baseline's own `CBM_LANG_TSX`/`CBM_LANG_TYPESCRIPT` split
    /// does, in case a future JSX-specific quirk (JSX element
    /// attributes as TYPE_REF-like signals, JSX-embedded component
    /// call detection, ...) ever needs to diverge from plain TS without
    /// touching TS's own row.
    pub const fn tsx() -> Self {
        Self {
            name: "tsx",
            ..Self::typescript()
        }
    }

    /// Solidity: node kinds copied from the baseline's
    /// `solidity_func_types`/`solidity_class_types`/
    /// `solidity_field_types`/`solidity_call_types`/
    /// `solidity_import_types`/`solidity_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:990-1012`),
    /// cross-checked node-kind-by-node-kind against the actual
    /// `node-types.json` shipped in the `tree-sitter-solidity` crate this
    /// row binds (crates.io `tree-sitter-solidity = "1.2"`, ABI 15) --
    /// every kind below is confirmed present with the field names this
    /// row (or [`crate::languages::generic::solidity_quirk`]) reads, and
    /// several were additionally verified against a real parse tree
    /// (`cargo run` against a scratch crate depending on
    /// `tree-sitter-solidity` directly, dumping every node kind/text)
    /// after a first-pass reading of `node-types.json` alone produced
    /// wrong assumptions for three of them -- see the specific
    /// corrections below.
    /// - `module_types` is empty, NOT `["source_file"]` (the baseline's
    ///   own value for this language): the C baseline's
    ///   `module_node_types` struct field
    ///   (`codebase-memory-mcp/internal/cbm/lang_specs.h:25`) has zero
    ///   consumers anywhere in that codebase (`grep -r
    ///   module_node_types` outside its own declaration/table-literal
    ///   sites returns nothing) -- it is dead data there. Our own
    ///   [`crate::languages::generic::walk`], by contrast, *does*
    ///   actively consume `module_types` (emits a
    ///   [`crate::parsers::SymbolKind::Module`] symbol named after the
    ///   node's first named child) -- porting `"source_file"` verbatim
    ///   would not reproduce a baseline behavior (there is none to
    ///   reproduce) but would *invent* a new, wrong one: `source_file`
    ///   is the whole file's root node, so its first named child is
    ///   arbitrarily whatever the first top-level pragma/contract/import
    ///   happens to be, not a module name.
    /// - `alias_types` is `["user_defined_type_definition"]`, NOT
    ///   `["type_alias"]`: a first reading of `node-types.json` alone
    ///   found a node kind literally named `type_alias` and assumed it
    ///   was Solidity's `type Foo is uint256;` declaration -- a real
    ///   parse tree dump proved this wrong on both ends. `type Foo is
    ///   uint256;` actually parses as `user_defined_type_definition`
    ///   (which DOES have a `name` field, so it uses this row's ordinary
    ///   `name_field` path with no quirk needed at all); the node kind
    ///   actually named `type_alias` is something else entirely -- the
    ///   bare-child library-name clause inside a `using X for Y;`
    ///   directive's children (see `solidity_quirk`'s `using_directive`
    ///   handling in `generic.rs`, which reads it for IMPORTS, not
    ///   DEFINES/TypeAlias).
    /// - `call_types` is `["call_expression"]` only, NOT
    ///   `["call_expression", "call", "new_expression"]`: the baseline's
    ///   own `solidity_call_types` array lists a bare `"call"` kind this
    ///   grammar version's `node-types.json` never generates (a leftover
    ///   from a different/older Solidity grammar the baseline vendored),
    ///   and `new_expression` is never itself a top-level call-shaped
    ///   node -- a real parse tree shows `new Helper(a)` is a
    ///   `call_expression` whose `function` field's `expression` wrapper
    ///   contains a nested `new_expression` (text `"new Helper"`), so
    ///   `call_expression` alone already reaches it via the ordinary
    ///   flat-field path (with the `new`-keyword prefix as part of the
    ///   captured callee text, matching how Go's `NewXxx(...)`
    ///   constructor-idiom convention is captured as literal callee text
    ///   too rather than stripped).
    pub const fn solidity() -> Self {
        Self {
            name: "solidity",
            func_types: &[
                "function_definition",
                "constructor_definition",
                "modifier_definition",
                "fallback_receive_definition",
            ],
            method_types: &[
                "function_definition",
                "constructor_definition",
                "modifier_definition",
                "fallback_receive_definition",
            ],
            class_types: &[
                "contract_declaration",
                "interface_declaration",
                "library_declaration",
                "struct_declaration",
            ],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            // See this const's own doc comment: `user_defined_type_definition`
            // has a real `name` field, not `"type_alias"`.
            alias_types: &["user_defined_type_definition"],
            field_types: &["state_variable_declaration", "struct_member"],
            // See this const's own doc comment: intentionally empty,
            // NOT `["source_file"]`.
            module_types: &[],
            // See this const's own doc comment: `call_expression` alone,
            // not `["call_expression", "call", "new_expression"]`.
            call_types: &["call_expression"],
            // A `call_expression`'s `function` field is always wrapped
            // in one `expression` node (verified via a real parse tree
            // dump) rather than pointing directly at the
            // identifier/member_expression/new_expression underneath --
            // `.utf8_text()` on the wrapper still yields the exact same
            // written callee text (`"helper"`, `"h.register"`,
            // `"new Helper"`) since text extraction reads the wrapper's
            // own byte range, which spans its single child exactly. Only
            // `crate::languages::generic::receiver_of_call`'s
            // `function_node.kind()` match cares about the *unwrapped*
            // kind (to recognize `member_expression` and set a
            // receiver hint) -- Solidity supplies its own
            // `solidity_call_override` quirk rather than relying on that
            // shared generic helper, precisely to unwrap this one extra
            // `expression` layer first.
            call_function_field: "function",
            // `call_expression`'s argument list is exposed as bare
            // `call_argument` children in `node-types.json`, not a named
            // field -- claimed in full by
            // `crate::languages::generic::solidity_call_override`
            // instead (this value is consequently never actually read).
            call_arguments_field: "UNUSED_SEE_SOLIDITY_CALL_OVERRIDE",
            import_types: &["import_directive", "using_directive"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "do_while_statement",
                "try_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// GDScript: node kinds copied from the baseline's
    /// `gdscript_func_types`/`gdscript_class_types`/
    /// `gdscript_field_types`/`gdscript_call_types`/
    /// `gdscript_import_types`/`gdscript_branch_types`/
    /// `gdscript_decorator_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1020-1034`),
    /// cross-checked node-kind-by-node-kind against the actual
    /// `node-types.json` shipped in the `tree-sitter-gdscript` crate this
    /// row binds (crates.io `tree-sitter-gdscript = "6"`, ABI 14), plus a
    /// real parse tree dump (`cargo run` against a scratch crate
    /// depending on `tree-sitter-gdscript` directly) that caught two
    /// wrong assumptions `node-types.json` alone did not surface -- see
    /// the specific corrections below.
    /// - `module_types` is empty, NOT `["source_file"]` -- identical
    ///   rationale to [`Self::solidity`]'s own doc comment (the
    ///   baseline's `module_node_types` field has zero consumers there;
    ///   our own generic walker's `module_types` field is live, so
    ///   porting `"source_file"` would invent behavior rather than
    ///   reproduce it).
    /// - `lambda` is deliberately absent from `class_types` even though
    ///   the baseline's `gdscript_func_types` includes it:
    ///   `node-types.json` shows `lambda`'s `name` field is not required
    ///   (anonymous lambdas are the common case), which the generic
    ///   engine's func/method branch already tolerates gracefully
    ///   (silently produces no symbol when the name field is absent) --
    ///   no quirk needed for that case.
    /// - `class_types` includes `"class_name_statement"` (`class_name
    ///   Widget` -- the top-level, no-body statement that names the
    ///   whole file's implicit class), with its Class symbol emitted by
    ///   [`crate::languages::generic::gdscript_quirk`] rather than the
    ///   ordinary flat-array class-shape fallback: `class_name_statement`
    ///   is also one of [`Self::gdscript`]'s `import_types` below (for
    ///   its own `extends` field's base-path text), and
    ///   [`crate::languages::generic::walk`] checks `import_types`
    ///   before `class_types` with an early `return` on a match -- a
    ///   node kind cannot be driven by both arrays' own generic branches
    ///   at once, so the quirk hook (which `walk`'s `import_types`
    ///   branch delegates to for every import node) does both jobs for
    ///   this one node kind. `class_definition` (the OTHER, body-bearing
    ///   `class Foo: ...` nested-inner-class syntax) is unaffected and
    ///   still uses the ordinary flat-array path.
    /// - `call_types` is `["call", "attribute_call", "base_call"]`, each
    ///   fully claimed by
    ///   [`crate::languages::generic::gdscript_call_override`] since
    ///   none exposes its callee through a named field -- verified by
    ///   the same real parse tree dump: `x.foo()` is one `attribute`
    ///   node whose children are the receiver path segment(s) followed
    ///   by an `attribute_call` node (the call itself has no receiver
    ///   field of its own); a bare `.foo()` (no receiver at all, the
    ///   "call the base implementation" idiom) is `base_call`; anything
    ///   else callable is plain `call`. `super.draw()` -- initially
    ///   assumed to be `base_call`'s trigger syntax -- is NOT special:
    ///   it parses as an ordinary `attribute`/`attribute_call` pair with
    ///   `super` as a perfectly ordinary identifier receiver, which
    ///   `gdscript_call_override`'s receiver-hint logic already handles
    ///   correctly with no dedicated `super`-text special case needed.
    pub const fn gdscript() -> Self {
        Self {
            name: "gdscript",
            func_types: &["function_definition", "constructor_definition", "lambda"],
            method_types: &["function_definition", "constructor_definition", "lambda"],
            // See this const's own doc comment: `class_name_statement`'s
            // Class symbol is emitted by `gdscript_quirk`, not this
            // array's own generic class-shape fallback (unreachable for
            // this one node kind since `import_types` wins first).
            class_types: &["class_definition", "class_name_statement"],
            interface_types: &[],
            enum_types: &["enum_definition"],
            alias_types: &[],
            field_types: &[
                "variable_statement",
                "export_variable_statement",
                "onready_variable_statement",
                "signal_statement",
            ],
            // See this const's own doc comment: intentionally empty,
            // NOT `["source_file"]`.
            module_types: &[],
            call_types: &["call", "attribute_call", "base_call"],
            call_function_field: "UNUSED_SEE_GDSCRIPT_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            // `extends_statement`/`class_name_statement` are the
            // baseline's own choice of "import-shaped" node kinds for
            // GDScript (there is no true import/module-path statement in
            // the language -- `extends "res://base.gd"`/`extends Base`
            // is the closest analog, and the baseline's own
            // `gdscript_import_types` array treats it as such rather
            // than as INHERITS) -- matched here for the same "port the
            // baseline's actual classification, not an idealized one"
            // reason [`Self::solidity`]'s doc comment gives. Bare
            // `"extends"` (a keyword token, not a named rule) from the
            // baseline's array is dropped: it cannot appear as a
            // `.kind()` on any *named* node this grammar produces (only
            // unnamed keyword tokens use bare keyword-text kinds, which
            // the generic walker's node-kind arrays are never matched
            // against in practice for any existing language row either).
            // `class_name_statement` is ALSO in `class_types` above --
            // see this const's own doc comment for how `gdscript_quirk`
            // does both jobs for that one node kind.
            import_types: &["extends_statement", "class_name_statement"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "match_statement",
            ],
            decorator_types: &["annotation"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Dart: node-kind names verified directly against the
    /// `tree-sitter-dart` 0.2.0 crate's own `src/node-types.json` (G2.1b
    /// grammar onboarding) -- NOT copied blindly from the baseline's
    /// `internal/cbm/lang_specs.c` `dart_*` arrays, several of which are
    /// stale against this grammar version:
    /// - `func_types`/`method_types` are `function_declaration`/
    ///   `method_declaration` -- NOT baseline's `function_signature`/
    ///   `method_signature` (the node kinds baseline's own
    ///   `resolve_dart_method_name` targets). A real parse tree proved
    ///   baseline's choice actively wrong for this crate's generic
    ///   engine specifically (not just imprecise): `function_signature`/
    ///   `method_signature` carry the `name`/`parameters` fields, but
    ///   their `body` field lives one level UP on the *wrapping*
    ///   `function_declaration`/`method_declaration` node as a sibling
    ///   field, not on the name-bearing node itself -- this file's
    ///   generic `name_field`/`body_field` mechanism assumes both live on
    ///   the SAME node
    ///   ([`crate::languages::generic::walk`]'s func/method branch reads
    ///   `spec.body_field` off the very node it just found `spec.name_field`
    ///   on), so pointing `func_types`/`method_types` at the inner
    ///   signature nodes silently drops every call inside the function
    ///   body from its correct `from_symbol` scope (the body is instead
    ///   walked as a separate, module-scoped sibling with `fn_scope`
    ///   reset to `None`) without ever failing loudly -- caught by
    ///   `tests/unit_languages_dart.rs`'s own
    ///   `call_inside_function_records_from_symbol_scope` test, not by
    ///   inspection. `function_declaration`/`method_declaration` are
    ///   instead fully claimed by
    ///   [`crate::languages::generic::dart_quirk`] (same "every array
    ///   fully claimed by quirk, generic branch never actually reached"
    ///   posture as [`Self::c`]/[`Self::cpp`]/[`Self::php`]), which reads
    ///   the nested signature node's own `name` field directly and walks
    ///   the OUTER node's own `body` field with the correct `fn_scope`
    ///   set -- see [`crate::languages::generic::dart_walk_function_body`].
    ///   `lambda_expression` (in baseline's `class_types`, oddly) does
    ///   not exist in this grammar at all (function literals are a plain
    ///   `function_expression`, never a def-shaped node), so it is
    ///   omitted from every array here.
    /// - `class_types` is `"class_declaration"`, NOT baseline's
    ///   `"class_definition"` (does not exist in this grammar).
    /// - `module_types` is empty, NOT baseline's `"program"` (this
    ///   grammar's root node is actually `source_file`, not `program`,
    ///   AND Dart's optional `library` directive is rare/deprecated in
    ///   modern Dart with no dedicated module-name symbol worth
    ///   extracting -- matches [`Self::python`]'s identical empty
    ///   `module_types` choice for the same "no natural module-name AST
    ///   node" reason).
    /// - `call_types` is `"call_expression"`, NOT baseline's `"selector"`
    ///   (does not exist in this grammar -- this grammar models a call
    ///   uniformly as `call_expression` with `function`/`arguments`
    ///   fields, the same shape the generic engine already expects
    ///   natively, no quirk needed for the base case).
    /// - `field_types` is empty (not baseline's `"declaration"`): a
    ///   `declaration` node's shape is a multi-kind wrapper (constructor/
    ///   getter/setter/field forms all share it) that cannot be
    ///   expressed by this file's flat `name_field` mechanism -- left
    ///   for a future G3 pass rather than guessed at, same "left empty
    ///   rather than wrong" posture as [`Self::rust`]'s `field_types`.
    ///   `call_function_field`/`call_arguments_field`/`name_field`/
    ///   `body_field` below are consequently never actually consulted for
    ///   `func_types`/`method_types` (only for `call_types`, which is not
    ///   quirk-claimed) -- same posture as [`Self::c`]'s identical
    ///   "arrays fully claimed elsewhere" doc note, but confined to the
    ///   func/method arrays only here (`class_types`/`enum_types`/
    ///   `alias_types` DO still use the ordinary flat `name_field` path for
    ///   Dart, unlike C's fully-claimed everything).
    pub const fn dart() -> Self {
        Self {
            name: "dart",
            func_types: &["function_declaration"],
            method_types: &["method_declaration"],
            class_types: &["class_declaration"],
            interface_types: &[],
            enum_types: &["enum_declaration"],
            alias_types: &["type_alias"],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_or_export"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "switch_statement",
                "&&",
                "||",
            ],
            decorator_types: &["annotation"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Scala: node-kind names verified directly against the
    /// `tree-sitter-scala` 0.26.0 crate's own `src/node-types.json` (G2.1b
    /// grammar onboarding) -- every one of the baseline's `scala_*`
    /// arrays in `internal/cbm/lang_specs.c` matched this grammar version
    /// exactly (unlike Dart/Groovy below, no stale entries found), so
    /// this row is a straight transcription. `trait_definition` is
    /// [`Self::interface_types`] rather than folded into
    /// [`Self::class_types`] the way the baseline's flatter
    /// `scala_class_types` array does (that C array has no
    /// interface/trait distinction to begin with) -- this crate's own
    /// [`LangSpec`] has a real `interface_types` field the baseline's
    /// `CBMLangSpec` does not, and a trait is Scala's actual
    /// interface-equivalent construct (same rationale as
    /// [`Self::rust`]'s `trait_item` -> `interface_types` and
    /// [`Self::typescript`]'s `interface_declaration` ->
    /// `interface_types`), so it is classified there instead;
    /// `enum_definition`/`type_definition` are
    /// [`Self::enum_types`]/[`Self::alias_types`] for the identical
    /// reason. `object_definition` (a singleton/companion-object
    /// declaration) has no closer match than [`Self::class_types`] (it
    /// is still a product-type-shaped declaration with a body), so it
    /// stays there. Every one of these five node kinds carries its own
    /// direct `name` field in this grammar (verified in
    /// `src/node-types.json`), so no quirk is needed for the base
    /// class-shape case at all -- see
    /// [`crate::languages::generic::scala_quirk`] for the heritage
    /// (`extends ... with ...`) handling this file's arrays cannot
    /// express.
    pub const fn scala() -> Self {
        Self {
            name: "scala",
            func_types: &["function_definition", "function_declaration"],
            method_types: &["function_definition", "function_declaration"],
            class_types: &["class_definition", "object_definition"],
            interface_types: &["trait_definition"],
            enum_types: &["enum_definition"],
            alias_types: &["type_definition"],
            field_types: &[],
            module_types: &["package_clause"],
            call_types: &[
                "call_expression",
                "generic_function",
                "field_expression",
                "infix_expression",
                "instance_expression",
            ],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_declaration"],
            branch_types: &[
                "if_expression",
                "for_expression",
                "while_expression",
                "match_expression",
                "case_clause",
                "try_expression",
                "catch_clause",
            ],
            decorator_types: &["annotation"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Groovy: node-kind names verified directly against the
    /// `tree-sitter-groovy` 0.1.2 crate's own `src/node-types.json`
    /// (G2.1b grammar onboarding) -- this grammar is considerably more
    /// Java-shaped than the baseline's own `groovy_*` arrays in
    /// `internal/cbm/lang_specs.c` assume, and several of those arrays
    /// are stale against it:
    /// - `func_types` keeps baseline's `"function_definition"` (a
    ///   top-level `def name(...) { ... }` script/def-shaped
    ///   declaration, confirmed unchanged in this grammar), but drops
    ///   baseline's second entry `"function_declaration"` -- no such
    ///   node kind exists in this grammar at all.
    /// - `method_types` is `"method_declaration"`, which baseline's
    ///   array does not list at all: this grammar gives a class body's
    ///   methods their own distinct, Java-shaped node kind (typed return,
    ///   no `def` keyword, direct `name`/`parameters`/`body` fields) that
    ///   is NOT one of `function_definition`'s occurrences anywhere in a
    ///   real parse tree -- omitting it (i.e. mirroring baseline's flat
    ///   array literally) would make every class method invisible to
    ///   this engine, not just imprecisely classified, so this is added
    ///   despite the baseline never having it.
    /// - `class_types` is `"class_declaration"`, NOT baseline's
    ///   `"class_definition"` (does not exist in this grammar).
    /// - `module_types` is `"program"` (this grammar's actual root node
    ///   kind, confirmed present unlike Dart's differently-named root)
    ///   -- matches baseline exactly here.
    /// - `call_types` is `"method_invocation"` + baseline's
    ///   `"juxt_function_call"` (the parenthesis-less call idiom,
    ///   confirmed unchanged), NOT baseline's `"function_call"` (does
    ///   not exist in this grammar -- ordinary parenthesized calls are
    ///   `method_invocation` with no `object` field, the same node kind
    ///   used for receiver-qualified calls, mirroring Java's
    ///   `method_invocation` shape exactly).
    /// - `import_types` is `"import_declaration"`, NOT baseline's
    ///   `"groovy_import"` (does not exist in this grammar -- imports are
    ///   modeled the same way Java's are).
    /// - `branch_types` swaps baseline's `"switch_statement"` (does not
    ///   exist in this grammar) for `"switch_expression"` (confirmed
    ///   present) plus adds `"enhanced_for_statement"` (Java-style
    ///   `for (T x : xs)`, confirmed present and a real decision point
    ///   this Java-shaped grammar clearly supports) alongside baseline's
    ///   `"if_statement"`/`"for_statement"`/`"while_statement"`.
    ///   `method_invocation`'s receiver/name split (like Java's) needs
    ///   [`crate::languages::generic::groovy_call_override`], and its
    ///   heritage fields need [`crate::languages::generic::groovy_quirk`]
    ///   -- see their own doc comments.
    pub const fn groovy() -> Self {
        Self {
            name: "groovy",
            func_types: &["function_definition"],
            method_types: &["method_declaration"],
            class_types: &["class_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["method_invocation", "juxt_function_call"],
            call_function_field: "UNUSED_SEE_GROOVY_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            import_types: &["import_declaration"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "enhanced_for_statement",
                "while_statement",
                "switch_expression",
                "&&",
                "||",
            ],
            decorator_types: &["annotation", "marker_annotation"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Ruby: node kinds verified directly against the `tree-sitter-ruby`
    /// 0.23.1 crate's own `src/node-types.json` (not blindly copied from
    /// the C baseline's `internal/cbm/lang_specs.c` `ruby_*` arrays,
    /// which target a different grammar snapshot -- e.g. the baseline's
    /// `extract_objc_callee` reads a `"selector"` field that does not
    /// exist on this crate's `message_expression`; see [`Self::objc`]'s
    /// own doc comment for that finding). G2.1c scope note: the
    /// baseline itself gives Ruby no dedicated `extract_base_classes`
    /// walker despite Ruby being class-based OOP (`internal/cbm/
    /// extract_defs.c`'s dedicated-walker list at :2377-2394 covers
    /// TS/TSX, PHP, Kotlin, Squirrel, Julia, F#, D, PowerShell, Pascal
    /// -- Ruby is absent), so Ruby's `class Sub < Base` INHERITS below
    /// rides the same generic `superclass`-field fallback the baseline's
    /// own `extract_base_classes` would reach for Ruby (its `fields[]`
    /// array at :2543 includes `"superclass"`), not a bespoke walker --
    /// this crate matches that same real (limited) depth rather than
    /// building a richer walker the baseline itself never has.
    /// - `func_types`/`method_types` both list `"method"`/
    ///   `"singleton_method"` (mirrors `ruby_func_types` exactly): the
    ///   generic engine's own nesting-based Function-vs-Method split
    ///   (a kind present in both arrays falls back to "is `enclosing`
    ///   set") then does the right thing for both a top-level `def foo`
    ///   (Function) and a `def foo` inside a `class`/`module` body
    ///   (Method) -- Ruby's grammar uses one node kind for both shapes,
    ///   same rationale as Rust's `function_item` in [`Self::rust`].
    /// - `call_types` includes `"command_call"` defensively (matching
    ///   `ruby_call_types`) even though this crate's grammar version
    ///   aliases `command_call`/`command_call_with_block`/
    ///   `_chained_command_call` to the plain `"call"` kind at the
    ///   parser level (verified against `grammar.js`'s own
    ///   `alias($.command_call, $.call)` -- a paren-less `puts "x"` and
    ///   a parenthesized `puts("x")` both surface as `.kind() ==
    ///   "call"`), so `"command_call"` never actually matches anything
    ///   for this specific crate version -- kept for baseline-parity
    ///   documentation, harmless as dead data.
    /// - `call_function_field`/`call_arguments_field` are placeholders,
    ///   never actually consulted: Ruby's `call` node's callee lives in
    ///   a `"method"` field (not `"function"`), with a separate
    ///   `"receiver"` field for the qualifying expression (`d.bark` ->
    ///   `method` = `bark`, `receiver` = `d`) -- a two-field split like
    ///   Java's `method_invocation`, so every one of `call_types` is
    ///   fully claimed by [`crate::languages::generic::ruby_call_override`]
    ///   before the generic engine's own single-field reconstruction
    ///   would ever run (same posture as [`Self::php`]'s
    ///   `call_function_field` doc).
    /// - `import_types` is empty (matches `ruby_import_types = {"call"}`
    ///   being a call-shaped node, not a syntactic import statement):
    ///   `require`/`require_relative` are ordinary `call` nodes,
    ///   detected and turned into IMPORTS edges by
    ///   [`crate::languages::generic::ruby_call_override`] itself
    ///   (mirrors `internal/cbm/extract_imports.c`'s
    ///   `parse_ruby_imports`/`ruby_require_method` exactly: callee text
    ///   is `require` or `require_relative`, first string argument is
    ///   the module path).
    pub const fn ruby() -> Self {
        Self {
            name: "ruby",
            func_types: &["method", "singleton_method"],
            method_types: &["method", "singleton_method"],
            class_types: &["class", "module"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call", "command_call"],
            call_function_field: "UNUSED_SEE_RUBY_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[
                "if", "unless", "while", "until", "for", "case", "when", "rescue", "elsif",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Zig: node kinds verified directly against the `tree-sitter-zig`
    /// 1.1.2 crate's own `src/node-types.json`. G2.1c scope note: Zig
    /// has no inheritance/subtyping syntax at all (no `class`/`extends`
    /// equivalent -- composition is explicit field embedding, not a
    /// written heritage clause), matching the baseline's own
    /// `internal/cbm/extract_defs.c` dedicated-walker list (:2377-2394)
    /// and generic `fields[]`/`base_types[]` fallback (:2543,:2613)
    /// having no Zig-specific entry either -- there is nothing to wire
    /// an INHERITS edge from, so [`crate::languages::generic::zig_quirks`]
    /// has no heritage handling, by design, not omission.
    /// - `class_types` (`struct_declaration`/`enum_declaration`/
    ///   `union_declaration`) have NO `name` field at all in this
    ///   grammar (verified: `{"type":"struct_declaration","fields":{}}`)
    ///   -- a Zig struct/enum/union is an anonymous *type expression*;
    ///   its name comes from being the right-hand side of a
    ///   `const Foo = struct {...}` `variable_declaration`, exactly one
    ///   level up the tree, not from any field on the type-expression
    ///   node itself (matches `internal/cbm/extract_defs.c`'s
    ///   `CBM_LANG_ZIG` case at :3731-3740: "the name is the parent
    ///   variable_declaration's identifier child"). `name_field` below
    ///   is consequently a placeholder never consulted for these three
    ///   kinds -- [`crate::languages::generic::zig_quirk`] claims them
    ///   in full, walking one level *up* via [`tree_sitter::Node::parent`]
    ///   rather than reading any field on the node itself.
    /// - `func_types` includes `"test_declaration"` (mirrors
    ///   `zig_func_types` exactly): `test "some name" { ... }` also has
    ///   NO `name` field (verified: positional children
    ///   `[test, string, block]`) -- the test's "name" is the quoted
    ///   string literal's `string_content` child, not an identifier
    ///   field, so this kind too is fully claimed by
    ///   [`crate::languages::generic::zig_quirk`] (mirrors
    ///   `internal/cbm/extract_defs.c`'s `resolve_zig_test_name`
    ///   exactly: find the `string` child, then its `string_content`
    ///   child).
    /// - `func_types` also includes `"function_signature"` (mirrors
    ///   `zig_func_types`): an `extern fn foo() void;`-style signature
    ///   with no body -- this crate's grammar gives it a working `name`
    ///   field directly (unlike the three cases above), so it flows
    ///   through the generic engine's own func/method branch with no
    ///   quirk needed (its `body_field` lookup simply finds nothing,
    ///   same as any other bodyless declaration).
    /// - `call_types` includes `"builtin_function"` (`@import(...)`,
    ///   `@compileLog(...)`, ...) alongside `call_expression`, mirroring
    ///   `zig_call_types`: this node kind has NO fields at all
    ///   (positional `[builtin_identifier, arguments]` children), so it
    ///   is fully claimed by
    ///   [`crate::languages::generic::zig_call_override`] rather than
    ///   the generic engine's own `call_function_field`-keyed
    ///   reconstruction -- `call_expression` itself DOES have a working
    ///   `"function"` field and needs no override.
    /// - `import_types` is empty (mirrors `zig_import_types =
    ///   {"builtin_function"}` being call-shaped, not a syntactic import
    ///   statement): `@import("std")` is an ordinary `builtin_function`
    ///   call whose callee is the builtin identifier `@import` --
    ///   [`crate::languages::generic::zig_call_override`] both records
    ///   the call AND, when the builtin identifier's text is
    ///   `"@import"`, pushes an IMPORTS edge from its first string
    ///   argument (mirrors the intent of the baseline's own
    ///   `zig_import_types` row, since no dedicated
    ///   `internal/cbm/extract_imports.c` Zig case exists to compare
    ///   against byte-for-byte -- Zig is not in that file's
    ///   language-dispatch `switch`, so `parse_generic_imports` never
    ///   runs for it there either; this crate's own explicit handling is
    ///   consequently *more* complete than the baseline's, an
    ///   improvement the "match the baseline's real depth" instruction
    ///   does not forbid when the baseline simply has a gap rather than
    ///   a deliberate design choice).
    pub const fn zig() -> Self {
        Self {
            name: "zig",
            func_types: &[
                "function_declaration",
                "test_declaration",
                "function_signature",
            ],
            method_types: &[],
            class_types: &[
                "struct_declaration",
                "enum_declaration",
                "union_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["container_field"],
            module_types: &["source_file"],
            call_types: &["call_expression", "builtin_function"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "switch_expression",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Objective-C: node kinds verified directly against the
    /// `tree-sitter-objc` 3.0.2 crate's own `src/node-types.json`. Built
    /// on the C grammar (`function_definition`/`struct_specifier`/
    /// `enum_specifier`/`union_specifier`/`type_definition`/
    /// `preproc_include` are the *same* node kinds [`Self::c`] already
    /// names) plus Objective-C-specific
    /// `class_interface`/`class_implementation`/`protocol_declaration`/
    /// `method_definition`/`method_declaration`/`message_expression`/
    /// `property_declaration` layered on top -- every array below is
    /// fully claimed by
    /// [`crate::languages::generic::objc_quirk`]/
    /// [`crate::languages::generic::objc_call_override`] rather than the
    /// generic engine's own `name_field`-keyed fallback, same posture
    /// and for the same declarator-unwrapping reasons as [`Self::c`]/
    /// [`Self::cpp`] (see their own doc comments) -- `name_field`/
    /// `call_function_field` below are consequently placeholders, never
    /// actually consulted.
    /// - `class_types` includes both the C-family shapes
    ///   (`struct_specifier`/`enum_specifier`/`type_definition`/
    ///   `union_specifier`, reusing [`crate::languages::generic::c_quirk`]'s
    ///   own declarator-unwrapping helpers directly since Objective-C's
    ///   grammar gives these kinds the identical field shapes C's does)
    ///   and the Objective-C-specific `class_interface`/
    ///   `class_implementation`/`protocol_declaration`. The latter three
    ///   have NO `name` field at all (verified: `class_interface`'s
    ///   fields are only `category`/`superclass` -- the class's own name
    ///   is the first positional `identifier` child); `class_interface`/
    ///   `class_implementation` DO have a working `superclass` field
    ///   (unlike Zig, giving Objective-C's `@interface Dog : Animal`
    ///   INHERITS a real source to read, verified: `{"type":
    ///   "class_interface","fields":{"superclass":{...
    ///   "types":[{"type":"identifier"}]}}}` -- a direct identifier,
    ///   not a wrapped node the way Ruby's `superclass` field is).
    /// - `func_types` includes `"function_definition"` (plain C-style
    ///   free function, mirrors `objc_func_types`) plus
    ///   `"method_definition"`/`"method_declaration"` (Objective-C
    ///   `-`/`+` method syntax) -- the latter two have NO fields at all
    ///   (verified: `{"type":"method_definition","fields":{}}`); a
    ///   method's full selector (`setName:withAge:` for a two-keyword
    ///   method, bare `bark` for a zero-argument one) is reconstructed
    ///   by joining every direct-child `identifier` node with `:`
    ///   (verified against a real parse: `- (void)setName:(NSString
    ///   *)name withAge:(int)age` yields exactly two direct-child
    ///   `identifier` nodes, `"setName"` and `"withAge"`, each
    ///   immediately followed by its own `method_parameter` sibling --
    ///   the nested `identifier` for the parameter's own local variable
    ///   name, e.g. `"name"`/`"age"`, lives one level deeper inside that
    ///   `method_parameter` node, so a *direct-child-only* scan never
    ///   confuses the two).
    /// - `call_types` includes `"call_expression"` (plain C-style call,
    ///   reuses the generic engine's own default single-field
    ///   reconstruction unchanged -- this kind's `"function"` field
    ///   works identically to C's) plus `"message_expression"`
    ///   (`[receiver method:arg]` Smalltalk-style send) -- the baseline
    ///   itself reads this node's callee off a `"selector"` field
    ///   (`internal/cbm/extract_calls.c`'s `extract_objc_callee`), which
    ///   does NOT exist on this specific grammar version (verified: no
    ///   `"selector"` field anywhere in `message_expression`'s
    ///   `node-types.json` entry) -- a real grammar-version drift this
    ///   worker's brief explicitly warned to check for rather than
    ///   blindly transcribe. This crate instead reads the verified
    ///   `"receiver"` field (the send target) and the `"method"` field
    ///   (`multiple: true` -- every keyword-selector part, `["setName",
    ///   "withAge"]` for a two-keyword send), joining the latter with
    ///   `:` the same way as the `method_definition`/`method_declaration`
    ///   case above so a call site's callee text and its own
    ///   definition's recorded name agree (`"setName:withAge:"` both
    ///   ways) -- both fully claimed by
    ///   [`crate::languages::generic::objc_call_override`].
    /// - `import_types` includes `"preproc_include"` only (not a
    ///   separate `"preproc_import"`, matching the finding that
    ///   `#import` and `#include` alias to the one `preproc_include`
    ///   node kind in this grammar, distinguished only by its own
    ///   `"directive"` field's literal-token text -- verified: `{"type":
    ///   "preproc_include","fields":{"directive":{"types":[{"type":
    ///   "#import"},{"type":"#include"}]}}}`) -- reuses
    ///   [`crate::languages::generic::c_quirk`]'s own `preproc_include`
    ///   handling unchanged (mirrors `internal/cbm/extract_imports.c`'s
    ///   own dispatch table at :2775, which routes `CBM_LANG_OBJC`
    ///   through the identical `parse_c_imports` C-family walker, not a
    ///   bespoke Objective-C one).
    pub const fn objc() -> Self {
        Self {
            name: "objc",
            func_types: &[
                "function_definition",
                "method_definition",
                "method_declaration",
            ],
            method_types: &["method_definition", "method_declaration"],
            class_types: &[
                "class_interface",
                "class_implementation",
                "protocol_declaration",
                "struct_specifier",
                "enum_specifier",
                "union_specifier",
                "type_definition",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["property_declaration"],
            module_types: &["translation_unit"],
            call_types: &["call_expression", "message_expression"],
            call_function_field: "UNUSED_SEE_OBJC_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            import_types: &["preproc_include"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "switch_statement",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::cpp()`).
            name_field: "UNUSED_SEE_OBJC_QUIRK",
            body_field: "body",
        }
    }

    /// Haskell: node kinds copied from the baseline's `haskell_func_types`/
    /// `haskell_class_types`/`haskell_module_types`/`haskell_call_types`/
    /// `haskell_import_types`/`haskell_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:515-521),
    /// cross-checked node-kind-by-node-kind against the actual
    /// `tree-sitter-haskell` 0.23.1 crate's own `src/node-types.json` (G2.2g
    /// grammar onboarding) plus a real parse-tree dump (`cargo run` against a
    /// scratch crate depending on `tree-sitter-haskell` directly) -- every
    /// node kind the baseline names is genuinely present in this grammar
    /// (Haskell's grammar-shape drift turned out milder than OCaml's/
    /// Erlang's own, see [`Self::ocaml`]/[`Self::erlang`]'s doc comments),
    /// with two corrections found against the baseline's own *effective*
    /// (not merely declared) behavior:
    /// - `func_types` is `["function", "bind"]`, NOT baseline's
    ///   `["function", "signature", "bind"]`: `signature` (a `foo :: Int ->
    ///   Int` type annotation, syntactically adjacent to but distinct from
    ///   the actual `foo x = ...` definition) genuinely has a working `name`
    ///   field in this grammar (verified: `signature.fields.name` ->
    ///   `prefix_id`/`variable`), so including it verbatim would emit a
    ///   SECOND, duplicate Function symbol for every type-annotated
    ///   definition -- but the baseline's own `cbm_resolve_func_name`
    ///   (`internal/cbm/extract_defs.c`:670-682) explicitly special-cases
    ///   `CBM_LANG_HASKELL && kind == "signature"` to return a null node
    ///   BEFORE its generic `name`-field fallback would ever fire, so no
    ///   symbol is ever actually emitted for a `signature` node in the real
    ///   baseline despite the array listing it -- this row matches that
    ///   *effective* suppression by omitting `signature` outright rather
    ///   than including it and then needing a quirk to null it back out
    ///   again for an identical result. `bind` (a nullary `x = 1` value
    ///   binding, confirmed present with the same `name` field shape as
    ///   `function`) is unaffected and kept as baseline has it.
    /// - `import_types` is `["import"]` only, NOT baseline's `["import",
    ///   "instance"]`: `instance Greet Shape where ...` is a real,
    ///   confirmed-present node kind (correctly classified under
    ///   [`Self::class_types`] below, not here) with no import-shaped
    ///   semantics at all -- baseline's own dedicated Haskell import walker
    ///   (`internal/cbm/extract_imports.c`'s `parse_haskell_imports`,
    ///   :1053-1075) only ever matches a literal `"import"`/`"imports"`
    ///   node kind, never consulting `haskell_import_types` itself for the
    ///   extraction dispatch at all -- `"instance"` in that array is
    ///   consequently dead data in the baseline, the same "array lists it,
    ///   the actual dispatch never reads the array" class of finding
    ///   [`Self::kotlin`]/[`Self::ruby`]'s own doc comments already record
    ///   for other languages, not reproduced here.
    ///
    /// `class_types` (`class`/`data_type`/`newtype`, all confirmed with a
    /// working `name` field) additionally includes `instance` (baseline
    /// correctly classifies it as call/def-adjacent structure, not import --
    /// this row just gives it a home in `class_types` since `instance
    /// Greet Shape where ...`'s own `name` field holds `Greet`, a
    /// reasonable Class-shaped symbol for "this type instantiates this
    /// class"). All four are fully claimed by
    /// [`crate::languages::generic::haskell_quirk`]'s own
    /// [`crate::languages::generic::haskell_class_like`] arm (reading the
    /// real `"name"` field directly, hardcoded, rather than through this
    /// row's own `name_field`) despite every one genuinely having a
    /// working field the generic engine's own class-shape fallback COULD
    /// read on its own -- see this const's own `name_field` bullet below
    /// for why (the field is shared with the func/method branch, which
    /// genuinely does need it to be a placeholder). `module_types` is
    /// intentionally empty, NOT baseline's
    /// `["haskell"]` (the grammar's own root node kind): identical rationale
    /// to [`Self::solidity`]'s own doc comment -- the baseline's
    /// `module_node_types` struct field has zero consumers anywhere in that
    /// codebase (dead data there), while this crate's own `module_types` is
    /// live (emits a real `SymbolKind::Module`), and `haskell`'s first named
    /// child is its `header` node (confirmed via a real parse: `module Main
    /// where` parses as `haskell.header{module: module{module_id: "Main"}}`),
    /// whose OWN text is the whole `"module Main where"` header line, not
    /// just `"Main"` -- porting `"haskell"` verbatim would invent new, wrong
    /// behavior (a noisy Module symbol) rather than reproduce a baseline
    /// behavior that does not exist. `call_types` (`infix`/`apply`, both
    /// confirmed present) and `field_types`/`call_function_field`/
    /// `call_arguments_field` are fully claimed by
    /// [`crate::languages::generic::haskell_quirk`]/
    /// [`crate::languages::generic::haskell_call_override`] rather than the
    /// generic engine's own single-field fallback -- `apply`'s two fields are
    /// `function`/`argument` (singular, right-associatively curried: `f a b`
    /// nests as `apply(function=apply(function=f,argument=a),argument=b)`,
    /// confirmed via the same real parse-tree dump), and `infix`'s callee is
    /// its own `operator` field's text (`x + y` records callee `"+"`, mirroring
    /// the baseline's own `extract_fp_callee`'s `infix`/`infix_expression`
    /// arm at `internal/cbm/extract_calls.c`:345-354 exactly) -- neither shape
    /// fits a single `function`/`arguments`-field pair the generic engine's
    /// default reconstruction assumes.
    /// - `name_field` is a placeholder (`"UNUSED_SEE_HASKELL_QUIRK"`), NOT
    ///   the real `"name"` string, DESPITE `function`/`bind` genuinely
    ///   having a working `name` field (confirmed above) -- this is
    ///   deliberate, not an oversight: the generic engine's own func/method
    ///   branch (`generic::walk`'s `spec.func_types.contains(&kind)` arm)
    ///   checks `name_field` FIRST, and on success pushes the Function
    ///   symbol AND RETURNS immediately, before `on_unmatched_node` (where
    ///   [`crate::languages::generic::haskell_quirk`]'s own `function`/`bind`
    ///   arm lives) would ever get a chance to run at all. Since Haskell's
    ///   `body_field` is ALSO a placeholder (there is no single flat body
    ///   field -- the actual body lives across one or more `match`
    ///   children, see [`crate::languages::generic::haskell_walk_scoped`]'s
    ///   own doc comment), a real `name_field` here would let the generic
    ///   branch push a correct symbol but then silently find no body to
    ///   recurse into (`body_field` resolves to nothing) -- EVERY call
    ///   inside every Haskell function/binding would go unrecorded, with no
    ///   error or panic to reveal it. This is the inverse of every other
    ///   quirk-claimed language's usual "array fully claimed by quirk,
    ///   `name_field` never consulted" posture in this file
    ///   ([`Self::c`]/[`Self::kotlin`]/[`Self::ocaml`]'s own docs): here the
    ///   field genuinely WOULD resolve correctly if consulted, and that is
    ///   exactly the problem -- caught by this row's own
    ///   `tests/unit_languages_haskell.rs`'s own call-recording tests
    ///   failing during this wave's own verification (symbols extracted
    ///   correctly, every single call silently missing) rather than by
    ///   inspection.
    pub const fn haskell() -> Self {
        Self {
            name: "haskell",
            func_types: &["function", "bind"],
            method_types: &[],
            class_types: &["class", "data_type", "newtype", "instance"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // See this const's own doc comment: intentionally empty,
            // NOT `["haskell"]`.
            module_types: &[],
            call_types: &["infix", "apply"],
            call_function_field: "UNUSED_SEE_HASKELL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_HASKELL_CALL_OVERRIDE",
            // See this const's own doc comment: `["import"]` only, NOT
            // `["import", "instance"]`.
            import_types: &["import"],
            branch_types: &["match", "guards", "if", "case", "do", "boolean"],
            decorator_types: &[],
            // See this const's own doc comment for why this is
            // deliberately a placeholder despite `function`/`bind` having
            // a real, working `"name"` field.
            name_field: "UNUSED_SEE_HASKELL_QUIRK",
            body_field: "UNUSED_SEE_HASKELL_QUIRK",
        }
    }

    /// OCaml: node kinds copied from the baseline's `ocaml_func_types`/
    /// `ocaml_class_types`/`ocaml_module_types`/`ocaml_call_types`/
    /// `ocaml_import_types`/`ocaml_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:525-536), verified
    /// node-kind-by-node-kind against the actual `tree-sitter-ocaml` 0.25.0
    /// crate's own `grammars/ocaml/src/node-types.json` (G2.2g grammar
    /// onboarding) plus a real parse-tree dump -- every node kind the
    /// baseline names is genuinely present in this grammar (unlike some
    /// other G2 languages, no stale/missing node-kind NAMES were found), but
    /// the *field shapes* diverge sharply from what a flat `name_field`
    /// could express for almost every one of them, cross-checked against
    /// `internal/cbm/extract_defs.c`'s own dedicated
    /// `resolve_ocaml_func_name` (:253-262) to confirm the baseline itself
    /// needs the identical two-level unwrap this row's quirk performs:
    /// - `func_types` is `["value_definition"]` only, NOT baseline's
    ///   `["value_definition", "constructor_declaration",
    ///   "method_definition"]`: `constructor_declaration` (a data
    ///   constructor, `Circle of float`) and `method_definition` (an
    ///   `object ... end`'s `method draw = ...`) are BOTH confirmed to have
    ///   NO `name` field of their own at all in this grammar (verified:
    ///   `constructor_declaration.fields == {}`, its name is an unfielded
    ///   `constructor_name` child; `method_definition.fields` has no `name`
    ///   key either, its name is an unfielded `method_name` child) -- the
    ///   baseline's own `cbm_resolve_func_name` has NO
    ///   `CBM_LANG_OCAML`-specific case for either kind (only
    ///   `"value_definition"` gets `resolve_ocaml_func_name`), so both fall
    ///   through to the fully generic `func_name_node` helper
    ///   (`internal/cbm/extract_defs.c`:197-204, a bare `name`-field read
    ///   with no fallback), which resolves a null node for both -- the
    ///   baseline lists them in its array but its own resolver NEVER
    ///   actually recovers a name for either, so no Function/Method symbol
    ///   is ever emitted for them in the real baseline despite the array
    ///   claiming otherwise (same "array lists it, the actual behavior
    ///   never reaches it" class of dead-array-entry finding
    ///   [`Self::haskell`]'s own doc comment above records). This row
    ///   improves on that real gap rather than reproducing it (matching the
    ///   "more complete than baseline when baseline has a gap rather than a
    ///   deliberate design choice" precedent [`Self::zig`]'s own doc comment
    ///   sets): [`crate::languages::generic::ocaml_quirk`] claims both
    ///   `constructor_declaration` and `method_definition` directly (via
    ///   their real `constructor_name`/`method_name` children) alongside
    ///   `value_definition`'s own baseline-mirrored two-level
    ///   `let_binding` -> `pattern`-field unwrap.
    /// - `class_types` is `["class_definition", "module_definition",
    ///   "exception_definition"]`, NOT baseline's full `["type_definition",
    ///   "class_definition", "module_definition", "exception_definition",
    ///   "record_declaration"]`: `type_definition` is instead
    ///   [`Self::alias_types`] below (its nested `type_binding` child DOES
    ///   carry a real, working `name` field -- `type_constructor`/
    ///   `type_constructor_path` -- unlike every other kind in this list, so
    ///   it does not need the quirk's child-kind-search unwrap at all, only
    ///   a one-level `type_binding` descent handled generically).
    ///   `record_declaration` is dropped entirely: verified via
    ///   `node-types.json` that it is NEVER a possible direct child of
    ///   `compilation_unit` (the root) at all -- it only ever appears
    ///   nested one level inside `constructor_declaration`
    ///   (`Circle of { x : float }`) or as `type_binding`'s own `body`
    ///   (`type point = { x : float }`), and has no `fields` of its own
    ///   whatsoever (a bare list of `field_declaration` children with no
    ///   name) -- there is no scenario where this kind could be reached as
    ///   a would-be top-level definition with a name to record, so including
    ///   it in `class_types` (as baseline's own array does) would be
    ///   unreachable dead data here too, not merely imprecise. `class_binding`/
    ///   `module_binding` (the wrapper children `class_definition`/
    ///   `module_definition` themselves are, one level further removed than
    ///   `value_definition`'s single `let_binding` wrapper: `class_binding`
    ///   carries an unfielded `class_name` child, `module_binding` an
    ///   unfielded `module_name` child) are claimed by `ocaml_quirk`'s own
    ///   `class_definition`/`module_definition` arms; `exception_definition`
    ///   reuses the same `constructor_name`-child-search the quirk's
    ///   `constructor_declaration` arm uses directly (an exception
    ///   declaration IS a bare `constructor_declaration` child, confirmed via
    ///   `node-types.json`: `exception_definition.children` includes
    ///   `constructor_declaration`).
    /// - `call_types` keeps baseline's full `["application_expression",
    ///   "infix_expression", "method_invocation", "module_application",
    ///   "new_expression"]` (every one confirmed present in
    ///   `node-types.json`, addressing this worker's brief's explicit
    ///   "verify these actually exist" instruction -- they do, this crate's
    ///   probe simply had no OOP/functor-application call site to exercise
    ///   them against directly, but their `node-types.json` field shapes
    ///   were cross-checked all the same). None fits the generic engine's
    ///   single `function`/`arguments`-field-pair assumption
    ///   (`application_expression`'s own `argument` field is `multiple:
    ///   true`, a curried list, not a single node the way
    ///   `call_arguments_field` assumes; `infix_expression` has `left`/
    ///   `operator`/`right`; `method_invocation` has `object`/`method`;
    ///   `module_application` has `functor`/`argument`; `new_expression` has
    ///   no fields at all, just an unfielded `class_path` child) -- every
    ///   one is fully claimed by
    ///   [`crate::languages::generic::ocaml_call_override`], mirroring the
    ///   baseline's own shared `extract_fp_callee`
    ///   (`internal/cbm/extract_calls.c`:324-356, used for
    ///   `CBM_LANG_HASKELL`/`CBM_LANG_OCAML`/`CBM_LANG_PURESCRIPT` alike)
    ///   `apply`/`application_expression` curried-descent-via-`function`-field
    ///   algorithm and `infix`/`infix_expression` operator-field-as-callee
    ///   algorithm exactly -- `method_invocation`/`module_application`/
    ///   `new_expression` have no baseline-shared helper to mirror (the
    ///   baseline's own `extract_fp_callee` never matches any of the three),
    ///   so this row's quirk adds its own direct field/child reads for them,
    ///   the same "more complete than baseline when baseline has a real gap"
    ///   posture as `func_types`'s correction above.
    /// - `import_types` is `["open_module"]` only, NOT baseline's
    ///   `["open_module", "include"]`: `"include"` (baseline's literal array
    ///   entry) is not a real *named* node kind in this grammar at all --
    ///   verified: `node-types.json` lists `include` only as an unnamed
    ///   keyword-token entry (`{"type": "include", "named": false}`), while
    ///   the real *named* include-statement node this grammar actually
    ///   produces is `include_module` (`include List` parses as
    ///   `include_module{module: module_path{module_name: "List"}}`) -- a
    ///   real grammar-version-drift finding this worker's brief explicitly
    ///   warned to check for. This row corrects the name to
    ///   `"include_module"`... except it is deliberately NOT added to this
    ///   array either: the baseline's own actual import-extraction dispatch
    ///   (`internal/cbm/extract_imports.c`:2809,
    ///   `parse_generic_imports(ctx, "open_module")`) only ever scans for a
    ///   literal `"open_module"` node kind -- `ocaml_import_types` (the
    ///   array baseline declares, listing both) is, like Haskell's `import_types`
    ///   above, never itself consulted by the actual per-language import
    ///   dispatch `switch` (each language's case hardcodes its own node-kind
    ///   string argument directly) -- matching that real dispatch rather
    ///   than the array is this row's `import_types` choice. `open_module`'s
    ///   own `module` field (a `_module_expression`, confirmed to resolve to
    ///   a plain `module_path`/`module_name` pair whose combined text is a
    ///   clean unqualified module name for a simple `open Printf`) is read
    ///   directly by the generic engine's own `on_unmatched_node` quirk gate
    ///   (see `walk`'s `import_types` branch) via a small
    ///   [`crate::languages::generic::ocaml_quirk`] arm, mirroring the
    ///   baseline's own `try_generic_path_fields`'s `"module"`-field
    ///   fallback exactly. `include_module` is deliberately left unhandled
    ///   here for the identical "match the baseline's real (narrower) depth,
    ///   don't invent extra behavior it never had" reason [`Self::ruby`]'s
    ///   own doc comment already establishes for a different language's
    ///   analogous gap.
    /// - `module_types` is intentionally empty, NOT baseline's
    ///   `["compilation_unit"]` -- identical rationale to [`Self::haskell`]/
    ///   [`Self::solidity`]'s own doc comments (baseline's own
    ///   `module_node_types` field has zero consumers anywhere in that
    ///   codebase; `compilation_unit` is this grammar's root node, whose
    ///   first child is arbitrarily whatever the file's first top-level item
    ///   happens to be, not a module name -- OCaml files are not required to
    ///   open with a `module X = struct ... end` the way, say, a Java file
    ///   opens with a `package` clause).
    ///
    /// One `Language`/[`crate::languages::generic::parse_ocaml`] entry point
    /// covers BOTH `.ml` (implementation) and `.mli` (interface) files,
    /// matching the baseline's own `src/discover/language.c` `EXT_TABLE`
    /// choice (both extensions map to the one `CBM_LANG_OCAML`, which itself
    /// only ever binds the one `tree_sitter_ocaml` implementation-grammar
    /// factory -- the baseline has no separate interface-grammar binding at
    /// all despite the `tree-sitter-ocaml` C sources it vendors including
    /// one). This crate's own `tree-sitter-ocaml` 0.25.0 crate additionally
    /// exposes a real, distinct `LANGUAGE_OCAML_INTERFACE` grammar
    /// specifically for `.mli` content -- confirmed NOT used here, matching
    /// baseline's one-grammar-for-both choice rather than "improving" past
    /// it, since the plain implementation grammar's own `node-types.json`
    /// already lists a working `value_specification` node kind (the `val f :
    /// int -> int` signature form that is a `.mli` file's entire content) for
    /// the identical construct a `module type X = sig ... end` block can
    /// embed directly inside an ordinary `.ml` file -- `.mli` syntax is
    /// consequently already a well-formed subset of what the one
    /// implementation grammar parses without error, so a second `LangSpec`
    /// row/`Language` variant purely to swap grammars would add real
    /// complexity (a second spec row, a second quirk set, a second
    /// `Language` enum arm every future cross-language switch statement in
    /// this crate would need to remember) for no observable extraction
    /// difference at this crate's current (Tier-2: defs+calls+imports only)
    /// depth.
    pub const fn ocaml() -> Self {
        Self {
            name: "ocaml",
            // See this const's own doc comment: `["value_definition"]`
            // only -- `constructor_declaration`/`method_definition` are
            // claimed directly by `ocaml_quirk` instead (neither has a
            // `name_field` this array's generic fallback could use).
            func_types: &["value_definition"],
            method_types: &[],
            // See this const's own doc comment: NOT baseline's full array
            // (`type_definition` moved to `alias_types`; `record_declaration`
            // dropped as unreachable dead data).
            class_types: &[
                "class_definition",
                "module_definition",
                "exception_definition",
            ],
            interface_types: &[],
            enum_types: &[],
            // See this const's own doc comment: `type_definition` lives
            // here, not in `class_types` -- its nested `type_binding` child
            // has a real, working `name` field the ordinary generic
            // class-shape fallback (one level of `type_binding` descent,
            // handled by `ocaml_quirk`) can use directly.
            alias_types: &["type_definition"],
            field_types: &[],
            // See this const's own doc comment: intentionally empty, NOT
            // `["compilation_unit"]`.
            module_types: &[],
            call_types: &[
                "application_expression",
                "infix_expression",
                "method_invocation",
                "module_application",
                "new_expression",
            ],
            call_function_field: "UNUSED_SEE_OCAML_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_OCAML_CALL_OVERRIDE",
            // See this const's own doc comment: `["open_module"]` only,
            // NOT `["open_module", "include"]` (`"include"` is not a real
            // named node kind in this grammar at all; the real named kind,
            // `include_module`, is deliberately left unhandled to match the
            // baseline's own narrower actual dispatch).
            import_types: &["open_module"],
            branch_types: &["match_expression", "if_expression", "match_case"],
            decorator_types: &[],
            // Never actually consulted: every one of `func_types`/
            // `class_types`/`call_types`/`import_types` above is fully
            // claimed by `ocaml_quirk`/`ocaml_call_override` before this
            // value would matter, except `alias_types`' `type_definition`,
            // which uses a dedicated one-level `type_binding` unwrap (also
            // inside `ocaml_quirk`, since the name field lives on the CHILD
            // node, not `type_definition` itself) rather than this flat
            // field -- same "not a real field name" posture as
            // [`Self::c`]'s identical doc note.
            name_field: "UNUSED_SEE_OCAML_QUIRK",
            body_field: "UNUSED_SEE_OCAML_QUIRK",
        }
    }

    /// Erlang: node kinds copied from the baseline's `erlang_func_types`/
    /// `erlang_class_types`/`erlang_module_types`/`erlang_call_types`/
    /// `erlang_import_types`/`erlang_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:613-619), verified
    /// node-kind-by-node-kind against the actual `tree-sitter-erlang` 0.19.0
    /// crate's own `src/node-types.json` (G2.2g grammar onboarding, the
    /// canonical WhatsApp-authored grammar -- confirmed via the crate's
    /// `repository` metadata) plus a real parse-tree dump -- Erlang shows
    /// the heaviest baseline-array staleness of this worker's three
    /// languages (this worker's brief's "functional languages are especially
    /// likely to have grammar-version drift" warning proved most true here),
    /// with baseline names that are consistently a truncated/differently-
    /// suffixed guess at the real node-type strings this grammar version
    /// actually emits:
    /// - `branch_types` is `["if_expr", "case_expr", "receive_expr"]`, NOT
    ///   baseline's `["if_expression", "case_expression",
    ///   "receive_expression"]`: every one of the three real node kinds
    ///   this grammar emits uses the short `_expr` suffix, confirmed absent
    ///   under the longer `_expression` spelling anywhere in
    ///   `node-types.json` (a grep for the literal baseline strings returns
    ///   zero matches) -- baseline's own array would silently match nothing
    ///   at all for this language if transcribed verbatim, an invisible
    ///   "0 branches ever counted" bug this crate's own
    ///   `tests/unit_languages_erlang.rs` proves does not happen here.
    /// - `import_types` is `["import_attribute"]`, NOT baseline's
    ///   `["module_attribute", "import", "include"]` on any of its three
    ///   counts: `"import"`/`"include"` are not real *named* node kinds this
    ///   grammar produces at all (both are unnamed keyword-token entries in
    ///   `node-types.json`, same "listed as bare keyword text, never a real
    ///   statement node" class of finding [`Self::gdscript`]'s own doc
    ///   comment already records for a bare `"extends"` token); the real
    ///   named include-statement kinds are `pp_include`/`pp_include_lib`
    ///   (a `-include("file.hrl").`/`-include_lib("file.hrl").` directive),
    ///   and the real named import-statement kind is `import_attribute`
    ///   (`-import(lists, [sort/1]).`, with real `module`/`funs` fields).
    ///   `module_attribute` (`-module(main).`, confirmed present with a
    ///   working `name` field) is ALSO real, but is deliberately NOT
    ///   included here despite being the ONE array entry the baseline's own
    ///   actual import-extraction dispatch call site literally passes
    ///   (`internal/cbm/extract_imports.c`:2807,
    ///   `parse_generic_imports(ctx, "module_attribute")`) -- reading that
    ///   call site's own generic field-fallback chain
    ///   (`try_generic_path_fields`'s `path`/`source`/`module`/`name` field
    ///   list) against `module_attribute`'s own single `name` field shows
    ///   the baseline's real behavior is to record `-module(main).`'s OWN
    ///   module name (`"main"`) as if the file were IMPORTING itself -- a
    ///   self-referential ImportRef with no useful "this file depends on
    ///   that other module" meaning at all, not a deliberate design choice
    ///   worth reproducing (unlike, say, [`Self::zig`]'s `@import` handling,
    ///   which is a real dependency edge). This row instead extracts the
    ///   semantically real dependency edge (`-import(Module, [Funs]).`'s own
    ///   `module` field text) via
    ///   [`crate::languages::generic::erlang_quirk`], the same "more
    ///   complete/more correct than baseline when baseline's own choice is a
    ///   real gap rather than a considered design decision" posture
    ///   [`Self::zig`]/[`Self::ocaml`]'s (`func_types` correction) own doc
    ///   comments already establish for other languages' analogous gaps.
    /// - `call_types` is `["call"]`, matching baseline exactly (confirmed
    ///   present with real `expr`/`args` fields, though NEITHER matches this
    ///   file's `call_function_field`/`call_arguments_field` single-field
    ///   convention directly: `args` points at an intermediate `expr_args`
    ///   wrapper node, not a flat argument list the way, say, Rust/Go/TS's
    ///   `"arguments"` field does) -- fully claimed by
    ///   [`crate::languages::generic::erlang_call_override`], mirroring the
    ///   baseline's own dedicated `extract_erlang_callee`
    ///   (`internal/cbm/extract_calls.c`:487-492) exactly: a `call` node's
    ///   callee is simply its own first (unnamed-included) child's text --
    ///   for an unqualified call (`helper(3)`) that is the bare callee atom
    ///   directly; for a REMOTE/qualified call (`io:format(...)`), this
    ///   grammar wraps the inner `call` node (whose own callee is only the
    ///   unqualified `format` half) inside a sibling `remote` node carrying
    ///   the `io:` module qualifier on a separate `module` field -- the
    ///   baseline's own `extract_erlang_callee` has no `"remote"`-specific
    ///   arm at all (it only ever matches `"call"`), so it records ONLY the
    ///   unqualified half (`"format"`, dropping the `io:` qualifier
    ///   entirely) for a remote call too -- this row's quirk matches that
    ///   real (qualifier-dropping) baseline depth rather than "improving" it
    ///   with a `io:format` fully-qualified reconstruction the baseline
    ///   itself never attempts, per the "match the baseline's real depth,
    ///   don't invent a richer one it never had" precedent [`Self::ruby`]'s
    ///   own doc comment already sets for a different language's analogous
    ///   choice.
    /// - `class_types` is `["type_alias"]`, matching baseline exactly
    ///   (confirmed present: a `-type shape() :: ... .` declaration) --
    ///   fully claimed by [`crate::languages::generic::erlang_quirk`] rather
    ///   than the generic engine's own flat `name_field` fallback: its own
    ///   `name` field points at an intermediate `type_name` wrapper node
    ///   (`shape()`, including the parenthesized arity/params suffix), not a
    ///   bare identifier -- the real type name text is that wrapper's OWN
    ///   nested `name` field (an `atom` node, confirmed via
    ///   `node-types.json`: `type_name.fields.name` -> `atom`), a two-level
    ///   unwrap the same general shape as [`Self::ocaml`]'s
    ///   `value_definition` -> `let_binding` -> `pattern` unwrap above,
    ///   applied to a different language's own two-level field nesting.
    /// - `func_types` is `["function_clause"]`, matching baseline exactly
    ///   (confirmed present with real `name`/`args`/`body`/`guard` fields --
    ///   this is the one node kind in this row that needs NO quirk at all
    ///   for the base name-extraction case, flowing through the generic
    ///   engine's ordinary func/method branch unchanged). Every real-world
    ///   `function_clause` is syntactically wrapped one level inside a
    ///   `fun_decl` (one or more clauses joined by `;`, confirmed via
    ///   `node-types.json`: `fun_decl.fields.clause` ->
    ///   `_function_or_macro_clause`), but `fun_decl` itself carries no name
    ///   of its own to extract -- the generic engine's ordinary recursive
    ///   walk already reaches each nested `function_clause` correctly
    ///   without `fun_decl` needing to be one of this row's own arrays at
    ///   all (mirrors how this crate's existing languages already handle
    ///   "the real definition is nested one syntactic layer below a
    ///   structural wrapper with no name of its own").
    /// - `module_types` is intentionally empty, NOT baseline's
    ///   `["source_file"]` -- identical rationale to every other language
    ///   row's own analogous doc comment in this file (baseline's own
    ///   `module_node_types` field has zero consumers anywhere in that
    ///   codebase; `source_file` is this grammar's root node, whose first
    ///   child is arbitrarily whichever top-level attribute/function
    ///   happens to come first, not a module name -- Erlang's REAL
    ///   module-naming construct, `-module(main).`, is instead correctly
    ///   available as plain source text on the `module_attribute` node this
    ///   row's `import_types` correction above deliberately does NOT treat
    ///   as an import edge; a future G3 pass wanting an actual Module symbol
    ///   for Erlang has a real, named, non-dead node kind to reach for here
    ///   rather than this crate needing to invent one now).
    pub const fn erlang() -> Self {
        Self {
            name: "erlang",
            func_types: &["function_clause"],
            method_types: &[],
            class_types: &["type_alias"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // See this const's own doc comment: intentionally empty, NOT
            // `["source_file"]`.
            module_types: &[],
            call_types: &["call"],
            call_function_field: "UNUSED_SEE_ERLANG_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_ERLANG_CALL_OVERRIDE",
            // See this const's own doc comment: `["import_attribute"]`,
            // NOT baseline's `["module_attribute", "import", "include"]`
            // (none of baseline's three literal array entries is both real
            // AND semantically a dependency edge -- see the corrections
            // above).
            import_types: &["import_attribute"],
            branch_types: &["if_expr", "case_expr", "receive_expr"],
            decorator_types: &[],
            // NOT a placeholder here, unlike this file's usual
            // "UNUSED_SEE_..." convention for a fully-quirk-claimed
            // array: `class_types`' own `type_alias` IS fully claimed by
            // `erlang_quirk`'s two-level `type_name` unwrap (so this
            // value is never consulted for THAT array), but `func_types`'
            // own `function_clause` is deliberately NOT quirk-claimed
            // (see this const's own doc comment) and flows through the
            // generic engine's ordinary func/method branch instead,
            // which reads this exact field by name
            // (`generic::walk`'s `child_text(node, spec.name_field, ..)`
            // call) -- a placeholder string here would silently break
            // `function_clause`'s own real, working `"name"` field
            // lookup (caught by this row's own
            // `tests/unit_languages_erlang.rs::extracts_function_clause_as_function`
            // failing during this wave's own verification, not by
            // inspection).
            name_field: "name",
            body_field: "body",
        }
    }

    /// Bash: node kinds verified directly against the `tree-sitter-bash`
    /// 0.25.1 crate's own `src/node-types.json` (G2.2f grammar
    /// onboarding), cross-checked against a real parse tree dump
    /// (`cargo run` against a scratch crate depending on
    /// `tree-sitter-bash` directly). Every one of the baseline's
    /// `bash_func_types`/`bash_module_types`/`bash_call_types`/
    /// `bash_import_types`/`bash_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:482-488) matched
    /// this grammar version exactly -- unlike several G2.1/G2.2 languages,
    /// no stale baseline entries were found here.
    /// - `func_types` is `["function_definition"]`: this single node
    ///   kind covers BOTH the `function greet() { ... }` keyword form and
    ///   the bare POSIX `greet() { ... }` form (confirmed by a real parse
    ///   of both -- they produce an identical `function_definition` node
    ///   shape, differing only in whether an unnamed `function` keyword
    ///   token precedes the `name` field). It carries a real `name` field
    ///   (a `word` node) and a real `body` field (`compound_statement`),
    ///   so it flows through the generic engine's own func/method branch
    ///   with no quirk needed at all.
    /// - `call_types` is `["command"]`: a `command`'s `name` field points
    ///   at a `command_name` WRAPPER node (not a bare `word` directly),
    ///   but -- same "the wrapper's own `.utf8_text()` spans the exact
    ///   same bytes as its single child" property already relied on by
    ///   [`Self::solidity`]'s `call_function_field` doc comment --
    ///   reading `.utf8_text()` on the `command_name` wrapper itself
    ///   yields the identical callee text a bare identifier would, so
    ///   this row's own `call_function_field = "name"` needs no quirk
    ///   for the base callee-text case either. What DOES need
    ///   [`crate::languages::generic::bash_call_override`]: `command` has
    ///   no `arguments`-field wrapper at all (a real parse confirms every
    ///   argument is instead a repeated, unfielded-by-name `argument`
    ///   field entry directly on the `command` node itself -- there is no
    ///   separate child node this row's `call_arguments_field` mechanism
    ///   could point `child_by_field_name` at), so `arg_texts`
    ///   reconstruction is claimed by the override instead; the override
    ///   also recognizes `source`/`.` as import-shaped (see
    ///   `import_types`'s own note below) before falling through to an
    ///   ordinary call record for every other command name.
    /// - `import_types` is empty, NOT `["command"]` (the baseline's own
    ///   value, `bash_import_types = {"command", NULL}`, consumed via
    ///   `parse_generic_imports(ctx, "command")`): a `command`-kind node
    ///   is ALSO this row's entire `call_types` array, and
    ///   [`crate::languages::generic::walk`]'s own dispatch checks
    ///   `call_types` (and, before that, `func_types`/`method_types`)
    ///   well before it would ever reach an `import_types`-driven branch
    ///   for the identical node kind -- listing `"command"` in BOTH
    ///   arrays would make the `import_types` entry pure dead data (same
    ///   "one node kind, one array wins" constraint [`Self::gdscript`]'s
    ///   own doc comment already documents for
    ///   `class_name_statement`/GDScript, and the reason its own
    ///   `class_types` entry is instead serviced by a quirk rather than
    ///   its OWN generic array's fallback). `source ./lib.sh`/
    ///   `. ./other.sh` import detection is instead handled entirely
    ///   inside [`crate::languages::generic::bash_call_override`] (the
    ///   same hook that already claims every `command` node for CALLS),
    ///   which additionally pushes an IMPORTS edge when the command's own
    ///   name text is exactly `"source"` or `"."`, using the first
    ///   `argument`-field child as the module path -- functionally
    ///   equivalent to the baseline's intent (source/dot commands ARE
    ///   Bash's closest import-statement analog) without inventing a
    ///   second, unreachable array entry.
    pub const fn bash() -> Self {
        Self {
            name: "bash",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["command"],
            call_function_field: "name",
            // Never actually consulted: `command` has no `arguments`
            // field at all -- see this const's own doc comment.
            // `bash_call_override` claims every `call_types` node before
            // this value would matter.
            call_arguments_field: "UNUSED_SEE_BASH_CALL_OVERRIDE",
            // See this const's own doc comment: intentionally empty,
            // NOT `["command"]` (dead data double-listing a node kind
            // this row's own `call_types` array already claims first).
            import_types: &[],
            branch_types: &[
                "if_statement",
                "while_statement",
                "for_statement",
                "case_statement",
                "elif_clause",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Lua: node kinds verified directly against the
    /// `tree-sitter-lua` 0.5.0 crate's own `src/node-types.json` (G2.2f
    /// grammar onboarding), cross-checked against a real parse tree dump.
    /// Two of the baseline's own `lua_branch_types` entries
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:410-411,
    /// `"for_in_statement"`) are STALE against this grammar version: this
    /// crate's grammar (like every actively-maintained Lua grammar
    /// lineage) has exactly ONE `for_statement` node kind, whose own
    /// `clause` field is either a `for_numeric_clause` (`for i = 1, 10`)
    /// or a `for_generic_clause` (`for k, v in pairs(t)`) -- there is no
    /// separate top-level `for_in_statement` node kind at all (confirmed:
    /// `node-types.json` has no such entry, and a real parse of a
    /// generic for-in loop produces a `for_statement` root, not a
    /// distinctly-named one). Listing the baseline's stale name here
    /// would be silently-dead data (same class of finding as
    /// [`Self::groovy`]'s missing-`method_declaration`/[`Self::dart`]'s
    /// stale-`function_signature` corrections) -- omitted rather than
    /// copied blindly; `for_statement` alone already covers both clause
    /// shapes since the decision point is the outer node, not its clause.
    /// - `func_types` is `["function_declaration"]` only, NOT also
    ///   baseline's `"function_definition"`: `function_declaration`
    ///   (`function foo() end` / `function t.foo() end` / `function
    ///   t:foo() end`) has a real `name` field, confirmed present for
    ///   all three shapes -- `dot_index_expression`'s and
    ///   `method_index_expression`'s own `.utf8_text()` naturally spans
    ///   their full qualified text (`"t.foo"`/`"t:foo"`), so this row
    ///   needs no quirk for the base case at all, matching the baseline's
    ///   own `cbm_func_name_node_text` (plain `.utf8_text()`, no
    ///   dot-stripping) doing the identical thing. `function_definition`
    ///   (the ANONYMOUS function-literal form, `local foo = function()
    ///   end`) by contrast has NO `name` field whatsoever (confirmed:
    ///   `node-types.json`'s `function_definition` entry has no `name`
    ///   key at all) -- its name, when one exists, comes from walking
    ///   UP to the enclosing `assignment_statement`'s own `variable_list`,
    ///   exactly mirroring the baseline's own dedicated
    ///   `resolve_lua_func_name` walker
    ///   (`codebase-memory-mcp/internal/cbm/extract_defs.c`:207-232) --
    ///   this row deliberately keeps `function_definition` OUT of
    ///   `func_types` (unlike the baseline's flat array, which lists it
    ///   alongside `function_declaration` even though the two need
    ///   completely different name-resolution mechanisms) and instead
    ///   routes it entirely through
    ///   [`crate::languages::generic::lua_quirk`]'s own
    ///   `on_unmatched_node` dispatch, so the flat-array/`name_field`
    ///   mechanism this file's OTHER rows rely on is never asked to
    ///   handle a fieldless node it cannot.
    /// - `call_types` is `["function_call"]`, matching baseline exactly.
    ///   Its own `name` field can be `identifier` (plain call, needs no
    ///   quirk), `method_index_expression` (`w:draw()`, a two-field
    ///   `table`/`method` split the generic engine's single
    ///   `call_function_field` cannot reconstruct receiver info from on
    ///   its own), `function_call` (immediately-invoked, e.g.
    ///   `foo()()`), or `parenthesized_expression` -- the receiver-hint-
    ///   bearing `method_index_expression` case is claimed by
    ///   [`crate::languages::generic::lua_call_override`] (mirrors
    ///   [`Self::ruby`]'s identical `method`/`receiver`-field-split
    ///   posture); every other shape falls through to this row's own
    ///   ordinary `call_function_field = "name"` flat path unchanged
    ///   (`.utf8_text()` on a plain `identifier`/`function_call`/
    ///   `parenthesized_expression` name field already yields the
    ///   correct full callee text with no unwrapping needed). `arguments`
    ///   is a real, directly-`child_by_field_name`-reachable field on
    ///   `function_call` holding unfielded `expression` children, the
    ///   same shape [`crate::languages::generic::call_arg_texts`] already
    ///   handles generically for Rust/Go/TS -- no override needed for
    ///   `arg_texts` even where `lua_call_override` DOES claim the node
    ///   for receiver-hint purposes (it still calls the shared
    ///   `call_arg_texts` helper directly rather than reimplementing it).
    /// - `import_types` is empty, NOT baseline's `["function_call"]`
    ///   (`bash_import_types`'s identical "one node kind cannot service
    ///   two arrays at once" constraint applies here too -- `function_call`
    ///   is already this row's entire `call_types`, so listing it again
    ///   in `import_types` would be unreachable dead data, matching
    ///   [`Self::bash`]'s own identical finding/doc note above).
    ///   `require(...)`/`require(...)` detection is instead handled by
    ///   [`crate::languages::generic::lua_call_override`] itself
    ///   (mirrors [`Self::ruby`]'s `require`/`require_relative`-via-
    ///   call-override precedent exactly, and the baseline's own intent:
    ///   `internal/cbm/extract_imports.c`'s dedicated `parse_lua_imports`
    ///   -- Lua is one of the few languages the baseline does NOT route
    ///   through its generic `"function_call"`-node-type
    ///   `parse_generic_imports` path at all, precisely because a bare
    ///   generic scan would need the same "is this call's own name text
    ///   literally `require`" filter this crate's override applies
    ///   directly).
    pub const fn lua() -> Self {
        Self {
            name: "lua",
            func_types: &["function_declaration"],
            method_types: &["function_declaration"],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["function_call"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            // See this const's own doc comment: intentionally empty,
            // NOT `["function_call"]` (dead data double-listing a node
            // kind this row's own `call_types` array already claims
            // first) -- `require(...)` IMPORTS come from
            // `lua_call_override` instead.
            import_types: &[],
            // See this const's own doc comment: NOT baseline's
            // `"for_in_statement"` (does not exist in this grammar --
            // `"for_statement"` alone already covers both its numeric-
            // and generic-clause shapes).
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "repeat_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Elixir: this grammar has NO dedicated `def`/`defmodule`/`defp`/
    /// `defmacro`/`import`/`alias`/`use`/`require` node kind at all --
    /// every one of them parses as the SAME plain `call` node
    /// (`macro_name(args) [do_block]`), a finding independently confirmed
    /// against a real parse tree dump (`cargo run` against a scratch
    /// crate depending on `tree-sitter-elixir` 0.3.5 directly) AND already
    /// the load-bearing assumption behind the C baseline's own dedicated
    /// `extract_elixir_call`/`extract_elixir_func_def`/
    /// `emit_elixir_module_class` walkers
    /// (`codebase-memory-mcp/internal/cbm/extract_defs.c`:4359-4463) and
    /// `extract_scripting_callee`'s `CBM_LANG_ELIXIR` branch
    /// (`internal/cbm/extract_calls.c`:433-440) -- this row's own flat
    /// `func_types`/`class_types`/`call_types` arrays are consequently
    /// ALL `["call"]` (same "arrays exist purely as at-a-glance
    /// documentation, the quirk hook claims literally everything before
    /// the generic engine's own field-name-keyed fallback would ever run"
    /// posture as [`Self::c`]/[`Self::cpp`]/[`Self::kotlin`]), fully
    /// serviced by [`crate::languages::generic::elixir_quirk`] +
    /// [`crate::languages::generic::elixir_call_override`], which
    /// disambiguate ALL of def/defmodule/import-family/ordinary-call by
    /// reading the `call` node's own `target` field's text (`"def"`,
    /// `"defmodule"`, `"alias"`, ...) exactly the way the baseline's own
    /// `strcmp(macro, "def")`-style dispatch does.
    /// - `module_types` is empty: `defmodule`'s own Class-not-Module
    ///   symbol (matching the baseline's own `emit_elixir_module_class`,
    ///   whose `def.label = "Class"`, NOT `"Module"`) is pushed directly
    ///   by [`crate::languages::generic::elixir_quirk`], not this row's
    ///   generic `module_types` branch -- there is no separate
    ///   "namespace clause distinct from a type declaration" concept in
    ///   Elixir the way Go's `package_clause`/Rust's `mod_item` are.
    /// - A REAL, DELIBERATE improvement over the baseline's own
    ///   `extract_elixir_func_def`, found via this same real-parse-tree
    ///   verification (not guessed): a guarded `def bar(x) when x > 0 do
    ///   ... end`'s first (and only) `arguments` child is a
    ///   `binary_operator` node (`operator` field text literally
    ///   `"when"`, `left` = the `bar(x)` call, `right` = the guard
    ///   expression) -- NEITHER a bare `call` NOR a bare `identifier`,
    ///   the only two shapes the baseline's own
    ///   `extract_elixir_func_def` (:4371-4377) ever checks, so a guarded
    ///   `def` silently resolves a NULL name there and is dropped
    ///   entirely (never extracted as a Function def at all) -- guard
    ///   clauses are common, idiomatic Elixir, not an obscure edge case,
    ///   so [`crate::languages::generic::elixir_quirk`] unwraps exactly
    ///   one `binary_operator` layer (taking its `left` child) whenever
    ///   the guarded first-argument shape is seen, before applying the
    ///   baseline's own `call`/`identifier` check to what remains --
    ///   strictly additive (every def name the baseline's own check
    ///   already recognizes is recognized here identically; only the
    ///   guarded shape it drops is additionally recovered), same
    ///   "baseline has a gap, not a deliberate design choice, so filling
    ///   it is not the same as inventing scope creep" reasoning
    ///   [`Self::zig`]'s own `import_types` doc comment already applies
    ///   to Zig's `@import` IMPORTS.
    /// - A SECOND real, deliberate improvement, also found via the same
    ///   real-parse-tree verification: `internal/cbm/extract_imports.c`'s
    ///   own Elixir dispatch
    ///   (`parse_generic_imports(ctx, "call")`, :2792-2795) only ever
    ///   scans the DIRECT children of the file's root node (a
    ///   non-recursive `ts_tree_cursor_goto_first_child`/
    ///   `goto_next_sibling` sibling walk, `internal/cbm/
    ///   extract_imports.c`:943-961) -- for the overwhelming majority of
    ///   real Elixir files (everything wrapped in exactly one top-level
    ///   `defmodule ... do ... end`), the root's ONLY direct child is
    ///   that one `defmodule` call itself, which has no `path`/`source`/
    ///   `module`/`name` field (`try_generic_path_fields` fails) and so
    ///   falls through to `generic_import_from_text` -- which takes the
    ///   ENTIRE multi-line `defmodule` call's own text, strips only the
    ///   first space-separated word and a trailing `;`, and pushes
    ///   whatever multi-line garbage remains as a bogus "import path".
    ///   This is not a deliberate design choice to imitate (a
    ///   non-recursive direct-children-only scan is a real bug against
    ///   Elixir's own idiomatic one-`defmodule`-wraps-everything shape,
    ///   not a considered scope limit), so
    ///   [`crate::languages::generic::elixir_quirk`] instead recognizes
    ///   `alias`/`import`/`use`/`require` by their own `target` field
    ///   text WHEREVER they appear in the tree (the generic engine's own
    ///   recursive `walk`/`walk_children` visits every `call` node
    ///   regardless of nesting depth, so no special traversal is needed
    ///   to reach them even inside a `do_block`), pushing the aliased
    ///   module's own dotted `alias`-node text (`"Bar.Baz"`, `"GenServer"`,
    ///   `"Logger"`) as the import path -- the same underlying `alias`/
    ///   `import`/`use`/`require` semantics the baseline's own dispatch
    ///   comment ("Elixir: import/use/alias/require are call nodes")
    ///   already documents as its INTENT, just actually reachable.
    pub const fn elixir() -> Self {
        Self {
            name: "elixir",
            func_types: &["call"],
            method_types: &[],
            class_types: &["call"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call"],
            call_function_field: "UNUSED_SEE_ELIXIR_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_ELIXIR_CALL_OVERRIDE",
            import_types: &[],
            branch_types: &["binary_operator"],
            decorator_types: &[],
            name_field: "UNUSED_SEE_ELIXIR_QUIRK",
            body_field: "UNUSED_SEE_ELIXIR_QUIRK",
        }
    }

    /// CUDA: the baseline's own `lang_specs.c` table (`internal/cbm/
    /// lang_specs.c` :1875-1879, `CBM_LANG_CUDA`) reuses C++'s node-type
    /// arrays VERBATIM (`cpp_func_types`/`cpp_class_types`/
    /// `cpp_field_types`/`cpp_module_types`/`cpp_call_types`/
    /// `cpp_import_types`/`cpp_branch_types`) rather than declaring any
    /// CUDA-specific ones at all -- the baseline's own comment literally
    /// reads "reuses C++ node types". This row mirrors that exactly (same
    /// `..Self::cpp()` pattern [`Self::tsx`] already uses for an identical
    /// "this language's grammar is a strict syntactic superset of an
    /// existing row's, reuse it verbatim" situation) rather than
    /// hand-declaring a parallel copy of every C++ array a second time.
    /// Confirmed empirically, not just asserted from the baseline's own
    /// comment: the `tree-sitter-cuda` crate (`tree-sitter-grammars/
    /// tree-sitter-cuda`, the actual CUDA grammar this row binds -- NOT a
    /// relabeled tree-sitter-cpp) is itself a full superset of
    /// `tree-sitter-cpp`'s grammar (every C++ construct --
    /// `lambda_expression`/`template_declaration`/
    /// `co_await_expression`/`requires_clause`/... -- is present
    /// unchanged) plus exactly two CUDA-specific additions layered on
    /// top: the `<<<...>>>` kernel-launch-configuration clause (its own
    /// `kernel_call_syntax` node kind) and a family of CUDA qualifier
    /// keywords (`__global__`/`__device__`/`__host__`/`__constant__`/
    /// `__shared__`/`__managed__`/`__grid_constant__`/`__launch_bounds__`/
    /// ...). Verified node-kind-by-node-kind against this crate's own
    /// `src/node-types.json` AND a real parse tree (`cargo run` against a
    /// scratch crate depending on `tree-sitter-cuda` directly, dumping
    /// every node kind/field for a source exercising a `__global__`
    /// kernel definition, a `__device__` free function, a class with
    /// `__host__ __device__` methods, `#include`, and both a bare
    /// `helper(5)` call and TWO kernel-launch calls
    /// (`addKernel<<<1, n>>>(...)`/
    /// `addKernel<<<numBlocks, threadsPerBlock, 0, stream>>>(...)`)):
    /// `function_definition`'s `declarator`/`body`/`type` fields,
    /// `class_specifier`/`struct_specifier`'s `name`/`body` fields,
    /// `call_expression`'s `function`/`arguments` fields, and
    /// `preproc_include`'s `path` field are ALL byte-for-byte identical
    /// in shape to [`Self::cpp`]'s own grammar dependency -- meaning
    /// every one of [`crate::languages::generic::cpp_quirk`]/
    /// [`crate::languages::generic::cpp_call_override`]'s existing
    /// field-based reads (which fully claim every one of [`Self::cpp`]'s
    /// arrays already, see [`Self::cpp`]'s own doc comment) work
    /// unchanged for CUDA source with ZERO new quirk code. The one
    /// CUDA-specific addition, `kernel_call_syntax`, sits as an
    /// UNFIELDED child of `call_expression` wedged between the callee
    /// and `argument_list` (`addKernel<<<1, n>>>(d_a, ...)` parses as
    /// `call_expression { identifier "addKernel", kernel_call_syntax
    /// "<<<1, n>>>", argument_list "(d_a, ...)" }`) -- entirely invisible
    /// to `cpp_call_override`'s existing `function`/`arguments`
    /// field-keyed reads (confirmed directly in the real parse tree
    /// dump: both kernel-launch calls above still populate `function`
    /// with the bare `addKernel` identifier and `arguments` with the
    /// full, correct argument list, with the launch-configuration clause
    /// simply skipped over as extra unread sibling data). This crate
    /// therefore adds no CUDA-specific quirk logic at all -- see
    /// [`crate::languages::generic::parse_cuda`], which is `parse_cpp`
    /// with only the grammar entry point and `name` swapped.
    pub const fn cuda() -> Self {
        Self {
            name: "cuda",
            ..Self::cpp()
        }
    }

    /// D: node kinds verified directly against the `tree-sitter-d` 0.8.2
    /// crate's (`gdamore/tree-sitter-d`, crates.io `tree-sitter-d`) own
    /// `src/node-types.json` AND a real parse tree (`cargo run` against a
    /// scratch crate depending on `tree-sitter-d` directly, exercising a
    /// module declaration, two `import` statements (one plain, one with
    /// a selective `: map, filter` bind list), a base class with a field
    /// and a constructor and a method, a derived class with multi-base
    /// heritage (`class Dog : Animal, Serializable`) whose own
    /// constructor calls `super(name)`, two free functions where one
    /// calls the other, an `interface`, a `struct` with two fields, and a
    /// `main` exercising `new Dog(...)`, a member call (`d.speak()`), and
    /// a bare paren-less-looking call (`helper()` is still parenthesized
    /// in D, unlike PowerShell -- there is no D syntax this crate found
    /// that omits the parens)) -- NOT copied blindly from the baseline's
    /// `internal/cbm/lang_specs.c` `d_*` arrays, one of which is
    /// confirmed stale/wrong against this grammar version (see below).
    /// G2.2b scope note: this grammar exposes EVERY node kind below with
    /// `"fields": {}` in its own `node-types.json` -- there are NO
    /// tree-sitter fields anywhere in this grammar at all (not even on
    /// `call_expression`/`import_declaration`/`variable_declaration`),
    /// confirmed for every kind this row names. This matches -- and
    /// directly explains -- why the baseline's own `extract_defs.c` D
    /// handling never once calls `ts_node_child_by_field_name` for this
    /// language (its `CBM_LANG_DLANG` branches at :3548/:3660/:2448 all
    /// use `cbm_find_child_by_kind`, a by-KIND child scan, exclusively):
    /// the baseline's own choice was not a stylistic preference, it was
    /// the only option this exact grammar shape ever allowed. Every one
    /// of this row's arrays is consequently fully claimed by
    /// [`crate::languages::generic::d_quirk`]/
    /// [`crate::languages::generic::d_call_override`] rather than the
    /// generic engine's own field-name-keyed fallback paths -- same
    /// "every array fully claimed by quirk, generic branch never actually
    /// reached" posture as [`Self::c`]/[`Self::cpp`]/[`Self::php`], for
    /// the analogous reason (no usable fields at all, rather than C/C++'s
    /// "fields exist but need declarator-unwrapping").
    /// - `func_types`/`method_types` are `["function_declaration",
    ///   "constructor", "destructor"]`, matching the baseline's own
    ///   `d_func_types` exactly by node-KIND name -- but `constructor`/
    ///   `destructor` are UNNAMED in this grammar (D's `this(...)`/
    ///   `~this()` special-method syntax carries no identifier child at
    ///   all, confirmed in the real parse tree: `this(string name) {...}`
    ///   is `constructor { this, parameters, function_body }`, the
    ///   keyword token `this` itself, not an identifier) --
    ///   [`crate::languages::generic::d_quirk`] records both as a
    ///   nameless [`crate::parsers::SymbolKind::Method`], the identical
    ///   "no name to recover, still walk the body" posture
    ///   [`crate::languages::generic::kotlin_function_like`]'s
    ///   `secondary_constructor` case (also unnamed in ITS grammar) and
    ///   [`crate::languages::generic::c_quirk`]'s anonymous-struct case
    ///   already establish elsewhere in this crate. One node kind is
    ///   shared by both `func_types` and `method_types` (matches Rust's
    ///   `function_item`/Ruby's `method` convention) -- the generic
    ///   engine's own nesting-based Function-vs-Method fallback would
    ///   apply here too, EXCEPT this row's func/method arrays are never
    ///   actually reached by that generic fallback at all (the quirk
    ///   claims every one directly, and does the identical nesting check
    ///   itself by threading `enclosing` through).
    /// - `class_types` is `["class_declaration", "struct_declaration",
    ///   "union_declaration"]`, NOT the baseline's full `d_class_types`
    ///   (which additionally lists `"interface_declaration"`,
    ///   `"module_declaration"`, and `"module_def"` all folded into ONE
    ///   flat array): `interface_declaration` is instead
    ///   [`Self::interface_types`] below (this crate's own [`LangSpec`]
    ///   has a real interface/class distinction the baseline's flatter
    ///   `CBMLangSpec` does not, same rationale as [`Self::rust`]'s
    ///   `trait_item` split), and `module_declaration`/`module_def` are
    ///   omitted entirely -- `module_declaration` (`module myapp.widgets;`)
    ///   is a genuine module-name-bearing statement handled directly by
    ///   [`crate::languages::generic::d_quirk`]'s own dedicated arm (NOT
    ///   folded into this array's generic class-shape fallback, which
    ///   would wrongly classify a module statement as a
    ///   [`crate::parsers::SymbolKind::Class`]), while `module_def` is
    ///   this grammar's own root-of-the-whole-module WRAPPER node (every
    ///   top-level declaration in a `.d` file with an explicit `module`
    ///   statement nests one level under it, confirmed in the real parse
    ///   tree: `source_file > module_def > {module_declaration,
    ///   import_declaration, class_declaration, ...}`) -- not a
    ///   class-shaped declaration at all, so including it in ANY
    ///   array here would be wrong; `walk_children`'s ordinary recursion
    ///   already descends through it transparently (`d_quirk` returns
    ///   `false`/unclaimed for it, same as every other pure-wrapper node
    ///   kind in this crate with no dedicated arm).
    /// - `field_types` is `["variable_declaration"]`, matching the
    ///   baseline's own `d_field_types` exactly -- this kind's own name
    ///   lives on a `declarator` unfielded child wrapping the bare
    ///   `identifier` (confirmed in the real parse tree: `int x;` inside
    ///   a `struct_declaration`'s `aggregate_body` is `variable_declaration
    ///   { type, declarator { identifier "x" }, ; }`), needing
    ///   `d_quirk`'s own by-kind child scan same as everything else here.
    /// - `module_types` is empty, NOT the baseline's own (dead-elsewhere)
    ///   `d_module_types = {"source_file"}` -- identical rationale to
    ///   [`Self::solidity`]/[`Self::gdscript`]/[`Self::dart`]'s own
    ///   identical choice (this crate's generic walker's `module_types`
    ///   field is LIVE, unlike the baseline's own dead
    ///   `module_node_types` struct field with zero consumers there, so
    ///   porting `"source_file"` verbatim would invent a
    ///   [`crate::parsers::SymbolKind::Module`] symbol named after
    ///   whatever the file's first top-level declaration happens to be,
    ///   not a real module name) -- D's real module name (`module
    ///   myapp.widgets;`) is instead emitted directly by `d_quirk`'s own
    ///   `module_declaration` arm (see `class_types`'s own note above),
    ///   reading the dotted name off `module_fqn` rather than this row's
    ///   generic `module_types` fallback at all.
    /// - `call_types` is `["call_expression"]` ONLY, NOT the baseline's
    ///   `d_call_types = {"call_expression", "function_call_expression",
    ///   "new_expression"}`: `function_call_expression` does not exist
    ///   anywhere in this grammar's full 373-entry node-type vocabulary
    ///   at all (confirmed by an exhaustive dump, not just a targeted
    ///   lookup) -- a stale/wrong baseline array entry, the same "the
    ///   baseline's own arrays are sometimes wrong" class of finding
    ///   [`Self::dart`]/[`Self::groovy`]'s own doc comments already
    ///   record for other languages. `new_expression` is real but is
    ///   NEVER itself a top-level call-shaped node needing its own
    ///   `call_types` entry -- a real parse tree shows `new Dog("Rex")`
    ///   is a `call_expression` whose own FIRST child is a nested
    ///   `new_expression` (itself wrapping `new`/`type` -- `type
    ///   "Dog"`/an `aggregate_body` for anonymous classes/a `base_class`
    ///   for anonymous-class heritage), with `named_arguments` as
    ///   `call_expression`'s own second child holding the constructor
    ///   arguments -- so `call_expression` alone already reaches every
    ///   `new`-expression callsite via the ordinary call-shaped path
    ///   (with the `new_expression` sub-node's own text, e.g. `"new
    ///   Dog"`, captured as the literal callee text, matching how
    ///   [`Self::solidity`]'s identical `new Helper(a)` finding and Go's
    ///   `NewXxx(...)` convention are both handled the same way
    ///   elsewhere in this file -- captured, not stripped).
    /// - `call_function_field`/`call_arguments_field` are placeholders,
    ///   never actually consulted: `call_expression` has NO fields at
    ///   all (confirmed above) -- its callee is ALWAYS the node's own
    ///   FIRST child (an `identifier` for a plain call, `super` for a
    ///   super-constructor call, a `type` node wrapping a
    ///   `.`-joined `identifier`/`identifier` pair for a receiver-
    ///   qualified member call (`d.speak()` parses as `call_expression {
    ///   type { identifier "d", ".", identifier "speak" },
    ///   named_arguments }` -- an odd grammar choice, an ordinary
    ///   MEMBER-ACCESS expression reusing the same `type` node kind a
    ///   type annotation would use, verified directly in the real parse
    ///   tree, not assumed from the name alone), or a `new_expression`
    ///   for a constructor call), and its argument list is
    ///   `named_arguments` (NOT a bare `arguments` field the way the
    ///   baseline's own naming convention might suggest) -- every one of
    ///   `call_types` is fully claimed by
    ///   [`crate::languages::generic::d_call_override`] before either
    ///   fallback would ever run.
    /// - `import_types` is `["import_declaration"]`, matching the
    ///   baseline's own `d_import_types` array's first entry (the
    ///   baseline's second entry, a bare `"import"` KEYWORD-token kind,
    ///   is dropped for the same "an unnamed keyword token never matches
    ///   a named-node walk" reason [`Self::gdscript`]'s own doc comment
    ///   already gives for an analogous baseline entry) -- the module
    ///   path lives on an `imported` unfielded child wrapping
    ///   `module_fqn` (plain `import std.stdio;`) or, for a selective
    ///   import (`import std.algorithm : map, filter;`), the SAME
    ///   `imported` > `module_fqn` shape plus sibling `import_bind`
    ///   children this row's quirk does not additionally chase (matches
    ///   the baseline's own `parse_dlang_imports`, which only ever reads
    ///   the one `module_fqn` descendant per `import_declaration` and
    ///   never visits `import_bind` at all).
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "foreach_statement", "while_statement", "do_statement",
    ///   "switch_statement", "try_statement", "catch_statement"]`,
    ///   matching the baseline's own `d_branch_types` array exactly by
    ///   node-kind name (every one of these eight kinds is confirmed
    ///   present in this grammar's full node-type vocabulary dump).
    ///   D has no `&&`/`||` short-circuit-operator branch_types entries
    ///   the way e.g. [`Self::go`]/[`Self::rust`] do for their own
    ///   `&&`/`||` token kinds -- this grammar aliases both to the
    ///   `logical_expression` rule (its own `operator` field
    ///   distinguishing `&&` from `||` textually, not by node kind), and
    ///   since [`crate::complexity::NodeKindTable`]'s branch-counting
    ///   convention keys purely off KIND name (not field-text
    ///   inspection), adding `"logical_expression"` here would
    ///   over-count every plain boolean-or-comparison expression that
    ///   uses neither operator too -- left uncounted for that reason
    ///   (a conservative under-count, not a guess), same "left empty
    ///   rather than wrong" posture [`Self::rust`]'s own `field_types`
    ///   doc comment already established for a different field.
    pub const fn d() -> Self {
        Self {
            name: "d",
            func_types: &["function_declaration", "constructor", "destructor"],
            method_types: &["function_declaration", "constructor", "destructor"],
            class_types: &[
                "class_declaration",
                "struct_declaration",
                "union_declaration",
            ],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &[],
            field_types: &["variable_declaration"],
            // See this const's own doc comment: intentionally empty, NOT
            // `["source_file"]` -- D's real module name is emitted
            // directly by `d_quirk`'s own `module_declaration` arm.
            module_types: &[],
            // See this const's own doc comment: `call_expression` alone,
            // NOT `["call_expression", "function_call_expression",
            // "new_expression"]`.
            call_types: &["call_expression"],
            call_function_field: "UNUSED_SEE_D_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_D_CALL_OVERRIDE",
            import_types: &["import_declaration"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "foreach_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
                "try_statement",
                "catch_statement",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::cpp()`).
            name_field: "UNUSED_SEE_D_QUIRK",
            body_field: "UNUSED_SEE_D_QUIRK",
        }
    }

    /// PowerShell: node kinds verified directly against the
    /// `tree-sitter-powershell` 0.26.4 crate's (`airbus-cert/
    /// tree-sitter-powershell`, crates.io `tree-sitter-powershell`) own
    /// `src/node-types.json` AND a real parse tree (`cargo run` against a
    /// scratch crate depending on `tree-sitter-powershell` directly,
    /// exercising two `using` directives (`using namespace ...`/`using
    /// module ...`), a base class with a typed property and a
    /// constructor and a method, a derived class (`class Dog : Animal`)
    /// whose own method calls a free function, two `function` statements
    /// (one calling the other with bare space-separated arguments, no
    /// parens), an `if`/`else` with a `throw`, and script-level
    /// `New-Object`/member-call/bare-command call sites) -- NOT copied
    /// blindly from the baseline's `internal/cbm/lang_specs.c`
    /// `powershell_*` arrays, one of which is confirmed dead/wrong
    /// against this grammar version (see `branch_types`' own note below).
    /// - `func_types`/`method_types` are both `["function_statement"]`,
    ///   matching the baseline's own `powershell_func_types` exactly by
    ///   node-kind name -- but this kind has NO fields at all (confirmed:
    ///   `{"type":"function_statement","fields":{}}`), so its name is a
    ///   `function_name` unfielded child (mirrors the baseline's own
    ///   `cbm_resolve_func_name`'s dedicated `CBM_LANG_POWERSHELL` branch
    ///   at `internal/cbm/extract_defs.c`:702-707 exactly: "the name is a
    ///   `function_name` child node") and its body is TWO unfielded
    ///   wrapper levels deep (`script_block > script_block_body >
    ///   statement_list`, confirmed in the real parse tree), not a
    ///   single `"body"` field -- [`crate::languages::generic::ps_quirk`]
    ///   fully claims this kind rather than the generic engine's own
    ///   `name_field`/`body_field` mechanism, same "every array claimed
    ///   by quirk" posture as [`Self::c`]/[`Self::cpp`]/[`Self::kotlin`].
    ///   PowerShell has no free-function-vs-method node-kind split at
    ///   the top level the way e.g. Go's `method_declaration` does --
    ///   `class_method_definition` (this row's OWN method-inside-a-class
    ///   node kind, a distinct kind from `function_statement`, see
    ///   `class_types`' own note below) is not one of `func_types`
    ///   /`method_types` at all, it is handled by the SAME `d_quirk`-
    ///   style class-body walk `class_statement`'s own arm performs, not
    ///   the generic engine's func/method branch.
    /// - `class_types` is `["class_statement"]`; `enum_types` is
    ///   `["enum_statement"]` -- both match the baseline's own
    ///   `powershell_class_types = {"class_statement", "enum_statement",
    ///   "type_spec"}` array MINUS its third entry, `type_spec`: a real
    ///   parse tree shows `type_spec` is an ordinary TYPE-ANNOTATION
    ///   node (`[string]$Name`'s `[string]` bracket-type-literal wraps a
    ///   `type_spec > type_name > type_identifier` chain), never itself a
    ///   class/struct/product-type DECLARATION the way this row's
    ///   `class_types` array is meant to classify -- a `Class 16`-style
    ///   stale baseline entry (a node-KIND name that is real in this
    ///   grammar but classifies something else entirely), the same
    ///   "trust the grammar, not the array literally" finding
    ///   [`Self::solidity`]'s own `alias_types` doc comment already
    ///   records for an analogous baseline mistake. Neither
    ///   `class_statement` nor `enum_statement` has a `name` field
    ///   either (confirmed: `"fields":{}`  for both) -- the name is the
    ///   FIRST `simple_name` child (mirrors the baseline's own
    ///   dedicated `CBM_LANG_POWERSHELL` name-resolution branches at
    ///   `internal/cbm/extract_defs.c`:3553-3555/:3666-3668 exactly:
    ///   "the name is the FIRST `simple_name` child") -- both fully
    ///   claimed by `ps_quirk` for that reason, PLUS `class_statement`'s
    ///   own heritage (`class Dog : Animal`, confirmed in the real parse
    ///   tree to be `class_statement { simple_name "Dog", :, simple_name
    ///   "Animal", {, ... }` -- every `simple_name` child AFTER the
    ///   literal `:` token is a base name) mirroring the baseline's own
    ///   dedicated `extract_base_classes` PowerShell walker
    ///   (`internal/cbm/extract_defs.c`:2477-2510, the exact walker this
    ///   worker's own brief named) node-for-node: "collect every
    ///   `simple_name` that appears after the first `:` token, stop at
    ///   `{`".
    /// - `field_types` is empty, NOT the baseline's own
    ///   `powershell_field_types` (this baseline table has no dedicated
    ///   PowerShell field-types entry at all -- its own `CBMLangSpec`
    ///   row literally passes `empty_types` in that slot, confirmed at
    ///   `internal/cbm/lang_specs.c`:2050): `class_property_definition`
    ///   (`[string]$Name`, a typed class-body property declaration) DOES
    ///   exist as its own dedicated node kind in this grammar, unlike the
    ///   baseline's own choice to have nothing there at all, but this
    ///   row deliberately matches the baseline's real (absent) depth
    ///   rather than inventing a field-DEFINES edge the baseline itself
    ///   never emits for this language -- the identical "match the
    ///   baseline's actual behavior, not an idealized one" posture
    ///   [`Self::ruby`]'s own doc comment already establishes for a
    ///   different language's analogous gap. Left for a future G3 pass
    ///   if ever wanted, not silently guessed at.
    /// - `call_types` is `["command", "invokation_expression"]`; ONLY
    ///   `command` is actually claimed by the quirk override below, but
    ///   `invokation_expression` still needs to be a real member of this
    ///   array (unlike e.g. [`Self::d`]'s dropped dead `"import"`
    ///   keyword-token entry) because it IS a real, walk-reachable NAMED
    ///   node kind in this grammar (`$d.Speak()`/`New-Object` chained
    ///   member access-with-call all parse as `invokation_expression`,
    ///   confirmed in the real parse tree: `$d.Speak()` is
    ///   `invokation_expression { variable "$d", ., member_name "Speak"
    ///   {simple_name}, argument_list }`) that the generic walker's OWN
    ///   `spec.call_types.contains(&kind)` gate at the top of its
    ///   call-shaped branch must recognize as call-shaped at all before
    ///   [`Quirks::call_override`] is ever consulted for it (unlike a
    ///   dead keyword-token entry, which can never reach that gate
    ///   regardless of whether it is listed) -- both kinds are fully
    ///   claimed by [`crate::languages::generic::ps_call_override`]
    ///   rather than the generic engine's own `call_function_field`-keyed
    ///   fallback, since neither has a single flat callee field
    ///   (`command`'s `command_name` field IS real and directly usable,
    ///   confirmed: `{"type":"command","fields":{"command_name":
    ///   {"required":true,...}}}` -- an improvement on the baseline's own
    ///   `extract_powershell_callee`, which manually scans
    ///   `ts_node_named_child_count` for a `command_name` child rather
    ///   than reading a field, presumably because an OLDER grammar
    ///   revision the baseline vendored lacked this field -- but
    ///   `command`'s own ARGUMENTS are bare space-separated
    ///   `command_elements` children, not a parenthesized `arguments`
    ///   field this row's flat `call_arguments_field` mechanism could
    ///   read directly, and `invokation_expression` has no fields
    ///   at all). The baseline's own `powershell_call_types` array's
    ///   THIRD implied member -- there is no third member; the baseline
    ///   itself only lists `{"invokation_expression", "command", NULL}`,
    ///   so this row's two-entry array already matches it exactly, unlike
    ///   [`Self::d`]'s array-shrinking correction above.
    /// - `call_function_field`/`call_arguments_field` are placeholders,
    ///   never actually consulted -- every `call_types` entry is claimed
    ///   by `ps_call_override` before either fallback would run (see
    ///   `call_types`'s own note above for why neither kind has a usable
    ///   single flat field pair).
    /// - `import_types` is empty, NOT the baseline's own
    ///   `powershell_import_types = {"using_statement"}`: this node kind
    ///   -- `using_statement` -- does not exist ANYWHERE in this
    ///   grammar's full 322-entry node-type vocabulary (confirmed by an
    ///   exhaustive dump), a stale baseline entry for the identical
    ///   "grammar version drift" reason [`Self::d`]'s own
    ///   `function_call_expression` finding already documents for a
    ///   different language. `using namespace System.Collections.Generic`
    ///   / `using module MyModule` are, per this crate's own real parse
    ///   tree, ORDINARY `command` nodes whose `command_name` field's text
    ///   is literally `"using"` -- mirrors the baseline's own
    ///   `parse_powershell_imports` (`internal/cbm/extract_imports.c`
    ///   :1587-1618) exactly: it does not gate on any dedicated
    ///   import-statement node kind either, it walks EVERY `command`
    ///   node, checks whether its `command_name` text is `"using"`, and
    ///   if so scans for the LAST `generic_token` descendant (skipping
    ///   the `module`/`namespace`/`assembly` keyword tokens themselves)
    ///   as the imported module/namespace/assembly path -- confirmed in
    ///   the real parse tree for both directives (`using namespace
    ///   System.Collections.Generic` -> `command_elements` holding
    ///   `generic_token "namespace"`, `generic_token
    ///   "System.Collections.Generic"` -- the LAST one, correctly
    ///   skipping the keyword). [`crate::languages::generic::ps_call_override`]'s
    ///   own `command` handling implements this same walk directly
    ///   (rather than a dedicated `on_unmatched_node` arm, since `command`
    ///   is already fully claimed there for its ordinary call-recording
    ///   job too) -- this row's `import_types` array itself stays empty
    ///   because there genuinely is no distinct import-STATEMENT node
    ///   kind to list, matching [`Self::ruby`]'s own identical
    ///   `require`-is-an-ordinary-call-not-a-statement `import_types`
    ///   emptiness for an analogous reason.
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "foreach_statement", "while_statement", "do_statement",
    ///   "switch_statement", "try_statement", "catch_clause"]`, matching
    ///   the baseline's own `powershell_branch_types` array's first seven
    ///   entries by node-kind name exactly (every one confirmed present
    ///   in this grammar's node-type vocabulary) -- but DROPS the
    ///   baseline's own final listed entry from its sibling
    ///   `powershell_throw_types = {"throw", NULL}` array (a SEPARATE
    ///   baseline array from `powershell_branch_types`, not itself one of
    ///   the eight branch entries copied above; noted here regardless
    ///   since this crate's own [`LangSpec`] folds throw-signal detection
    ///   into `branch_types` for languages that have one, e.g.
    ///   [`Self::rust`]'s `"match_arm"`/[`Self::typescript`]'s
    ///   `"catch_clause"`): a real parse tree confirms this grammar's
    ///   `"throw"` node kind is UNNAMED (confirmed:
    ///   `{"type":"throw","named":false}`, a bare keyword token, not a
    ///   statement-shaped rule with children of its own) -- the actual
    ///   PowerShell throw STATEMENT is `flow_control_statement`
    ///   (verified: `throw "bad"` parses as `flow_control_statement {
    ///   throw, pipeline }`, the SAME wrapper kind `return`/`break`/
    ///   `continue`/`exit` all share, distinguished only by which literal
    ///   keyword token is its first child) -- since
    ///   [`crate::complexity::NodeKindTable`]'s branch-counting
    ///   convention keys purely off KIND name, and `flow_control_statement`
    ///   is not itself a decision-point/branch construct the way an
    ///   `if`/`for`/`catch` is (a bare `throw`/`return` does not branch
    ///   control flow, it terminates it unconditionally at that point --
    ///   no existing language row in this file counts `return_statement`/
    ///   `break_statement` as a branch_types entry either), this crate
    ///   correctly omits both the dead unnamed `"throw"` keyword token
    ///   AND declines to add `"flow_control_statement"` as a new branch
    ///   kind never counted for any other language -- same "an unnamed
    ///   keyword token can never match a named-node walk regardless of
    ///   whether it is listed" finding [`Self::gdscript`]'s own doc
    ///   comment already gives for an analogous baseline array entry
    ///   (GDScript's bare `"extends"`), just for a genuinely dead
    ///   SIBLING array here (`powershell_throw_types`) rather than a
    ///   within-array entry.
    pub const fn powershell() -> Self {
        Self {
            name: "powershell",
            func_types: &["function_statement"],
            method_types: &["function_statement"],
            class_types: &["class_statement"],
            interface_types: &[],
            enum_types: &["enum_statement"],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["command", "invokation_expression"],
            call_function_field: "UNUSED_SEE_PS_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_PS_CALL_OVERRIDE",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "for_statement",
                "foreach_statement",
                "while_statement",
                "do_statement",
                "switch_statement",
                "try_statement",
                "catch_clause",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::cpp()`).
            name_field: "UNUSED_SEE_PS_QUIRK",
            body_field: "UNUSED_SEE_PS_QUIRK",
        }
    }

    /// F#: node kinds verified directly against the `tree-sitter-fsharp`
    /// 0.3.1 crate's own `fsharp/src/node-types.json` (the `ionide`-org
    /// grammar, the same actively maintained lineage the language's own
    /// tooling org publishes) -- NOT copied blindly from the baseline's
    /// `internal/cbm/lang_specs.c` `fsharp_*` arrays, several of which
    /// name node kinds this grammar version never generates at all:
    /// - `func_types`/`method_types` are `["function_or_value_defn"]`,
    ///   NOT baseline's `{"function_declaration", "value_declaration",
    ///   "member_defn", "additional_constr_defn"}`: none of those four
    ///   literal strings exists as a node kind in this grammar. A real
    ///   `let foo x y = ...` parses as `function_or_value_defn` wrapping
    ///   a positional `function_declaration_left` child (the signature)
    ///   -- and crucially, `function_declaration_left` itself has NO
    ///   `name` field at all (verified: `{"type":
    ///   "function_declaration_left","fields":{}}`); its own name is an
    ///   unfielded `identifier`/`op_identifier` direct child, and its
    ///   sibling `body` field lives one level UP on the *outer*
    ///   `function_or_value_defn` node -- the identical "signature node
    ///   and body field live on two different nodes" shape
    ///   [`Self::dart`]'s own doc comment already found for Dart's
    ///   `function_signature`/`function_declaration` split, so this row
    ///   hits the same class of bug the C baseline transcription would
    ///   silently reproduce. `function_or_value_defn` is consequently
    ///   fully claimed by [`crate::languages::generic::fsharp_quirk`]
    ///   (reads the inner node's own name, walks the outer node's own
    ///   `body` field with `fn_scope` correctly set) rather than the
    ///   generic engine's flat `name_field`/`body_field` mechanism --
    ///   `member_defn`/`additional_constr_defn` are dropped entirely:
    ///   both exist as real node kinds in this grammar, but represent
    ///   class-member/secondary-constructor syntax nested inside a type
    ///   definition's own body block, a deeper OOP-in-F# tier the
    ///   baseline's own dedicated walkers do not reach either (F#'s own
    ///   `extract_base_classes` case only ever reads `class_inherits_decl`,
    ///   never walks `member_defn` bodies for nested methods) -- matching
    ///   the "baseline's real depth, not an idealized one" instruction.
    /// - `class_types` is `["anon_type_defn", "record_type_defn",
    ///   "union_type_defn", "enum_type_defn", "type_abbrev_defn"]`, NOT
    ///   baseline's `{"type_definition", "exception_definition"}`:
    ///   `type_definition` IS a real node kind here, but it is a thin
    ///   wrapper with EMPTY fields whose actual payload is one of these
    ///   FIVE `_type_defn_body` variant kinds as a positional child
    ///   (verified: `_type_defn_body` is a hidden/inlined supertype rule,
    ///   so the grammar never emits a node literally named
    ///   `_type_defn_body` -- only its own concrete alternatives ever
    ///   appear in a real parse tree) -- pointing `class_types` at the
    ///   never-instantiated wrapper would silently match nothing.
    ///   `anon_type_defn` (an F# class/struct/interface with a primary
    ///   constructor, e.g. `type Animal(name: string) = inherit
    ///   Base(name) member this.Speak() = ...` -- OOP-flavored F#, the
    ///   language's actual `class`-equivalent construct) was caught only
    ///   by dumping a real parse tree for exactly this shape, NOT by
    ///   reading `node-types.json` alone: a first pass over the JSON
    ///   listed `record_type_defn`/`union_type_defn`/`enum_type_defn`/
    ///   `type_abbrev_defn` as the only four `_type_defn_body`
    ///   alternatives that "looked class-shaped", but a real parse of
    ///   the `inherit`+`member` idiom above resolved to `anon_type_defn`
    ///   instead -- omitting it would make every F# class with a primary
    ///   constructor (this crate's closest analog to Rust `impl`/TS
    ///   `class`) invisible to this engine, not just imprecisely
    ///   classified, exactly the class of bug this wave's brief warned
    ///   to check for. `anon_type_defn`'s own `class_inherits_decl` (a
    ///   `block`-fielded child, confirmed via the same real parse) is
    ///   this row's INHERITS source -- see
    ///   [`crate::languages::generic::fsharp_inherits_base_name`].
    ///   Every one of these five listed kinds carries its own
    ///   `type_name` node as a positional child (NOT a field --
    ///   `type_name` here is the grammar's own concrete NODE KIND,
    ///   confusingly reused as that same node's OWN field name for its
    ///   inner identifier), so `name_field` is a placeholder never
    ///   consulted for any of them --
    ///   [`crate::languages::generic::fsharp_quirk`] finds the `type_name`
    ///   child directly, then reads ITS OWN real `type_name` field (this
    ///   grammar reuses the literal string `"type_name"` as both a node
    ///   kind AND that same node kind's own field name, verified against
    ///   the grammar source: `type_name: ($) => prec(2, seq(...,
    ///   field("type_name", $.long_identifier), ... ))`) for the actual
    ///   written name text. `type_abbrev_defn` itself has a real,
    ///   GENUINE grammar ambiguity discovered only by dumping real parse
    ///   trees for several different right-hand sides, not visible from
    ///   `node-types.json` alone: `type Handler = int -> string` (a
    ///   function-type RHS) and `type IntList = list<int>` (a
    ///   generic-type RHS) both correctly parse as `type_abbrev_defn`,
    ///   but `type Meters = float` (a BARE single-identifier RHS) instead
    ///   parses as `union_type_defn` with exactly one `union_type_case`
    ///   named `"float"` -- i.e. this grammar cannot syntactically
    ///   distinguish "a type alias to a bare type name" from "a union
    ///   with a single, zero-field, confusingly-lowercase-named case" at
    ///   the RHS-is-one-identifier shape, and resolves the ambiguity by
    ///   always preferring the union interpretation. This means a simple
    ///   `type X = Y` (no further RHS structure) surfaces through this
    ///   engine as a `SymbolKind::Class` (from the `union_type_defn` arm
    ///   below), not `SymbolKind::TypeAlias`, purely as an artifact of
    ///   this grammar's own real, unavoidable ambiguity --
    ///   `type_abbrev_defn` is consequently NOT dead data (it fires
    ///   correctly for every other RHS shape checked), so it stays in
    ///   this array; this finding is recorded here only so a future
    ///   maintainer does not mistake the occasional
    ///   `SymbolKind::Class`-tagged simple type alias for a bug in
    ///   [`crate::languages::generic::fsharp_quirk`] itself.
    ///   `exception_definition` IS dropped despite existing as a real
    ///   node kind with a working `exception_name` field (verified) --
    ///   deliberately, matching this row's "no idealized depth beyond
    ///   the baseline" posture: the baseline's own `fsharp_class_types`
    ///   array lists it, but nothing else in this crate's design gives
    ///   an `exception ... of ...` declaration a natural
    ///   Class/Struct/Enum/TypeAlias-shaped home the way the five listed
    ///   kinds do -- left for a future G3 pass if ever wanted, same
    ///   "left out rather than force-fit" posture as [`Self::dart`]'s own
    ///   dropped `lambda_expression`.
    /// - `module_types` is `["named_module", "namespace"]`, NOT
    ///   baseline's `{"file"}`: `file` IS this grammar's real root node
    ///   kind, but (mirrors [`Self::solidity`]'s/[`Self::dart`]'s own
    ///   "porting the baseline's dead `module_node_types` column would
    ///   invent behavior, not reproduce it" finding) the baseline's own
    ///   `module_node_types` struct column has zero consumers in that
    ///   codebase, while this crate's generic walker DOES actively read
    ///   `module_types` -- and `file`'s own first named child is
    ///   arbitrarily whichever top-level declaration happens to come
    ///   first, not a module name. `named_module`/`namespace` (an
    ///   explicit `module Foo = ...`/`namespace Foo.Bar` declaration)
    ///   both carry a real working `name` field (a `long_identifier`,
    ///   unfielded dot-joined `identifier` children -- read via
    ///   [`crate::languages::generic::fsharp_long_identifier_text`], same
    ///   join-with-separator pattern as
    ///   [`crate::languages::generic::objc_join_selector_parts`]) --
    ///   `module_defn` (a NESTED, non-top-level `module Foo = ...` inside
    ///   another module) is deliberately excluded: it has no `name` FIELD
    ///   at all (verified: `{"type":"module_defn","fields":{"block":
    ///   {...}},"children":{"types":["access_modifier","attributes",
    ///   "identifier"]}}` -- an unfielded positional `identifier` child,
    ///   same "no name field, only a positional child" shape as
    ///   `function_declaration_left` above), and this crate's
    ///   `module_types` generic branch (unlike `class_types`) has no
    ///   quirk seam at all in [`crate::languages::generic::walk`] (its
    ///   `first_named_child_text` call is unconditional) -- rather than
    ///   add one for this single, rare nested-module case, it is left
    ///   unrecognized (falls through to plain recursion via the walker's
    ///   own catch-all, so nothing inside it is lost, just not itself
    ///   recorded as a Module symbol) -- G3's job if ever wanted.
    ///   Verified-safe subtlety: `namespace`'s own direct-child `"namespace"`
    ///   KEYWORD TOKEN (unnamed, `is_named() == false`) shares its exact
    ///   `.kind()` string with the NAMED `namespace` declaration node
    ///   itself (verified via a real parse tree dump) -- when
    ///   [`crate::languages::generic::walk`] is later called on that bare
    ///   keyword leaf (as an ordinary recursion target, since
    ///   `walk_children` visits every child, named or not), its own
    ///   `module_types.contains(&"namespace")` check matches too, but
    ///   [`crate::languages::generic::first_named_child_text`] on a
    ///   zero-children leaf loop-body-never-executes to `None`
    ///   immediately, so no spurious empty-name Module symbol is ever
    ///   pushed -- confirmed harmless by directly tracing this exact
    ///   code path, not merely assumed safe.
    /// - `call_types` is `["application_expression"]` only, NOT
    ///   baseline's `{"application_expression", "dot_expression"}`:
    ///   ground-truthed directly against
    ///   `internal/cbm/extract_calls.c`'s own dedicated
    ///   `extract_fsharp_callee` (:514-523), which gates on
    ///   `nk == "application_expression"` exclusively and returns NULL
    ///   for anything else -- `dot_expression` is never itself
    ///   independently resolved to a callee anywhere in that file (it
    ///   only ever appears as an application's INNER child, e.g.
    ///   `x.Method` as the callee of `x.Method arg`), so baseline's own
    ///   real behavior for a bare `dot_expression` "call" is "produce no
    ///   CALLS edge at all" -- copying it into this array would add a
    ///   node kind whose generic-engine dispatch could only ever find no
    ///   matching `call_function_field` either (dead data, not a
    ///   deliberate signal), so it is dropped rather than ported as dead
    ///   weight. `application_expression` itself needs
    ///   [`crate::languages::generic::fsharp_call_override`] rather than
    ///   the generic engine's own single-field reconstruction: this
    ///   grammar's curried-application shape means `f x y` parses as
    ///   NESTED `application_expression(application_expression(f, x),
    ///   y)` nodes (verified against the grammar's own
    ///   `_low_prec_app: ($) => prec.left(PREC.APP_EXPR, seq($._expression,
    ///   $._expression))` rule -- two expression slots per node, chained
    ///   left-associatively, never a flat N-ary node), and
    ///   `extract_fsharp_callee` itself ONLY ever inspects the OUTER
    ///   application's first named child -- for the simple `f x` shape
    ///   that head IS the bare callee (`long_identifier_or_op`/
    ///   `long_identifier`/`identifier`), but for a genuinely curried
    ///   `f x y` the outer node's first child is the INNER
    ///   `application_expression`, which matches none of those three
    ///   kinds, so the baseline's own real behavior silently produces NO
    ///   callee for a multi-argument curried call at all -- this row
    ///   matches that same real (narrow) depth intentionally rather than
    ///   "improving" on it by walking down through the nesting (which
    ///   would also require un-reversing the arg order the left-recursive
    ///   nesting naturally collects in), per the "baseline's real depth,
    ///   not an idealized one" instruction.
    /// - `import_types` is `["import_decl"]` only, NOT baseline's
    ///   `{"import_decl", "open_expression", "instance"}`: neither
    ///   `open_expression` nor `instance` exists as a node kind anywhere
    ///   in this grammar's `node-types.json` at all (confirmed: F#'s
    ///   `open System.Text` statement is grammatically named
    ///   `import_decl` itself -- verified directly against the grammar's
    ///   own rule, `import_decl: ($) => seq("open", optional("type"),
    ///   $.long_identifier)` -- so `import_decl` was already the correct,
    ///   only-needed entry; the other two are phantom baseline entries
    ///   for a different/older grammar snapshot). `import_decl` has no
    ///   fields (its `long_identifier` child is positional), so it is
    ///   claimed by [`crate::languages::generic::fsharp_quirk`] rather
    ///   than the generic engine's own field-keyed import handling (which
    ///   is minimal-by-design for exactly this "path lives arbitrarily
    ///   deep" reason -- see [`crate::languages::generic::walk`]'s own
    ///   `import_types` branch doc comment).
    /// - `branch_types` keeps baseline's `if_expression`/`for_expression`/
    ///   `while_expression` (all confirmed present, unchanged shape) and
    ///   adds `match_expression`/`try_expression` (both confirmed
    ///   present and clearly decision points a cyclomatic-complexity-
    ///   shaped branch list should count, mirroring every other
    ///   pattern-matching language row's own `match_expression`/
    ///   `case`/`when_expression`-family entries, e.g. [`Self::rust`]'s
    ///   `match_arm`, [`Self::kotlin`]'s `when_expression`/`when_entry`) --
    ///   the baseline's OWN `fsharp_branch_types` array already has
    ///   neither, an omission this row does not carry forward, matching
    ///   the "no idealized MORE depth than baseline" instruction's
    ///   explicit carve-out for filling a baseline GAP rather than
    ///   inventing new semantics (same rationale [`Self::zig`]'s own
    ///   IMPORTS-from-`@import` doc comment already gives for a parallel
    ///   case).
    /// - No `decorator_types`: F# `[<Attribute>]` syntax exists in this
    ///   grammar (an `attributes`/`attribute` node pair, confirmed
    ///   present) but the baseline's own `fsharp_*` row has no
    ///   decorator-types column entry at all (`NULL` in the table
    ///   literal) -- matching baseline's real depth rather than adding a
    ///   richer decorator tier it never had either.
    /// - `field_types` is empty: `record_field`'s own shape (positional
    ///   `identifier`/`_type` children, no `name` field) mirrors
    ///   [`Self::rust`]'s identical "left empty rather than wrong" choice
    ///   for the same "flat `name_field` mechanism cannot express a
    ///   positional-child field name" reason -- left for a future G3
    ///   pass rather than guessed at.
    ///
    ///   `class_inherits_decl`'s own INHERITS handling (`inherit Base(...)`)
    ///   and `application_expression`'s full callee-head reconstruction are
    ///   both [`crate::languages::generic::fsharp_quirk`]/
    ///   [`crate::languages::generic::fsharp_call_override`] hooks -- see
    ///   their own doc comments.
    pub const fn fsharp() -> Self {
        Self {
            name: "fsharp",
            func_types: &["function_or_value_defn"],
            method_types: &["function_or_value_defn"],
            class_types: &[
                "anon_type_defn",
                "record_type_defn",
                "union_type_defn",
                "enum_type_defn",
                "type_abbrev_defn",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["named_module", "namespace"],
            call_types: &["application_expression"],
            call_function_field: "UNUSED_SEE_FSHARP_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_FSHARP_CALL_OVERRIDE",
            import_types: &["import_decl"],
            branch_types: &[
                "if_expression",
                "for_expression",
                "while_expression",
                "match_expression",
                "try_expression",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::kotlin()`).
            name_field: "UNUSED_SEE_FSHARP_QUIRK",
            body_field: "UNUSED_SEE_FSHARP_QUIRK",
        }
    }

    /// Gleam: node kinds verified directly against the `tree-sitter-gleam`
    /// 1.0.0 crate's own `src/node-types.json` (the official
    /// `gleam-lang`-org grammar -- the language's own tooling org, same
    /// "actively maintained, canonical lineage" bar every G2.1 grammar
    /// choice already applied) -- several of the baseline's own
    /// `gleam_*` arrays in `internal/cbm/lang_specs.c` name node kinds
    /// this grammar version never generates at all:
    /// - `func_types`/`method_types` keep baseline's
    ///   `"function"`/`"anonymous_function"`/`"external_function"`
    ///   unchanged (all three confirmed present with a working `name`
    ///   field on the two named-def kinds -- `anonymous_function` has no
    ///   `name` field at all, which this crate's generic engine's own
    ///   func/method branch already tolerates gracefully, silently
    ///   producing no symbol, same as any other unnamed-lambda case
    ///   e.g. [`Self::gdscript`]'s own bare `lambda`). No quirk needed at
    ///   all for the base case: `function`'s `body` field is a real
    ///   `block` child, so the generic engine's own DEFINES-scoped
    ///   body-walk-on-symbol-push path (triggered automatically by
    ///   `func_types`/`method_types` membership) already threads
    ///   `fn_scope` correctly with zero extra code.
    /// - `class_types` is `["type_definition", "type_alias"]`, NOT
    ///   baseline's `{"type_definition", "type_alias", "custom_type"}`:
    ///   `custom_type` does NOT exist as a node kind anywhere in this
    ///   grammar's `node-types.json` at all -- Gleam's `type Shape {
    ///   Circle(radius: Float) }` "custom type" declaration is
    ///   grammatically just `type_definition` itself (verified: the
    ///   baseline's OWN source comment at
    ///   `internal/cbm/lang_specs.c:1340` even says "custom_type (a type
    ///   REFERENCE inside field_type) and type_definition (LHS of a...",
    ///   i.e. baseline's own authors already knew `custom_type` was a
    ///   type-USE-site concept, not a decl-site node kind, yet still put
    ///   it in the decl-site `gleam_class_types` array -- a real,
    ///   baseline-acknowledged-elsewhere bug this row does not carry
    ///   forward) -- same "phantom baseline array entry" class of finding
    ///   as [`Self::solidity`]'s dropped bare `"call"` kind. Neither
    ///   `type_definition` nor `type_alias` has a `name` FIELD of its own
    ///   (verified: both have `"fields":{}`, empty) -- each instead
    ///   carries a positional `type_name` child (a DIFFERENT node shape
    ///   from F#'s identically-named-but-unrelated `type_name` node
    ///   above -- Gleam's own `type_name` node has a real `name` field
    ///   holding a `type_identifier`/`remote_type_identifier` leaf) --
    ///   [`crate::languages::generic::gleam_quirk`] finds the `type_name`
    ///   child directly, then reads ITS `name` field.
    /// - `field_types` is empty, NOT baseline's `{"field"}`: `field` does
    ///   not exist as a node kind in this grammar at all (verified: the
    ///   closest node with that literal name anywhere is `field_access`,
    ///   an unrelated expression-level construct, not a type-body member
    ///   declaration) -- another phantom baseline entry, dropped rather
    ///   than guessed at. Gleam's real per-variant labeled-argument
    ///   syntax (`Circle(radius: Float)`) is a `data_constructor_argument`
    ///   node nested two levels inside `type_definition`'s own
    ///   `data_constructors` child, a depth this crate's flat
    ///   `field_types` mechanism (which only ever fires for a DIRECT
    ///   child of a class-shaped node's own body, per
    ///   [`crate::languages::generic::walk`]'s `field_types` branch) has
    ///   no natural reach into -- left for a future G3 pass rather than
    ///   force-fit, matching [`Self::rust`]'s/[`Self::fsharp`]'s own
    ///   "left empty rather than wrong" posture.
    /// - `module_types` is empty, NOT baseline's `{"source_file"}`:
    ///   identical rationale to every prior row's own finding here
    ///   ([`Self::solidity`]/[`Self::dart`]/[`Self::fsharp`]) -- the
    ///   baseline's `module_node_types` column has zero real consumers,
    ///   while this crate's own `module_types` field is live, so porting
    ///   the root node kind verbatim would invent a wrong "module name is
    ///   whatever came first in the file" behavior rather than reproduce
    ///   an existing one. Gleam's own `import X as Y` `alias` field is
    ///   the closest thing to a module-rename construct, handled by
    ///   [`crate::languages::generic::gleam_quirk`]'s IMPORTS handling
    ///   below directly, not as a Module symbol.
    /// - `call_types` is `["function_call"]`, matching baseline exactly:
    ///   confirmed this grammar's `function_call` node has REAL, directly
    ///   usable `function`/`arguments` fields (verified:
    ///   `function_call: ($) => seq(field("function",
    ///   $._maybe_function_expression), field("arguments", $.arguments))`)
    ///   -- the only one of this wave's three languages whose call-shaped
    ///   node needs ZERO quirk/override at all, flowing through the
    ///   generic engine's own default single-field reconstruction
    ///   unchanged (`call_function_field: "function"`,
    ///   `call_arguments_field: "arguments"`, both real field names this
    ///   grammar actually has, unlike every quirk-claimed row elsewhere
    ///   in this file).
    /// - `import_types` is `["import"]`, matching baseline's
    ///   `{"import", "unqualified_import"}` MINUS `unqualified_import`:
    ///   `unqualified_import` IS a real node kind here (`import
    ///   foo.{bar}`'s `{bar}` clause), but it is a POSITIONAL CHILD of
    ///   `import`'s own OPTIONAL `imports` field
    ///   (`unqualified_imports`), never itself a standalone top-level
    ///   import-shaped statement a file's own body would directly
    ///   contain -- listing it in `import_types` would have the generic
    ///   walker's `import_types` branch fire a SECOND time (redundantly,
    ///   and with no `module`-field context of its own to record a
    ///   useful path) for every already-recorded import's own nested
    ///   clause, so it is intentionally excluded; `gleam_quirk`'s own
    ///   `import` handling records each `unqualified_import`'s symbol
    ///   name directly if ever wanted (left as future G3 scope, not done
    ///   here, matching this wave's "defer rather than force-fit" bar).
    ///   `import`'s own `module` field (verified real, required) points
    ///   at a `module` node that is ITSELF an unfielded leaf whose own
    ///   full text span already IS the complete dotted/slashed module
    ///   path as written (`"gleam/io"`) -- verified against the
    ///   grammar's own `module: ($) => seq($._name, repeat(seq("/",
    ///   $._name)))` rule (a repeated `/`-joined sequence, still one
    ///   single node whose byte range covers the whole path) -- so
    ///   [`crate::languages::generic::gleam_quirk`] reads it with a
    ///   single `.utf8_text()` call, no dotted-segment-joining helper
    ///   needed (unlike F#'s `long_identifier`, which IS multiple
    ///   sibling `identifier` nodes requiring an explicit join).
    /// - `branch_types` is `["case", "case_clause"]`, matching baseline
    ///   exactly (both confirmed present, unchanged shape) -- Gleam has
    ///   no `if`/`while`/`for` statement-level control flow at all (a
    ///   deliberate language-design choice -- everything branches through
    ///   pattern-matching `case`), so this short list is already this
    ///   grammar's complete decision-point vocabulary, not an
    ///   undercount.
    /// - No `interface_types`/`enum_types`/`alias_types`/
    ///   `decorator_types`: confirmed via an exhaustive
    ///   extends/implements/inherit/protocol/trait/interface node-name
    ///   search across the grammar's own `node-types.json` (NONE found,
    ///   matching Gleam's own language design -- structural typing, no
    ///   classes/traits/protocols/decorators at all) -- the baseline's
    ///   own `extract_base_classes` dedicated-walker list
    ///   (`internal/cbm/extract_defs.c`:2377-2394) has no Gleam entry
    ///   either, confirming this is a real language property, not a
    ///   coverage gap either codebase happens to share. `type_alias`
    ///   (Gleam's `type Meters = Float`) is folded into `class_types`
    ///   rather than getting its own `alias_types` entry, matching
    ///   baseline's own flatter classification for this language (its
    ///   `gleam_class_types` already lists `type_alias` alongside
    ///   `type_definition`) since this crate's generic engine's
    ///   class-shape branch (which `alias_types` also feeds into) treats
    ///   the distinction as "which `SymbolKind` to tag it", not "which
    ///   quirk claims it" -- [`crate::languages::generic::gleam_quirk`]
    ///   distinguishes the two directly by node kind.
    pub const fn gleam() -> Self {
        Self {
            name: "gleam",
            func_types: &["function", "anonymous_function", "external_function"],
            method_types: &["function", "anonymous_function", "external_function"],
            class_types: &["type_definition", "type_alias"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["function_call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import"],
            branch_types: &["case", "case_clause"],
            decorator_types: &[],
            // Never actually consulted for `class_types` (see this
            // const's own doc comment -- `gleam_quirk` claims both kinds
            // in full); `func_types`/`method_types` DO use the ordinary
            // generic name-field path (`function`/`external_function`
            // both have a real `"name"` field), so this value is live for
            // those two, unlike every other placeholder row in this file.
            name_field: "name",
            body_field: "body",
        }
    }

    /// GLSL: the baseline's own `lang_specs.c` row
    /// (`internal/cbm/lang_specs.c:1960-1964`) reuses C's node-type
    /// arrays VERBATIM (`c_func_types`/`c_class_types`/`c_field_types`/
    /// `c_module_types`/`c_call_types`/`c_import_types`/
    /// `c_branch_types`, only the grammar factory pointer
    /// (`tree_sitter_glsl` vs `tree_sitter_c`) differs) -- GLSL is a
    /// C-like shader language whose function/struct/call/`#include`/
    /// control-flow syntax is a strict subset of C's own (no classes, no
    /// namespaces, no templates). This row mirrors that reuse exactly,
    /// following the same `{ name: "...", ..Self::other() }` pattern
    /// [`Self::tsx`] already established for an analogous "distinct
    /// baseline language row, byte-for-byte identical vocabulary" case:
    /// every field below is [`Self::c`]'s own, verified unchanged (this
    /// crate's `tree-sitter-c = "0.24"` dependency is ALREADY pinned to
    /// the exact version [`Self::c`]'s own doc comment was verified
    /// against, and GLSL needs no separate grammar dependency at all --
    /// see [`crate::languages::generic::parse_glsl`]).
    ///
    /// Verified via a real parse tree dump (this reuse claim is NOT
    /// taken on faith just because the baseline's own table row makes
    /// it): every construct this crate's extraction actually cares about
    /// for GLSL -- `function_definition`s (incl. their own
    /// `call_expression`s, `return`/`if`/`for`/`while` branches),
    /// `struct_specifier`s and their own `field_declaration` members,
    /// `#include`/`#version` preprocessor lines -- parses through
    /// `tree-sitter-c` with ZERO error nodes, confirmed against a
    /// realistic fragment/vertex-shader-shaped sample (structs, a
    /// helper function, `main()`, nested calls, a `field_expression`
    /// receiver). GLSL's shader-specific STORAGE QUALIFIERS on
    /// GLOBAL-SCOPE variable declarations -- `uniform`/`in`/`out`
    /// (bare) and `layout(location = N) in ...` (qualified) -- do NOT
    /// parse cleanly against plain C grammar, since C has no such
    /// keywords at all: `uniform Light light;`/`out vec4 FragColor;`
    /// each still produce a `declaration` node (so the variable's own
    /// NAME still parses as that declaration's ordinary `declarator`
    /// field), but the qualifier keyword itself is misread as the
    /// declaration's `type` field with the REAL type name wrapped in a
    /// nested `ERROR` node instead (`uniform`/`out`/`in` become the
    /// `[field=type]` text, `Light`/`vec4`/`vec3` end up inside an
    /// `ERROR`); `layout (location = 0) in vec3 aPos;` is worse -- the
    /// `layout(...)` call-shaped syntax plus the following `in` keyword
    /// are absorbed into ONE `ERROR` node together, so the declaration
    /// never materializes as a `declaration` node at all (the variable
    /// is invisible, not merely misclassified). This is a REAL, verified
    /// parse-error boundary, not a hypothetical -- but it is also
    /// entirely CONTAINED to global-scope qualified variable
    /// declarations: it never touches a `function_definition`'s own
    /// parse (confirmed: a full vertex-shader-shaped sample with
    /// `layout`/`in`/`out`/`uniform` qualifiers on its globals still
    /// parses `main()`'s own body -- including a nested `field_expression`-
    /// receiver call and an assignment referencing the malformed globals
    /// by name -- with zero errors of its own), which is this crate's
    /// entire GLSL extraction scope in the first place: this row's
    /// (i.e. [`Self::c`]'s own) `field_types`/`var`-shaped depth for
    /// bare global variables was already this shallow for plain C too
    /// (a `declaration` node's own quirk arm just emits a
    /// Variable/Constant symbol keyed off whichever declarator identifier
    /// it can find -- see [`crate::languages::generic::c_top_level_declaration_symbols`] --
    /// with no qualifier/attribute reasoning at all), so a GLSL global's
    /// qualifier keyword being misparsed changes nothing about what this
    /// engine would have extracted from it either way. `root_node().has_error()`
    /// being `true` for a realistic shader file is consequently expected
    /// and harmless for this crate's purposes, not a regression to chase.
    ///
    /// A distinct `LangSpec`/`Language::Glsl` row (rather than routing
    /// `.glsl`/`.vert`/`.frag`/... straight to [`Language::C`]) exists
    /// purely so a caller can distinguish "this file is a GLSL shader"
    /// from "this file is C" the same way the baseline's own
    /// `CBM_LANG_GLSL`/`CBM_LANG_C` split does, in case a future
    /// shader-specific quirk (pipeline-stage detection from file
    /// extension, `layout(location = N)` binding-slot extraction, a
    /// dedicated qualifier-tolerant global-variable quirk, ...) ever
    /// needs to diverge from plain C without touching C's own row.
    pub const fn glsl() -> Self {
        Self {
            name: "glsl",
            ..Self::c()
        }
    }

    /// Ada: node kinds copied from the baseline's `ada_func_types`/
    /// `ada_class_types`/`ada_field_types`/`ada_call_types`/
    /// `ada_import_types`/`ada_branch_types` (`codebase-memory-mcp/
    /// internal/cbm/lang_specs.c:1118-1138`), cross-checked node-kind-by-
    /// node-kind against the actual `node-types.json`/`grammar.js` shipped
    /// in the `tree-sitter-ada` crate this row binds (crates.io
    /// `tree-sitter-ada = "0.1"`, `briot/tree-sitter-ada`), plus a real
    /// parse tree dump (`cargo run` against a scratch crate depending on
    /// `tree-sitter-ada` directly) that caught several baseline
    /// assumptions this specific grammar contradicts -- see the
    /// corrections below. G2.2a scope note: this crate's own
    /// `internal/cbm/extract_defs.c:722-736` ALREADY documents (in a code
    /// comment, not just the array) that `subprogram_body`/
    /// `subprogram_declaration` need special handling -- confirmed
    /// correct and extended below to a THIRD/FOURTH case the baseline's
    /// own comment does not mention (`expression_function_declaration`,
    /// `entry_declaration`).
    /// - `func_types`/`method_types` are EMPTY, NOT baseline's
    ///   `{"subprogram_declaration", "subprogram_body", "entry_declaration",
    ///   "expression_function_declaration"}`: every one of these four node
    ///   kinds has NO `name` field on itself (verified via real parse
    ///   dumps) --
    ///   `subprogram_body`/`subprogram_declaration`/`expression_function_declaration`
    ///   carry their name on a nested `procedure_specification`/
    ///   `function_specification` child's OWN `name` field (this crate's
    ///   generic `walk`'s func/method branch reads `spec.name_field` off
    ///   the very node it matched in `func_types`, not a descendant, so
    ///   pointing these here would silently emit nothing for every Ada
    ///   subprogram); `entry_declaration` carries its name on a
    ///   DIFFERENT field entirely (`entry_name`, not `name` -- an Ada
    ///   grammar-specific field name this crate's single flat
    ///   `name_field` cannot express for one kind while every other kind
    ///   uses plain `name`). All four are consequently fully claimed by
    ///   [`crate::languages::generic::ada_quirk`] instead (same "arrays
    ///   fully claimed by quirk, generic fallback never actually
    ///   reached" posture as [`Self::c`]/[`Self::cpp`]/[`Self::php`]).
    /// - `class_types` is `["package_declaration", "package_body",
    ///   "full_type_declaration"]`, NOT baseline's fuller
    ///   `{"type_declaration", "full_type_declaration", "package_declaration",
    ///   "protected_type_declaration", "task_type_declaration",
    ///   "component_declaration", "object_declaration",
    ///   "record_type_definition"}`: `type_declaration` is not a real node
    ///   kind this grammar ever emits at all (a real parse always produces
    ///   `full_type_declaration`, `private_type_declaration`, or
    ///   `private_extension_declaration` instead -- `type_declaration`
    ///   does not appear anywhere in this grammar's own `grammar.js`
    ///   `rules` at all); `record_type_definition` is a plain
    ///   *type-definition* node nested INSIDE `full_type_declaration`
    ///   (verified: `type Widget_Type is record ... end record;` parses
    ///   as `full_type_declaration` -> `record_type_definition` ->
    ///   `record_definition`), never itself carrying a name -- including
    ///   it here would double-count the same declaration under two node
    ///   kinds; `component_declaration` is this row's own
    ///   [`Self::field_types`] (a record's member, not a product type in
    ///   its own right -- see this doc's own field_types note below);
    ///   `object_declaration` is a variable binding, not a type/product
    ///   declaration (Ada has no class-like use for it the way some
    ///   other languages' "object declaration" terminology might
    ///   suggest); `protected_type_declaration`/`task_type_declaration`
    ///   are real but narrow, concurrency-specific Ada constructs with no
    ///   fixture coverage in this wave -- omitted rather than guessed at
    ///   (same "left out rather than wrong" posture as e.g.
    ///   [`Self::dart`]'s omitted `declaration` field_types) -- a future
    ///   wave can add them once a concrete fixture justifies the guess.
    ///   `package_body` is ADDED (absent from baseline's own array
    ///   entirely): this crate's generic engine needs it to see a
    ///   `Class`-shaped symbol so `ada_quirk`'s scoped-body walk can DEFINES
    ///   its subprograms -- both `package_declaration` (a spec-only
    ///   `package Widget is ... end Widget;`) and `package_body` (`package
    ///   body Widget is ... end Widget;`) carry a working `name` field
    ///   (verified) so neither needs a quirk claim for the base
    ///   name-extraction case, only for `full_type_declaration`'s heritage
    ///   (see [`crate::languages::generic::ada_quirk`]).
    /// - `field_types` is `["component_declaration"]` (matches baseline
    ///   exactly): a record's member has NO `name` field either (verified:
    ///   positional `[identifier, ':', component_definition, ';']`
    ///   children) -- fully claimed by `ada_quirk` alongside everything
    ///   else this row's flat arrays cannot express.
    /// - `module_types` is empty, NOT baseline's `{"compilation"}`: this
    ///   crate's generic engine's `module_types` branch emits a
    ///   [`crate::parsers::SymbolKind::Module`] symbol named after the
    ///   node's FIRST NAMED CHILD -- `compilation`'s first named child is
    ///   whatever the file's first `with_clause`/`use_clause`/
    ///   `compilation_unit` happens to be (verified via a real parse:
    ///   `compilation`'s children are a flat list of sibling
    ///   `compilation_unit` wrappers, one per top-level declaration/
    ///   clause -- there is no single "this compilation's own name" node
    ///   at all), so porting `"compilation"` verbatim would invent a
    ///   wrong Module symbol rather than reproduce a baseline behavior
    ///   (identical rationale to [`Self::solidity`]/[`Self::gdscript`]/
    ///   [`Self::dart`]'s own identical `module_types` corrections).
    /// - `call_types` is `["function_call", "procedure_call_statement"]`
    ///   (matches baseline exactly) with `call_function_field: "name"`:
    ///   BOTH kinds carry a real, working `name` field (verified against
    ///   real parses of `Helper ("x");` and `Compute (Y)`) -- no quirk
    ///   needed for the base call-callee case at all, matching this
    ///   crate's own dedicated `internal/cbm/extract_calls.c`
    ///   `extract_ada_callee` (which reads the exact same `"name"` field,
    ///   falling back to the first named child only for a grammar
    ///   variant this crate's own version does not need).
    /// - `import_types` is `["with_clause", "use_clause"]` (matches
    ///   baseline exactly): NEITHER kind has any module-path field at all
    ///   (verified against `grammar.js`: `with_clause` is
    ///   `field('is_limited', ...), field('is_private', ...), 'with',
    ///   $._name_list, ';'` -- the name list itself is a bare, unfielded
    ///   child) -- fully claimed by `ada_quirk`, mirroring this crate's
    ///   own `internal/cbm/extract_imports.c` `parse_ada_imports` exactly
    ///   (scan named children of kind `identifier`/`selected_component`/
    ///   `name`, each one its own dotted import path).
    /// - `branch_types` matches baseline's `ada_branch_types` exactly
    ///   (`if_statement`/`for_loop_statement`/`loop_statement`/
    ///   `while_loop_statement`/`case_statement`/`select_statement`) --
    ///   every one of these six IS a real node kind in this grammar
    ///   (verified: a real parse of a `for`/`while`/plain `loop` all
    ///   surface as `loop_statement` with different `iteration_scheme`
    ///   shapes underneath, so the three loop-flavor entries are each
    ///   real and reachable, not redundant).
    /// - INHERITS: Ada's `type Derived is new Base with record ... end
    ///   record;` (a tagged-type extension) and the plainer `type Alias is
    ///   new Integer;` (a derived-type-without-extension) both parse as
    ///   `full_type_declaration` -> `derived_type_definition`, whose OWN
    ///   `subtype_mark` field names the base type directly (verified via
    ///   a real parse of both shapes) -- [`crate::languages::generic::ada_quirk`]
    ///   reads it for an INHERITS edge. This crate's own
    ///   `internal/cbm/extract_defs.c` dedicated-walker list (:2377-2394)
    ///   has no Ada entry and its generic `fields[]` fallback (:2543) has
    ///   no `"subtype_mark"` entry either -- Ada tagged-type derivation is
    ///   genuinely invisible to the baseline's own INHERITS extraction
    ///   today. Wiring it here is consequently *more* complete than the
    ///   baseline, not a baseline-parity port -- same "the baseline simply
    ///   has a gap, not a deliberate design choice" posture
    ///   [`Self::zig`]'s own IMPORTS doc comment already established as
    ///   acceptable, and required here regardless since Tier 3 demands at
    ///   least one of inherits/deep-type-refs/decorators and Ada's
    ///   grammar has no decorator/attribute syntax at all (matching
    ///   [`Self::zig`]'s identical "no such syntax" finding) and no
    ///   dedicated deep-type-ref infrastructure exists in this engine yet.
    /// - `decorator_types` is empty: Ada has no attribute/annotation/
    ///   decorator syntax at all (aspect specifications --
    ///   `with Pre => X`-style contract clauses -- are the closest
    ///   analog, but they attach to a declaration's OWN signature grammar
    ///   rule, not a separate decorator-shaped node any array here could
    ///   name) -- matches [`Self::zig`]'s identical finding for the
    ///   identical reason.
    /// - `name_field`/`body_field` are placeholders, never actually
    ///   consulted: every one of `func_types`/`method_types`/`class_types`/
    ///   `field_types` above is empty or fully quirk-claimed (see each
    ///   bullet above) -- same posture as [`Self::c`]/[`Self::cpp`].
    pub const fn ada() -> Self {
        Self {
            name: "ada",
            func_types: &[],
            method_types: &[],
            class_types: &[
                "package_declaration",
                "package_body",
                "full_type_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["component_declaration"],
            module_types: &[],
            call_types: &["function_call", "procedure_call_statement"],
            call_function_field: "name",
            call_arguments_field: "actual_parameter_part",
            import_types: &["with_clause", "use_clause"],
            branch_types: &[
                "if_statement",
                "for_loop_statement",
                "loop_statement",
                "while_loop_statement",
                "case_statement",
                "select_statement",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::cpp()`).
            name_field: "UNUSED_SEE_ADA_QUIRK",
            body_field: "UNUSED_SEE_ADA_QUIRK",
        }
    }

    /// Apex: node kinds copied from the baseline's `apex_func_types`/
    /// `apex_class_types`/`apex_field_types`/`apex_call_types`/
    /// `apex_import_types`/`apex_branch_types`/`apex_decorator_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1565-1580`),
    /// cross-checked node-kind-by-node-kind against the actual
    /// `node-types.json`/`grammar.js` shipped in the `tree-sitter-sfapex`
    /// crate's `apex` grammar this row binds (crates.io
    /// `tree-sitter-sfapex = "3.0"`, `aheber/tree-sitter-sfapex` --
    /// EXACT copyright-line match against this crate's own vendored
    /// `internal/cbm/vendored/grammars/apex/LICENSE`, confirming this is
    /// the identical grammar lineage the baseline itself vendored), plus
    /// a real parse tree dump. This grammar is deliberately, thoroughly
    /// Java-shaped (`class_declaration`/`interface_declaration`/
    /// `enum_declaration`/`method_declaration`/`method_invocation` all
    /// carry the IDENTICAL field names [`Self::java`]'s own row already
    /// documents: `name`/`superclass`/`interfaces`/`body`,
    /// `object`/`name`/`arguments`, ...) -- every array below matched the
    /// baseline's own guess with NO corrections needed, unlike every
    /// other language this wave onboarded:
    /// - `func_types` adds `"trigger_declaration"` (matches baseline's
    ///   `apex_func_types` exactly, alongside `method_declaration`/
    ///   `constructor_declaration`): Apex's own `trigger Foo on Bar (before
    ///   insert) { ... }` top-level construct, unique to this language
    ///   (no Java equivalent) -- verified it carries a real `name` field
    ///   (`WidgetTrigger`) directly on itself (unlike `method_declaration`/
    ///   `constructor_declaration`, whose bodies/heritage still need
    ///   [`crate::languages::generic::apex_quirk`]'s DEFINES-scoping the
    ///   same way [`Self::java`]'s own method/constructor handling does),
    ///   so it needs no quirk for the base name-extraction case, only for
    ///   its `object`/`events` fields (which this row's flat arrays
    ///   cannot express) -- see `apex_quirk`.
    /// - `class_types` is `["class_declaration", "interface_declaration",
    ///   "enum_declaration"]` (matches baseline exactly): every one
    ///   carries a working `name` field (verified) plus Java-identical
    ///   `superclass`/`interfaces`/`extends_interfaces` heritage fields --
    ///   see [`crate::languages::generic::apex_quirk`] for the
    ///   INHERITS/IMPLEMENTS wiring (a direct, non-generic-fallback port
    ///   of [`Self::java`]'s own `java_superclass_name`/
    ///   `java_super_interfaces`/`java_extends_interfaces` logic, since
    ///   the field shapes are identical).
    /// - `field_types` is `["field_declaration"]` (matches baseline
    ///   exactly): SAME two-level name shape as Java's own
    ///   [`Self::java`] row (`field_declaration`'s own name lives one
    ///   level down on its `variable_declarator` child's `name` field,
    ///   verified) -- like Java, an ordinary field is invisible to this
    ///   row's own flat array (`name_field` would look for a `name` field
    ///   directly on `field_declaration` itself, which does not exist);
    ///   unlike Java, this crate does NOT gate Apex fields on a
    ///   `static final`-equivalent modifier check before recording a
    ///   DEFINES edge (Apex's constant convention is less rigid and this
    ///   wave found no fixture-driving need to replicate Java's exact
    ///   gate) -- [`crate::languages::generic::apex_quirk`] emits a plain
    ///   DEFINES edge for every field unconditionally instead, a
    ///   deliberately simpler choice than Java's row documented as such
    ///   rather than silently copied.
    /// - `module_types` is empty, NOT baseline's `{"parser_output"}`:
    ///   `parser_output` is this grammar's ROOT node (confirmed via a
    ///   real parse: every top-level class/interface/enum/trigger is a
    ///   direct child of one `parser_output` node) -- identical
    ///   "baseline's `module_node_types` is dead data there, but this
    ///   crate's `module_types` is LIVE, so porting the root node verbatim
    ///   would invent a nonsensical Module symbol named after whatever
    ///   the file's first top-level declaration happens to be" rationale
    ///   as [`Self::solidity`]/[`Self::gdscript`]/[`Self::dart`]/
    ///   [`Self::ada`]'s own identical corrections.
    /// - `call_types` is `["method_invocation"]` (matches baseline
    ///   exactly) with `object`/`name`/`arguments` fields IDENTICAL to
    ///   Java's own `method_invocation` shape (verified) -- reuses this
    ///   row's own [`crate::languages::generic::apex_call_override`]
    ///   (a direct, non-generic-fallback port of
    ///   [`crate::languages::generic::java_call_override`]'s logic).
    /// - `import_types` is EMPTY, NOT baseline's `{"extends", "with_clause"}`:
    ///   neither is a real top-level import-shaped node in this grammar
    ///   at all -- `"extends"` is only ever the bare unnamed keyword token
    ///   inside `superclass`/`extends_interfaces` (already captured as
    ///   INHERITS, not IMPORTS, above), and `with_clause` does not exist
    ///   anywhere in this grammar's own `grammar.js` `rules` (Apex has no
    ///   `import`/`use`/`require`-equivalent statement at all -- every
    ///   class in an org's shared namespace is simply globally visible,
    ///   with no explicit per-file import list to extract). Porting
    ///   either verbatim would be inventing IMPORTS edges from node kinds
    ///   that either are not imports (`extends`) or do not exist
    ///   (`with_clause`) -- left empty rather than wrong, matching this
    ///   wave's own "verify, do not blindly transcribe" mandate.
    /// - `branch_types` swaps baseline's bare `"switch_expression"` entry
    ///   for the SAME name (matches exactly) plus every other baseline
    ///   entry confirmed present (`if_statement`/`for_statement`/
    ///   `while_statement`/`do_statement`/`try_statement`/`catch_clause`/
    ///   `enhanced_for_statement`) via a real parse of a `try`/`catch`/
    ///   `finally` block and a plain `for`/`while`/`do` loop.
    /// - `decorator_types` is `["annotation"]` (matches baseline exactly):
    ///   `@RestResource(...)`/`@AuraEnabled`/`@isTest`-style annotations
    ///   attach via a `modifiers` child exactly like Java's own
    ///   `marker_annotation`/`annotation` split -- EXCEPT this grammar
    ///   uses ONE unified `annotation` kind for both the bare
    ///   (`@AuraEnabled`) and argument-bearing (`@RestResource(...)`)
    ///   forms (verified: no separate `marker_annotation` kind exists at
    ///   all in this grammar, unlike Java's) -- reuses this row's own
    ///   [`crate::languages::generic::apex_quirk`] DECORATES wiring (a
    ///   simplified single-kind port of
    ///   [`crate::languages::generic::java_annotations`]'s logic).
    /// - No route-registration wiring at all: this crate's own
    ///   `internal/cbm/service_patterns.c` `route_reg_libraries` table
    ///   (:318-381) has ZERO Apex/Salesforce entries -- Apex's own
    ///   `@RestResource`/`@HttpGet`-style REST-service annotations are a
    ///   REAL route-registration convention this specific grammar and
    ///   language absolutely have, but the baseline itself never wired
    ///   route detection for them (confirmed: `grep -r Salesforce
    ///   internal/cbm/service_patterns.c` and `grep -r RestResource
    ///   internal/cbm/` both return nothing) -- this is a genuine baseline
    ///   gap, not a design choice to intentionally omit Apex from route
    ///   detection, matched here rather than invented per this wave's own
    ///   brief ("if it's absent, that's a real baseline gap, match it,
    ///   don't invent").
    pub const fn apex() -> Self {
        Self {
            name: "apex",
            func_types: &[
                "method_declaration",
                "constructor_declaration",
                "trigger_declaration",
            ],
            method_types: &["method_declaration", "constructor_declaration"],
            class_types: &[
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["field_declaration"],
            module_types: &[],
            call_types: &["method_invocation"],
            call_function_field: "UNUSED_SEE_APEX_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "do_statement",
                "switch_expression",
                "try_statement",
                "catch_clause",
                "enhanced_for_statement",
            ],
            decorator_types: &["annotation"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Crystal: node kinds copied from the baseline's `crystal_func_types`/
    /// `crystal_class_types`/`crystal_field_types`/`crystal_call_types`/
    /// `crystal_import_types`/`crystal_branch_types`/
    /// `crystal_decorator_types` (`codebase-memory-mcp/internal/cbm/
    /// lang_specs.c:1196-1208`), cross-checked node-kind-by-node-kind
    /// against the actual `node-types.json`/`grammar.js` in the
    /// `crystal-lang-tools/tree-sitter-crystal` repository this row binds
    /// (git dependency pinned to commit `50ca9e6` -- EXACT copyright-line
    /// match, "Gabriel Holodak", against this crate's own vendored
    /// `internal/cbm/vendored/grammars/crystal/LICENSE`, confirming this
    /// is the identical grammar lineage the baseline itself vendored; not
    /// published to crates.io under this or any other discoverable name
    /// as of this wave -- confirmed via `cargo search tree-sitter-crystal`
    /// returning only an unrelated "Crystal Reports formula language"
    /// decoy crate, not this grammar -- see this crate's `Cargo.toml`
    /// comment for the git-dependency rationale), plus a real parse tree
    /// dump that caught several baseline assumptions this specific
    /// grammar version contradicts -- see the corrections below.
    /// - `func_types`/`method_types` both list `"method_def"`/
    ///   `"abstract_method_def"` (matches baseline's `crystal_func_types`
    ///   exactly): both carry a working `name` field (verified); a
    ///   top-level `def foo` (Function) vs. one nested inside a
    ///   `class`/`module`/`struct` body (Method) is told apart by this
    ///   crate's own nesting-based fallback, same rationale as Ruby's
    ///   identical `method`/`singleton_method` dual-listing in
    ///   [`Self::ruby`]. `abstract_method_def` (an `abstract def foo`
    ///   inside an `abstract class`, no body at all) also carries a
    ///   working `name` field directly (verified) -- no quirk needed for
    ///   either kind's base case.
    /// - `class_types` is `["class_def", "struct_def", "module_def",
    ///   "enum_def", "annotation_def"]`, NOT baseline's fuller
    ///   `{"class_def", "struct_def", "module_def", "enum_def",
    ///   "annotation_def", "type_declaration"}`: `type_declaration`
    ///   (`@name : String` -- an instance-variable TYPE ANNOTATION, not a
    ///   product-type declaration at all, confirmed via a real parse:
    ///   `type_declaration`'s own fields are `var`/`type`, pointing at an
    ///   `instance_var` and a type-name `constant` respectively, nothing
    ///   name-of-a-new-type-shaped) is a real node kind in this grammar
    ///   but is NOT a class/struct/module/enum-equivalent declaration by
    ///   any reasonable reading -- porting it here would misclassify
    ///   every one of a class's own `@ivar : Type` annotations as its own
    ///   nested Class/Interface/Enum/TypeAlias symbol, a real
    ///   misclassification bug, not just an imprecision. Every one of the
    ///   five kept kinds carries a working `name` field (verified) --
    ///   `class_def`/`struct_def` ADDITIONALLY carry a working, optional
    ///   `superclass` field (verified: `class Sub < Base` / `struct Sub <
    ///   Base`) that this row's own flat arrays cannot express -- see
    ///   [`crate::languages::generic::crystal_quirk`] for the INHERITS
    ///   wiring, a direct, non-generic-fallback port of
    ///   [`crate::languages::generic::ruby_superclass_name`]'s identical
    ///   field-unwrap logic (Crystal's `superclass` field wraps a bare
    ///   `constant`/`generic_instance_type` node directly, unlike Ruby's
    ///   own wrapped-with-the-`<`-token shape -- Crystal's own field
    ///   value is ALREADY just the base type, no unwrap needed at all,
    ///   confirmed via a real parse).
    /// - `field_types` is `["instance_var", "class_var"]` (matches
    ///   baseline exactly): NEITHER kind has a `name` field at all --
    ///   each one, wherever it appears (bare inside a `type_declaration`,
    ///   as the LHS of an `assign`/`op_assign`, or as a bound
    ///   constructor-shorthand `param` inside `def initialize(@name)`),
    ///   IS itself a single leaf token whose OWN text already is the
    ///   field's full name INCLUDING its `@`/`@@` sigil (verified: a real
    ///   parse of `@name : String` shows `instance_var` with zero
    ///   children at all, `.utf8_text()` on the node itself yielding
    ///   `"@name"` directly) -- this row's generic `field_types` branch
    ///   (which calls `child_by_field_name(spec.name_field)` looking for
    ///   a NAMED FIELD) can never succeed here, so both kinds are fully
    ///   claimed by [`crate::languages::generic::crystal_quirk`] instead,
    ///   which reads the node's own text directly rather than any field.
    /// - `module_types` is EMPTY, NOT baseline's `{"program"}`: `program`
    ///   is not a real node kind this grammar emits AT ALL -- this
    ///   grammar's own `grammar.js` names its root rule `expressions`
    ///   (confirmed: `rules: { expressions: $ => seq(optional($._statements)),
    ///   ... }` is the very first rule, and a real parse's root node
    ///   kind is literally `expressions`) -- identical "baseline's
    ///   `module_node_types` is dead data there, but this crate's
    ///   `module_types` is LIVE" rationale as every other language this
    ///   wave corrected the same way ([`Self::ada`]/[`Self::apex`]/
    ///   [`Self::solidity`]/[`Self::gdscript`]/[`Self::dart`]).
    /// - `call_types` is `["call"]` ONLY, NOT baseline's fuller
    ///   `{"call", "command", "implicit_object_call"}`: `command` is a
    ///   real node kind in this grammar, but it is Crystal's BACKTICK
    ///   SHELL-COMMAND LITERAL (`` `ls -la` ``,
    ///   `command: $ => seq(alias($._command_literal_start, '`'),
    ///   optional($._string_literal_content), alias($._command_literal_end, '`'))`
    ///   in `grammar.js`) -- a STRING-shaped literal with no callee/
    ///   arguments semantics at all, never a paren-less method call the
    ///   way its name (and Ruby's differently-named but similarly-shaped
    ///   `command_call`) might suggest; confirmed via a real parse:
    ///   `puts "hello"` (the paren-less call syntax `command` was
    ///   originally guessed to cover) parses as an ordinary `call` node
    ///   with `method`/`arguments` fields, NOT `command` at all.
    ///   `implicit_object_call` (the rare `.foo` bare-dot-call-inside-a-
    ///   block idiom with no explicit receiver) is a real, distinct node
    ///   kind confirmed present in `node-types.json`, but this wave could
    ///   not produce it from valid Crystal source in a real parse tree
    ///   (its only documented trigger, a leading-dot method chain inside
    ///   a `tap`/block-with-implicit-self context, repeatedly produced a
    ///   grammar `ERROR` node instead across two independent attempts) --
    ///   left OUT rather than included on unverified faith, matching this
    ///   wave's own "verify, do not blindly transcribe" mandate; a future
    ///   wave with a confirmed working trigger can add it once verified.
    ///   `call`'s own `method`/`receiver`/`arguments`/`block` fields
    ///   (verified) mirror Ruby's `call` shape exactly -- reuses
    ///   [`crate::languages::generic::crystal_call_override`], a direct
    ///   port of [`crate::languages::generic::ruby_call_override`]'s
    ///   logic (same receiver/method field split, same bare-callee
    ///   fallback when no receiver is present).
    /// - `import_types` is EMPTY (matches baseline's own
    ///   `crystal_import_types = {"require"}` being a call-shaped node,
    ///   not a syntactic import statement, in spirit -- though this
    ///   grammar version gives `require` its OWN distinct node kind
    ///   rather than aliasing it to plain `call` the way Ruby's
    ///   `require`/`require_relative` do): `require "json"` is a
    ///   dedicated `require: $ => seq('require', $.string)` grammar rule
    ///   (verified) -- a bare keyword token plus one unfielded `string`
    ///   child, with NO callee/arguments shape at all, so it is NOT
    ///   included in `call_types` (unlike Ruby's `require`, which IS a
    ///   `call`-shaped node there) -- instead
    ///   [`crate::languages::generic::crystal_quirk`] claims the
    ///   `"require"` node kind directly via `on_unmatched_node`, pushing
    ///   an IMPORTS edge from its `string` child's stripped-quotes text,
    ///   mirroring this crate's own `internal/cbm/extract_imports.c`
    ///   `parse_crystal_imports` (:2401-2413) exactly (walk to any
    ///   `"require"`-kind node, push its string-descendant text as the
    ///   import path).
    /// - `branch_types` matches baseline's `crystal_branch_types` exactly
    ///   (`if`/`unless`/`case`/`while`/`until`/`begin_block`/
    ///   `rescue_block`) -- every one confirmed a real, reachable node
    ///   kind via a real parse of an `if`/`unless`/`case`/`when`/`else`
    ///   chain.
    /// - `decorator_types` is `["annotation"]` (matches baseline exactly):
    ///   `@[Widget::Attr]` custom-annotation syntax is a real, distinct
    ///   `annotation` node kind (verified: `@[` / `constant` / `]`
    ///   children, no fields at all) -- this crate defers wiring an
    ///   actual DECORATES edge for it this wave (no fixture-driving need
    ///   found; INHERITS from `superclass`/IMPORTS from `require` already
    ///   satisfy this language's Tier 3 "at least one of inherits/
    ///   deep-type-refs/decorators" requirement on their own), matching
    ///   this wave's brief that complexity/richer-tier extraction may be
    ///   deferred -- kept here purely as an accurate, at-a-glance
    ///   grammar-vocabulary record for a future wave, same posture as
    ///   several other languages' documented-but-unwired arrays already
    ///   established (e.g. [`Self::gdscript`]'s `decorator_types` is
    ///   likewise recorded without this row's own `class_types`/
    ///   `func_types` consuming it for a DECORATES edge).
    /// - `name_field`/`body_field` are placeholders, never actually
    ///   consulted for `field_types` (fully quirk-claimed, see above) --
    ///   `func_types`/`method_types`/`class_types` DO still use the
    ///   ordinary flat `name_field`/`body_field` path (both real fields on
    ///   every one of those kinds, verified), unlike [`Self::c`]'s
    ///   fully-claimed-everything posture -- same "confined to specific
    ///   arrays only" posture as [`Self::dart`]'s identical split.
    pub const fn crystal() -> Self {
        Self {
            name: "crystal",
            func_types: &["method_def", "abstract_method_def"],
            method_types: &["method_def", "abstract_method_def"],
            class_types: &[
                "class_def",
                "struct_def",
                "module_def",
                "enum_def",
                "annotation_def",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["instance_var", "class_var"],
            module_types: &[],
            call_types: &["call"],
            call_function_field: "UNUSED_SEE_CRYSTAL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_CRYSTAL_CALL_OVERRIDE",
            import_types: &[],
            branch_types: &[
                "if",
                "unless",
                "case",
                "while",
                "until",
                "begin_block",
                "rescue_block",
            ],
            decorator_types: &["annotation"],
            name_field: "name",
            body_field: "body",
        }
    }
    /// R: node kinds copied from the baseline's `r_func_types`/
    /// `r_module_types`/`r_call_types`/`r_import_types`/`r_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:625-630),
    /// cross-checked node-kind-by-node-kind against the actual
    /// `tree-sitter-r` 1.3.0 crate's own `src/node-types.json` (G2.2h grammar
    /// onboarding) plus a real parse-tree dump (`cargo run` against a scratch
    /// crate depending on `tree-sitter-r` directly) -- every node kind the
    /// baseline names is genuinely present in this grammar, but the baseline's
    /// own `resolve_r_func_name` (`internal/cbm/extract_defs.c`:529-545) turns
    /// out load-bearing rather than a defensive extra, confirmed by the real
    /// parse dump:
    /// - `func_types` is `["function_definition"]` (matches baseline), but
    ///   this node kind's own `name` field is confirmed, by the real parse
    ///   dump, to always resolve to the literal `function` KEYWORD token
    ///   itself (`{"type":"function","named":false}`) -- R functions are
    ///   anonymous expressions (`f <- function(x) {...}`/`f = function(x)
    ///   {...}`), never a named declaration; the written name lives one level
    ///   UP, as the enclosing `binary_operator`'s `lhs` child (confirmed:
    ///   `helper <- function(x, y) {...}` parses as one `binary_operator`
    ///   node whose `lhs` is `helper` and whose `rhs` is the nameless
    ///   `function_definition`). This file's generic `name_field` mechanism
    ///   cannot express "read a field on the PARENT node instead" at all, so
    ///   `func_types`'s node kind is fully claimed by
    ///   [`crate::languages::generic::r_quirk`] (mirrors
    ///   `resolve_r_func_name` exactly: walk to the parent `binary_operator`,
    ///   read `lhs`/`left`, falling back to the first named child if neither
    ///   field is present) rather than the generic engine's own
    ///   `name_field`-keyed fallback, which would silently mint every R
    ///   function with the literal name `"function"` were this row to omit a
    ///   quirk (a real bug this worker caught via the parse dump, not
    ///   inspection of `node-types.json` alone -- the field genuinely exists
    ///   and resolves, it just resolves to the wrong node for this grammar's
    ///   idiom).
    /// - `module_types` is empty, NOT baseline's `["program"]`: identical
    ///   rationale to [`Self::solidity`]'s own doc comment -- the C
    ///   baseline's `module_node_types` struct field has zero consumers
    ///   anywhere in that codebase, so `"program"` is dead data there, while
    ///   this crate's own [`crate::languages::generic::walk`] *does* actively
    ///   consume `module_types` (emits a spurious Module symbol off the root
    ///   node's first named child) -- porting it verbatim would invent
    ///   behavior, not reproduce it.
    /// - `call_types` is `["call"]` (matches baseline exactly), and this
    ///   node's `function` field needs NO override for ordinary callee
    ///   reconstruction: the real parse dump confirms `.utf8_text()` on the
    ///   field already yields the correct full written callee for every
    ///   receiver shape this grammar has -- a bare `identifier` (`helper`), a
    ///   `namespace_operator` (`stats::sd`, `box::use`), and an
    ///   `extract_operator` (`Widget$new`) all read back their own exact
    ///   source text through the wrapper with no unwrapping needed (same
    ///   "wrapper's own byte range already spans the full written form"
    ///   reasoning as [`Self::solidity`]'s `call_expression`/`function` doc
    ///   comment). What DOES need a quirk is import detection: `library(x)`/
    ///   `require(x)`/`requireNamespace(x)`/`loadNamespace(x)`/`source(x)`/
    ///   `box::use(pkg/mod)` are ordinary `call`-shaped nodes, not a
    ///   dedicated import statement (mirrors baseline's own
    ///   `r_import_types = {"call"}` being call-shaped, matching its
    ///   dedicated `r_collect_imports`/`parse_r_imports` walker rather than a
    ///   generic field-path extractor) -- [`crate::languages::generic::r_quirk`]
    ///   additionally recognizes these specific callee names off the SAME
    ///   `call` node the generic engine's own call branch already records a
    ///   CallRef for (an `on_unmatched_node` hook fires again after the call
    ///   branch's own push, per [`crate::languages::generic::walk`]'s trailing
    ///   fallthrough call to it for every node kind), pushing an ImportRef
    ///   from the first positional argument's text -- narrower than the
    ///   baseline's own recursive whole-tree scan only in that it does not
    ///   special-case `box::use`'s N-argument fan-out into N separate
    ///   imports (this row records `box::use(pkg/mod)` as one import of
    ///   `"pkg/mod"`, matching every other single-argument import call
    ///   uniformly, since a multi-argument `box::use` is a rarer shape this
    ///   Tier-2 scope does not need to special-case beyond the baseline's own
    ///   single-`box::use` real-world frequency).
    /// - `import_types` is empty (matches baseline's `r_import_types`
    ///   being call-shaped, not syntactic -- see previous bullet).
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "while_statement"]`, matching baseline exactly and confirmed present
    ///   with exactly this shape by the real parse dump (`if_statement`'s
    ///   `condition`/`consequence` fields, `for_statement`'s `variable`/
    ///   `sequence`/`body` fields, `while_statement`'s `condition`/`body`
    ///   fields -- none needed for this row itself since branch counting only
    ///   needs the node KIND, not its fields).
    /// - `call_function_field`/`call_arguments_field`/`name_field`/
    ///   `body_field` are real values (`"function"`/`"arguments"` work
    ///   unmodified for `call_types`; `name_field`/`body_field` are
    ///   placeholders since `func_types` is fully quirk-claimed, same
    ///   "arrays fully claimed elsewhere" posture as [`Self::c`]'s own doc
    ///   note, confined to the func/method arrays only here the same way
    ///   [`Self::dart`]'s doc comment describes for its own analogous case).
    pub const fn r() -> Self {
        Self {
            name: "r",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // See this const's own doc comment: intentionally empty,
            // NOT `["program"]`.
            module_types: &[],
            call_types: &["call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &["if_statement", "for_statement", "while_statement"],
            decorator_types: &[],
            // Never actually consulted for `func_types` -- see this const's
            // own doc comment (`r_quirk` claims `function_definition` in
            // full before this generic engine's own `name_field`/`body_field`
            // fallback would ever run for it).
            name_field: "UNUSED_SEE_R_QUIRK",
            body_field: "UNUSED_SEE_R_QUIRK",
        }
    }

    /// Perl: node kinds verified directly against the `ts-parser-perl` 1.2.1
    /// crate's own `src/node-types.json` (G2.2h grammar onboarding) plus a
    /// real parse-tree dump (`cargo run` against a scratch crate depending on
    /// `ts-parser-perl` directly) -- NOT copied from the baseline's
    /// `perl_func_types`/`perl_module_types`/`perl_call_types`/
    /// `perl_import_types`/`perl_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:588-595), and NOT
    /// bound to the crates.io crate literally named `tree-sitter-perl`
    /// (ganezdragon, v1.1.2) either -- both are real, deliberate departures
    /// this worker's own verification pass found, not oversights:
    /// - Grammar crate choice: `tree-sitter-perl` v1.1.2's OWN published
    ///   manifest (`Cargo.toml`, confirmed via both the crates.io sparse index
    ///   at `https://index.crates.io/tr/ee/tree-sitter-perl` and the
    ///   downloaded `.crate` tarball's normalized `Cargo.toml` -- not merely
    ///   its GitHub repo source, which differs from what was actually
    ///   published) declares `tree-sitter = "0.26.3"` as a genuine, real
    ///   NORMAL (not dev) runtime dependency (`kind: "normal"`, `optional:
    ///   false` in the sparse index) -- incompatible with this workspace's
    ///   `tree-sitter = "0.25"` core (needs `>=0.26.3,<0.27`; workspace needs
    ///   `>=0.25.0,<0.26`, no overlap), and there is only the one published
    ///   version, so no older-version escape hatch exists either. This row
    ///   instead binds `ts-parser-perl` (crates.io, currently 1.2.1, org
    ///   `tree-sitter-perl/tree-sitter-perl` on GitHub, "Actively maintained
    ///   tree-sitter grammar for Perl" per its own `Cargo.toml` description)
    ///   -- the SAME grammar lineage the baseline itself vendors at
    ///   `internal/cbm/vendored/grammars/perl/` (LICENSE line "Copyright 2025
    ///   Avishai \"Veesh\" Goldman" matches this org's repo exactly), just
    ///   published to crates.io under a different crate name than the naive
    ///   `tree-sitter-<lang>` guess. Its OWN manifest depends on
    ///   `tree-sitter-language = "0.1.5"` as the only real (normal) runtime
    ///   dependency (`tree-sitter = "0.25.8"` is a genuine dev-only
    ///   dependency, confirmed the same sparse-index way) -- the same
    ///   ABI-stable-shim pattern every other onboarded grammar in this
    ///   workspace's `Cargo.toml` uses, verified end-to-end by a real `cargo
    ///   build` of all three G2.2h grammars together against this
    ///   workspace's `tree-sitter = "0.25"` core (resolved cleanly to
    ///   `tree-sitter 0.25.10`, no `links` conflict).
    /// - This grammar's own vocabulary is considerably richer/differently
    ///   shaped than the baseline's `perl_*` arrays assume (a "modern Perl"
    ///   grammar with Perl-7-style `class`/`method`/`try` support layered on
    ///   top of the classic surface) -- see each field below for the
    ///   specific corrections, all confirmed by the real parse-tree dump
    ///   after `node-types.json` alone left some field shapes ambiguous
    ///   (e.g. whether a wrapper field's `.utf8_text()` already spans the
    ///   full written callee, which only a real parse confirms).
    /// - `func_types`/`method_types` is `["subroutine_declaration_statement"]`
    ///   (matches baseline's array literally, unlike every other field
    ///   below) -- confirmed present with a real `name` field (type
    ///   `bareword`, always resolves; the dump's bodyless-forward-declaration
    ///   case is out of scope here since every fixture/test scenario has a
    ///   body) and a real `body` field (type `block`) on the SAME node, so
    ///   this flows through the generic engine's own func/method branch with
    ///   NO quirk needed for the base case -- unlike R's identically-shaped-
    ///   looking but actually-broken `name` field (see [`Self::r`]'s own doc
    ///   comment for the contrast). In THIS row's current behavior every
    ///   `sub` is always classified [`crate::parsers::SymbolKind::Function`],
    ///   never [`crate::parsers::SymbolKind::Method`], regardless of lexical
    ///   position: this generic engine's own func/method disambiguation
    ///   (`generic::walk`'s `is_method` check) falls back to "is `enclosing`
    ///   set", and `enclosing` is set only by [`Self::class_types`]'s own
    ///   generic branch, which this row leaves empty -- correctly reflecting
    ///   that Perl's OOP convention (`bless`-based, method dispatch by
    ///   package name, not syntactic class-body nesting) gives no
    ///   `sub`-textually-inside-a-class-body signal to detect in the first
    ///   place (`package Widget; sub new {...}` are lexical SIBLINGS in this
    ///   grammar, confirmed by the real parse dump -- `package_statement`
    ///   and `subroutine_declaration_statement` never nest). `method_types`
    ///   is still populated (rather than left empty) purely so a future G3
    ///   pass that wires up this grammar's OPTIONAL `package_statement`
    ///   block-body form (`package Widget { sub new {...} }`, confirmed
    ///   present via this node kind's own `block`-typed child field) as a
    ///   real [`Self::class_types`] entry would not also need to revisit
    ///   this array -- harmless today, forward-compatible later, same
    ///   "kept populated for a plausible future richer pass" posture as
    ///   several other rows' documented-but-currently-inert array entries.
    /// - `call_types` is
    ///   `["function_call_expression", "ambiguous_function_call_expression",
    ///   "func1op_call_expression", "func0op_call_expression",
    ///   "method_call_expression"]` -- baseline's own four-kind
    ///   `perl_call_types` array (`ambiguous_function_call_expression`,
    ///   `function_call_expression`, `func1op_call_expression`,
    ///   `method_call_expression`) is confirmed present verbatim in this
    ///   grammar too, PLUS `func0op_call_expression` added (a zero-argument
    ///   builtin call -- `time`, `wantarray`, `__PACKAGE__`, ... -- the same
    ///   shape as `func1op_call_expression` one arity down, absent from the
    ///   baseline's own array but present and real in this grammar). Every
    ///   one of these five is fully claimed by
    ///   [`crate::languages::generic::perl_call_override`] rather than the
    ///   generic engine's own single-field reconstruction, for two distinct
    ///   reasons the real parse dump proved out (not assumed from
    ///   `node-types.json` alone):
    ///   - `function_call_expression`/`ambiguous_function_call_expression`'s
    ///     `function` field is a wrapper node (kind literally `"function"`,
    ///     itself with no fields) whose single unfielded child is the real
    ///     callee (a `varname`-bearing leaf for an ordinary `helper(...)`
    ///     call, or the literal builtin keyword token for a builtin like
    ///     `print(...)`/`bless(...)`) -- `.utf8_text()` on the wrapper field
    ///     itself already spans the exact same text (confirmed: `helper(1,
    ///     2)`'s `function` field's own text is exactly `"helper"`), so no
    ///     unwrapping is actually needed, but is still routed through the
    ///     override (rather than the generic engine's own default path) so
    ///     this row's `call_arguments_field` does not have to reconcile
    ///     THREE different field names across the five call-shaped kinds
    ///     (see next two bullets) with one flat `LangSpec` field.
    ///   - `func1op_call_expression`/`func0op_call_expression`'s `function`
    ///     field is not a wrapper at all -- it is the literal, UNNAMED
    ///     builtin-keyword token itself (`{"type":"shift","named":false}` for
    ///     `shift`, `{"type":"length","named":false}` for `length $x`, ...),
    ///     confirmed by the real parse dump to still read back correctly via
    ///     `.utf8_text()` (tree-sitter allows reading an unnamed token
    ///     field's own text the same as any named node); `func1op_call_expression`
    ///     additionally has its single argument as a bare, unfielded child
    ///     (not an `"arguments"` field at all -- confirmed: `length $x`'s
    ///     `$x` is a direct child with no field name), a shape the generic
    ///     engine's own `call_arg_texts` helper (keyed by a single flat
    ///     `call_arguments_field` name) cannot reach.
    ///   - `method_call_expression` splits the receiver and method name into
    ///     TWO separate fields (`invocant`/`method`), the same two-field-split
    ///     shape as Ruby's `call`/Java's `method_invocation` -- confirmed:
    ///     `Widget->new()`'s `invocant` field is `"Widget"` (a `bareword`),
    ///     `$w->draw()`'s `invocant` field is `"$w"` (a `scalar`), and both
    ///     calls' `method` field is a wrapper node (kind `"method"`) whose
    ///     own text already gives the bare method name (`"new"`/`"draw"`)
    ///     with no further unwrapping.
    /// - `module_types` is empty, NOT baseline's `["source_file"]` --
    ///   identical rationale to [`Self::r`]'s own doc comment (the C
    ///   baseline's `module_node_types` field has zero consumers there; this
    ///   crate's own `module_types` is live and would mint a spurious symbol
    ///   off the root node).
    /// - `import_types` is empty, NOT baseline's
    ///   `["use_statement", "require_statement", "require"]`: this grammar
    ///   has NO `require_statement` node kind at all (confirmed absent from
    ///   the full node-kind listing) -- `require Foo::Bar` parses as a plain
    ///   `expression_statement` wrapping a `require_expression`, and a bare
    ///   top-level `require` likewise never surfaces as its own dedicated
    ///   statement kind. `use_statement`'s own `module` field is real (type
    ///   `package`, a fieldless leaf whose own text is the dotted package
    ///   name -- confirmed: `use POSIX qw(floor ceil);`'s `module` field text
    ///   is exactly `"POSIX"`, unaffected by the trailing `qw(...)` import-
    ///   list clause, which is a separate unfielded sibling child this row
    ///   does not need to read at all for Tier-2's "record the imported
    ///   module path" scope) and `require_expression`'s own bareword child
    ///   (confirmed: `require Data::Dumper`'s child text is the FULL
    ///   `"Data::Dumper"`, dotted-package syntax intact) -- both fully
    ///   claimed by [`crate::languages::generic::perl_quirk`] rather than
    ///   this row's own `import_types` array, mirroring the intent of
    ///   baseline's array (`use`/`require` ARE the two real Perl import
    ///   idioms) while routing through this grammar's real node shapes
    ///   instead of the baseline's stale ones.
    /// - `branch_types` is
    ///   `["conditional_statement", "loop_statement", "for_statement",
    ///   "cstyle_for_statement"]`, NOT baseline's
    ///   `["if_statement", "unless_statement", "for_statement",
    ///   "foreach_statement", "while_statement"]`: this grammar has NO
    ///   `if_statement`/`unless_statement`/`while_statement`/
    ///   `foreach_statement` node kinds at all (confirmed absent from the
    ///   full node-kind listing) -- `if (...)  {...} elsif (...) {...} else
    ///   {...}` AND `unless (...) {...}` both parse as the SAME
    ///   `conditional_statement` node kind (distinguished only by its own
    ///   first unnamed child's literal token text, `"if"` vs `"unless"`,
    ///   confirmed by the real parse dump), `while (...) {...}`/`until
    ///   (...) {...}` both parse as the SAME `loop_statement` kind (same
    ///   token-text-only distinction), and `foreach my $j (...) {...}`
    ///   parses as the SAME `for_statement` kind ordinary `for my $i (...)
    ///   {...}` does (again distinguished only by the leading token's own
    ///   text, `"for"` vs `"foreach"`) -- a genuinely different grammar
    ///   shape than the baseline's five-way split assumes, not merely a
    ///   rename. `cstyle_for_statement` (C-style `for (init; cond; incr)
    ///   {...}`, confirmed present with real `initialiser`/`condition`/
    ///   `iterator`/`block` fields) is additionally included since it is a
    ///   real, distinct decision point this baseline array never accounted
    ///   for at all -- omitting it would silently undercount a real branch
    ///   this grammar's own vocabulary makes visible.
    /// - `decorator_types` is empty (matches baseline's `perl_decorator_types`
    ///   not existing at all -- Perl's `:` attribute syntax
    ///   (`sub foo :lvalue {...}`) is a `subroutine_declaration_statement`'s
    ///   own `attributes` field, not a standalone decorator-shaped node any
    ///   existing bespoke extractor or baseline walker treats as a DECORATES
    ///   edge for any language).
    pub const fn perl() -> Self {
        Self {
            name: "perl",
            func_types: &["subroutine_declaration_statement"],
            method_types: &["subroutine_declaration_statement"],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // See this const's own doc comment: intentionally empty,
            // NOT `["source_file"]`.
            module_types: &[],
            call_types: &[
                "function_call_expression",
                "ambiguous_function_call_expression",
                "func1op_call_expression",
                "func0op_call_expression",
                "method_call_expression",
            ],
            // Never actually consulted -- every one of `call_types` above is
            // fully claimed by `perl_call_override` before this generic
            // engine's own single-field reconstruction would ever run. See
            // this const's own doc comment.
            call_function_field: "UNUSED_SEE_PERL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_PERL_CALL_OVERRIDE",
            // See this const's own doc comment: intentionally empty --
            // `use_statement`/`require_expression` IMPORTS are claimed by
            // `perl_quirk` instead, since neither shares one common flat
            // field-name shape with the other.
            import_types: &[],
            branch_types: &[
                "conditional_statement",
                "loop_statement",
                "for_statement",
                "cstyle_for_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Clojure: node kinds verified directly against the `tree-sitter-clojure`
    /// 0.1.0 crate's own `src/node-types.json` (fetched from its real upstream
    /// grammar source, `sogaiu/tree-sitter-clojure` -- the crates.io package
    /// wraps this repo as a git submodule and re-exports its `node-types.json`
    /// verbatim via `include_str!`, confirmed by reading the crate's own
    /// `src/lib.rs`) plus a real parse-tree dump (`cargo run` against a
    /// scratch crate depending on `tree-sitter-clojure` directly) -- NOT
    /// blindly transcribed from the baseline's `clojure_func_types`/
    /// `clojure_module_types`/`clojure_call_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:704-706), though this
    /// language's three arrays matched byte-for-byte on cross-check (a rare
    /// case among this wave's languages where the baseline's own choice
    /// needed no correction at all -- Lisp-family grammars are unusually
    /// uniform: every form is a `list_lit`, so there is very little for a
    /// grammar-version drift to land on). This crate's own [`LangSpec`] has
    /// no `class_types`/`interface_types`/`field_types`/`import_types`/
    /// `branch_types`/`decorator_types` fields with any Clojure-specific
    /// content at all, matching the baseline's row exactly (`empty_types` for
    /// EVERY one of `class_node_types`/`field_node_types`/
    /// `import_node_types`/`import_from_types`/`branching_node_types`/
    /// `decorator_node_types` -- Clojure's baseline row is the only one this
    /// worker found with `branching_node_types` genuinely empty rather than
    /// merely absent-by-omission, since `if`/`when`/`cond`/... are
    /// syntactically indistinguishable `list_lit` calls the baseline itself
    /// makes no attempt to recognize as decision points for complexity
    /// purposes -- matched here rather than "improved on", per this whole
    /// wave's "match the real depth, don't over-build" instruction).
    /// - `func_types` is `["list_lit"]` (matches baseline exactly): a
    ///   `(defn foo [x] ...)` form and an ordinary `(println "x")` call are
    ///   BOTH `list_lit` nodes with no syntactic distinction whatsoever in
    ///   this grammar -- confirmed by the real parse dump, every one of
    ///   `defn`/`def`/`defrecord`/`definterface`/`if`/`when`/`println`/`+`
    ///   parses to the identical `list_lit` node kind, differing only in
    ///   their own first `value`-field child's text (the head symbol). This
    ///   is a genuinely different disambiguation problem than every
    ///   C-family/Java/Python-style language onboarded so far (where a
    ///   function DEFINITION's own node kind is never ambiguous with a plain
    ///   call expression's) -- this file's generic func/method branch checks
    ///   `spec.func_types.contains(&kind)` unconditionally and would mint a
    ///   spurious Function symbol named after `list_lit`'s OWN `name_field`
    ///   lookup (which does not exist as a real field on this node at all)
    ///   for literally every list in the file, defs and plain calls alike,
    ///   were this row to omit a quirk. `list_lit` is instead fully claimed
    ///   by [`crate::languages::generic::clojure_quirk`], which mirrors the
    ///   baseline's own `lisp_is_def_head`/`extract_lisp_def`
    ///   (`internal/cbm/extract_defs.c`:5995-6072) exactly: read the first
    ///   named child's (the head symbol's) own text, check it against the
    ///   SAME fixed keyword table (`defn`/`defn-`/`def`/`defmacro`/
    ///   `defmulti`/`defmethod`/`defprotocol`/`defrecord`/`deftype`/
    ///   `definterface`/`defonce`) restricted to Clojure's own real subset
    ///   (the baseline's table is shared across Clojure/Scheme/Racket/Common
    ///   Lisp/Emacs Lisp/Fennel, several of whose heads -- `define`,
    ///   `define-syntax`, `struct`, ... -- are not real Clojure forms at
    ///   all), and if a match, resolve the definition's own name from the
    ///   SECOND named child (index 1) -- itself unwrapped one level further
    ///   if that second child is ANOTHER `list_lit` (the `(defn (foo args)
    ///   ...)`-style nested-name shape the baseline's own comment documents,
    ///   even though idiomatic Clojure `defn` never actually writes it this
    ///   way -- kept for baseline-parity completeness, harmless as an
    ///   unreachable branch for real-world Clojure). `defrecord`/
    ///   `deftype`/`definterface`/`defprotocol` are recorded as
    ///   [`crate::parsers::SymbolKind::Struct`]/[`crate::parsers::SymbolKind::Interface`]
    ///   respectively rather than a generic Function, mirroring the
    ///   baseline's own `lisp_label` special-casing exactly (confirmed by the
    ///   real parse dump: `(defrecord Point [x y])`'s own second named child
    ///   is the bare symbol `"Point"`, no nested-list unwrapping needed for
    ///   this common shape).
    /// - `call_types` is `["list_lit"]` (matches baseline exactly) -- ALSO
    ///   fully claimed, by
    ///   [`crate::languages::generic::clojure_call_override`], which mirrors
    ///   the baseline's own `extract_lisp_callee`
    ///   (`internal/cbm/extract_calls.c`:497-510) exactly: EVERY `list_lit`'s
    ///   head symbol is recorded as a call callee, unconditionally, with NO
    ///   exclusion for a def-form's own head keyword -- confirmed by reading
    ///   `extract_lisp_callee`'s own source, which has no knowledge of
    ///   `lisp_is_def_head` at all and is invoked from a completely separate
    ///   dispatch path (`extract_calls.c` vs `extract_defs.c`); the baseline
    ///   genuinely records BOTH a Function/Struct/Interface definition named
    ///   `"foo"` AND a call whose callee is the literal string `"defn"` for
    ///   every def-form, since nothing in the real baseline ever filters the
    ///   call side by definition-head-ness. This row matches that literally,
    ///   not "improves" on it, per this whole wave's "reproduce the real
    ///   depth" instruction -- confirmed harmless in practice since no
    ///   caller of this crate's own [`crate::parsers::CallRef::callee`] would
    ///   ever expect a real function literally named `defn`/`def`/`if`/`when`
    ///   to exist to resolve against, the same way a Ruby `require`
    ///   call recorded as BOTH an import and an ordinary call
    ///   ([`Self::ruby`]'s own doc comment) is intentional double-bookkeeping
    ///   rather than a bug. The head symbol's own text already includes any
    ///   namespace-qualifying prefix (confirmed: `(str/join "," xs)`'s head
    ///   `sym_lit` node's own `.utf8_text()` is the full `"str/join"`, since
    ///   `sym_lit`'s `namespace`/`delimiter`/`name` fields are three adjacent
    ///   children spanning the node's own byte range contiguously -- no
    ///   manual `namespace + "/" + name` reconstruction needed, same
    ///   "wrapper's own byte range already spans the full written form"
    ///   reasoning as [`Self::solidity`]'s doc comment). Neither
    ///   `call_function_field` nor `call_arguments_field` is ever actually
    ///   consulted -- `list_lit`'s children live under one `multiple: true`
    ///   `"value"` field (every argument after the head, in written order,
    ///   confirmed by the real parse dump) rather than a single-callee-field
    ///   plus separate-arguments-field split any other onboarded language's
    ///   call-shaped node has, which `child_by_field_name` (returning only
    ///   the FIRST match of a multiple field) cannot enumerate on its own --
    ///   [`crate::languages::generic::clojure_call_override`] instead walks
    ///   `list_lit`'s own NAMED children directly (skipping index 0, the
    ///   head symbol itself) for the argument list, mirroring how the
    ///   baseline's own `extract_lisp_def`/`extract_lisp_callee` both use
    ///   `ts_node_named_child(node, N)` positional indexing rather than any
    ///   field lookup at all for this same reason.
    /// - `module_types` is `["source"]` (matches baseline's
    ///   `clojure_module_types` exactly) -- confirmed present as this
    ///   grammar's actual root node kind by the real parse dump, with no
    ///   fields of its own (matches [`Self::gdscript`]'s identical
    ///   "root node, no name field, no meaningful text" shape) so this row's
    ///   OWN `module_types` field, unlike [`Self::r`]/[`Self::perl`]/
    ///   [`Self::solidity`]/[`Self::gdscript`]'s deliberately-emptied one, is
    ///   left populated exactly as baseline has it: this crate's generic
    ///   `walk`'s own `module_types` branch reads the root's FIRST NAMED
    ///   CHILD's own text as the module's "name" (not the root node's own
    ///   text, which for `source` would be the entire file) -- for a real
    ///   Clojure file whose first top-level form is `(ns app.core ...)`, that
    ///   first named child is the `(ns ...)` form's own `list_lit` node,
    ///   whose OWN text is the entire multi-line namespace-declaration form,
    ///   not a clean module name at all. This is a real, acknowledged
    ///   imprecision (not a silent regression -- the field the baseline
    ///   itself never wires any consumer for either, see [`Self::solidity`]'s
    ///   `module_node_types`-is-dead-in-the-baseline finding) rather than one
    ///   this Tier-2 scope's `ns`-form IMPORTS handling ([`Self::clojure`]'s
    ///   own quirk-hook bullet below) attempts to fix, since a real module
    ///   name IS separately, correctly captured there off the very same
    ///   `(ns app.core ...)` form's own second child text (`"app.core"`) --
    ///   this row keeps `module_types` populated for baseline-parity
    ///   documentation rather than emptying it defensively, since (unlike
    ///   Solidity/GDScript's `source_file`/R/Perl's `source_file`, which are
    ///   the literal WHOLE-FILE root with no redeeming narrower signal even
    ///   in principle) a real, well-formed Clojure namespace declaration is
    ///   commonly the very first form in the file, so the imprecision is
    ///   bounded (a malformed or `ns`-less file's spurious "module name"
    ///   symbol is still a real, if unhelpful, symbol -- never a panic or a
    ///   dropped file).
    /// - `import_types` is empty (matches baseline's
    ///   `clojure_import_from_types = empty_types` -- the baseline's own
    ///   `parse_lisp_imports`/`lisp_process_list`
    ///   (`internal/cbm/extract_imports.c`:1730-1825) is a SEPARATE,
    ///   unconditional whole-tree walk dispatched directly off
    ///   `CBM_LANG_CLOJURE` in `extract_imports.c`'s own `switch`, entirely
    ///   independent of this row's `import_node_types` array -- mirrored here
    ///   by [`crate::languages::generic::clojure_quirk`] (this row's
    ///   `on_unmatched_node` hook, which -- unlike every call/def-shaped
    ///   `list_lit` above -- ALSO fires for a `list_lit` the generic engine's
    ///   own call/func branches already recorded a Def/Call for, since
    ///   [`crate::languages::generic::walk`]'s trailing fallthrough call to
    ///   `on_unmatched_node` runs for every node kind unconditionally, not
    ///   only genuinely-unmatched ones) recognizing the real-world-common
    ///   `(ns app.core (:require [some.ns :as alias] [other.ns]))` form (the
    ///   `ns` head plus each `(:require ...)` clause's own bracketed module
    ///   vectors, confirmed by the real parse dump: `[clojure.string :as
    ///   str]`'s own first named child, `clojure.string`, is a plain
    ///   `sym_lit`) and the plain `(require 'some.ns)` form (a `quoting_lit`
    ///   wrapping a bare `sym_lit`, confirmed present) -- the two most common
    ///   real-world Clojure import idioms, matching this wave's Tier-2
    ///   "defs+calls+imports, no dedicated richer edge" scope rather than
    ///   the baseline's own fuller `(:use ...)`/Common-Lisp-`defpackage`/
    ///   `in-package` machinery (irrelevant to Clojure specifically, which
    ///   never writes those forms).
    /// - `branch_types` is empty (matches baseline's row exactly -- see this
    ///   const's own opening doc-comment paragraph for why this is a
    ///   deliberate match, not an oversight).
    pub const fn clojure() -> Self {
        Self {
            name: "clojure",
            // See this const's own doc comment: fully claimed by
            // `clojure_quirk`, which disambiguates def-forms from plain
            // calls by reading the head symbol against the baseline's own
            // `lisp_is_def_head` keyword table.
            func_types: &["list_lit"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source"],
            // See this const's own doc comment: fully claimed by
            // `clojure_call_override`, which records every `list_lit`'s
            // head symbol as a callee unconditionally (matches the
            // baseline's own unfiltered `extract_lisp_callee` exactly --
            // a def-form's own head keyword, e.g. `"defn"`, is ALSO
            // recorded as a call callee, by design, not a bug).
            call_types: &["list_lit"],
            call_function_field: "UNUSED_SEE_CLOJURE_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_CLOJURE_CALL_OVERRIDE",
            // See this const's own doc comment: intentionally empty --
            // `ns`/`:require`/plain `require` IMPORTS are claimed by
            // `clojure_quirk` instead, mirroring the baseline's own
            // dedicated, `import_node_types`-independent `parse_lisp_imports`
            // walker.
            import_types: &[],
            // See this const's own doc comment: matches the baseline's row
            // exactly -- genuinely empty there too, not merely omitted.
            branch_types: &[],
            decorator_types: &[],
            // Never actually consulted -- `list_lit` is fully claimed by
            // `clojure_quirk` before this generic engine's own
            // `name_field`/`body_field`-keyed fallback would ever run for
            // it. See this const's own doc comment.
            name_field: "UNUSED_SEE_CLOJURE_QUIRK",
            body_field: "UNUSED_SEE_CLOJURE_QUIRK",
        }
    }
    /// Julia: node kinds cross-checked against the baseline's
    /// `julia_func_types`/`julia_class_types`/`julia_module_types`/
    /// `julia_call_types`/`julia_import_types`/`julia_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:728-740`) AND
    /// against the actual `tree-sitter-julia` 0.23.1 crate's own
    /// `src/node-types.json` PLUS a real parse tree dump (a scratch
    /// `cargo run` against a minimal crate depending on
    /// `tree-sitter-julia` directly) -- G2.2d grammar onboarding. This
    /// grammar's function/struct/call-shaped nodes are ENTIRELY
    /// unfielded (every one of them has `"fields": {}` in
    /// `node-types.json` -- confirmed for `function_definition`,
    /// `struct_definition`, `abstract_definition`,
    /// `primitive_definition`, `call_expression`,
    /// `broadcast_call_expression`, `assignment`, `import_statement`,
    /// `using_statement`, `export_statement` alike), so this row has the
    /// same "every array exists purely to document the vocabulary at a
    /// glance; every one of them is fully claimed by
    /// [`crate::languages::generic::julia_quirk`]/
    /// [`crate::languages::generic::julia_call_override`] before the
    /// generic engine's own `name_field`/`call_function_field`-keyed
    /// fallback paths would ever run" posture as [`Self::c`]/[`Self::cpp`]/
    /// [`Self::objc`] -- `name_field`/`call_function_field`/
    /// `call_arguments_field`/`body_field` below are consequently
    /// placeholders, never actually consulted.
    /// - `func_types` is `["function_definition"]` only, NOT baseline's
    ///   `{"function_definition", "short_function_definition",
    ///   "assignment", NULL}`: this grammar version has NO
    ///   `short_function_definition` node kind at all (confirmed absent
    ///   from `node-types.json` and from every real parse tree dumped --
    ///   Julia's short-form `square(x) = x * x` parses as a plain
    ///   `assignment` whose LHS happens to be a `call_expression`, exactly
    ///   as the baseline's OWN source comment directly above
    ///   `julia_func_types` in `lang_specs.c` already documents: "the
    ///   resolver names it only when the LHS is a call, so plain `x = 5`
    ///   is not a def" -- so `assignment` is deliberately NOT listed in
    ///   this row's `func_types` either (the flat-array mechanism cannot
    ///   express "only when your LHS is call-shaped"); instead
    ///   [`crate::languages::generic::julia_quirk`]'s own `"assignment"`
    ///   arm applies that exact same LHS-is-a-call gate directly, via the
    ///   walker's single final catch-all (`assignment` is not itself one
    ///   of `class_types`/`field_types`/... above it, so there is no
    ///   double-invocation risk the way [`Self::c`]'s `struct_specifier`
    ///   arm has to guard against).
    /// - `method_types` is empty: Julia has no lexical class-body
    ///   nesting for functions to be "inside" of at all -- multiple
    ///   dispatch means every method of a generic function (`f(x::Int)`,
    ///   `f(x::String)`, ...) is its own top-level `function_definition`/
    ///   `assignment`, never nested inside the struct it dispatches on
    ///   (there is no `impl`/`class` body equivalent in this language) --
    ///   matches the baseline itself having no Julia-specific
    ///   method/DEFINES-container concept anywhere in
    ///   `internal/cbm/extract_defs.c` either.
    /// - `class_types` is `["struct_definition", "abstract_definition",
    ///   "primitive_definition"]`, matching baseline's `julia_class_types`
    ///   exactly.
    /// - `module_types` is `["module_definition"]`, NOT baseline's
    ///   `julia_module_types = {"source_file", NULL}`: identical
    ///   "baseline's own `module_node_types` field has zero consumers in
    ///   that C codebase, so porting a dead-there value into this crate's
    ///   OWN live `module_types` consumer would invent behavior rather
    ///   than reproduce it" finding [`Self::solidity`]/[`Self::gdscript`]/
    ///   [`Self::dart`]'s own doc comments already record -- `source_file`
    ///   is simply this grammar's root node, not a module-name-bearing
    ///   one. This grammar's real `module MyMod ... end` construct is a
    ///   DIFFERENT, distinctly-named `module_definition` node kind that
    ///   DOES carry a genuine `"name"` field (confirmed in
    ///   `node-types.json`: `{"type":"module_definition","fields":{"name":
    ///   {"types":[{"type":"identifier"},{"type":
    ///   "interpolation_expression"}]}}}`) -- a strictly better, live
    ///   module-name source than the dead baseline entry, so it is used
    ///   here instead. Flows through the generic engine's own
    ///   `module_types` branch unchanged (real `name_field`-equivalent
    ///   lookup via `first_named_child_text`, which finds this field's
    ///   `identifier` child directly since it is the node's first named
    ///   child either way) -- no quirk needed for this one kind.
    /// - `call_types` is `["call_expression", "broadcast_call_expression"]`,
    ///   matching baseline's `julia_call_types` exactly, but BOTH are
    ///   fully claimed by [`crate::languages::generic::julia_call_override`]
    ///   rather than the generic engine's own single-field
    ///   reconstruction: `call_expression`'s callee is its own first
    ///   (unfielded) child (an `identifier` for a bare call, a
    ///   `field_expression` for `obj.draw(...)`/`Base.show(...)`, verified
    ///   against a real parse tree), with `argument_list` as an unfielded
    ///   sibling -- and `broadcast_call_expression`'s shape is even more
    ///   irregular (`map.(x, y)` parses as positional
    ///   `[identifier, ".", argument_list]` children, no wrapping
    ///   `call_expression` at all, confirmed via the same parse tree
    ///   dump), so a single flat field name cannot address either kind.
    /// - `import_types` is `["import_statement", "using_statement",
    ///   "export_statement"]`, matching baseline's `julia_import_types`
    ///   exactly (baseline additionally lists a bare `"import"` keyword
    ///   token, which -- like every other language row's own dropped bare
    ///   keyword-text entries, see [`Self::gdscript`]'s doc comment for
    ///   the identical finding -- can never be a NAMED node's own
    ///   `.kind()` in a real parse tree, so it is omitted here). All
    ///   three are positional-children-only (`node-types.json`:
    ///   `"fields": {}` for every one), holding one or more
    ///   `identifier`/`import_path`/`selected_import`/`scoped_identifier`
    ///   children -- claimed by
    ///   [`crate::languages::generic::julia_quirk`] rather than the
    ///   generic engine's own minimal import handling (which defers
    ///   entirely to the quirk hook for every language, see `walk`'s own
    ///   `import_types` branch doc comment) since a real IMPORTS edge
    ///   needs the actual module-path text, not just "this node exists".
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "while_statement", "try_statement"]`, matching baseline's
    ///   `julia_branch_types` exactly (confirmed present via a real parse
    ///   tree dump for every one, including `try_statement` for
    ///   `try ... catch ... end`).
    pub const fn julia() -> Self {
        Self {
            name: "julia",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &[
                "struct_definition",
                "abstract_definition",
                "primitive_definition",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["module_definition"],
            call_types: &["call_expression", "broadcast_call_expression"],
            call_function_field: "UNUSED_SEE_JULIA_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_JULIA_CALL_OVERRIDE",
            import_types: &["import_statement", "using_statement", "export_statement"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "try_statement",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::cpp()`/
            // `LangSpec::objc()`).
            name_field: "UNUSED_SEE_JULIA_QUIRK",
            body_field: "UNUSED_SEE_JULIA_QUIRK",
        }
    }
    /// Odin: node kinds cross-checked against the baseline's
    /// `odin_func_types`/`odin_class_types`/`odin_field_types`/
    /// `odin_call_types`/`odin_import_types`/`odin_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1153-1164`) AND
    /// against the actual `tree-sitter-odin` 1.3.0 crate's own
    /// `src/node-types.json` PLUS a real parse tree dump -- G2.2d
    /// grammar onboarding. The baseline gives Odin no dedicated
    /// `extract_base_classes` walker (`internal/cbm/extract_defs.c`'s
    /// dedicated-walker list at :2377-2394 covers TS/TSX, PHP, Kotlin,
    /// Squirrel, Julia, F#, D, PowerShell, Pascal -- Odin is absent, this
    /// worker's own brief already anticipated Odin "likely uses the
    /// generic fallback"), and indeed Odin has no classical
    /// inheritance/subtyping syntax at all (no `class`/`extends`
    /// equivalent) -- but it DOES have a real, grammar-visible
    /// COMPOSITION idiom (`using base: Animal` inside a struct body,
    /// confirmed via a real parse tree dump: a `field` node whose first
    /// positional child is the literal `using` keyword token) directly
    /// analogous to Go's embedded-struct-field INHERITS signal
    /// ([`Self::go`]'s own `struct_item`/`go_struct_fields` handling) --
    /// wired as an INHERITS edge by
    /// [`crate::languages::generic::odin_quirk`] for that reason, an
    /// improvement over "no heritage signal at all" the "match the
    /// baseline's real depth" instruction does not forbid when the
    /// baseline simply never modeled Odin's heritage-equivalent construct
    /// to begin with (same posture [`Self::zig`]'s own `@import`
    /// IMPORTS-detection doc comment already establishes for "the
    /// baseline has a gap, not a deliberate design choice").
    /// - `func_types` is `["procedure_declaration"]` only, NOT baseline's
    ///   `{"procedure_declaration", "overloaded_procedure_declaration",
    ///   NULL}`: a real parse tree shows `overloaded_procedure_declaration`
    ///   (Odin's `f :: proc{f_int, f_string}` explicit-overload-group
    ///   syntax) has no fields at all and wraps a LIST of existing
    ///   `identifier` references to other, already-separately-declared
    ///   procedures -- it is a second, alternate NAME for an existing
    ///   group of procs, not a new procedure body of its own to walk, so
    ///   there is no new Function symbol/body for it to usefully claim
    ///   (unlike `test_declaration`'s case in [`Self::zig`], there is no
    ///   analogous "this needs its own symbol despite the unusual
    ///   shape" finding here) -- deliberately left unclaimed rather than
    ///   guessed at.
    /// - `method_types` is empty: Odin has no lexical class-body nesting
    ///   either (same "no `impl`/class body construct at all" finding as
    ///   [`Self::julia`]'s own `method_types` doc comment) -- every
    ///   `proc` is syntactically top-level, with any struct/receiver
    ///   association expressed only as an ordinary typed parameter
    ///   (`bark :: proc(d: ^Dog) {...}`), never as a receiver clause or
    ///   nesting the way Go's `func (w *Widget) Draw()` is -- confirmed
    ///   via a real parse tree dump showing no DEFINES-container signal
    ///   to hang an [`Quirks::on_method_defined`] hook on the way
    ///   [`Self::go`]'s receiver-clause case has.
    /// - `class_types` is `["struct_declaration", "enum_declaration",
    ///   "union_declaration"]`, NOT baseline's full
    ///   `{"struct_declaration", "enum_declaration", "union_declaration",
    ///   "package_declaration", NULL}`: `package_declaration` (Odin's
    ///   `package main` file-header clause) is not a product-type/class
    ///   shape at all -- it carries the file's own package name, which
    ///   this crate has no natural symbol kind for and the baseline's own
    ///   `extract_defs.c` never turns into a Class-shaped definition
    ///   either (folding it into `class_types` in the C array appears to
    ///   be a baseline categorization convenience, not a real semantic
    ///   claim); omitted here rather than mis-typed as a Class.
    /// - `field_types` is `["field"]`, NOT baseline's
    ///   `{"field_declaration", NULL}`: this grammar's real struct-field
    ///   node kind is `field` (confirmed absent from `node-types.json`:
    ///   `field_declaration` does not exist anywhere in this grammar
    ///   version at all) -- porting the baseline's stale name verbatim
    ///   would make every struct field invisible to DEFINES, not just
    ///   imprecisely classified, the same class of finding this worker's
    ///   own brief warned to check for. `field`'s own `name`/`type`
    ///   fields do NOT exist either (confirmed: `{"type":"field",
    ///   "fields":{}}`, fully positional -- a bare `identifier` (or, for
    ///   the `using base: Animal` composition case, a `using` keyword
    ///   token then the `identifier`) followed by `:` and a `type` node)
    ///   -- claimed in full by [`crate::languages::generic::odin_quirk`]
    ///   rather than this row's ordinary flat-array
    ///   `field_types`/`name_field` fallback (same "arrays exist to
    ///   document the vocabulary, quirk claims the real shape" posture as
    ///   [`Self::c`]), which also distinguishes the `using`-prefixed case
    ///   (pushed as an INHERITS edge, see this const's own doc comment
    ///   above) from an ordinary field (pushed as a DEFINES edge).
    /// - `call_types` is `["call_expression", "selector_call_expression"]`,
    ///   matching baseline's `odin_call_types` exactly -- BOTH confirmed
    ///   present via a real parse tree dump, though `selector_call_expression`
    ///   is considerably rarer than a first guess would suggest: it is
    ///   Odin's POINTER-dereference method-call syntax specifically
    ///   (`p->bark()`, `p` a `^Dog`-typed pointer) -- NOT the ordinary
    ///   `obj.draw()` dot-call idiom on a plain (non-pointer) value, which
    ///   instead parses as an entirely different `member_expression` node
    ///   wrapping a nested, ordinary `call_expression` (confirmed: `obj.draw()`
    ///   dumps as `member_expression{identifier "obj", ".",
    ///   call_expression{[field=function] identifier "draw", ...}}` --
    ///   `member_expression` is NOT itself call-shaped and is NOT one of
    ///   this row's `call_types`, so a plain dot-call is recorded as the
    ///   INNER `call_expression`'s own ordinary callee text `"draw"`
    ///   alone, with no receiver captured at all for this grammar's
    ///   specific `member_expression`-wrapping shape -- left as a known,
    ///   documented gap for a future richer-tier pass rather than a
    ///   guessed-at receiver hint, since [`receiver_of_call`]'s own shared
    ///   node-kind dispatch has no `"member_expression"` arm and adding
    ///   one is beyond this wave's "structural extraction, not a new
    ///   receiver-hint convention" scope). Both confirmed node kinds DO
    ///   have a real, working `"function"` field on their innermost
    ///   `call_expression` (unlike Julia/Pascal's entirely unfielded call
    ///   nodes) -- but `call_expression`'s OWN arguments are exposed as a
    ///   REPEATED `"argument"` field (singular, `multiple: true` in
    ///   `node-types.json`), not a single `"arguments"` wrapper field the
    ///   way this row's `call_arguments_field` default and the generic
    ///   engine's shared `call_arg_texts` helper both assume (that helper
    ///   calls `child_by_field_name` once and iterates ITS OWN children --
    ///   for a repeated field, `child_by_field_name` only ever returns the
    ///   FIRST match, a bare argument expression with no children of its
    ///   own to iterate, so every Odin call's `arg_texts` would silently
    ///   come back empty through the unmodified generic path, a real bug
    ///   caught by writing a probe that dumped `helper(1, 2)`'s own real
    ///   field tags rather than trusting `node-types.json`'s field LIST
    ///   alone) -- so BOTH `call_expression` (fixing up
    ///   [`crate::languages::generic::odin_call_arg_texts`]'s own
    ///   repeated-field read) and `selector_call_expression` (whose OWN
    ///   top-level `"function"` field holds the pointer identifier, not
    ///   the eventual method name -- the real callee, `bark`, is one level
    ///   down inside its nested `call_expression` child) are fully claimed
    ///   by [`crate::languages::generic::odin_call_override`].
    /// - `import_types` is `["import_declaration"]` only, NOT baseline's
    ///   `{"import_declaration", "import", "using_statement", NULL}`:
    ///   bare `"import"` is a dropped keyword-token entry (same
    ///   never-a-named-node's-own-kind finding as
    ///   [`Self::julia`]/[`Self::gdscript`]'s own identical doc-comment
    ///   findings), and `using_statement` in THIS grammar is Odin's
    ///   `import`-namespace-flattening statement (`using fmt` after
    ///   `import "core:fmt"`, bringing a package's exports into scope
    ///   unqualified) -- a real, distinct statement kind, but one with no
    ///   module PATH of its own to record as an IMPORTS edge (it names an
    ///   already-imported package's local alias, not a new path) --
    ///   correctly excluded rather than guessed at; the field-embedding
    ///   `using` keyword (this const's own doc comment above, inside a
    ///   struct body) is a syntactically unrelated use of the same
    ///   keyword the grammar happens to reuse for two different
    ///   constructs, confirmed distinct via `node-types.json` (`field`
    ///   vs. `using_statement` are two entirely separate node kinds, never
    ///   conflated in a real parse tree).
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "switch_statement"]`, NOT baseline's full `{"if_statement",
    ///   "for_statement", "switch_statement", "when_statement", NULL}`:
    ///   `when_statement` (Odin's COMPILE-TIME `when cond {...} else
    ///   {...}` conditional-compilation directive, confirmed present via
    ///   a real parse tree dump) is deliberately excluded from cyclomatic
    ///   complexity/decision-point counting here -- it is resolved at
    ///   compile time from build-configuration constants
    ///   (`ODIN_OS`/`ODIN_ARCH`/...), producing exactly ONE of its
    ///   branches in the final binary, so counting it as a runtime
    ///   decision point would overstate a function's real cyclomatic
    ///   complexity the same way a C preprocessor `#ifdef` is not counted
    ///   as one either (this crate's own [`Self::c`]/[`Self::cpp`] rows
    ///   have no `branch_types` entry for `#if`/`#ifdef` preprocessor
    ///   conditionals for the identical reason) -- complexity extraction
    ///   is deferred entirely for this wave regardless (see this row's
    ///   own module-level "no `complexity_language` wiring this wave"
    ///   convention), so this exclusion is a documented design choice for
    ///   whichever future wave DOES wire Odin's `NodeKindTable`, not a
    ///   silent omission today.
    pub const fn odin() -> Self {
        Self {
            name: "odin",
            func_types: &["procedure_declaration"],
            method_types: &[],
            class_types: &[
                "struct_declaration",
                "enum_declaration",
                "union_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["field"],
            module_types: &[],
            call_types: &["call_expression", "selector_call_expression"],
            call_function_field: "UNUSED_SEE_ODIN_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_ODIN_CALL_OVERRIDE",
            import_types: &["import_declaration"],
            branch_types: &["if_statement", "for_statement", "switch_statement"],
            decorator_types: &[],
            // Never actually consulted for `func_types`/`class_types`/
            // `field_types` (all fully claimed by
            // `crate::languages::generic::odin_quirk`) -- see this
            // const's own doc comment.
            name_field: "UNUSED_SEE_ODIN_QUIRK",
            body_field: "UNUSED_SEE_ODIN_QUIRK",
        }
    }
    /// Pascal: node kinds cross-checked against the baseline's
    /// `pascal_func_types`/`pascal_class_types`/`pascal_field_types`/
    /// `pascal_call_types`/`pascal_import_types`/`pascal_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1058-1069`) AND
    /// against the actual `tree-sitter-pascal` 0.10.2 crate's own
    /// `src/node-types.json` PLUS a real parse tree dump -- G2.2d grammar
    /// onboarding. This grammar's node-kind names (`declProc`,
    /// `declClass`, `exprCall`, `kIf`, ...) are the SAME distinctive
    /// camelCase-prefixed vocabulary the baseline's own `pascal_*` arrays
    /// already use verbatim, confirming this crate binds the identical
    /// grammar lineage the baseline vendored (`Isopod/tree-sitter-pascal`)
    /// -- most of baseline's own array entries verified correct outright;
    /// see the specific corrections below for the few that did not.
    /// `internal/cbm/extract_defs.c`'s dedicated `extract_base_classes`
    /// Pascal walker (:2513-2542: collect every `declClass` child field
    /// literally named `"parent"`, skipping the `(`/`)` delimiter tokens
    /// which the grammar ALSO tags `parent`, via `ts_node_is_named`) is
    /// reproduced byte-for-byte by
    /// [`crate::languages::generic::pascal_quirk`]'s own `declClass`
    /// handling -- confirmed via a real parse tree dump of
    /// `TDog = class(TAnimal, IFoo)`: `declClass`'s children are exactly
    /// `[kClass, [field=parent] "(", [field=parent] typeref "TAnimal",
    /// [field=parent] ",", [field=parent] typeref "IFoo", [field=parent]
    /// ")", ...]`, i.e. the delimiter tokens ARE unnamed (`is_named() ==
    /// false`) while the two `typeref` heritage entries are named, exactly
    /// the shape the baseline's own filter assumes.
    /// - `func_types`/`method_types` are BOTH `["declProc"]` (mirrors
    ///   [`Self::rust`]'s identical `function_item`-in-both-arrays
    ///   convention: this grammar gives a class-body method declaration
    ///   -- `procedure Bark;` nested inside `declClass` -- and a
    ///   free-standing procedure/function the SAME node kind, so the
    ///   generic walker's own nesting-based Function-vs-Method
    ///   classification, not array membership alone, tells them apart),
    ///   NOT baseline's `pascal_func_types = {"defProc", "declProc",
    ///   NULL}`: a real parse tree shows `defProc` is instead the
    ///   OUT-OF-BODY implementation wrapper (`procedure TDog.Bark; begin
    ///   ... end;` in a unit's `implementation` section) -- it has real
    ///   `"header"` (a NESTED `declProc`) and `"body"` fields, but no
    ///   `"name"` field of its own at all (confirmed:
    ///   `{"type":"defProc","fields":{"body":{...},"header":{...}}}`) --
    ///   listing it directly in `func_types` would make `child_text(node,
    ///   spec.name_field, ...)` fail for every `defProc` (same "the name
    ///   lives on a nested node, not this one" finding [`Self::dart`]'s
    ///   own `function_declaration`/`function_signature` doc comment
    ///   already records for an analogous split), silently orphaning
    ///   every implementation body from its own `fn_scope` the exact way
    ///   that doc comment warns about. `declProc` alone already reaches
    ///   the ordinary, in-place case correctly (a bodyless
    ///   interface-section forward declaration, `procedure DoWork;`,
    ///   confirmed real `"name"` field, flows through the generic
    ///   engine's own default func/method branch unchanged); `defProc`
    ///   (the real implementation body) is handled separately by
    ///   [`crate::languages::generic::pascal_quirk`]'s own `defProc` arm,
    ///   which reads the nested `header` `declProc`'s `"name"` field
    ///   directly (confirmed this CAN be a `genericDot` node rather than a
    ///   bare `identifier` for an out-of-line class-method implementation,
    ///   e.g. `TDog.Bark` -- this crate's own [`child_text`] helper still
    ///   returns the WHOLE dotted text in that case, since it reads the
    ///   field node's own byte range, spanning the entire `genericDot`
    ///   subtree) and classifies Method-vs-Function by whether that name
    ///   contains a `.` (mirrors [`Self::cpp`]'s own `Class::method`
    ///   out-of-line-scoping DEFINES convention), then walks the OUTER
    ///   `defProc`'s own `"body"` field with that resolved name as
    ///   `fn_scope` -- mirroring
    ///   [`crate::languages::generic::dart_walk_function_body`]'s
    ///   identical "read the nested signature's name, walk the outer
    ///   node's own body" mechanics. This grammar ALSO gives
    ///   `function Add(...): Integer;` the identical `declProc`/`defProc`
    ///   node-kind pair (distinguished only by a `kFunction` vs.
    ///   `kProcedure` leading keyword token, confirmed via a real parse
    ///   tree dump) -- so this row naturally covers Pascal's `function`s
    ///   too with no separate array entry needed, an
    ///   unremarked-but-verified assumption the baseline's own choice of
    ///   `declProc` for BOTH keywords already implies.
    /// - `class_types` is `["declClass", "declIntf", "declHelper",
    ///   "declObject", "declRecord"]`, matching baseline's
    ///   `pascal_class_types` exactly (every one confirmed present in
    ///   `node-types.json`; `declRecord` additionally confirmed via a real
    ///   parse tree of `TPoint = record ... end` -- Pascal's `record`
    ///   keyword gets a DIFFERENT node kind than `class`'s `declClass`,
    ///   unlike e.g. Rust's `struct`/`class`-equivalent sharing one kind).
    ///   Every one of these five DOES carry a real `"name"` field on its
    ///   OWN node... except it does not: a real parse tree shows the name
    ///   actually lives one level UP, on the WRAPPING `declType` node's own
    ///   `"name"` field (`TDog = class(...) ... end` parses as
    ///   `declType{[field=name] identifier "TDog", "=", [field=type]
    ///   declClass{...}}` -- `declClass` itself has NO name field, only a
    ///   `"parent"`-tagged heritage list) -- so, like
    ///   [`Self::zig`]'s anonymous struct/enum/union naming-from-parent
    ///   finding, [`crate::languages::generic::pascal_quirk`] fully claims
    ///   every one of these five kinds (reads the PARENT `declType`'s
    ///   `"name"` field directly) rather than relying on this row's
    ///   ordinary flat-array `name_field` fallback, which would find
    ///   nothing for any of them.
    /// - `field_types` is `["declField", "declProp"]`, matching baseline's
    ///   `pascal_field_types` exactly -- BOTH confirmed to carry real,
    ///   directly-readable `"name"` fields (no quirk needed for the base
    ///   DEFINES case; `declProp`'s own `"getter"`/`"setter"` fields,
    ///   Pascal property accessor bindings, are additionally available
    ///   but out of this wave's structural-extraction scope, same
    ///   "deferred to a future richer-tier pass" posture as every other
    ///   G2 language's unclaimed extra fields).
    /// - `call_types` is `["exprCall", "exprDot"]`, NOT baseline's
    ///   `{"exprCall", NULL}` alone: a real parse tree of
    ///   `Obj.Draw;` (a bare, PARENLESS method-style call -- extremely
    ///   common idiomatic Pascal, since empty-argument-list parens are
    ///   OPTIONAL) shows this is `exprDot` (`{[field=lhs] identifier "Obj",
    ///   [field=operator] kDot ".", [field=rhs] identifier "Draw"}`), a
    ///   node kind the baseline's own `pascal_call_types` array never
    ///   lists at all -- so every parenless-dot-call statement in real
    ///   Pascal source is entirely invisible to the baseline's own
    ///   extractor, a genuine baseline gap (not a deliberate design
    ///   choice: `internal/cbm/extract_calls.c` has no Pascal-specific
    ///   dispatch case either, so `exprDot` is not deliberately excluded
    ///   there, simply never considered) this row closes rather than
    ///   reproduces, matching this worker's own brief's "if truly a gap
    ///   rather than a deliberate choice, an improvement is not
    ///   forbidden" allowance (same posture [`Self::zig`]'s `@import`
    ///   IMPORTS-detection doc comment already establishes). A CRITICAL
    ///   caveat found via that same parse tree: `exprDot` is ALSO Pascal's
    ///   ordinary, non-call MEMBER-ACCESS node (`Obj.Field`, a plain field
    ///   read with no call semantics at all) -- syntactically
    ///   indistinguishable from a parenless call at the `exprDot` node
    ///   level alone (both are exactly
    ///   `{identifier, ".", identifier}`); this crate follows the same
    ///   "record it as a call anyway" convention
    ///   [`crate::languages::generic::gdscript_call_override`]'s own
    ///   `base_call`-vs-plain-field-read ambiguity note and
    ///   [`Self::julia`]'s `assignment`-LHS-is-a-call gate both already
    ///   accept as an unavoidable false-positive/negative trade-off
    ///   inherent to a statement-position-only static heuristic (a real
    ///   type-checker would be needed to fully disambiguate) -- every
    ///   `exprDot` is recorded as a call by
    ///   [`crate::languages::generic::pascal_call_override`] regardless,
    ///   matching this crate's every-other-language bias toward
    ///   over-recording a call edge rather than silently dropping a real
    ///   one (a spurious CALLS edge to a field name is a strictly cheaper
    ///   mistake than a missing one for this crate's downstream
    ///   consumers, same as every "no type-checker, syntax-only" caveat
    ///   this crate's other quirks already carry). `exprCall`'s own
    ///   `"entity"` field (the callee) can ALSO be an `exprDot` (for
    ///   `Exception.Create('bad')`'s dotted callee, confirmed via a real
    ///   parse tree) -- an ordinary, correctly-nested case needing no
    ///   special handling beyond what `exprCall`'s own quirk arm already
    ///   does (reads `"entity"`'s own text, which spans the whole dotted
    ///   expression either way). Both `exprCall`'s `"entity"`/`"args"`
    ///   field NAMES (not `"function"`/`"arguments"`, this row's own
    ///   defaults) and `exprDot`'s entirely different `"lhs"`/`"rhs"` shape
    ///   are why both kinds are fully claimed by
    ///   [`crate::languages::generic::pascal_call_override`] rather than
    ///   the generic engine's own single-field reconstruction (this row's
    ///   `call_function_field`/`call_arguments_field` below are
    ///   consequently placeholders, never actually consulted).
    /// - `import_types` is `["declUses"]`, matching baseline's
    ///   `pascal_import_types` exactly -- confirmed via a real parse tree
    ///   that a single `uses SysUtils, Classes, Foo.Bar;` clause holds
    ///   MULTIPLE positional `moduleName` children (one per comma-
    ///   separated unit, each itself either a bare `identifier` or a
    ///   dotted `identifier "." identifier` pair for a namespaced unit
    ///   name like `Foo.Bar`) -- claimed by
    ///   [`crate::languages::generic::pascal_quirk`] (walks every
    ///   `moduleName` child directly) rather than the generic engine's own
    ///   minimal import handling, for the same "a real IMPORTS edge needs
    ///   the actual path text" reason [`Self::julia`]'s own `import_types`
    ///   doc comment gives.
    /// - `branch_types` is `["if", "ifElse", "while", "repeat", "for",
    ///   "foreach", "try", "case"]`, matching baseline's
    ///   `pascal_branch_types` exactly -- every one confirmed present via
    ///   a real parse tree dump, INCLUDING confirming `if` (bare, no
    ///   `else` clause) and `ifElse` (with one) really are two DISTINCT
    ///   node kinds in this grammar (not one `if` kind with an optional
    ///   field) -- both counted as decision points equally once a future
    ///   wave wires `NodeKindTable` for this language (deferred this wave,
    ///   see this row's own module-level "no `complexity_language` wiring
    ///   this wave" convention), same as every other language's
    ///   `branch_types` array already treats an if/if-else pair as
    ///   equally one decision point apiece.
    /// - `module_types` is empty, NOT baseline's
    ///   `pascal_module_types = {"source_file", NULL}`: THIS specific
    ///   grammar's actual root node kind is literally named `"root"`, not
    ///   `"source_file"` at all (confirmed in `node-types.json`:
    ///   `{"type":"root","root":true,...}`) -- so baseline's own value is
    ///   not merely dead-data-if-ported (the recurring finding
    ///   [`Self::solidity`]/[`Self::gdscript`]/[`Self::dart`]/[`Self::julia`]'s
    ///   own doc comments already establish for OTHER languages' honestly
    ///   dead `module_node_types` baseline entries) but ADDITIONALLY,
    ///   independently wrong for this grammar version specifically: no
    ///   node kind named `source_file` exists anywhere in this grammar at
    ///   all, so porting it verbatim would not even silently do nothing --
    ///   it would simply never match, same practical outcome but a
    ///   distinct root cause worth recording precisely. This grammar's
    ///   real module-name-bearing construct is `unit MyUnit;`'s own
    ///   `moduleName` child (a genuinely positional, unfielded child of
    ///   the `unit`/`program` node, confirmed: neither `unit` nor
    ///   `program` has ANY fields at all in `node-types.json`) -- handled
    ///   by [`crate::languages::generic::pascal_quirk`]'s own `unit`/
    ///   `program` arms (which push a Module symbol from the `moduleName`
    ///   child's own text, itself either a bare `identifier` or a dotted
    ///   `identifier "." identifier` pair for a namespaced unit) rather
    ///   than this row's `module_types` array, which -- unlike
    ///   [`Self::julia`]'s live `module_definition` alternative -- has no
    ///   live, directly-fielded candidate node to point at here at all.
    pub const fn pascal() -> Self {
        Self {
            name: "pascal",
            // `declProc` is BOTH `func_types` AND `method_types` (mirrors
            // `LangSpec::rust`'s identical `function_item` convention):
            // this grammar uses the ONE node kind for a class-body method
            // declaration (`procedure Bark;` nested inside `declClass`)
            // AND a free-standing procedure/function alike -- the
            // generic walker's own nesting-based Function-vs-Method
            // split (`walk`'s `is_method` check, which falls back to
            // "is `enclosing` set" precisely because a kind shared by
            // both arrays cannot be classified from the array membership
            // alone) then does the right thing for both shapes: a
            // top-level `procedure DoWork;` (Function, `enclosing` is
            // `None`) and a `procedure Bark;` inside a `declClass`'s own
            // body (Method, `enclosing` is `Some("TDog")` once
            // `pascal_walk_decl_type`'s own scoped recursion sets it).
            func_types: &["declProc"],
            method_types: &["declProc"],
            class_types: &[
                "declClass",
                "declIntf",
                "declHelper",
                "declObject",
                "declRecord",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["declField", "declProp"],
            module_types: &[],
            call_types: &["exprCall", "exprDot"],
            call_function_field: "UNUSED_SEE_PASCAL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_PASCAL_CALL_OVERRIDE",
            import_types: &["declUses"],
            branch_types: &[
                "if", "ifElse", "while", "repeat", "for", "foreach", "try", "case",
            ],
            decorator_types: &[],
            // `"name"`/`"body"` ARE real, consulted field names --
            // UNLIKE `LangSpec::c()`/`LangSpec::cpp()`/`LangSpec::objc()`/
            // `LangSpec::julia()`'s identically-worded placeholders, this
            // row is only PARTIALLY quirk-claimed: `declProc` (this row's
            // sole `func_types`/`method_types` entry) has a genuine
            // `"name"` field of its own (confirmed in `node-types.json`),
            // so it flows through the generic engine's own default
            // func/method branch completely unchanged -- no quirk
            // involvement at all for the ordinary, in-place
            // forward-declaration/bodyless-signature case (only the
            // SEPARATE `defProc` out-of-body-implementation wrapper kind
            // needs `crate::languages::generic::pascal_quirk`'s own
            // `defProc` arm, precisely because ITS OWN name is not a
            // field on itself at all -- see this const's own doc
            // comment). `declField`/`declProp` (`field_types`) likewise
            // both have genuine `"name"` fields and need no quirk either.
            // `body_field` is simply never found for `declProc` (a
            // bodyless declaration has no `"body"` field to match,
            // exactly like `LangSpec::zig()`'s own `function_signature`
            // case) -- not a placeholder, just correctly absent on that
            // specific node in practice. Only `class_types` (`declClass`/
            // `declIntf`/`declHelper`/`declObject`/`declRecord`, none of
            // which has ANY field of its own, name included -- see this
            // const's own doc comment) is fully quirk-claimed, via the
            // class-shape branch's OWN early `on_unmatched_node` check
            // (which runs before this row's `name_field` fallback would
            // ever be consulted for one of those five kinds specifically).
            name_field: "name",
            body_field: "body",
        }
    }

    /// QML (Qt Modeling Language, `.qml`): node kinds copied from the
    /// baseline's `qml_class_types`/`qml_field_types`/`qml_import_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:254-266,
    /// `CBM_LANG_QML` row at :2022-2027), which reuses TypeScript/
    /// JavaScript's own shared `ts_func_types`/`js_module_types`/
    /// `js_call_types`/`js_branch_types`/`ts_decorator_types` arrays
    /// verbatim (the baseline's own comment: "QMLJS grammar is a
    /// TypeScript superset plus declarative ui_* nodes") -- verified
    /// directly against the real `tree-sitter-qmljs` crate's own
    /// `src/node-types.json` AND a real parse-tree dump (a scratch
    /// `cargo run` against a minimal crate binding
    /// `tree-sitter-qmljs`'s vendored `parser.c` directly), not
    /// transcribed blindly:
    /// - Every one of the baseline's `qml_class_types` five entries
    ///   (`class_declaration`, `class`, `abstract_class_declaration`,
    ///   `enum_declaration`, `ui_inline_component`) is confirmed present
    ///   in this grammar with a working `name_field`/`body_field` shape
    ///   for the four ordinary ones -- `interface_declaration` is
    ///   deliberately split out to [`Self::interface_types`] instead
    ///   (this crate's own [`LangSpec`] has a real `interface_types`
    ///   field the baseline's flatter `CBMLangSpec` does not; same
    ///   rationale as [`Self::typescript`]'s identical split for plain
    ///   TS). `ui_inline_component`'s own shape (`component Circle:
    ///   Rectangle {...}`) has NO `name`/`body` field pair the ordinary
    ///   four share (its fields are `name`/`component`, the latter
    ///   pointing at a nested `ui_object_definition`, not a body block)
    ///   -- fully claimed by [`generic::qml_quirk`] rather than this
    ///   array's own generic class-shape fallback.
    /// - `field_types` matches the baseline's `qml_field_types` exactly
    ///   (`ui_property`, `ui_signal`, `public_field_definition`) -- all
    ///   three confirmed to carry a working `name` field (real parse
    ///   tree: `property int count: 0` is `ui_property` with `name`
    ///   field = `count`; `signal clicked(int x)` is `ui_signal` with
    ///   `name` field = `clicked`), so no quirk is needed for the field
    ///   DEFINES case at all -- ordinary flat `name_field` path. NOTE
    ///   this deliberately diverges from [`Self::typescript`]'s own
    ///   choice to leave `field_types` empty (that row's own doc
    ///   comment: the bespoke `languages/typescript.rs` extractor never
    ///   had a `"public_field_definition"` match arm, so including it
    ///   there would be a regression against that zero-regression
    ///   migration target) -- QML has no such pre-existing bespoke
    ///   extractor to match, and the baseline's own `qml_field_types`
    ///   explicitly includes it, so this row honors the baseline's real
    ///   intended depth instead.
    /// - `import_types` matches the baseline's `qml_import_types` exactly
    ///   (`import_statement`, `import`, `ui_import`) -- confirmed via
    ///   real parse tree: `import QtQuick` and `import "./x.js" as
    ///   Helpers` both parse as `ui_import` (fields `source`/`alias`,
    ///   neither a clean single "the whole path" field for the aliased
    ///   form), and a bare `import` (no operand at all) is realistically
    ///   unreachable in this always-`ui_import`-shaped grammar but kept
    ///   for baseline-parity documentation (matches this crate's own
    ///   convention elsewhere of carrying a provably-inert baseline
    ///   entry rather than silently dropping it, see e.g.
    ///   [`Self::ruby`]'s `"command_call"` doc note) -- fully claimed by
    ///   [`generic::qml_quirk`] (neither shape is a single clean
    ///   "module path" field the way an ordinary flat import scan could
    ///   read).
    /// - `call_types`/`branch_types`/`decorator_types` are IDENTICAL to
    ///   [`Self::typescript`]'s own arrays (the baseline's own verbatim
    ///   reuse), EXTENDED with `new_expression` in `call_types` (the
    ///   baseline's `js_call_types = {"call_expression",
    ///   "new_expression"}`, which [`Self::typescript`] itself does NOT
    ///   include -- that row's own doc comment has no such note, and its
    ///   bespoke `languages/typescript.rs` predecessor never recorded
    ///   `new Foo()` as its own CALLS edge either; QML has no such
    ///   pre-existing bespoke extractor to preserve zero-regression
    ///   against, so this row honors the baseline's fuller real array
    ///   instead). `call_expression` itself keeps the ordinary
    ///   `"function"`/`"arguments"` field pair (a clean shape, confirmed
    ///   via the real parse-tree dump, needing no override at all);
    ///   `new_expression`'s own callee lives in a DIFFERENT field
    ///   (`constructor`, not `function` -- confirmed via the same dump:
    ///   `new Widget()` is `new_expression` with fields
    ///   `constructor`=`Widget`, `arguments`=`()`) -- claimed by
    ///   [`generic::qml_call_override`] rather than this row's single
    ///   `call_function_field`, which cannot express two different
    ///   field names for two different node kinds in `call_types` at
    ///   once (same reasoning as every other language row whose
    ///   `call_types` mixes shapes, e.g. [`Self::php`]).
    /// - `module_types` is intentionally EMPTY, NOT the baseline's
    ///   `js_module_types = {"program"}`: identical rationale to
    ///   [`Self::solidity`]/[`Self::gdscript`]'s own doc comments (the
    ///   baseline's `module_node_types` field is consumed by a
    ///   completely different mechanism there than what this crate's
    ///   own live `module_types` array drives) -- ADDITIONALLY, and more
    ///   fundamentally for QML specifically: this grammar's `program`
    ///   root node has a REQUIRED `root` field that must itself be a
    ///   `ui_annotated_object`/`ui_object_definition` (confirmed: a bare
    ///   top-level statement with no wrapping QML object at all does not
    ///   parse cleanly -- produces `ERROR` nodes in a real parse-tree
    ///   dump), so `program`'s own "first named child" is never a
    ///   meaningful module-name candidate the way porting `"program"`
    ///   into a live `module_types` array would assume; there is no
    ///   analogous named top-level module/namespace construct in real
    ///   QML source to extract a Module symbol from at all.
    pub const fn qml() -> Self {
        Self {
            name: "qml",
            func_types: &["function_declaration"],
            method_types: &["method_definition"],
            class_types: &[
                "class_declaration",
                "class",
                "abstract_class_declaration",
                "enum_declaration",
                "ui_inline_component",
            ],
            interface_types: &["interface_declaration"],
            enum_types: &[],
            alias_types: &[],
            field_types: &["ui_property", "ui_signal", "public_field_definition"],
            module_types: &[],
            call_types: &["call_expression", "new_expression"],
            // Real field names, actually consulted: `call_expression`
            // (not claimed by `qml_call_override`) uses this ordinary
            // pair directly; `new_expression` (claimed in full) never
            // reads them -- see this const's own doc comment.
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_statement", "import", "ui_import"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "for_in_statement",
                "while_statement",
                "switch_statement",
                "switch_case",
                "switch_default",
                "try_statement",
                "catch_clause",
                "do_statement",
            ],
            decorator_types: &["decorator"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// ReScript (`.res`/`.resi`): node kinds copied from the baseline's
    /// `rescript_func_types`/`rescript_class_types`/`rescript_call_types`/
    /// `rescript_import_types`/`rescript_branch_types`/
    /// `rescript_decorator_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1165-1176,
    /// `CBM_LANG_RESCRIPT` row at :2129-2133) -- verified directly
    /// against the `arborium-rescript` crate's own
    /// `grammar/src/node-types.json` (this crate's grammar dependency;
    /// see this row's own `parse_rescript` doc comment for why that
    /// crate rather than a standalone `tree-sitter-rescript`) AND a real
    /// parse-tree dump (a scratch `cargo run` against a minimal crate
    /// binding that grammar's vendored `parser.c` directly), which
    /// caught the SAME real name-resolution gap the baseline's own
    /// `cbm_resolve_func_name` already has a dedicated case for --
    /// confirming rather than merely trusting it:
    /// - `func_types`/`method_types` are both `["function"]`, matching
    ///   the baseline's `rescript_func_types` exactly. This grammar's
    ///   `function` node (an arrow-function-shaped `(a, b) => {...}`)
    ///   has NO `name` field at all (confirmed:
    ///   `{"type":"function","fields":{"body":...,"parameters":...}}`,
    ///   no `name` entry) -- the written name lives one level UP, on the
    ///   enclosing `let_binding`'s own `pattern` field (`let add = (a,
    ///   b) => {...}` parses as `let_declaration > let_binding(pattern=
    ///   add, body=function)`). `internal/cbm/extract_defs.c`'s
    ///   `cbm_resolve_func_name` has an exact dedicated case for this
    ///   (:761-772: "the `function` (arrow) node ... has no name; the
    ///   binding name is on the enclosing let_binding's `pattern`
    ///   field... Resolving via the parent keeps plain value
    ///   let-bindings out of func_types") -- [`generic::rescript_quirk`]
    ///   mirrors that exact climb-to-parent algorithm rather than
    ///   inventing a different one, since a plain value binding (`let x
    ///   = 42`) has a `number`/other non-`function` node in that same
    ///   `body` field position and consequently never reaches this
    ///   quirk arm at all (confirmed via the same parse-tree dump: `let
    ///   x = 42`'s `let_binding.body` is a bare `number`, never
    ///   `function`) -- the baseline's own parenthetical about "keeps
    ///   plain value let-bindings out of func_types" describes exactly
    ///   this natural exclusion, not a separate check this crate must
    ///   add. `function` has no `body_field` either in the sense this
    ///   file's generic mechanism needs (its OWN `body` field is a
    ///   plain expression, not a name-bearing symbol to further recurse
    ///   as a class/method scope) -- fully claimed by
    ///   [`generic::rescript_quirk`], which after resolving the name
    ///   still walks the `function` node's own `body` field generically
    ///   with `fn_scope` correctly set (mirrors
    ///   [`generic::zig_walk_container_body`]'s identical "resolve name
    ///   externally, then re-enter the generic walk with it" shape).
    /// - `class_types` is `["module_declaration", "type_declaration"]`,
    ///   matching the baseline's `rescript_class_types` exactly. NEITHER
    ///   kind has a direct `name` field of its own (confirmed:
    ///   `module_declaration`'s only documented child is
    ///   `module_binding`; `type_declaration`'s is `type_binding`) --
    ///   the real name one level down, on that sole child's own `name`
    ///   field (`module_binding.name`/`type_binding.name`, both
    ///   confirmed present via the same parse-tree dump: `module Point =
    ///   {...}` is `module_declaration > module_binding(name=Point,
    ///   definition=...)`; `type t = {...}` is `type_declaration >
    ///   type_binding(name=t, body=...)`) -- mirrors
    ///   `internal/cbm/extract_defs.c`'s own `CBM_LANG_RESCRIPT` case at
    ///   :3678-3686 ("type_declaration > type_binding(name=
    ///   type_identifier)") exactly, extended the identical way for
    ///   `module_declaration`/`module_binding` (the baseline has no
    ///   separate case for that pair since its own generic name-field
    ///   resolution elsewhere already happens to reach through
    ///   one-child wrapper nodes; this crate's [`generic::rescript_quirk`]
    ///   claims both explicitly instead, since this file's flat
    ///   `name_field` mechanism cannot express "descend into your one
    ///   child, then read ITS name field" on its own).
    /// - `field_types` is empty, matching the baseline's own `empty_types`
    ///   for this array (its `CBM_LANG_RESCRIPT` row's 3rd positional
    ///   field, `field_node_types`, is `empty_types` -- see this
    ///   language's own struct-literal row in `lang_specs.c`) -- no
    ///   field/property DEFINES edges for ReScript, by design, not
    ///   omission.
    /// - `module_types` is intentionally EMPTY, NOT the baseline's own
    ///   `rescript_module_types = {"source_file"}`: identical rationale
    ///   to [`Self::solidity`]/[`Self::gdscript`]/[`Self::qml`]'s own
    ///   doc comments -- `source_file` is this grammar's whole-file root
    ///   node, so its first named child is arbitrarily whatever
    ///   statement happens to come first, never a real module-name AST
    ///   node; porting it verbatim would invent new (wrong) Module-symbol
    ///   behavior, not reproduce a real baseline one (the baseline's own
    ///   `module_node_types` field has the identical "consumed by a
    ///   completely different, unrelated mechanism" status those other
    ///   rows' doc comments already establish).
    /// - `call_types` is `["call_expression"]`, matching the baseline's
    ///   `rescript_call_types` exactly. Confirmed via the parse-tree
    ///   dump to have a completely ordinary, clean `function`/`arguments`
    ///   field pair (unlike QML/Squirrel's own call-shape quirks in this
    ///   file) -- `add(1, 2)` and the qualified `Js.log("x")` (whose
    ///   `function` field is a `value_identifier_path`, its own
    ///   `.utf8_text()` already the full qualified `"Js.log"` text) both
    ///   flow through the ordinary flat-field path with no quirk needed
    ///   at all.
    /// - `import_types` is `["open_statement", "include_statement",
    ///   "include"]`, matching the baseline's `rescript_import_types`
    ///   exactly. `open_statement`/`include_statement` both have NO
    ///   fields (confirmed: `open Belt` is `open_statement` wrapping a
    ///   single positional `module_identifier` child, no field name at
    ///   all) -- fully claimed by [`generic::rescript_quirk`], which
    ///   reads the first named child's text directly (the same
    ///   `first_named_child_text` helper [`generic::walk`]'s own
    ///   `module_types` branch already uses, reused here rather than
    ///   duplicated). Bare `"include"` (a keyword token, not a named
    ///   rule in this grammar -- confirmed absent from
    ///   `node-types.json`'s node list entirely) can never actually
    ///   match any real `.kind()` this grammar emits, kept only for
    ///   baseline-parity documentation (same "provably inert baseline
    ///   entry, not silently dropped" convention as
    ///   [`Self::qml`]/[`Self::ruby`]'s own doc notes).
    /// - `branch_types` is `["if_expression", "switch_expression",
    ///   "try_expression"]`, matching the baseline's `rescript_branch_types`
    ///   exactly -- all three confirmed present via the parse-tree dump
    ///   (`if x > 0 {...} else {...}` is `if_expression`; `switch x
    ///   {...}` is `switch_expression`; `try_expression` mirrors the
    ///   identical children shape, not separately exercised but
    ///   confirmed present in `node-types.json` with the same
    ///   `expression`/`switch_match` children list `switch_expression`
    ///   has).
    /// - `decorator_types` is `["decorator"]`, matching the baseline's
    ///   `rescript_decorator_types` exactly. Confirmed via the parse-tree
    ///   dump that `@react.component` is a PRECEDING SIBLING of the
    ///   `let_declaration` it annotates (not a child/field of it) --
    ///   the exact same shape [`generic::ts_preceding_decorators`]'s own
    ///   `prev_sibling()` walk already handles, reused directly by
    ///   [`generic::rescript_quirk`] rather than duplicated (this
    ///   grammar's own `decorator` node wraps a single
    ///   `decorator_identifier` child rather than TS's
    ///   `call_expression`/`identifier` split, so the decorator's own
    ///   name is read via [`generic::first_named_child_text`] instead of
    ///   [`generic::ts_decorator_name`]).
    pub const fn rescript() -> Self {
        Self {
            name: "rescript",
            func_types: &["function"],
            method_types: &["function"],
            class_types: &["module_declaration", "type_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["open_statement", "include_statement", "include"],
            branch_types: &["if_expression", "switch_expression", "try_expression"],
            decorator_types: &["decorator"],
            // Never actually consulted for `func_types`/`method_types`/
            // `class_types` (all three fully claimed by
            // `generic::rescript_quirk` -- see this const's own doc
            // comment); `call_types`'s `call_expression` uses the
            // ordinary "function"/"arguments" field pair directly, so
            // this value only needs to be a real field name for that
            // one case, which it already is.
            name_field: "name",
            body_field: "body",
        }
    }

    /// Squirrel (`.nut`): node kinds copied from the baseline's
    /// `squirrel_func_types`/`squirrel_class_types`/`squirrel_call_types`/
    /// `squirrel_branch_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1487-1496,
    /// `CBM_LANG_SQUIRREL` row at :2433-2437) -- verified directly
    /// against the `tree-sitter-squirrel` crate's own `src/node-types.json`
    /// AND a real parse-tree dump (a scratch `cargo run` against a
    /// minimal crate binding that grammar's vendored `parser.c`
    /// directly), which surfaced this grammar as almost entirely
    /// FIELD-FREE for the constructs this row cares about -- every
    /// array below is consequently fully claimed by
    /// [`generic::squirrel_quirk`]/[`generic::squirrel_call_override`]
    /// rather than this file's own flat `name_field`/`body_field`
    /// mechanism, same "arrays exist so this row still documents the
    /// vocabulary at a glance" posture as [`Self::c`]/[`Self::cpp`]/
    /// [`Self::php`]/[`Self::objc`] (see their own doc comments):
    /// - `func_types` is `["function_declaration", "anonymous_function",
    ///   "lambda_expression"]`, matching the baseline's
    ///   `squirrel_func_types` exactly; `method_types` is
    ///   `["function_declaration"]` only (NOT the full trio -- an
    ///   `anonymous_function`/`lambda_expression` is by definition
    ///   unnamed the way this grammar's own generic function/method
    ///   branch would need a name for, so neither can be a "Method"
    ///   distinct from "Function" symbol at all; only a NAMED function
    ///   inside a class body is meaningfully a method). None of the
    ///   three has a `name` field (confirmed: `function_declaration`'s
    ///   own `fields` entry in `node-types.json` is empty; a named
    ///   function's name is instead a plain positional `identifier`
    ///   child immediately following the `function` keyword token and
    ///   preceding `(`) -- `internal/cbm/extract_defs.c`'s own
    ///   `cbm_resolve_func_name` has a dedicated case for exactly this
    ///   shape (:709-718: "Cairo / D / Odin / Squirrel: the def node has
    ///   no `name` field; the name is a plain `identifier` child") --
    ///   [`generic::squirrel_quirk`] mirrors it directly. `body` is
    ///   likewise no field but a positional `block` child (the LAST
    ///   child for a well-formed definition) -- claimed the same way.
    ///   Squirrel's `constructor(...) {...}` class-member shorthand
    ///   is deliberately NOT one of `func_types`/`method_types`: a real
    ///   parse-tree dump proved it is NOT wrapped in a
    ///   `function_declaration` node at all -- the enclosing
    ///   `member_declaration`'s own children are directly
    ///   `constructor`(keyword) + `parameters` + `block`, with no
    ///   `function_declaration` in between. This matches
    ///   `internal/cbm/extract_defs.c`'s own "Squirrel wraps each class
    ///   member in a member_declaration node; the method is the inner
    ///   function_declaration. Peek through the wrapper." comment
    ///   (:4192-4194) precisely -- that "peek through" search
    ///   (`cbm_find_child_by_kind(child, "function_declaration")`) finds
    ///   NOTHING for a constructor member, so the baseline itself never
    ///   extracts a Squirrel class's `constructor` as a Method symbol
    ///   either. [`generic::squirrel_quirk`] matches that same real
    ///   (limited) depth rather than building a richer constructor-aware
    ///   walker the baseline itself does not have, mirroring
    ///   [`Self::ruby`]'s own identical "match the baseline's real
    ///   depth, not an idealized one" precedent.
    /// - `class_types` is `["class_declaration", "enum_declaration"]`,
    ///   matching the baseline's `squirrel_class_types` exactly. Neither
    ///   has a `name` field (confirmed empty `fields` in
    ///   `node-types.json` for both) -- the name is a plain positional
    ///   `identifier` child, mirrors
    ///   `internal/cbm/extract_defs.c`'s own dedicated case at :3659
    ///   ("CBM_LANG_SQUIRREL: class_declaration > identifier") exactly,
    ///   extended identically for `enum_declaration` (confirmed via the
    ///   parse-tree dump: `enum Color {...}`'s own children are
    ///   literally `enum`(keyword) + `identifier`(name) + `{` +
    ///   `identifier`/`const_value` pairs (the enum's members, each a
    ///   bare positional child, no wrapping node at all) + `}`).
    ///   `class_declaration` ALSO has NO `body` field (confirmed:
    ///   `internal/cbm/extract_defs.c`'s own "Squirrel: class_declaration
    ///   has no body field — member_declaration nodes ... are direct
    ///   children of the class" comment at :3936-3939, confirmed by the
    ///   same parse-tree dump: no wrapping `class_body` node exists at
    ///   all, `member_declaration` nodes are direct children of
    ///   `class_declaration` itself, interspersed with the `identifier`/
    ///   `extends`-keyword/base-class-`identifier` children) --
    ///   `class_declaration`'s own member DEFINES scan and heritage
    ///   (`extends`) detection are consequently both driven directly off
    ///   the class node's own children by [`generic::squirrel_quirk`],
    ///   not a separate body-node lookup.
    /// - `extends` (`class Dog extends Animal`): heritage is a plain
    ///   `identifier` child directly following the literal `extends`
    ///   KEYWORD TOKEN (confirmed `"extends"` is `"named": false` in
    ///   `node-types.json` -- an unnamed/anonymous token, not a named
    ///   rule at all, so it cannot appear in any of this row's own
    ///   arrays the way a real node-kind name could) -- mirrors
    ///   `internal/cbm/extract_defs.c`'s own dedicated
    ///   `extract_base_classes` Squirrel walker (:2395-2421) exactly:
    ///   scan the class node's direct children (via [`Node::child`], not
    ///   [`Node::named_child`], since the search target keyword itself
    ///   is unnamed) for the literal `"extends"` token, then take the
    ///   very next `identifier` sibling as the one base-class name (this
    ///   grammar allows only single inheritance, so the baseline's own
    ///   walker never loops past the first match either) -- implemented
    ///   as part of [`generic::squirrel_quirk`]'s own `class_declaration`
    ///   handling directly (this crate's [`Quirks`] seam has no separate
    ///   "extract_base_classes"-shaped hook the way the baseline's own
    ///   C structure does; every other language's own heritage handling
    ///   in this file is likewise folded into its class-shape
    ///   `on_unmatched_node` arm, e.g. [`Self::typescript`]'s own
    ///   `ts_heritage_refs` call inside [`generic::ts_quirk`]).
    /// - `field_types` is empty, matching the baseline's own `empty_types`
    ///   for this array (its `CBM_LANG_SQUIRREL` row's 3rd positional
    ///   field, `field_node_types`, is `empty_types`) -- no field/
    ///   property DEFINES edges for Squirrel (a class body's bare
    ///   `name = "";`-shaped member assignments are NOT extracted), by
    ///   design, not omission.
    /// - `module_types` is empty, matching the baseline's own
    ///   `squirrel_module_types = {"source_file"}` being DEAD data
    ///   there for the identical reason [`Self::solidity`]/
    ///   [`Self::gdscript`]/[`Self::qml`]/[`Self::rescript`]'s own doc
    ///   comments already establish for their own root-node entries (a
    ///   whole-file root's first named child is never a real module-name
    ///   AST node) -- this grammar's ACTUAL root node kind is `script`
    ///   (confirmed via the parse-tree dump), not even literally
    ///   `"source_file"` the baseline's own dead array names, making the
    ///   baseline's choice doubly inapplicable here.
    /// - `call_types` is `["call_expression"]`, matching the baseline's
    ///   `squirrel_call_types` exactly. This one DOES have a working
    ///   `function` field (confirmed:
    ///   `{"type":"call_expression","fields":{"function":{"required":false,...}}}`,
    ///   and the parse-tree dump shows both a bare `helper()` and a
    ///   qualified `h.register()`/`base.speak()` -- the latter's own
    ///   `function` field is a `deref_expression` wrapping receiver
    ///   `.` name, `.utf8_text()` on it already yielding the full
    ///   written `"h.register"` text) -- but its own argument list is
    ///   exposed as a single unnamed `call_args` child (confirmed absent
    ///   from `node-types.json`'s own `"fields"` entry for
    ///   `call_expression`, present only in its `"children"` list),
    ///   which this file's flat `call_arguments_field` mechanism cannot
    ///   read directly -- [`generic::squirrel_call_override`] reuses the
    ///   clean `function`-field callee/receiver-hint reconstruction but
    ///   additionally walks the positional `call_args` child for
    ///   argument texts (mirrors [`generic::zig_builtin_arg_texts`]'s
    ///   identical "scan children for the unnamed arguments-holder kind,
    ///   then take ITS OWN children" shape).
    /// - `branch_types` is `["if_statement", "switch_statement",
    ///   "while_statement"]`, matching the baseline's
    ///   `squirrel_branch_types` exactly -- all three confirmed present
    ///   via the parse-tree dump, PLUS `for_statement`/`foreach_statement`
    ///   deliberately kept OUT despite this grammar clearly supporting
    ///   both (confirmed present and exercised in the dump) -- the
    ///   baseline's own array simply does not include them for this
    ///   language (unlike, say, its `js_branch_types`' fuller list),
    ///   and this crate's stated brief is to match the baseline's real
    ///   intended depth rather than silently broadening it (same
    ///   "matches the baseline's own choice, not an idealized one"
    ///   posture as [`Self::ruby`]/[`Self::zig`]'s doc comments already
    ///   establish elsewhere in this file) -- `branch_types` is in any
    ///   case not consulted anywhere in [`generic::walk`] itself today
    ///   (see this crate's own module-level doc note on
    ///   [`crate::complexity::NodeKindTable`] being this array's real,
    ///   separate consumer; complexity extraction is deferred entirely
    ///   for this language per this wave's own scope, matching G2.1's
    ///   convention), so this choice is documentation-only for now.
    /// - `import_types` is empty, NOT the baseline's own
    ///   `squirrel_import_types = {"extends"}`. This is PROVABLY dead
    ///   data in the baseline itself, not merely unused-by-choice: the
    ///   baseline's own generic spec-driven import scan
    ///   (`internal/cbm/extract_imports.c`'s `parse_spec_imports`,
    ///   :1352-1373, the `default:` fallthrough every language absent
    ///   from that file's own per-language `switch` reaches -- confirmed
    ///   Squirrel IS absent from that switch entirely) walks ONLY the
    ///   direct children of the whole file's ROOT node
    ///   (`ts_tree_cursor_goto_first_child` once, then
    ///   `goto_next_sibling` repeatedly, no recursion at all) -- but
    ///   `extends` is a keyword token nested many levels deep inside a
    ///   `class_declaration`'s own children, never itself a direct
    ///   child of the file's `script` root, so this entry can never
    ///   actually match anything at the only scan depth
    ///   `parse_spec_imports` operates at, confirmed by tracing that
    ///   function's own implementation, not merely by inspection. Unlike
    ///   [`Self::qml`]/[`Self::ruby`]'s own "provably inert but
    ///   harmless, kept for documentation" dead-array entries, porting
    ///   `"extends"` here verbatim would NOT be harmless: this crate's
    ///   own [`generic::walk`] recurses into EVERY node depth-first
    ///   (unlike the baseline's shallow root-only scan), so it WOULD
    ///   actually match every `extends` keyword anywhere in a Squirrel
    ///   file and attempt to record it as an IMPORTS edge with no real
    ///   module path behind it -- inventing new, wrong behavior the
    ///   baseline itself never produces, rather than reproducing a real
    ///   (if buggy) one. Left empty instead.
    pub const fn squirrel() -> Self {
        Self {
            name: "squirrel",
            func_types: &[
                "function_declaration",
                "anonymous_function",
                "lambda_expression",
            ],
            method_types: &["function_declaration"],
            class_types: &["class_declaration", "enum_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "UNUSED_SEE_SQUIRREL_CALL_OVERRIDE",
            import_types: &[],
            branch_types: &["if_statement", "switch_statement", "while_statement"],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment (same posture as `LangSpec::c()`/`LangSpec::cpp()`/
            // `LangSpec::php()`/`LangSpec::objc()`): every one of
            // `func_types`/`method_types`/`class_types` is fully claimed
            // by `generic::squirrel_quirk` before this file's own
            // `name_field`-keyed fallback would ever run, and
            // `call_types`'s sole entry uses the clean `"function"`
            // field directly (a real field name, just not consulted via
            // THIS particular struct field).
            name_field: "UNUSED_SEE_SQUIRREL_QUIRK",
            body_field: "UNUSED_SEE_SQUIRREL_QUIRK",
        }
    }

    /// Sway (Fuel Labs' smart-contract language, `.sw`) -- language-parity
    /// wave G2.3e. Grammar VENDORED (no crates.io release exists for
    /// `tree-sitter-sway` at all -- see
    /// `crates/enforcer-memory/vendor/tree-sitter-sway-local/`'s own doc
    /// comment): FuelLabs' own published crate hard-pins `tree-sitter =
    /// "~0.20.3"` as a normal runtime dependency (confirmed by fetching that
    /// repo's own `bindings/rust/Cargo.toml` directly), the same
    /// incompatible-ABI-generation blocker `tree-sitter-squirrel-local`
    /// documents. Sway is heavily Rust-inspired -- confirmed via a real
    /// `node-types.json` fetched from the upstream repo (pinned commit
    /// `9b7845c`, ABI 14) -- and every node kind below uses the SAME
    /// Rust-shaped fields this crate's own [`Self::rust`] row already
    /// relies on (`name`/`body` on `function_item`, `function`/`arguments`
    /// on `call_expression`, `argument` on `use_declaration`), so the ONLY
    /// quirk this row needs at all is `impl_item`'s name (the baseline's own
    /// `extract_defs.c:3669`'s `CBM_LANG_SWAY` case: "name is the
    /// implemented type, field `type`" -- confirmed present on this
    /// grammar's real `impl_item` node) plus the baseline's own
    /// `extract_defs.c:3770` struct-vs-class relabeling (Sway/WGSL structs
    /// get `SymbolKind::Struct`, Sway `abi_item` blocks get
    /// `SymbolKind::Interface`). `sway_import_types` baseline array's
    /// `"use_declaration"` entry is clean (real node, real `argument`
    /// field) -- no correction needed there, unlike several other
    /// languages' stale arrays this wave found. `closure_expression` is
    /// listed in the baseline's own `sway_func_types` array but is
    /// DELIBERATELY omitted here: an anonymous closure can never have a
    /// `name_field` match (confirmed via `node-types.json`: this node kind
    /// has no `name` field at all, only `parameters`/`body`), so including
    /// it would only ever silently no-op through the generic func/method
    /// branch's own `child_text(...).is_none()` fallthrough -- omitting it
    /// entirely is equivalent in every observable output and avoids a
    /// dead-looking array entry.
    pub const fn sway() -> Self {
        Self {
            name: "sway",
            func_types: &["function_item", "function_signature_item"],
            method_types: &["function_item", "function_signature_item"],
            class_types: &["struct_item", "enum_item", "trait_item", "abi_item"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["call_expression", "abi_call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["use_declaration"],
            branch_types: &[
                "if_expression",
                "match_expression",
                "while_expression",
                "for_expression",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Starlark (Bazel's Python-like config language, `.bzl`/`.bazel`/
    /// `BUILD`/`WORKSPACE`) -- language-parity wave G2.3e. Grammar:
    /// `tree-sitter-starlark` 1.3.0 (crates.io, `repository`:
    /// github.com/tree-sitter-grammars/tree-sitter-starlark -- copyright
    /// line "Amaan Qureshi" byte-matches this crate's own vendored
    /// `internal/cbm/vendored/grammars/starlark/LICENSE` exactly, confirming
    /// the identical grammar lineage the baseline itself vendored). Depends
    /// on `tree-sitter-language` (not `tree-sitter` core) as its runtime
    /// binding -- its own `Cargo.toml` declares `tree-sitter` only as a
    /// `[dev-dependencies]` entry, same ABI-range-agnostic pattern as every
    /// grammar this crate already depends on. `function_definition`'s
    /// `name`/`body` fields and `call`'s `function`/`arguments` fields both
    /// match this crate's own defaults exactly (confirmed via this grammar's
    /// real `node-types.json`) -- no quirk needed for def/call extraction at
    /// all. ONE real correction against the baseline's own `starlark_*`
    /// arrays, found only by inspecting this grammar's actual node shape (a
    /// baseline-array-name-existing-but-being-a-red-herring class of bug,
    /// not a missing-node-kind one): `starlark_import_types`' sole entry
    /// `"with_clause"` is a REAL node kind in this grammar, but it is the
    /// child of an ordinary Python-style `with_statement` (context
    /// managers) -- Starlark's actual import mechanism, `load("//path:file.bzl",
    /// "symbol")`, is a plain `call` node whose `function` field text is the
    /// literal string `"load"` (confirmed directly against this crate's own
    /// baseline source, `extract_imports.c`'s `parse_starlark_imports`,
    /// which implements exactly this `call`-node detection and never reads
    /// `with_clause` for anything import-related at all -- the array entry
    /// is dead code in the baseline's own extractor, evidently a copy/paste
    /// artifact from a Python-family sibling row). This crate's own
    /// `starlark_quirk` mirrors the baseline's REAL `load(...)`-detection
    /// behavior rather than the baseline's stale array, so `import_types`
    /// is deliberately left empty here (there is no distinct import-shaped
    /// NODE KIND at all in this grammar -- `load(...)` is an ordinary
    /// `call`, indistinguishable at the node-kind level from any other call,
    /// so it is caught via [`crate::languages::generic::Quirks::call_override`]
    /// instead of the flat `import_types` array mechanism).
    pub const fn starlark() -> Self {
        Self {
            name: "starlark",
            func_types: &["function_definition", "lambda"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["module"],
            call_types: &["call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            // See this const's own doc comment: `load(...)` is caught via
            // `call_override`, not this array (no distinct import node kind
            // exists in this grammar at all).
            import_types: &[],
            branch_types: &["if_statement", "for_statement"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Templ (Go HTML-templating DSL, `.templ`) -- language-parity wave
    /// G2.3e. Grammar: `tree-sitter-templ` 2.2.0 (crates.io, `repository`:
    /// github.com/vrischmann/tree-sitter-templ -- copyright line "Vincent
    /// Rischmann" byte-matches this crate's own vendored
    /// `internal/cbm/vendored/grammars/templ/LICENSE` exactly). Depends on
    /// `tree-sitter-language` as its runtime binding (`tree-sitter` core is
    /// a `[dev-dependencies]`-only entry), same ABI-range-agnostic pattern
    /// as every grammar here. This grammar is Go's OWN grammar plus
    /// `templ`-specific `component_declaration`/`method_elem` nodes layered
    /// on top (confirmed via a real `node-types.json` diff: `function_declaration`/
    /// `method_declaration`/`type_declaration`/`import_declaration` all have
    /// the IDENTICAL field shapes this crate's own [`Self::go`] row and
    /// `generic::go_quirk` already handle byte-for-byte) -- def/call
    /// extraction needs no quirk at all (`name`/`body` on
    /// `function_declaration`/`method_declaration`, `function`/`arguments`
    /// on `call_expression` all match this file's defaults directly).
    /// TWO real corrections against the baseline's own `templ_*` arrays,
    /// both found only via this grammar's actual `node-types.json` (not
    /// assumed from the Go-grammar-reuse guess above): (1)
    /// `templ_func_types`'s `"method_elem"` entry is real but describes an
    /// INTERFACE method signature with no `body` field at all (confirmed:
    /// `method_elem`'s own fields are `name`/`parameters`/`result` only) --
    /// harmless to include since the generic func/method branch's
    /// `child_by_field_name(body_field)` lookup simply returns `None` and
    /// skips the body-recursion step, exactly the same as a Go bare
    /// interface-method signature would; (2) `templ_import_types`'s
    /// `"import"` entry is a PHANTOM: this grammar's own `node-types.json`
    /// marks `"import"` `"named": false` (it is the bare keyword TOKEN
    /// `import`, never a distinct AST node an extractor could ever match
    /// against), leaving `"import_declaration"` (whose own `import_spec`/
    /// `import_spec_list` children mirror Go's shape exactly) as the only
    /// real import node kind -- caught by `generic::templ_quirk` reusing
    /// [`crate::languages::generic::go_import_paths`]-equivalent logic
    /// rather than the dead flat-array path, mirroring this same wave's
    /// Starlark finding: baseline arrays for this wave's languages
    /// contained more dead/phantom entries than clean ones once verified
    /// against real grammars, not fewer.
    pub const fn templ() -> Self {
        Self {
            name: "templ",
            func_types: &["function_declaration", "method_elem"],
            method_types: &["method_declaration", "method_elem"],
            class_types: &["component_declaration", "type_spec"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &["type_alias"],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            // See this const's own doc comment: `"import"` is a phantom
            // unnamed token in this grammar; `import_declaration` is
            // claimed via `on_unmatched_node` (grouped-import shape),
            // mirroring `generic::go_quirk`'s identical posture for Go's
            // own `import_declaration`.
            import_types: &[],
            branch_types: &[
                "if_statement",
                "for_statement",
                "expression_switch_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Typst (typesetting/markup language, `.typ`) -- language-parity wave
    /// G2.3e. Grammar: `arborium-typst` 2.18.1 (crates.io, part of the
    /// `bearcove/arborium` tree-sitter grammar bundle -- same bundle this
    /// crate's own [`Self::rescript`] row already depends on via
    /// `arborium-rescript`). Depends on `tree-sitter-language` as its
    /// runtime binding. Cross-checked byte-for-byte against the sibling
    /// `codebook-tree-sitter-typst` 0.12.1 crate (`repository`:
    /// github.com/uben0/tree-sitter-typst) via a full `node-types.json`
    /// diff -- IDENTICAL upstream grammar lineage, either would work;
    /// `arborium-typst` chosen for this crate's own existing-dependency
    /// precedent. Copyright on this crate's own vendored
    /// `internal/cbm/vendored/grammars/typst/LICENSE` ("Gerbais-Nief Eddie")
    /// does not byte-match either candidate's stated author, but the real
    /// node-kind vocabulary (`let`/`call`/`import`/`include`/`set`, all
    /// confirmed below) is unambiguous either way. TWO real, non-obvious
    /// findings against the baseline's own `typst_*` arrays, both requiring
    /// a real parse-tree/field dump (not `node-types.json` alone) to catch,
    /// mirroring the baseline's OWN documented `extract_calls.c`/
    /// `extract_defs.c` special-casing for this exact language (this is the
    /// one language in this wave where the C baseline ITSELF already needed
    /// dedicated non-generic code, confirming this grammar really is this
    /// unusual): (1) `call`'s own field is named `item`, NOT `function` --
    /// and `call` has NO separate `arguments` field at all (the whole
    /// argument list is folded inside that same `item` sub-expression) --
    /// `extract_typst_callee`'s own implementation confirms this exactly
    /// (`ts_node_child_by_field_name(node, "item")`, no `arguments` field
    /// read anywhere), so this row's `call_function_field`/
    /// `call_arguments_field` are UNUSED placeholders and
    /// [`crate::languages::generic::Quirks::call_override`] fully claims
    /// `call` instead; (2) `typst_func_types`' `"let"` entry describes a
    /// node that is ONLY a function definition when its OWN `pattern` field
    /// is itself a nested `call` node (`#let greet(name) = ...`) -- a plain
    /// `#let x = 1` binding is the exact same `let` node kind with a
    /// non-call `pattern`, and the real function name then lives on THAT
    /// nested call's own `item` field, not on the outer `let` at all
    /// (confirmed via `extract_defs.c:935`'s dedicated `CBM_LANG_TYPST`
    /// case, which implements exactly this nested-pattern-unwrap and
    /// explicitly documents that a non-call pattern resolves to no name at
    /// all, "keeping value bindings out of func_types"). Both quirks fully
    /// claim their respective flat-array entries; `call_types`/
    /// `func_types` below list the real node kinds anyway (matching every
    /// other quirk-claimed row's convention, e.g. [`Self::c`]/[`Self::d`])
    /// so a caller inspecting this table sees the true vocabulary even
    /// though the generic engine's own default single-field reconstruction
    /// never actually reaches them.
    pub const fn typst() -> Self {
        Self {
            name: "typst",
            func_types: &["let"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["call"],
            // Unused placeholders -- see this const's own doc comment:
            // `call` is fully claimed by `generic::typst_call_override`
            // (its real field is `item`, with no separate `arguments`
            // field to point at at all).
            call_function_field: "UNUSED_SEE_TYPST_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_TYPST_CALL_OVERRIDE",
            import_types: &["import", "include"],
            branch_types: &["if", "for", "while"],
            decorator_types: &[],
            // Unused -- see this const's own doc comment: `let`'s naming
            // is fully claimed by `generic::typst_quirk` (nested-`call`-
            // pattern unwrap), never the flat `name_field` lookup.
            name_field: "UNUSED_SEE_TYPST_QUIRK",
            body_field: "UNUSED_SEE_TYPST_QUIRK",
        }
    }

    /// WGSL (WebGPU Shading Language, `.wgsl`) -- language-parity wave
    /// G2.3e. Grammar: `tree-sitter-wgsl-bevy` 0.1.4 (crates.io,
    /// `repository`: github.com/tree-sitter-grammars/tree-sitter-wgsl-bevy,
    /// an actively-maintained fork of `szebniok/tree-sitter-wgsl` --
    /// copyright line "Konrad Bochnia" (szebniok's real name) byte-matches
    /// this crate's own vendored `internal/cbm/vendored/grammars/wgsl/LICENSE`
    /// exactly, confirming the identical grammar lineage the baseline itself
    /// vendored; the PLAIN `tree-sitter-wgsl` crate name on crates.io is
    /// REJECTED -- last published 2022-06 at v0.0.6, hard-pins `tree-sitter
    /// ~0.20.6` as a normal runtime dependency, the same incompatible-ABI
    /// blocker this wave's Sway/Wolfram grammars also hit). Depends on
    /// `tree-sitter-language` as its runtime binding (`tree-sitter` core is
    /// `[dev-dependencies]`-only). `function_declaration`/`struct_declaration`
    /// DO have ordinary `name` fields, AND `function_declaration` has an
    /// ordinary required `body` field (a `compound_statement`) -- confirmed
    /// via a real `node-types.json` dump -- NOT the fully-unfielded posture
    /// this crate's own [`Self::d`] row has), so def extraction needs no
    /// quirk at all. THREE real corrections against the baseline's own
    /// `wgsl_*` arrays: (1)
    /// `wgsl_call_types`' sole entry,
    /// `"type_constructor_or_function_call_expression"`, is real but has
    /// ZERO fields of its own at all (confirmed: `"fields": {}`) -- the
    /// callee is only reachable by descending through nested wrapper
    /// children to the first `identifier` leaf, EXACTLY mirroring this
    /// crate's own baseline source `extract_wgsl_callee`'s
    /// `while (named_child_count > 0 && type != "identifier") head =
    /// named_child(head, 0)` loop -- caught via `call_override`, this
    /// row's `call_function_field`/`call_arguments_field` are unused
    /// placeholders; (2) `wgsl_module_types`' `"translation_unit"` entry
    /// does not exist anywhere in this grammar's real node-kind vocabulary
    /// at all (confirmed via an exhaustive `node-types.json` scan) -- this
    /// grammar's actual (and only) root rule is named `source_file`
    /// (matching plain WGSL/Bevy-preprocessor source with no distinct
    /// "translation unit" concept at all), corrected here rather than
    /// copied blindly; (3) an implementation slip THIS row itself
    /// initially introduced (caught only by actually RUNNING the
    /// extractor against a real fixture with a call nested inside a
    /// function body, not by `node-types.json` inspection alone):
    /// `body_field` was first written as an unused placeholder string on
    /// the mistaken assumption that this language's own def/call shapes
    /// needed no body recursion at all -- in fact `function_declaration`
    /// DOES have a real `body` field (see this doc comment's opening
    /// sentence), and leaving it unset meant [`crate::languages::generic::walk`]'s
    /// own func/method branch's `node.child_by_field_name(spec.body_field)`
    /// lookup always returned `None`, silently never recursing into ANY
    /// WGSL function body at all -- every call/branch/nested construct
    /// inside a function was invisible until this was corrected to the
    /// real `"body"` field name. Sway/WGSL additionally share the
    /// baseline's own `extract_defs.c:3770` struct-vs-class relabeling
    /// quirk (both languages' `struct_*` node emits
    /// [`crate::parsers::SymbolKind::Struct`], not the generic class-shape
    /// fallback's default `Class`) -- see [`Self::sway`]'s own doc comment
    /// for the shared rationale.
    pub const fn wgsl() -> Self {
        Self {
            name: "wgsl",
            func_types: &["function_declaration"],
            method_types: &[],
            class_types: &[
                "struct_declaration",
                "type_alias_declaration",
                "type_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["type_constructor_or_function_call_expression"],
            // Unused placeholders -- see this const's own doc comment:
            // fully claimed by `generic::wgsl_call_override` (deepest-
            // identifier-descent reconstruction, no fields on this node
            // kind at all).
            call_function_field: "UNUSED_SEE_WGSL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_WGSL_CALL_OVERRIDE",
            import_types: &["enable_directive"],
            branch_types: &[
                "if_statement",
                "switch_statement",
                "for_statement",
                "while_statement",
                "loop_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Wolfram Language (Mathematica, `.wl`/`.wls`/`.m`) -- language-parity
    /// wave G2.3e. Grammar VENDORED (no crates.io release exists under any
    /// discoverable name -- confirmed via the crates.io sparse index, which
    /// 404s on `tree-sitter-wolfram`): sourced from
    /// `LumaKernel/tree-sitter-wolfram` (GitHub-only, MIT-licensed) --
    /// confirmed the SAME grammar lineage this crate's own vendored
    /// `internal/cbm/vendored/grammars/wolfram/parser.c` targets by directly
    /// cross-checking that vendored `parser.c`'s own symbol table
    /// (`sym_set_delayed_top`/`sym_set_top`/`sym_apply`/`sym_user_symbol`/
    /// `sym_builtin_symbol`, ABI 13, `FIELD_COUNT 0`) against this repo's own
    /// `grammar.js` rule names -- EXACT match on every symbol (the vendored
    /// copy's own `LICENSE` copyright line, "DeusData" 2025, is the
    /// baseline PROJECT's own re-license note when vendoring -- confirmed
    /// via a web search that `DeusData/codebase-memory-mcp` on GitHub is
    /// literally the baseline project's own upstream repo -- not a
    /// competing/different upstream author, so it does not contradict this
    /// match). This repo's own published `Cargo.toml` (last real release
    /// 0.1.1, 2022) hard-pins `tree-sitter = ">= 0.19, < 0.21"` as a normal
    /// runtime dependency AND was never even published to crates.io at all
    /// (confirmed via the sparse index 404) -- bound via
    /// `crates/enforcer-memory/vendor/tree-sitter-wolfram-local/`, the same
    /// local-path-vendor pattern as `tree-sitter-squirrel-local`/
    /// `tree-sitter-sway-local` (not a plain `git = "..."` dependency, unlike
    /// Crystal's precedent: Crystal's own upstream `lib.rs` already used
    /// `tree-sitter-language`, this repo's does not). This grammar is
    /// ENTIRELY unfielded (`FIELD_COUNT 0` -- confirmed directly from the
    /// vendored `parser.c`'s own `#define`, the most extreme case of this
    /// posture in this wave, even more so than WGSL/D), so EVERY construct
    /// below needs a full quirk claim, mirroring the baseline's OWN
    /// dedicated non-generic code for this exact language (`extract_defs.c`'s
    /// `resolve_wolfram_func_name`, `extract_calls.c`'s
    /// `extract_wolfram_callee`, `extract_imports.c`'s
    /// `process_wolfram_get_top`/`process_wolfram_needs`) -- confirmed via
    /// this repo's own `grammar.js`: `set`/`set_delayed`/... are a bare
    /// `seq(EXPR, operator_token, EXPR)` with no field names at all; `apply`
    /// (Wolfram's `f[x]` call syntax) is `seq(EXPR, "[", EXPR, "]")`, same
    /// positional shape. One behavior explicitly carried over from the
    /// baseline rather than left to this crate's own default (documented so
    /// it reads as a deliberate choice, not an oversight): `extract_defs.c`'s
    /// own dispatcher (`descend_into_func`) explicitly lists
    /// `CBM_LANG_WOLFRAM` alongside JS/TS/TSX as one of the few languages
    /// whose function bodies get walked for NESTED named definitions too
    /// (Wolfram's own idiom of a `Module[{...}, inner[x_] := ...]` local
    /// helper pattern) -- `wolfram_quirk`'s own scoped-body walk mirrors
    /// this by recursing into a `set`/`set_delayed`'s RHS unconditionally
    /// (the generic engine's own func/method branch would otherwise never
    /// reach it at all, since this whole construct is quirk-claimed, not
    /// flat-array-dispatched). One real, non-obvious grammar-shape finding
    /// caught only by running the extractor against a real fixture (not
    /// `node-types.json`/`grammar.js` inspection alone): `apply`'s bracket
    /// DELIMITER tokens (`apply_bracket_begin`/`apply_bracket_end`, the
    /// `[`/`]` themselves) are ALSO marked `.named = true` in this
    /// grammar's generated `parser.c` -- meaning a naive "every named
    /// child after the callee head is an argument" positional scan wrongly
    /// counts the brackets as extra "arguments", and the baseline's own
    /// `process_wolfram_needs`'s hardcoded `SECOND_IDX = 1` read for
    /// `Needs["pkg\`"]`'s import path actually lands on
    /// `apply_bracket_begin`'s own `"["` text, not the real package
    /// string -- a genuine baseline defect (confirmed against the
    /// baseline's OWN vendored `parser.c`, which has the identical
    /// `.named = true` metadata on that symbol, not merely a difference
    /// between grammar copies). `generic::wolfram_needs_import_path`/
    /// `generic::wolfram_apply_arg_texts` both fix this as a genuine,
    /// documented improvement (find the `string`-kind child by KIND, and
    /// exclude the two bracket-delimiter kinds from argument texts)
    /// rather than reproducing it -- see those functions' own doc comments
    /// for the full finding.
    pub const fn wolfram() -> Self {
        Self {
            name: "wolfram",
            func_types: &["set_delayed_top", "set_top", "set_delayed", "set"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["apply"],
            // Unused placeholders -- see this const's own doc comment:
            // `apply` is fully claimed by `generic::wolfram_call_override`
            // (positional-child reconstruction, `FIELD_COUNT 0` in this
            // grammar).
            call_function_field: "UNUSED_SEE_WOLFRAM_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_WOLFRAM_CALL_OVERRIDE",
            // `get_top`'s import detection needs a positional scan (see
            // `generic::wolfram_quirk`'s own doc comment) -- this array is
            // never consulted for Wolfram (the walker's own `import_types`
            // branch expects a distinct import-shaped node kind AND relies
            // on the quirk hook regardless, so leaving this non-empty would
            // only cause `get_top` to be claimed twice); `apply`-where-
            // head-is-`Needs` has no distinct node kind of its own at all
            // (it is an ordinary `apply`), so it is caught in the SAME
            // `call_types` branch as every other call, not via this array.
            import_types: &["get_top"],
            branch_types: &[],
            decorator_types: &[],
            // Unused placeholders -- see this const's own doc comment:
            // `set`/`set_delayed`/... naming is fully claimed by
            // `generic::wolfram_quirk` (positional-child reconstruction,
            // `FIELD_COUNT 0` in this grammar).
            name_field: "UNUSED_SEE_WOLFRAM_QUIRK",
            body_field: "UNUSED_SEE_WOLFRAM_QUIRK",
        }
    }

    /// Slang (shader language, superset of HLSL/GLSL-family syntax, `.slang`)
    /// -- language-parity wave G2.3e. Grammar: `tree-sitter-slang` 0.3.1
    /// (crates.io, `repository`: github.com/theHamsta/tree-sitter-slang --
    /// copyright line "Stephan Seitz" byte-matches this crate's own
    /// vendored `internal/cbm/vendored/grammars/slang/LICENSE` exactly;
    /// this is the SAME author who also publishes this crate's own
    /// [`Self::wgsl`] grammar dependency, `tree-sitter-wgsl-bevy`, though
    /// the two are unrelated grammar lineages). Depends on
    /// `tree-sitter-language` as its runtime binding. This grammar extends
    /// tree-sitter-cpp's own grammar (Slang's real-world syntax is a C/C++-
    /// family superset with shader-specific additions) -- confirmed via a
    /// real `node-types.json` dump: every node kind below (`function_definition`/
    /// `class_specifier`/`struct_specifier`/`enum_specifier`/`call_expression`)
    /// uses the SAME C++-shaped fields this crate's own [`Self::cpp`] row
    /// already handles via `generic::cpp_quirk`'s full claim. Unlike C/C++
    /// (whose `function_definition` needs `cpp_quirk`'s dedicated declarator-
    /// unwrap for out-of-line/templated definitions), this grammar's
    /// `function_definition` has an ordinary `declarator`-nested `name`
    /// resolvable the SAME way -- rather than duplicate `cpp_quirk`'s
    /// C-declarator-unwrap logic for a language with no dedicated bespoke
    /// extractor of its own to reproduce, this row reuses
    /// [`crate::languages::generic::cpp_quirks`] verbatim (same posture as
    /// [`Self::cuda`]/[`Self::glsl`]'s own documented C/C++-array-reuse
    /// choice) -- confirmed via a real parse-tree dump that every field
    /// `cpp_quirk` reads is shape-identical here. `module_declaration`
    /// (this grammar's own namespace-like construct, absent from C++'s own
    /// array) is the one Slang-specific addition beyond plain C++ -- added
    /// to `class_types` here since its own `name` field resolves the SAME
    /// way as a namespace name would, and [`crate::languages::generic::walk`]'s
    /// own generic class-shape fallback (not a dedicated quirk) already
    /// handles an unrecognized-by-quirk class-shaped node correctly via
    /// plain `name_field` lookup. Every def/call array entry matched this
    /// grammar's real vocabulary exactly on cross-check (the cleanest
    /// def/call result of any language this wave researched), but
    /// `import_types` needed a real, non-cosmetic correction found only
    /// by RUNNING the resulting extractor against a real fixture (not
    /// `node-types.json` inspection alone): a bare `cpp_quirks()` reuse
    /// has NO import handling at all (C++ itself has no equivalent
    /// construct in this shape, so `cpp_quirk`'s own `on_unmatched_node`
    /// never claims `import_statement`/`import_declaration`, silently
    /// dropping every Slang import edge) -- this row is now paired with a
    /// dedicated `generic::slang_quirk` (composed on top of `cpp_quirk`
    /// for every other construct, not a full reimplementation) rather
    /// than the plain `cpp_quirks()` reuse this doc comment originally
    /// described. `import_types`' `"import"` entry is additionally a
    /// PHANTOM (this grammar's own `node-types.json` marks it
    /// `"named": false` -- the bare keyword TOKEN, never a distinct AST
    /// node any extractor could match against), the same class of dead
    /// array entry this wave's Starlark/Templ rows also found -- see
    /// `generic::slang_quirk`'s own doc comment for the full fix.
    pub const fn slang() -> Self {
        Self {
            name: "slang",
            func_types: &["function_definition", "lambda_expression"],
            method_types: &["function_definition"],
            class_types: &[
                "class_specifier",
                "enum_specifier",
                "module_declaration",
                "struct_specifier",
                "type_definition",
                "union_specifier",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression", "new_expression"],
            call_function_field: "UNUSED_SEE_SLANG_CPP_REUSE",
            call_arguments_field: "UNUSED_SEE_SLANG_CPP_REUSE",
            import_types: &["import", "import_declaration", "import_statement"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "switch_statement",
                "case_statement",
                "&&",
                "||",
            ],
            decorator_types: &[],
            name_field: "UNUSED_SEE_SLANG_CPP_REUSE",
            body_field: "UNUSED_SEE_SLANG_CPP_REUSE",
        }
    }

    /// SCSS (`.scss`) -- language-parity wave G2.3a. Node kinds verified
    /// directly against the `tree-sitter-scss` 1.0.0 crate's own
    /// `src/node-types.json` PLUS a real parse-tree dump (`cargo run`
    /// against a scratch crate depending on the vendored grammar directly
    /// -- see [`crate::languages::generic::scss_quirks`]'s own doc comment
    /// for why this grammar is vendored rather than a plain crates.io
    /// dependency). Two real, confirmed corrections against the baseline's
    /// own `scss_*` arrays (`codebase-memory-mcp/internal/cbm/
    /// lang_specs.c`:641-647):
    /// - The baseline's own `extract_defs.c` doc comment (:907-915) claims
    ///   `function_statement`/`mixin_statement` have "no `name` field...
    ///   the def name is a plain `name` child node" -- FALSE for this
    ///   actual grammar version: `node-types.json` gives both node kinds a
    ///   genuine `"name"` FIELD (not merely a same-named child), confirmed
    ///   by the real parse-tree dump too (`name: identifier` shown with a
    ///   real field label). This row's own `name_field: "name"` therefore
    ///   works directly through the generic engine's ordinary
    ///   `child_text(node, spec.name_field, ..)` path with NO quirk needed
    ///   for the def-name case at all -- a genuine simplification the
    ///   baseline's own comment did not anticipate.
    /// - `mixin_statement`/`function_statement`'s own body-holding child is
    ///   named `"block"` in the tree but is NOT a real field (`"fields":
    ///   {}` for both in `node-types.json`; `block` is listed only under
    ///   `"children"`) -- so [`Self::body_field`] below is a placeholder,
    ///   never actually consulted:
    ///   [`crate::languages::generic::scss_on_method_defined`] finds the
    ///   `block` child by KIND (not field) and recurses into it itself,
    ///   mirroring the baseline's own `include_statement`/`call_expression`
    ///   posture of reaching an unfielded child by kind rather than
    ///   assuming a field exists. This row's own FIRST implementation
    ///   wired this recursion through
    ///   [`crate::languages::generic::scss_quirk`]'s own
    ///   `on_unmatched_node` hook instead -- a REAL, CONFIRMED bug (not a
    ///   hypothetical): `walk`'s own func/method branch reaches
    ///   `function_statement`/`mixin_statement` directly (this row's own
    ///   `name_field: "name"` IS a real, working field), records the
    ///   symbol, and returns BEFORE `on_unmatched_node` would ever run for
    ///   either node kind -- `on_method_defined` (called unconditionally
    ///   right before that same early return) is the correct seam
    ///   instead, and is what this row now uses.
    /// - `call_expression`'s callee is a `function_name` child (confirmed
    ///   real via both `node-types.json` and the parse dump) -- also
    ///   unfielded (`"fields": {}`), matching the baseline's own
    ///   `extract_scss_callee`'s `cbm_find_child_by_kind(node,
    ///   "function_name")` (NOT `ts_node_child_by_field_name`) exactly, so
    ///   [`Self::call_function_field`] below is likewise a placeholder --
    ///   [`crate::languages::generic::scss_call_override`] claims it via
    ///   kind-search instead.
    /// - `include_statement`'s target is a bare `identifier` child
    ///   (unfielded, confirmed via both sources) -- matches the baseline's
    ///   own `extract_scss_callee`'s `cbm_find_child_by_kind(node,
    ///   "identifier")` exactly; also claimed by
    ///   [`crate::languages::generic::scss_call_override`] (recorded as a
    ///   call whose callee is the included mixin's own name, same
    ///   "@include foo" -> CALLS "foo" convention the baseline's own
    ///   `extract_scss_callee`'s `include_statement` branch already uses).
    /// - `@import`/`@use` (this row's `import_types`) target a
    ///   `string_value` node whose own text (quotes included, confirmed:
    ///   no separate `string_content` child exists for a plain unquoted-
    ///   looking literal like `"base"` in this grammar -- verified by the
    ///   real parse dump, NOT assumed) needs manual quote-stripping --
    ///   [`crate::languages::generic::scss_quirk`] finds `string_value` by
    ///   kind-search and strips quote characters itself, mirroring the
    ///   baseline's own `css_push_import_from_stmt`'s `strip_quotes`
    ///   fallback path (`internal/cbm/extract_imports.c`:1987-1989) --
    ///   this crate's own SCSS row does NOT reproduce that function's
    ///   OTHER `string_content`-descendant-preferred branch, since this
    ///   grammar's plain-literal shape never actually reaches it (dead
    ///   code for this specific import-statement shape, confirmed by the
    ///   dump, not merely assumed absent).
    pub const fn scss() -> Self {
        Self {
            name: "scss",
            func_types: &["function_statement", "mixin_statement"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["stylesheet"]` -- the same
            // root-node-as-module-clause bug [`Self::fortran`]'s/
            // [`Self::elm`]'s/[`Self::vimscript`]'s own doc comments
            // already document in full: `stylesheet` is this grammar's
            // own ROOT node, so the generic engine's own `module_types`
            // branch would grab whatever def/statement happens to come
            // FIRST as a garbage "Module" symbol name -- SCSS has no
            // repeatable, name-only module/namespace CLAUSE construct at
            // all (its own `@use`/`@import` are already captured as
            // IMPORTS above, not a separate module-declaration concept).
            module_types: &[],
            call_types: &["call_expression", "include_statement"],
            // Never actually consulted -- see this const's own doc
            // comment: `call_expression`'s real callee field is the
            // unfielded `function_name` child, claimed by
            // `generic::scss_call_override` via kind-search instead.
            call_function_field: "UNUSED_SEE_SCSS_CALL_OVERRIDE",
            call_arguments_field: "arguments",
            import_types: &["import_statement", "use_statement"],
            branch_types: &["if_statement"],
            decorator_types: &[],
            name_field: "name",
            // Never actually consulted -- see this const's own doc
            // comment: the real `block` child is unfielded, found by
            // kind-search in `generic::scss_quirk` instead.
            body_field: "UNUSED_SEE_SCSS_QUIRK",
        }
    }

    /// CMake (`.cmake`/`CMakeLists.txt`) -- language-parity wave G2.3a.
    /// Node kinds verified directly against the `tree-sitter-cmake` 0.7.2
    /// crate's own `src/node-types.json` PLUS a real parse-tree dump.
    /// This grammar is entirely FIELD-FREE for every construct this row
    /// cares about (`node-types.json` gives every one of
    /// `function_def`/`macro_def`/`function_command`/`macro_command`/
    /// `normal_command` an empty `"fields": {}`, confirmed by the real
    /// dump too -- no field label on ANY child of these node kinds) --
    /// [`Self::name_field`]/[`Self::body_field`]/[`Self::call_function_field`]
    /// below are consequently all placeholders, matching this row's own
    /// [`crate::languages::generic::cmake_quirk`]/
    /// [`crate::languages::generic::cmake_call_override`] fully claiming
    /// every one of `func_types`/`call_types` via kind-search before the
    /// generic engine's own field-keyed fallback would ever run -- same
    /// "arrays exist purely as at-a-glance documentation" posture as
    /// [`Self::c`]/[`Self::squirrel`]. Matches the baseline's own
    /// `internal/cbm/extract_defs.c` CMake name-resolution comment
    /// (:1056-1073, "the name is nested as *_command > argument_list >
    /// argument > unquoted_argument"). CMake has no dedicated
    /// `extract_*_callee` at all in the baseline -- a `normal_command`'s
    /// callee is simply its own first named child, an `identifier`, which
    /// this row's generic `call_types` dispatch reaches once
    /// `generic::cmake_call_override` claims it. `branch_types` is
    /// intentionally empty: the baseline's
    /// own `cmake_*` arrays have no `branch_types` entry either (CMake's
    /// `if_condition` is a control-flow wrapper this crate does not treat
    /// as complexity-relevant for this Tier-2 scope, matching baseline
    /// depth).
    pub const fn cmake() -> Self {
        Self {
            name: "cmake",
            func_types: &["function_def", "macro_def"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["source_file"]` -- the same
            // root-node-as-module-clause bug [`Self::fortran`]'s/
            // [`Self::elm`]'s/[`Self::vimscript`]'s own doc comments
            // already document in full: `source_file` is this grammar's
            // own ROOT node, so the generic engine's own `module_types`
            // branch would grab whatever def/statement happens to come
            // FIRST as a garbage "Module" symbol name -- CMake has no
            // repeatable, name-only module/namespace CLAUSE construct at
            // all (confirmed via a real parse-tree dump of the fixture
            // this row's own tests use).
            module_types: &[],
            call_types: &["normal_command"],
            // Never actually consulted -- see this const's own doc
            // comment.
            call_function_field: "UNUSED_SEE_CMAKE_CALL_OVERRIDE",
            call_arguments_field: "argument_list",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "UNUSED_SEE_CMAKE_QUIRK",
            body_field: "UNUSED_SEE_CMAKE_QUIRK",
        }
    }

    /// Makefile (`Makefile`/`makefile`/`GNUmakefile`/`*.mk`) --
    /// language-parity wave G2.3a. Node kinds verified directly against
    /// the `tree-sitter-make` 1.1.1 crate's own `src/node-types.json` PLUS
    /// a real parse-tree dump. One real, confirmed correction against the
    /// baseline's own `makefile_call_types` array
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:838): includes
    /// `"shell_function"` (a real, distinct node kind for `$(shell ...)`,
    /// confirmed) alongside `"function_call"` -- matches this row's own
    /// `call_types` below exactly, no correction needed there. The real
    /// correction is narrower: the baseline's own `extract_make_callee`
    /// (`internal/cbm/extract_calls.c`:958-970) hardcodes the literal
    /// string `"shell"` for a `shell_function` node's callee rather than
    /// reading a field, on the assumption it has none -- but
    /// `node-types.json` (and the real dump) confirm `shell_function`
    /// genuinely DOES carry a real `"function"` FIELD (whose sole possible
    /// child kind is the anonymous `shell` keyword token) -- the exact
    /// same field name and shape as `function_call`'s own `"function"`
    /// field, so this row's own [`Self::call_function_field`] reads BOTH
    /// `function_call` and `shell_function` uniformly through the one real
    /// `"function"` field via the generic engine's ordinary single-field
    /// path -- a genuine simplification the baseline's own hardcoded-
    /// string special case did not need to take.
    /// - [`Self::call_arguments_field`] below is a placeholder, NOT a
    ///   real fix: `node-types.json` confirms `function_call`/
    ///   `shell_function` EACH have exactly ONE field, `"function"` --
    ///   NEITHER has an `"arguments"` field at all (a real bug this row's
    ///   own initial implementation had, assuming one existed without
    ///   checking specifically for THIS grammar -- silently empty
    ///   `arg_texts` for every Makefile call, caught only via a dedicated
    ///   `arg_texts` assertion added during this same wave's cross-
    ///   language field-vs-kind audit). The real `arguments`/
    ///   `shell_command` child is unfielded, found by kind in
    ///   [`crate::languages::generic::makefile_call_override`] instead --
    ///   claimed in full (both the callee-text half, which the generic
    ///   engine's own default path WOULD have gotten right on its own,
    ///   and the arguments half, which it cannot express) rather than
    ///   splitting the two into a field-based default plus a partial
    ///   override.
    /// - `rule`'s own target/name is NOT reachable via any field for the
    ///   common `target: prereqs` shape (confirmed by the real dump: the
    ///   `targets` child carries no field label) -- despite
    ///   `node-types.json` listing singular `target`/`prerequisite` FIELDS
    ///   on `rule`, those are for a different syntactic variant (a static
    ///   pattern rule) this row's own fixture does not exercise; the
    ///   common case genuinely has no working field, matching the
    ///   baseline's own `resolve_makefile_func_name`
    ///   (`internal/cbm/extract_defs.c`:394-400) exactly: find the
    ///   `targets` child by KIND, take its own first named child (a
    ///   `word`). [`crate::languages::generic::makefile_quirk`] mirrors
    ///   this exactly.
    /// - `include_directive`'s target is nested two levels deep
    ///   (`filenames` field -> `list` -> `word`, confirmed by the real
    ///   dump), not a direct field/child text -- also handled by
    ///   [`crate::languages::generic::makefile_quirk`].
    /// - The baseline's own `parse_generic_imports` default-case dispatch
    ///   (Makefile is not in `extract_imports.c`'s explicit per-language
    ///   switch, so it falls through to the generic, DIRECT-CHILDREN-OF-
    ///   ROOT-ONLY scan) would still work correctly for `include_directive`
    ///   specifically IF it always appears at the top level -- but this
    ///   crate's own [`crate::languages::generic::walk`]/`walk_children`
    ///   is unconditionally recursive regardless, so an `include_directive`
    ///   nested inside a `conditional` block (common in real Makefiles) is
    ///   still found here where the baseline's own root-direct-children-
    ///   only scan would miss it -- same "this crate's recursive walk
    ///   doesn't have that bug" precedent as [`Self::elixir`]'s own doc
    ///   comment already documents for Elixir's `defmodule`-wrapping.
    pub const fn makefile() -> Self {
        Self {
            name: "makefile",
            func_types: &["rule"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["makefile"]` -- the same
            // root-node-as-module-clause bug [`Self::fortran`]'s/
            // [`Self::elm`]'s/[`Self::vimscript`]'s own doc comments
            // already document in full: `makefile` is this grammar's own
            // ROOT node, so the generic engine's own `module_types`
            // branch would grab whatever def/statement happens to come
            // FIRST as a garbage "Module" symbol name -- Make has no
            // repeatable, name-only module/namespace CLAUSE construct at
            // all (confirmed via a real parse-tree dump of the fixture
            // this row's own tests use).
            module_types: &[],
            call_types: &["function_call", "shell_function"],
            // Both never actually consulted -- see this const's own doc
            // comment: `generic::makefile_call_override` unconditionally
            // claims every `call_types` entry (returns `true` for both
            // `function_call`/`shell_function`), so `walk`'s own
            // field-based fallback path (which these two constants feed)
            // never runs for Makefile at all -- claimed in full rather
            // than split between a working default callee-field path and
            // an arguments-only override, since the override needs the
            // node kind's own real `"function"` field read anyway to
            // build one coherent `CallRef`.
            call_function_field: "UNUSED_SEE_MAKEFILE_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_MAKEFILE_CALL_OVERRIDE",
            import_types: &["include_directive"],
            branch_types: &[],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment: `rule`'s real name comes from its unfielded
            // `targets` child, found by kind-search in
            // `generic::makefile_quirk` instead.
            name_field: "UNUSED_SEE_MAKEFILE_QUIRK",
            body_field: "UNUSED_SEE_MAKEFILE_QUIRK",
        }
    }

    /// Fortran (`.f90`/free-form modern Fortran) -- language-parity wave
    /// G2.3a. Node kinds verified directly against the `tree-sitter-fortran`
    /// 0.6.0 crate's own `src/node-types.json` PLUS a real parse-tree dump.
    /// One real, confirmed correction against the baseline's own
    /// `fortran_call_types` array (`codebase-memory-mcp/internal/cbm/
    /// lang_specs.c`:781): `["call_expression", "keyword_argument",
    /// "call"]` -- `"call"` does not exist as a node kind in this grammar
    /// at all (confirmed via `node-types.json`'s full node-kind list, not
    /// merely absent from a small fixture), and while `"call_expression"`
    /// IS real (a plain function-call expression, e.g. `a = helper(r) *
    /// 2`, confirmed by the dump), a `call helper(x)` SUBROUTINE-CALL
    /// STATEMENT -- extremely common, idiomatic Fortran, present in this
    /// row's own fixture -- parses as a THIRD, entirely different node
    /// kind, `"subroutine_call"`, which the baseline's own array omits
    /// completely. This is a real baseline gap (not a deliberate scope
    /// choice -- `call` statements are not an obscure edge case), so this
    /// row's own `call_types` below ADDS `"subroutine_call"` alongside the
    /// baseline's real `"call_expression"`, dropping only the baseline's
    /// own dead `"keyword_argument"`/`"call"` entries (the former is a
    /// real node kind in this grammar, confirmed, but for an entirely
    /// unrelated construct -- a `key=value` argument inside a type-spec
    /// like `character(len=*)`, never a call site -- including it in
    /// `call_types` would silently emit bogus CALLS edges from ordinary
    /// type declarations, so it is deliberately excluded here rather than
    /// blindly transcribed). `call_expression`'s own callee is a real
    /// `"function"` FIELD (confirmed) but `subroutine_call`'s own callee
    /// field is instead named `"subroutine"` (also confirmed, distinct
    /// field name for the same semantic role) -- the generic engine's
    /// single [`Self::call_function_field`] cannot serve both node kinds
    /// with one field name, so [`Self::call_function_field`] below is a
    /// placeholder and [`crate::languages::generic::fortran_call_override`]
    /// claims both kinds explicitly, reading whichever real field applies.
    /// - `function`/`subroutine` (this row's own `func_types`, the OUTER
    ///   wrapping node) have NO fields at all (confirmed, `"fields": {}`)
    ///   -- the real name lives on the NESTED `function_statement`/
    ///   `subroutine_statement` child's own genuine `"name"` field (a
    ///   `name`-typed node), exactly matching the baseline's own
    ///   documented quirk (`internal/cbm/extract_defs.c`:812-826, "the
    ///   outer node walk_defs matched has no name itself"). This row's
    ///   own [`Self::name_field`]/[`Self::body_field`] are consequently
    ///   both placeholders -- [`crate::languages::generic::fortran_quirk`]
    ///   fully claims `function`/`subroutine`, finding the inner
    ///   `*_statement` child by kind, reading ITS real `"name"` field, and
    ///   recursing into the OUTER node's own body content itself (the
    ///   outer node's un-fielded remaining children after the
    ///   `*_statement`/`end_*_statement` pair).
    /// - `use_statement`'s own `module_name` child is real but unfielded
    ///   (confirmed) -- the baseline is not in `extract_imports.c`'s
    ///   explicit per-language switch, so it falls through to the
    ///   generic, direct-children-of-root-only scan
    ///   (`parse_generic_imports`) -- a real, confirmed miss for
    ///   virtually all real Fortran files (this row's own fixture wraps
    ///   everything in one top-level `module`, the same "everything
    ///   nested one level down, root's own direct children never see it"
    ///   shape as Elixir's `defmodule`-wrapping and Puppet's
    ///   class/node-body nesting) -- this crate's own unconditionally
    ///   recursive walk does not have that bug, same precedent as
    ///   [`Self::elixir`]'s own doc comment.
    pub const fn fortran() -> Self {
        Self {
            name: "fortran",
            func_types: &["function", "subroutine"],
            method_types: &[],
            class_types: &["derived_type_definition"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["translation_unit"]` -- see this
            // const's own doc comment's own final paragraph for the full
            // finding: the generic engine's own `module_types` branch
            // (`walk`'s `first_named_child_text(node, ..)`) grabs the
            // FIRST NAMED CHILD's entire text as the "module name",
            // correct only for a real, repeatable, name-only module/
            // namespace CLAUSE (Go's `package_clause`, C#'s
            // `namespace_declaration`) -- `translation_unit` is this
            // grammar's own ROOT node, whose first named child is
            // whatever def/statement happens to come first (confirmed via
            // a real parse-tree dump: a `module widget ... end module`
            // wrapping everything produces a garbage "Module" symbol
            // whose name is the ENTIRE multi-line `module` node's own
            // text, not `"widget"`). Fortran's real, repeatable,
            // Go-`package_clause`-equivalent construct (`module_statement`,
            // confirmed via `node-types.json` to have a genuine own `name`
            // child) is nested one level inside the outer `module` wrapper
            // node this row's own `func_types` array does NOT list at all
            // (only `function`/`subroutine` are, per this row's own doc
            // comment) -- wiring it correctly would need a THIRD
            // `on_unmatched_node` arm (claiming `module` itself) this
            // wave's own Tier-2 scope (defs+calls+imports; Module symbols
            // are bonus, not required) does not need; left as a genuine,
            // documented gap rather than a plausible-looking but wrong
            // symbol.
            module_types: &[],
            call_types: &["call_expression", "subroutine_call"],
            // Never actually consulted -- see this const's own doc
            // comment: the two real call-shaped node kinds use two
            // DIFFERENT field names (`function` vs `subroutine`),
            // claimed explicitly by `generic::fortran_call_override`.
            call_function_field: "UNUSED_SEE_FORTRAN_CALL_OVERRIDE",
            call_arguments_field: "argument_list",
            import_types: &["use_statement", "include_statement"],
            branch_types: &[
                "if_statement",
                "do_loop_statement",
                "where_statement",
                "select_case_statement",
            ],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment: `function`/`subroutine` are fully claimed by
            // `generic::fortran_quirk` before this file's own
            // `name_field`/`body_field`-keyed fallback would ever run.
            name_field: "UNUSED_SEE_FORTRAN_QUIRK",
            body_field: "UNUSED_SEE_FORTRAN_QUIRK",
        }
    }

    /// VimScript (`.vim`) -- language-parity wave G2.3a. Node kinds
    /// verified directly against the `tree-sitter-vim` 0.4.0 crate's own
    /// `src/node-types.json` PLUS a real parse-tree dump. Two real,
    /// confirmed corrections against the baseline's own `vim_*` arrays
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:743-750):
    /// - `vim_func_types = ["function_definition", "function_declaration",
    ///   "lambda_expression"]` double-lists two node kinds that are NEVER
    ///   independent top-level definitions: `function_declaration` (which
    ///   DOES carry the real `name`/`parameters` fields, confirmed) is
    ///   ALWAYS a nested, non-optional child of the OUTER
    ///   `function_definition` wrapper (confirmed by both `node-types.json`
    ///   -- `function_definition`'s own required-children list includes
    ///   exactly one `function_declaration` -- and the real parse dump),
    ///   and critically has NO `"body"` field of its own (the function's
    ///   actual statement body is a SIBLING `body` child of the OUTER
    ///   `function_definition`, not nested inside `function_declaration`
    ///   at all). Listing both kinds in one flat `func_types` array would
    ///   make the generic engine's own func/method branch try
    ///   `function_declaration` as an independent def (succeeding at the
    ///   name lookup, since its `name` field IS real) but then find no
    ///   `body_field` match and silently skip recursing into the body --
    ///   silently losing every call/nested-def inside every VimScript
    ///   function in the whole crate, a severe correctness bug, not a
    ///   cosmetic one. This row's own `func_types` below therefore lists
    ///   ONLY the outer `"function_definition"`; `generic::vimscript_quirk`
    ///   fully claims it, reading the name off the nested
    ///   `function_declaration`'s own real `name` field and recursing into
    ///   the OUTER node's own real `body` field. `lambda_expression` is
    ///   dropped entirely (an anonymous closure literal with no name to
    ///   recover -- matches this crate's "no recoverable name, no symbol"
    ///   posture elsewhere, e.g. [`Self::lua`]'s unnamed-anonymous-function
    ///   case).
    /// - `vim_import_types = ["include"]` names a node kind that is
    ///   `"named": false` in this grammar's own `node-types.json` --
    ///   i.e. `"include"` is an ANONYMOUS KEYWORD TOKEN, never a real
    ///   named AST node the generic engine's (or the baseline's own)
    ///   kind-dispatch could ever match against via `.kind()`/
    ///   `ts_node_type()` (both operate on named nodes; an anonymous
    ///   token is not independently visited as a dispatch target the way
    ///   a named statement node is). Cross-checked against this grammar's
    ///   own full `script_file` statement-kind list (confirmed via
    ///   `node-types.json`): there is no `include_statement` or
    ///   equivalent named node anywhere -- classic Vimscript (as opposed
    ///   to newer Vim9-script's own distinct `import`/`export` syntax,
    ///   which this grammar version does not appear to model at all) has
    ///   no dedicated import/include statement construct for this row to
    ///   recognize. This row's own `import_types` below is consequently
    ///   empty, matching the "dead/unreachable baseline array entry,
    ///   documented rather than blindly transcribed" precedent
    ///   [`Self::lua`]'s own doc comment already sets for a different
    ///   reason (there, a real-but-double-listed node kind; here, a
    ///   baseline entry that names no reachable node kind at all).
    ///
    /// `call_expression`'s own callee is a real `"function"` FIELD
    /// (confirmed) -- no quirk needed for the call-callee case at all.
    pub const fn vimscript() -> Self {
        Self {
            name: "vimscript",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["script_file"]` -- a real,
            // confirmed bug this row's own initial implementation had
            // (the same root-node-as-module-clause mistake
            // [`Self::fortran`]'s/[`Self::elm`]'s own doc comments
            // already document for a different reason): `script_file` is
            // this grammar's own ROOT node; the generic engine's own
            // `module_types` branch (`first_named_child_text`) grabs the
            // FIRST NAMED CHILD's entire text as the "module name" --
            // whatever def/statement happens to come first, producing a
            // garbage "Module" symbol whose name is an entire multi-line
            // `function_definition`'s own text, not a real module/script
            // name (VimScript has no repeatable, name-only module/
            // namespace CLAUSE construct analogous to Go's
            // `package_clause` at all -- there is nothing correct this
            // row COULD point `module_types` at instead).
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "UNUSED_NO_ARGUMENTS_FIELD",
            // See this const's own doc comment: intentionally empty, NOT
            // `["include"]` (an anonymous token, not a real named node --
            // dead baseline data for this grammar).
            import_types: &[],
            branch_types: &["if_statement", "for_loop", "while_loop", "try_statement"],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment: `function_definition` is fully claimed by
            // `generic::vimscript_quirk` before this file's own
            // `name_field`/`body_field`-keyed fallback would ever run.
            name_field: "UNUSED_SEE_VIMSCRIPT_QUIRK",
            body_field: "UNUSED_SEE_VIMSCRIPT_QUIRK",
        }
    }

    /// Puppet (`.pp`) -- language-parity wave G2.3a. Node kinds verified
    /// directly against the `tree-sitter-puppet` 1.3.0 crate's own
    /// `src/node-types.json` PLUS a real parse-tree dump. This grammar is
    /// entirely FIELD-FREE for every construct this row cares about
    /// (`node-types.json` gives `class_definition`/`defined_resource_type`/
    /// `function_declaration`/`node_definition`/`include_statement`/
    /// `type_declaration`/`function_call` ALL an empty `"fields": {}`,
    /// confirmed by the real dump too) -- matches the baseline's own
    /// exclusively kind-search-based name/callee resolution exactly
    /// (`internal/cbm/extract_defs.c`:1077-1086,3607-3614;
    /// `extract_calls.c`:987-1000). One real, confirmed correction against
    /// the baseline's own `puppet_class_types` array
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1508-1509):
    /// `["class_definition", "node_definition", "resource_declaration",
    /// "type_declaration"]` omits `"defined_resource_type"` entirely --
    /// Puppet's `define foo($x) { ... }` construct (a reusable,
    /// parameterized resource template, callable by name exactly like a
    /// class -- confirmed via the real parse dump to be a distinct,
    /// common, idiomatic top-level construct, not an obscure corner of the
    /// language) parses as this node kind and would be silently invisible
    /// to the generic engine with the baseline's own array as-is. This is
    /// a real baseline gap (defined types are a first-class, everyday
    /// Puppet module-authoring construct, not a deliberate scope
    /// limitation), so this row's own `class_types` below adds
    /// `"defined_resource_type"` alongside the baseline's real four,
    /// mirroring this crate's own "fill a confirmed baseline gap, document
    /// it, do not silently drop the baseline's real depth" precedent (see
    /// e.g. [`Self::elixir`]'s guard-clause-def fix, [`Self::fortran`]'s
    /// `subroutine_call` addition above). Its own name resolution mirrors
    /// `class_definition`'s exactly (a plain `identifier` or
    /// `class_identifier` direct child, confirmed by both sources) --
    /// [`crate::languages::generic::puppet_quirk`] shares one helper for
    /// both node kinds.
    /// - `resource_declaration` (e.g. `notify { "found": }`) has REAL
    ///   fields (`type`/`title`, confirmed) unlike every other node kind
    ///   this row cares about -- but its own "name" (the resource TYPE,
    ///   e.g. `notify`) is semantically closer to a call-site than a
    ///   definition (the baseline's own `puppet_call_types` array below
    ///   already lists it too, matching this row's `call_types`) --
    ///   this row records it purely as a Class-kind symbol keyed off its
    ///   own `type` field (matching baseline depth: the baseline emits no
    ///   richer resource-declaration-specific symbol kind either).
    /// - The baseline is not in `extract_imports.c`'s explicit
    ///   per-language switch (falls through to the generic,
    ///   direct-children-of-root-only `parse_generic_imports` scan) -- a
    ///   real, confirmed miss for virtually all real Puppet code (an
    ///   `include_statement` is almost always nested inside a
    ///   `class_definition`/`node_definition` body, confirmed by this
    ///   row's own fixture, never bare at top level) -- this crate's own
    ///   unconditionally recursive walk does not have that bug, same
    ///   precedent as [`Self::elixir`]/[`Self::fortran`] above.
    /// - `"include_statement"` is DELIBERATELY ABSENT from `call_types`
    ///   below despite ALSO being an `import_types` entry (and despite
    ///   also needing a CallRef, matching the baseline's own
    ///   `puppet_call_types` array listing it too) -- a real, confirmed
    ///   architectural finding: `walk`'s own `import_types` branch has an
    ///   UNCONDITIONAL trailing `walk_children(..); return;` after its
    ///   own `on_unmatched_node` call, regardless of that handler's own
    ///   return value, so `walk`'s `call_types` check is NEVER reached
    ///   for a node kind in BOTH arrays (confirmed by a real, initially-
    ///   failing test -- the IMPORT half worked, the CALL half never
    ///   fired). `resource_declaration`'s own identical-looking dual
    ///   `class_types`+`call_types` membership does NOT share this
    ///   problem (`walk`'s `class_types` branch has no equivalent
    ///   unconditional trailing return, so it genuinely falls through to
    ///   `call_types` when unclaimed) -- this row's own initial
    ///   implementation assumed both dual-membership node kinds would
    ///   behave symmetrically and was wrong. Both the IMPORT and the
    ///   CALL are instead pushed together directly inside
    ///   [`crate::languages::generic::puppet_quirk`]'s own
    ///   `"include_statement"` arm (the only place actually reached for
    ///   this node kind) -- see that arm's own doc comment for why the
    ///   resulting CallRef's `from_symbol`/`from_symbol_line` are left
    ///   `None` (a genuine, documented seam limitation, not a further
    ///   bug).
    pub const fn puppet() -> Self {
        Self {
            name: "puppet",
            func_types: &["function_declaration"],
            method_types: &[],
            // See this const's own doc comment: `"defined_resource_type"`
            // ADDED beyond the baseline's own four -- a real, confirmed
            // gap, not a deliberate scope limitation.
            class_types: &[
                "class_definition",
                "node_definition",
                "resource_declaration",
                "type_declaration",
                "defined_resource_type",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["source_file"]` -- the same
            // root-node-as-module-clause bug [`Self::fortran`]'s/
            // [`Self::elm`]'s/[`Self::vimscript`]'s own doc comments
            // already document in full: `source_file` is this grammar's
            // own ROOT node, so the generic engine's own `module_types`
            // branch would grab whatever def/statement happens to come
            // FIRST as a garbage "Module" symbol name -- Puppet's own
            // module/namespace-like concepts (`class`/`node`) are already
            // fully captured via `class_types` above; there is no
            // SEPARATE, distinct "module clause" construct to point this
            // at instead.
            module_types: &[],
            // Deliberately NOT `"include_statement"` -- see this const's
            // own doc comment for the full "both import_types and
            // call_types membership never reaches call_types at all"
            // finding.
            call_types: &["function_call", "resource_declaration"],
            // Never actually consulted -- see this const's own doc
            // comment.
            call_function_field: "UNUSED_SEE_PUPPET_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_NO_ARGUMENTS_FIELD",
            import_types: &["include_statement"],
            branch_types: &["if_statement", "unless_statement", "case_statement"],
            decorator_types: &[],
            // Never actually consulted -- see this const's own doc
            // comment: every one of `func_types`/`class_types` is fully
            // claimed by `generic::puppet_quirk`.
            name_field: "UNUSED_SEE_PUPPET_QUIRK",
            body_field: "UNUSED_SEE_PUPPET_QUIRK",
        }
    }

    /// Elm (`.elm`) -- language-parity wave G2.3a. Node kinds verified
    /// directly against the `tree-sitter-elm` 5.9.0 crate's own
    /// `src/node-types.json` PLUS a real parse-tree dump. No corrections
    /// against the baseline's own `elm_*` arrays
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:766-772) -- every
    /// one of `elm_func_types`/`elm_class_types`/`elm_call_types`/
    /// `elm_import_types`/`elm_branch_types` names a real, confirmed node
    /// kind used exactly as the baseline itself describes:
    /// - `value_declaration` (this row's own `func_types`) genuinely has
    ///   NO `"name"` field of its own (confirmed) -- the real name is the
    ///   nested `functionDeclarationLeft` field's `function_declaration_left`
    ///   child's own FIRST child, a bare `lower_case_identifier` (confirmed
    ///   real, both by `node-types.json`'s `"children"` list and the real
    ///   dump), matching the baseline's own `resolve_elm_func_name`
    ///   (`internal/cbm/extract_defs.c`:957-971) exactly. Unlike most of
    ///   this wave's other languages, `value_declaration` DOES have a real
    ///   `"body"` FIELD (confirmed) -- [`crate::languages::generic::elm_quirk`]
    ///   reads that real field directly (a genuine simplification over
    ///   finding it by kind), rather than needing a kind-search fallback.
    ///   [`Self::name_field`]/[`Self::body_field`] below remain
    ///   placeholders purely because `value_declaration` still needs a
    ///   full quirk claim for the NAME half (no working name field), even
    ///   though the body half could technically use the generic engine's
    ///   own `body_field` mechanism if reached -- it never is, since name
    ///   resolution fails first in the generic engine's own func/method
    ///   branch, so the quirk claims the whole node, not half of it.
    /// - `function_call_expr`'s own `target`/`arg` fields are both real
    ///   (confirmed) -- its `target` field is a `value_expr` wrapping a
    ///   `name` field pointing at a `value_qid`, whose OWN last
    ///   `lower_case_identifier` child (confirmed via the real dump's
    ///   module-qualified `List.length` case: `value_qid` there has
    ///   `upper_case_identifier`+`dot`+`lower_case_identifier` children,
    ///   not merely a bare identifier) is the real callee name -- a
    ///   module-qualified call's own `Module.` prefix is dropped, matching
    ///   the baseline's own `extract_elm_callee`
    ///   (`internal/cbm/extract_calls.c`:704-730) exactly (same
    ///   qualifier-dropping depth as [`Self::erlang`]'s own `io:format`
    ///   handling).
    /// - `import_clause`'s own `moduleName` field is real (confirmed,
    ///   points at an `upper_case_qid`) -- reached directly rather than
    ///   the baseline's own more roundabout
    ///   `find_first_descendant_of(node, "upper_case_qid", ..)`
    ///   (`internal/cbm/extract_imports.c`:2481), since the real field
    ///   already IS that exact node, confirmed by the real dump.
    pub const fn elm() -> Self {
        Self {
            name: "elm",
            func_types: &["value_declaration"],
            method_types: &[],
            class_types: &[
                "type_declaration",
                "type_alias_declaration",
                "module_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Intentionally empty, NOT `["file"]` -- a real, confirmed bug
            // this row originally had (found only because
            // `tests/unit_languages_elm.rs`'s own assertions on a Module/
            // Class symbol caught it, not by inspection alone): the
            // generic engine's own `module_types` branch
            // (`first_named_child_text`) grabs the FIRST NAMED CHILD's
            // entire text as the "module name", correct only for a real,
            // repeatable, name-only module/namespace CLAUSE (Go's
            // `package_clause`, C#'s `namespace_declaration`) --
            // `file` is this grammar's own ROOT node, whose first named
            // child is `module_declaration` (a real, structured, multi-
            // field node, confirmed via `node-types.json`), so this
            // produced a garbage "Module" symbol whose name was the
            // ENTIRE `"module Widget exposing (area)"` line, not
            // `"Widget"`. `module_declaration` is already listed in this
            // row's own `class_types` above instead (a real node with its
            // own genuine `name` field pointing at an `upper_case_qid`) --
            // NOT yet actually wired to emit a symbol for it either
            // (`class_types`' own generic dispatch also needs a working
            // `name_field`, and this row's own [`Self::name_field`] is a
            // placeholder for `value_declaration`'s sake -- `elm_quirk`
            // does not claim `module_declaration`, so it currently falls
            // through with no symbol emitted, just still recursed into) --
            // left as a genuine, documented gap rather than a plausible-
            // looking but wrong one; this wave's own Tier-2 scope
            // (defs+calls+imports) does not require it.
            module_types: &[],
            call_types: &["function_call_expr"],
            // Never actually consulted -- see this const's own doc
            // comment: the real callee requires a multi-level unwrap
            // (`target` -> `value_expr` -> `name` -> `value_qid`'s own
            // last `lower_case_identifier`), claimed by
            // `generic::elm_call_override` instead.
            call_function_field: "UNUSED_SEE_ELM_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_NO_ARGUMENTS_FIELD",
            import_types: &["import_clause"],
            branch_types: &["case_of_expr", "if_else_expr"],
            decorator_types: &[],
            // A REAL field, unlike every other placeholder `name_field` in
            // this wave's own batch -- confirmed via `node-types.json`:
            // `type_declaration`/`type_alias_declaration`/
            // `module_declaration` (this row's own `class_types` entries)
            // ALL genuinely have their own `"name"` field, so this row's
            // `class_types` dispatch correctly extracts a real Class
            // symbol for each via the generic engine's own ordinary
            // `child_text(node, spec.name_field, ..)` fallback path with
            // NO quirk needed. Does not affect `value_declaration`'s own
            // handling: that node kind has no `"name"` field at all
            // (confirmed), so `walk`'s func/method branch's own identical
            // `child_text` lookup fails regardless of what this constant
            // is set to, falling through to `elm_quirk` exactly as
            // intended either way -- a genuine, confirmed-safe
            // simplification found only after `tests/unit_languages_elm.rs`
            // caught `type_declaration`/`module_declaration` silently
            // emitting NO symbol at all with the original placeholder
            // value (this row originally copied the "fully quirk-claimed"
            // placeholder convention from languages like R/Puppet without
            // checking whether it was actually needed for every one of
            // THIS row's own def-shaped node kinds, not just
            // `value_declaration`).
            name_field: "name",
            body_field: "UNUSED_SEE_ELM_QUIRK",
        }
    }

    /// Bicep (Azure IaC, `.bicep`). Language-parity wave G2.3c. Node
    /// kinds copied from the baseline's `bicep_func_types`/
    /// `bicep_class_types`/`bicep_import_types`/`bicep_var_types`/
    /// `bicep_call_types`/`bicep_module_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1361-1369`),
    /// cross-checked node-kind-by-node-kind against the actual
    /// `node-types.json` shipped in `tree-sitter-bicep` 1.1.0 (crates.io,
    /// `tree-sitter-grammars/tree-sitter-bicep`) plus a real parse-tree
    /// dump (`cargo run` against a scratch crate depending on
    /// `tree-sitter-bicep` directly), which found real corrections:
    /// - `module_types` is `["infrastructure"]`, NOT baseline's
    ///   `["program"]`: this grammar's actual root node kind is
    ///   `infrastructure` -- `"program"` does not appear anywhere in this
    ///   grammar's own `node-types.json` at all.
    /// - `branch_types` is EMPTY, matching the baseline's OWN choice
    ///   (`bicep_branch_types` in `lang_specs.c` is itself `empty_types`)
    ///   -- Bicep is a purely declarative IaC language with no
    ///   branching/control-flow syntax at all (confirmed: no
    ///   `if`/`for`-shaped statement node exists in this grammar either,
    ///   only a `for` EXPRESSION used as a resource-loop VALUE, which is
    ///   not a decision point in the cyclomatic-complexity sense).
    /// - `resource_declaration`/`module_declaration`/`type_declaration`/
    ///   `variable_declaration`/`output_declaration`/
    ///   `parameter_declaration` all have `"fields": {}` in this
    ///   grammar's own `node-types.json` -- NONE of the six exposes a
    ///   `name` field the generic engine's own `name_field`-keyed
    ///   fallback could read; every one of their own NAMES is instead a
    ///   positional (unfielded) `identifier` child immediately following
    ///   the declaration's leading keyword token. Only
    ///   `user_defined_function` has a real `name` field (confirmed) --
    ///   the odd one out, so `func_types` alone still uses the ordinary
    ///   generic name-field path while `class_types`-shaped declarations
    ///   are fully claimed by [`crate::languages::generic::bicep_quirk`]
    ///   instead (same posture as [`Self::d`]'s all-unfielded-node-kinds
    ///   row, just split per-array rather than crate-wide).
    /// - `call_types`'s `call_expression` DOES have real, working
    ///   `function`/`arguments` fields (confirmed) -- no override
    ///   needed, the generic engine's own default reconstruction is
    ///   correct unchanged.
    /// - `import_types` keeps baseline's `import_statement`/
    ///   `using_statement` (both real, confirmed via `node-types.json`);
    ///   `module_declaration` is deliberately NOT also listed here
    ///   despite baseline including it (a Bicep `module` block both
    ///   DEFINES a deployment reference AND functions as an import-like
    ///   reference to another `.bicep` file's outputs) -- this row
    ///   instead classifies it purely as a [`SymbolKind::Class`]-shaped
    ///   definition via the quirk (matching how every other declaration
    ///   kind here is classified), rather than double-counting it as
    ///   both a def AND an import edge, which the baseline's own flat
    ///   array structure cannot avoid but this row can.
    pub const fn bicep() -> Self {
        Self {
            name: "bicep",
            func_types: &["user_defined_function"],
            method_types: &[],
            class_types: &[
                "resource_declaration",
                "type_declaration",
                "module_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["infrastructure"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_statement", "using_statement"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// BitBake (Yocto build recipes, `.bb`/`.bbappend`/`.bbclass`/`.inc`).
    /// Language-parity wave G2.3c. Node kinds copied from the baseline's
    /// `bitbake_func_types`/`bitbake_var_types`/`bitbake_call_types`/
    /// `bitbake_import_types`/`bitbake_module_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1437-1445`),
    /// cross-checked against the actual `node-types.json` (vendored --
    /// see this crate's own `Cargo.toml` for why: `tree-sitter-bitbake`
    /// 1.1.0, the crates.io latest, hard-pins `tree-sitter = "~0.20.10"`
    /// as a normal dependency) plus a real parse-tree dump, which found:
    /// - `module_types` is `["recipe"]`, NOT baseline's `["source_file"]`:
    ///   this grammar's actual root node kind is `recipe`.
    /// - `function_definition`/`anonymous_python_function` both have
    ///   `"fields": {}` -- NEITHER exposes a `name` field; a
    ///   `function_definition`'s own name (`do_compile`, ...) and an
    ///   `anonymous_python_function`'s own name (the task name following
    ///   the `python` keyword) are both positional `identifier` children,
    ///   and neither has a `body`-labelled field either (a
    ///   `function_definition`'s shell-script body is a flat run of
    ///   `shell_content`/other children straight after the `{`; an
    ///   `anonymous_python_function`'s embedded-Python body is a
    ///   positional `block` child) -- both fully claimed by
    ///   [`crate::languages::generic::bitbake_quirk`].
    /// - `import_types` ADDS two real, baseline-MISSING node kinds this
    ///   grammar actually generates for BitBake's own dedicated
    ///   directives -- `inherit_directive` (`inherit cmake`) and
    ///   `require_directive` (`require common.inc`) -- neither appears
    ///   anywhere in the baseline's own `bitbake_import_types` array at
    ///   all, confirmed via a real parse-tree dump showing both as
    ///   distinct, real, non-error node kinds with their own path child
    ///   (`inherit_path`/`include_path` respectively); this is a genuine
    ///   improvement over the baseline's own coverage, not merely a
    ///   correction, since real-world BitBake recipes use `inherit`
    ///   constantly (the Yocto class-mixin mechanism) and `require`
    ///   routinely. `include_directive`/`export_statement` (both real,
    ///   confirmed) are kept from baseline unchanged. Baseline's other
    ///   four entries (`import`/`import_from_statement`/`import_statement`/
    ///   `with_clause`, all bare-Python-import node-kind names that would
    ///   only ever appear NESTED inside an `anonymous_python_function`'s
    ///   embedded-Python body, e.g. a `python do_foo() { import os }`
    ///   task) are kept too: this grammar genuinely re-parses that body
    ///   region with an embedded Python-like sub-grammar (confirmed via
    ///   the same parse-tree dump: a bare `def foo():`/`bb.note(...)`
    ///   inside such a body parses as real `python_function_definition`/
    ///   `call` nodes with real fields), so these ARE reachable real node
    ///   kinds in principle even though this wave's own fixture does not
    ///   happen to exercise one.
    /// - `call_types`'s `call` node (the embedded-Python call-expression
    ///   shape, confirmed real with working `function`/`arguments`
    ///   fields) is kept from baseline unchanged -- no override needed.
    pub const fn bitbake() -> Self {
        Self {
            name: "bitbake",
            func_types: &["function_definition", "anonymous_python_function"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["recipe"],
            call_types: &["call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[
                "export_statement",
                "import",
                "import_from_statement",
                "import_statement",
                "include_directive",
                "inherit_directive",
                "require_directive",
                "with_clause",
            ],
            branch_types: &[],
            decorator_types: &[],
            // Never actually consulted for `func_types` (both node kinds
            // fully claimed by `bitbake_quirk` -- see this const's own
            // doc comment).
            name_field: "UNUSED_SEE_BITBAKE_QUIRK",
            body_field: "UNUSED_SEE_BITBAKE_QUIRK",
        }
    }

    /// Cairo (StarkNet smart contracts, `.cairo`). Language-parity wave
    /// G2.3c. Node kinds copied from the baseline's `cairo_func_types`/
    /// `cairo_class_types`/`cairo_call_types`/`cairo_import_types`/
    /// `cairo_branch_types`/`cairo_var_types`/`cairo_assign_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1465-1475`),
    /// cross-checked against the actual `node-types.json` shipped in
    /// `tree-sitter-grammars/tree-sitter-cairo` (vendored -- see this
    /// crate's own `Cargo.toml` for why: this repo's own `Cargo.toml`
    /// still hard-pins `tree-sitter = "~0.20.10"` as a normal dependency
    /// at HEAD, and crates.io only ever published the OLDER pre-Cairo1
    /// `0.0.1` release of the SAME repo, never the modern one) plus a
    /// real parse-tree dump, which found:
    /// - `module_types` is `["program"]`, matching baseline -- but the
    ///   PRACTICAL top-level container one level below `program` is
    ///   `cairo_1_file` (a `cairo_0_file`/`cairo_1_file` split baseline
    ///   never mentions at all, this grammar's own dual-dialect
    ///   Cairo0/Cairo1 support) -- not added as its own module row since
    ///   `program` already gives the generic walker a Module symbol at
    ///   the true root and still recurses into `cairo_1_file`'s children
    ///   generically regardless (an un-named-in-the-baseline wrapper
    ///   layer, not a missed construct).
    /// - `func_types` keeps baseline's `function_definition`/
    ///   `function_signature` (both real, confirmed) -- but NEITHER has a
    ///   `name` field on itself (`"fields"` only has `returns`) -- both
    ///   fully claimed by [`crate::languages::generic::cairo_quirk`]
    ///   (positional `identifier` child, same "unfielded name" shape as
    ///   [`Self::d`]/[`Self::odin`]).
    /// - `class_types` is `["struct_item", "enum_item", "trait_item",
    ///   "mod_item"]`, NOT baseline's fuller `{"struct_definition",
    ///   "enum_item", "trait_item", "impl_item", "struct_item",
    ///   "type_definition"}`: `struct_definition`/`type_definition` do
    ///   not exist anywhere in this grammar at all (the real struct-decl
    ///   kind is `struct_item` only, and there is no dedicated type-alias
    ///   node kind -- Cairo's `type X = Y;` parses as the CATCH-ALL
    ///   `type_item`, not present in the baseline's own array either, so
    ///   left off this row too rather than invented); `impl_item` is
    ///   dropped -- confirmed `"fields": {}`, no `name` field AND no
    ///   positional identifier child either (an `impl` block's own
    ///   heading is a bare `Trait for Type`/`Trait<T>` type-expression
    ///   sequence with no single canonical "name" at all, unlike Rust's
    ///   `impl_item`, which this crate's own `LangSpec::rust` never lists
    ///   as a class-shaped node for the identical reason) -- kept out
    ///   entirely rather than emitting a wrong/empty name. `struct_item`/
    ///   `enum_item`/`trait_item`/`mod_item` ALL have real, working
    ///   `name` fields (confirmed) -- the ordinary generic name-field path
    ///   handles every one unchanged, no quirk claim needed for these
    ///   four despite `func_types` needing one.
    /// - `call_types`'s `call_expression` needs
    ///   [`crate::languages::generic::cairo_call_override`]: this
    ///   grammar's `call_expression` has NO wrapping argument-list node at
    ///   all (confirmed via a real parse-tree dump of `helper(1, 2, 3)`:
    ///   the `(`/`number`/`,`/`number`/`,`/`number`/`)` tokens are ALL
    ///   direct, positional children of `call_expression` itself) -- this
    ///   crate's own `node-types.json` DOES list a same-named `"arguments"`
    ///   node kind elsewhere in the grammar's vocabulary (a different
    ///   call-shape entirely, not this one), which a first-pass
    ///   implementation mistakenly assumed `call_expression` nested a
    ///   child of -- caught by this row's own hard test
    ///   (`extracts_call_with_multiple_args_via_by_kind_arguments_lookup`),
    ///   which found `arg_texts` silently empty before the fix.
    ///   [`crate::languages::generic::cairo_call_arg_texts`] instead walks
    ///   `call_expression`'s own direct children, skipping the callee's
    ///   own `[field=function]` child (by node identity, not kind, since
    ///   an argument could itself happen to share the callee's node kind)
    ///   and punctuation. Baseline's bare `["call"]` entry is dropped
    ///   (does not exist in this grammar at all -- Cairo0-only legacy
    ///   register-call syntax this grammar's own dual-dialect design
    ///   keeps as a SEPARATE, distinct node kind never actually named
    ///   `"call"`).
    /// - `import_types`'s `use_declaration` ALSO needs
    ///   [`crate::languages::generic::cairo_quirk`]'s own claim despite
    ///   having a real, working `"argument"` field (confirmed): the
    ///   generic walker's own `import_types` branch has no default
    ///   fallback of its own at all (unlike its `call_types` branch's
    ///   single-field reconstruction default) -- it ONLY ever calls
    ///   `on_unmatched_node`, so every import-shaped language row needs a
    ///   quirk claim regardless of how well-fielded the node itself is
    ///   (caught by this row's own hard test
    ///   `extracts_use_declaration_as_import`, which found the imports
    ///   list silently empty before this arm was added).
    /// - `branch_types`/`var_types` keep baseline's `{if_expression,
    ///   match_expression, loop_expression}`/`let_declaration` unchanged
    ///   (all real,
    ///   confirmed with working fields) -- baseline's own
    ///   `cairo_var_types` additionally lists `const_item`, also real and
    ///   kept.
    pub const fn cairo() -> Self {
        Self {
            name: "cairo",
            func_types: &["function_definition", "function_signature"],
            method_types: &[],
            class_types: &["struct_item", "enum_item", "trait_item", "mod_item"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "UNUSED_SEE_CAIRO_CALL_OVERRIDE",
            import_types: &["use_declaration"],
            branch_types: &["if_expression", "match_expression", "loop_expression"],
            decorator_types: &["attribute_item"],
            // Never actually consulted for `func_types` (both node kinds
            // fully claimed by `cairo_quirk`); `class_types`' four node
            // kinds all use the real `"name"` field directly, so this
            // value IS live for those -- same "unused for some arrays,
            // live for others" split as [`Self::gleam`].
            name_field: "name",
            body_field: "body",
        }
    }

    /// CFScript (ColdFusion/CFML script dialect, `.cfc`). Language-parity
    /// wave G2.3c. Node kinds copied from the baseline's
    /// `cfscript_func_types`/`cfscript_field_types`/
    /// `cfscript_import_types` plus the shared `js_module_types`/
    /// `js_call_types`/`js_branch_types` arrays its own baseline row
    /// reuses verbatim (`codebase-memory-mcp/internal/cbm/lang_specs.c:
    /// 271-274, 2029-2034`, the row's own comment: "JS-like grammar ...
    /// Reuses the JS call/branch/var/module arrays"), cross-checked
    /// against the actual `node-types.json` shipped in the OFFICIAL
    /// `tree-sitter-cfml` crate (crates.io, `cfmleditor/tree-sitter-cfml`,
    /// which bundles CFML/CFScript/CFQuery as three SEPARATE grammars in
    /// one crate/repo, exposing `LANGUAGE_CFML`/`LANGUAGE_CFSCRIPT`/
    /// `LANGUAGE_CFQUERY` as three distinct `LanguageFn` consts -- this
    /// row binds `LANGUAGE_CFSCRIPT` only, the baseline's own
    /// `CBM_LANG_CFSCRIPT`; `CBM_LANG_CFML`'s separate tag-dialect row is
    /// still unclaimed) plus a real parse-tree dump, which found:
    /// - `field_types` is EMPTY, NOT baseline's `["property_declaration"]`:
    ///   `property_declaration` does not exist ANYWHERE in this grammar's
    ///   own `grammar.js`/`node-types.json` at all (confirmed via both) --
    ///   ColdFusion's script-mode `property name="x" type="string";`
    ///   declaration instead parses as an ordinary `tag_statement` (this
    ///   grammar's general "any bare `identifier(args);` inside a
    ///   component body that isn't a recognized statement" catch-all,
    ///   also used for CFML's other pseudo-tag-like script constructs)
    ///   whose own `tag` field is the identifier `property` and whose
    ///   `arguments` field holds each `name="x"`/`type="string"` as its
    ///   own `assignment_expression` -- fully claimed by
    ///   [`crate::languages::generic::cfscript_quirk`] instead (this
    ///   grammar has NO dedicated field-declaration node kind for
    ///   CFScript at all, so there is no flat array value that could
    ///   express it, same "quirk claims what the flat array cannot"
    ///   posture as every other unfielded/irregular case in this file).
    /// - `func_types` keeps baseline's `function_declaration`/
    ///   `function_expression`/`arrow_function`/`method_definition` --
    ///   all four real; `function_declaration`/`function_expression`/
    ///   `method_definition` have working `name`/`parameters`/`body`
    ///   fields (the ordinary generic path handles them unchanged);
    ///   `arrow_function` has NO `name` field (a lambda, expected) but
    ///   IS still listed here matching baseline, since the generic
    ///   engine's own func/method branch degrades gracefully (no name
    ///   found -> no symbol emitted, not a panic) for any node kind whose
    ///   `name_field` lookup comes up empty, exactly the same as every
    ///   other language row with an anonymous-function shape in its own
    ///   `func_types` (e.g. [`Self::gleam`]'s `anonymous_function`).
    /// - `import_types` keeps baseline's `import_statement` (real,
    ///   confirmed with a working `source` field) -- baseline's OTHER
    ///   entry, bare `"import"`, is dropped: confirmed via
    ///   `node-types.json` this is an UNNAMED keyword token only
    ///   (`{"type":"import","named":false}`, the `import.meta`/`new
    ///   .target` meta-property keyword, never a real import-statement
    ///   node a real `.kind()` match against an ordinary AST node could
    ///   ever ambiguously collide with in practice, but also never
    ///   itself the STATEMENT this array is meant to capture) -- keeping
    ///   it would add a dead array entry with no behavior either way,
    ///   same "prune the confirmed-dead entry" discipline as every other
    ///   row's own baseline corrections in this file. `import_statement`
    ///   ALSO still needs
    ///   [`crate::languages::generic::cfscript_quirk`]'s own claim
    ///   despite having a real, working `source` field: the generic
    ///   walker's own `import_types` branch has no default fallback of
    ///   its own at all (unlike its `call_types` branch's single-field
    ///   reconstruction default) -- it ONLY ever calls
    ///   `on_unmatched_node`, so every import-shaped language row in this
    ///   whole file needs a quirk claim regardless of how well-fielded
    ///   the node itself is (caught by this row's own hard test
    ///   `extracts_import_statement`, which found the imports list
    ///   silently empty before this arm was added -- the SAME class of
    ///   bug this wave's Bicep/BitBake/Cairo rows independently hit and
    ///   fixed too).
    /// - `call_types`/`branch_types`/`module_types` (`call_expression`+
    ///   `new_expression`/the full JS if/for/while/do/switch-case/&&/||
    ///   set/`program`) are all real, confirmed, kept from the shared
    ///   JS arrays unchanged, matching this baseline row's own explicit
    ///   "reuses the JS arrays" design.
    pub const fn cfscript() -> Self {
        Self {
            name: "cfscript",
            func_types: &[
                "function_declaration",
                "function_expression",
                "arrow_function",
                "method_definition",
            ],
            method_types: &["method_definition"],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["call_expression", "new_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_statement"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "for_in_statement",
                "while_statement",
                "do_statement",
                "switch_case",
                "&&",
                "||",
            ],
            decorator_types: &["decorator"],
            name_field: "name",
            body_field: "body",
        }
    }

    /// FunC (TON smart contracts, `.fc`/`.func`). Language-parity wave
    /// G2.3c. Node kinds copied from the baseline's `func_func_types`/
    /// `func_call_types`/`func_import_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1497-1500`),
    /// cross-checked against the actual `node-types.json` (vendored --
    /// see this crate's own `Cargo.toml` for why: the crates.io
    /// `tree-sitter-func` 1.0.0 hard-pins `tree-sitter = "~0.20.10"` as a
    /// normal dependency) plus a real parse-tree dump, which found:
    /// - `import_types` is EMPTY, NOT baseline's `["include_directive"]`:
    ///   `include_directive` does not exist anywhere in this grammar at
    ///   all -- confirmed via both `node-types.json` and a real parse
    ///   dump showing `#include "stdlib.fc";` parses as a bare `ERROR`
    ///   node (this grammar has NO preprocessor-directive support
    ///   whatsoever, not even a differently-named one) -- there is
    ///   consequently no real node kind this row could point at instead;
    ///   FunC's own `#include`/`#pragma` lines are simply unextractable
    ///   through this grammar, a genuine, confirmed grammar limitation
    ///   (not a spec-writing gap) rather than something a quirk could
    ///   work around (a quirk still needs SOME real, non-`ERROR` node to
    ///   hang off).
    /// - `call_types` keeps baseline's `method_call`/`function_application`
    ///   (both real) but BOTH need
    ///   [`crate::languages::generic::func_call_override`]: `method_call`'s
    ///   own argument-list field is correctly spelled `"arguments"`, but
    ///   `function_application`'s own is a CONFIRMED TYPO in this
    ///   grammar's own generated field vocabulary -- `"agruments"`
    ///   (missing the `n`), not `"arguments"` (verified twice: directly in
    ///   `node-types.json`'s own `"fields"` key name, and again in a real
    ///   parse-tree dump showing `[field=agruments]` verbatim on both
    ///   `function_definition`'s own parameter-list field AND
    ///   `function_application`'s own argument-list field -- the SAME
    ///   typo appears on both node kinds in this grammar, not an isolated
    ///   one-off) -- since [`crate::languages::spec::LangSpec`] has only
    ///   ONE flat `call_arguments_field` string shared by every entry in
    ///   `call_types`, it cannot itself hold two different field-name
    ///   spellings for two different node kinds at once, so both are
    ///   claimed by the override rather than splitting the array (which
    ///   would still leave one of the two unhandled by the generic path
    ///   alone). Both fields ALSO point at exactly ONE wrapper node whose
    ///   own shape depends on argument COUNT, not a flat argument-list --
    ///   confirmed via real parse-tree dumps of all three cases: zero
    ///   arguments is a bare `unit_literal` (`()`); exactly one is a
    ///   `parenthesized_expression`; two or more is a `tensor_expression`
    ///   -- see [`crate::languages::generic::func_call_arg_texts`]'s own
    ///   doc comment for the full finding (caught by this row's own hard
    ///   test `extracts_function_application_call_despite_agruments_typo`,
    ///   which found `arg_texts` come back as the WHOLE wrapper node's own
    ///   text, e.g. `["(1, 2)"]` instead of `["1", "2"]`, before the fix).
    /// - `func_types`/`module_types` keep baseline's `function_definition`/
    ///   `["source_file"]`... corrected to `["translation_unit"]`: this
    ///   grammar's real root node kind is `translation_unit`, NOT
    ///   `source_file` (confirmed via a real parse dump) --
    ///   `function_definition` itself is unchanged (real, confirmed) and
    ///   has a real, working `name` field (`function_name`-typed) despite
    ///   ALSO having the same `agruments` typo on its own parameter-list
    ///   field -- irrelevant to definition extraction (which only reads
    ///   `name_field`/`body_field`, both spelled correctly: `"name"`/
    ///   `"body"`), so the ordinary generic path handles function
    ///   definitions unchanged with no quirk needed there.
    /// - `branch_types` is deliberately EMPTY, matching the baseline's OWN
    ///   choice (`func_branch_types` in `lang_specs.c` is itself
    ///   `empty_types`) even though this grammar DOES have real
    ///   `if_statement`/`while_statement`/`repeat_statement` branch-shaped
    ///   nodes (confirmed) -- this wave keeps Tier-2 parity with the
    ///   baseline's own choice rather than adding complexity-extraction
    ///   depth the baseline itself never wires for FunC (G3's job, if
    ///   ever wanted, per this workpack's own "complexity extraction may
    ///   be deferred this wave" allowance).
    pub const fn func() -> Self {
        Self {
            name: "func",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["translation_unit"],
            call_types: &["method_call", "function_application"],
            call_function_field: "UNUSED_SEE_FUNC_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_FUNC_CALL_OVERRIDE",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Move (Aptos/Sui smart contracts, `.move`). Language-parity wave
    /// G2.3c. Node kinds copied from the baseline's `move_func_types`/
    /// `move_call_types`/`move_import_types`/`move_branch_types`/
    /// `move_var_types`/`move_module_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1476-1486`),
    /// cross-checked against the actual `node-types.json` (vendored --
    /// see this crate's own `Cargo.toml` for why: `tzakian/tree-sitter-move`
    /// has never published a crates.io release at all) plus a real
    /// parse-tree dump, which found REAL, CONFIRMED CORRECTIONS -- this
    /// grammar generation is considerably richer than the baseline's own
    /// comment above `move_func_types` claims:
    /// - `func_types` is `["function_definition"]`, NOT baseline's
    ///   `["function_item"]`: `function_item` does not exist anywhere in
    ///   this grammar at all -- the real function-definition node kind is
    ///   `function_definition`, with REAL, working `name`
    ///   (`function_identifier`-typed)/`parameters`/`body`/`return_type`
    ///   fields (confirmed) -- the ordinary generic path handles this
    ///   unchanged, no quirk needed for definitions at all.
    /// - `class_types` is `["struct_definition", "enum_definition"]`, a
    ///   DELIBERATE IMPROVEMENT over baseline's empty array: the
    ///   baseline's own source comment claims "this vendored move grammar
    ///   models only function_item + module as named defs; struct/enum
    ///   exist only as anonymous keyword tokens, never as parent nodes" --
    ///   this is CONFIRMED WRONG for this grammar generation: both
    ///   `struct_definition` (`struct Counter has key { value: u64 }`) and
    ///   `enum_definition` both have real, working `name`
    ///   (`struct_identifier`/`enum_identifier`-typed) fields (confirmed
    ///   via both `node-types.json` and a real parse-tree dump showing
    ///   `Counter` resolves cleanly off `struct_definition`'s own `name`
    ///   field) -- the ordinary generic path handles both unchanged too.
    ///   Real-world Move contracts define resource structs constantly
    ///   (the entire point of Move's resource-oriented type system), so
    ///   this closes a genuine, high-value coverage gap the baseline's own
    ///   stale comment left unaddressed rather than reproducing a
    ///   confirmed-incorrect limitation.
    /// - `call_types`'s `call_expression` needs
    ///   [`crate::languages::generic::move_call_override`]: this node's
    ///   own callee is a POSITIONAL (unfielded) `name_expression` child
    ///   (itself wrapping a `module_access` -> `[field=member] identifier`
    ///   chain for the actual dotted/qualified name text), not a field on
    ///   `call_expression` itself -- only its OWN argument-list child is
    ///   fielded (`"args"`, confirmed real). Baseline's `call_expression`
    ///   node-kind-name guess is correct; only the field-name assumption
    ///   the generic engine's default single-field reconstruction would
    ///   otherwise make (a `"function"`-named field) does not hold.
    /// - `import_types`'s `use_declaration` needs
    ///   [`crate::languages::generic::move_quirk`]: this node has
    ///   `"fields": {}` (confirmed) -- its own imported-module path is
    ///   one of FOUR possible nested child shapes (`use_module`/
    ///   `use_module_member`/`use_module_members`/`use_fun`), each with
    ///   its own further-nested `module_identity` -- too irregular for a
    ///   flat field-name assumption, same "vary too much across shapes,
    ///   quirk owns it" rationale as Go's own import-path handling.
    /// - `branch_types`/`var_types`/`module_types` keep baseline's
    ///   `{if_expression, while_expression, loop_expression}`/`{let,
    ///   const}`/`["source_file"]` UNCHANGED except `var_types`'s `let`
    ///   entry is corrected to `let_statement` (the real node kind;
    ///   bare `"let"` is only the keyword token) -- `const` similarly
    ///   does not appear as its own top-level definition node in this
    ///   grammar at MODULE level in the sampled fixture depth this wave's
    ///   Tier-2 scope covers, so it is dropped rather than guessed; all
    ///   three branch entries and `source_file` are real and confirmed
    ///   unchanged.
    pub const fn move_lang() -> Self {
        Self {
            name: "move",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &["struct_definition", "enum_definition"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["call_expression"],
            call_function_field: "UNUSED_SEE_MOVE_CALL_OVERRIDE",
            call_arguments_field: "args",
            import_types: &["use_declaration"],
            branch_types: &["if_expression", "while_expression", "loop_expression"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Nickel (config language, `.ncl`). Language-parity wave G2.3c.
    /// Node kinds copied from the baseline's `nickel_func_types`/
    /// `nickel_call_types`/`nickel_import_types`/`nickel_branch_types`/
    /// `nickel_var_types`/`nickel_module_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1185-1195`),
    /// cross-checked against the actual `node-types.json` shipped in
    /// `tree-sitter-nickel` 0.5.0 (crates.io, the OFFICIAL
    /// `nickel-lang/tree-sitter-nickel` grammar) plus a real parse-tree
    /// dump, which found this grammar generation has drifted
    /// substantially from what the baseline's own arrays describe:
    /// - `module_types` is EMPTY, NOT baseline's `["source_file"]` (nor
    ///   the seemingly-obvious `["term"]`): this grammar's actual root
    ///   node kind IS `term` (confirmed via both `node-types.json`'s own
    ///   `"root": true` marker and a real parse dump; `source_file` does
    ///   not exist anywhere in this grammar at all), BUT `term` is also
    ///   this grammar's own wrapper for EVERY expression position, not
    ///   merely the file root -- pointing `module_types` at it would tag
    ///   every single nested expression as its own
    ///   [`crate::parsers::SymbolKind::Module`] symbol (a real bug this
    ///   row's own first-pass implementation had, caught by this row's
    ///   own hard test `plain_value_binding_produces_no_symbol`) -- there
    ///   is no analogous single, non-recursive "this is the file/module"
    ///   node kind in this grammar at all.
    /// - `func_types` keeps baseline's `["fun_expr"]` (real, confirmed) --
    ///   but `fun_expr` genuinely has NO name field of its own at all
    ///   (`"fields": {}`), matching the baseline's OWN source comment
    ///   above this array verbatim ("its name lives on the enclosing
    ///   let_binding's pat field") -- fully claimed by
    ///   [`crate::languages::generic::nickel_quirk`], which additionally
    ///   claims `let_binding`/`field_decl` themselves (a Nickel
    ///   NAMED-function definition, `let f = fun x y => ...` OR a record
    ///   field whose value is a `fun_expr`, is recognized by climbing
    ///   from the binding/field to its own `pat`/`path` name -- see that
    ///   function's own doc comment for the full three-shape walk).
    /// - `call_types` keeps baseline's `["applicative"]` (real, confirmed,
    ///   matching the baseline's own source comment: "A function
    ///   application (f x y) is an applicative node") -- but needs
    ///   [`crate::languages::generic::nickel_call_override`]: this
    ///   grammar's CURRYING is structurally LEFT-RECURSIVE (`f a b c`
    ///   parses as `applicative(t1=applicative(t1=applicative(t1=f,
    ///   t2=a), t2=b), t2=c)`, confirmed via a real parse-tree dump of a
    ///   deliberate 3-argument call) -- the true callee is only reachable
    ///   by walking DOWN the `t1` chain until it stops being an
    ///   `applicative` itself, which the generic engine's own
    ///   single-field default reconstruction cannot express (there is no
    ///   flat "the callee is always in field X" answer -- it depends on
    ///   how deep the curry chain is).
    /// - `import_types` is `["import"]` ONLY, NOT baseline's `["import",
    ///   "include"]`: `import` is real -- confirmed as a genuine,
    ///   DEDICATED unnamed keyword-token node kind
    ///   (`{"type":"import","named":false}`) that appears as an
    ///   `applicative`'s own POSITIONAL (unfielded) child specifically for
    ///   the `import "path.ncl"` syntax (confirmed via a real parse-tree
    ///   dump: `applicative` -> bare `import` token + `[field=s]
    ///   static_string`, a DIFFERENT shape from the ordinary curried-call
    ///   `t1`/`t2` fields `nickel_call_override` otherwise handles) --
    ///   claimed by [`crate::languages::generic::nickel_quirk`]'s own
    ///   `applicative` arm. Baseline's `"include"` is DROPPED: confirmed
    ///   via TWO separate real parse-tree dumps (`include "foo.txt"` used
    ///   both as a bare expression and inside a `let`) that `include` is
    ///   ALWAYS a plain generic `ident` in this grammar version, matching
    ///   the module's OWN `include` token-kind vocabulary entry (which
    ///   does exist, `{"type":"include","named":false}`) NEVER actually
    ///   being produced by any grammar rule that reaches it -- there is
    ///   consequently no structural way to distinguish a real `include`
    ///   "import" from an ordinary function call to a variable that
    ///   happens to be named `include`, so keeping it would only ever
    ///   match user code that shadows the name, never Nickel's own import
    ///   mechanism (which does not appear to use this keyword in this
    ///   grammar version's practice despite the vocabulary entry
    ///   existing).
    /// - `branch_types` is `["ite_expr"]`, NOT baseline's `["if", "match"]`:
    ///   NEITHER `if` nor `match` exist as real node kinds in this
    ///   grammar at all -- confirmed via both `node-types.json` and a real
    ///   parse dump that `if ... then ... else ...` parses as `ite_expr`
    ///   and Nickel's pattern-match construct parses as `match_expr`
    ///   (dropped here rather than added: this wave's Tier-2 scope keeps
    ///   complexity extraction minimal per this workpack's own "may be
    ///   deferred" allowance, and `match_expr`'s own `cases` field shape
    ///   would need its own quirk to walk correctly rather than being a
    ///   flat decision-point match the way `ite_expr` already is).
    /// - `var_types` keeps baseline's `["let"]` -- real, confirmed
    ///   (`let_expr`'s own nested `let_in_block` -- the bare `let`
    ///   keyword token itself, matching this row's own module-level
    ///   local-binding intent).
    pub const fn nickel() -> Self {
        Self {
            name: "nickel",
            func_types: &["fun_expr"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // EMPTY, NOT `["term"]`: `term` is this grammar's own
            // wrapper for EVERY expression position, not merely the
            // file root (confirmed via every real parse-tree dump this
            // row's own doc comment already includes -- `term` nests
            // recursively at every sub-expression, not just once at the
            // top) -- pointing `module_types` at it would tag every
            // single nested expression as its own [`SymbolKind::Module`]
            // symbol, caught by this row's own hard test
            // `plain_value_binding_produces_no_symbol`, which found
            // spurious Module symbols for `isProd`/`true`/the whole file
            // alike before this fix. There is no analogous single,
            // non-recursive "this is the file/module" node kind in this
            // grammar at all (the true root is simply `term` -- see this
            // row's own doc comment for the module_types finding).
            module_types: &[],
            call_types: &["applicative"],
            call_function_field: "UNUSED_SEE_NICKEL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_NICKEL_CALL_OVERRIDE",
            import_types: &["import"],
            branch_types: &["ite_expr"],
            decorator_types: &[],
            // Never actually consulted -- `fun_expr` is fully claimed by
            // `nickel_quirk` (see this const's own doc comment).
            name_field: "UNUSED_SEE_NICKEL_QUIRK",
            body_field: "UNUSED_SEE_NICKEL_QUIRK",
        }
    }

    /// Jsonnet (JSON templating, `.jsonnet`/`.libsonnet`). Language-parity
    /// wave G2.3c. Node kinds copied from the baseline's
    /// `jsonnet_func_types`/`jsonnet_call_types`/`jsonnet_import_types`/
    /// `jsonnet_branch_types`/`jsonnet_var_types`/`jsonnet_module_types`
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c:1323-1328`),
    /// cross-checked against the actual `node-types.json` shipped in
    /// `tree-sitter-jsonnet` 0.0.1 (crates.io, published 2026-05-05 --
    /// the newest grammar dependency in this whole crate) plus a real
    /// parse-tree dump across THREE distinct function-definition shapes,
    /// which found this grammar generation has drifted completely from
    /// every node-kind name the baseline's own arrays list:
    /// - `module_types` is `["source_file"]`, NOT baseline's
    ///   `["document"]`: this grammar's actual root node kind is
    ///   `source_file` -- `document` does not exist anywhere in this
    ///   grammar at all.
    /// - `func_types`/`class_types` are BOTH empty (no flat array can
    ///   express this grammar's shape at all): baseline's
    ///   `["anonymous_function", "bind"]` are WRONG -- `anonymous_function`
    ///   does not exist anywhere in this grammar; `bind` is real but is
    ///   NOT itself a function-shaped node -- it is the generic
    ///   local-BINDING node (`local x = ...;`), used for BOTH a plain
    ///   local variable AND a local function (there is no separate
    ///   `function_expr`-wrapped shape at all in practice: confirmed via
    ///   THREE separate real parse-tree dumps -- `local f = function(x)
    ///   x + 1;`, an object method-sugar `fn(x): x + 1`, and an explicit
    ///   `fn: function(x) x + 1` field value ALL flatten the `function`
    ///   keyword token + `params` + body directly as POSITIONAL siblings
    ///   of `bind`/`field` rather than ever wrapping them in the
    ///   `function_expr` node `node-types.json`'s own "possible children"
    ///   list claims exists as a legal child shape). Recognizing "is this
    ///   bind/field actually a function" consequently requires scanning
    ///   for a `function`-keyword-or-`params`-child sibling, which no flat
    ///   array/single-field-name pair could express -- fully claimed by
    ///   [`crate::languages::generic::jsonnet_quirk`] instead (its own
    ///   `bind`/`field` arms), matching the baseline's own comment for
    ///   this exact language ("the lambda node is fun_expr ... its name
    ///   lives on the enclosing let_binding's pat field" -- Nickel's row
    ///   above, but the identical shape recurs here for a DIFFERENT
    ///   underlying reason: no wrapping node exists at all, not merely no
    ///   name field on one that does).
    /// - `call_types` is `["suffix_apply"]`, NOT baseline's
    ///   `["functioncall"]`: `functioncall` does not exist anywhere in
    ///   this grammar -- a call is instead `<callee-expr> suffix_apply`,
    ///   where `suffix_apply` (the parenthesized argument list, confirmed
    ///   real with a repeated, unfielded `arg` child list) is a SIBLING
    ///   of its own callee, not a wrapping node with a callee field at
    ///   all -- needs
    ///   [`crate::languages::generic::jsonnet_call_override`], which reads
    ///   the callee from `suffix_apply`'s own PRECEDING SIBLING (the
    ///   generic engine's `walk` never visits a `suffix_apply` node with
    ///   direct access to its sibling any other way).
    /// - `import_types` is `["import_expr"]`, NOT baseline's `["import",
    ///   "importstr"]`: NEITHER bare name exists as its own node kind --
    ///   `import "other.libsonnet"` parses as a real, dedicated
    ///   `import_expr` wrapper node (confirmed with a working nested
    ///   `string`/`string_block`/`verbatim_string` child, found by kind
    ///   since `import_expr` itself has `"fields": {}`) -- claimed by
    ///   [`crate::languages::generic::jsonnet_quirk`]'s own arm.
    ///   `importstr` (Jsonnet's raw-string-import variant) is dropped:
    ///   this grammar has no separate node kind for it at all in this
    ///   version (it appears to parse identically through `import_expr`
    ///   with a plain string child, not a distinguishable second form).
    /// - `branch_types` is `["if_then_else"]`, NOT baseline's
    ///   `["conditional"]`: `conditional` does not exist anywhere in this
    ///   grammar -- the real node kind is `if_then_else` (confirmed real,
    ///   found generically as a decision point by node KIND alone the
    ///   same way every other language's `branch_types` entry already
    ///   works -- this row does not need the CONDITION's own value for
    ///   anything, only the node's presence).
    /// - `var_types` is `["bind"]`, NOT baseline's `["local_bind"]`:
    ///   `local_bind` does not exist -- `bind` (the same node `func_types`
    ///   discusses above) is Jsonnet's one local-binding node for both
    ///   plain values and functions alike.
    pub const fn jsonnet() -> Self {
        Self {
            name: "jsonnet",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["suffix_apply"],
            call_function_field: "UNUSED_SEE_JSONNET_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_JSONNET_CALL_OVERRIDE",
            import_types: &["import_expr"],
            branch_types: &["if_then_else"],
            decorator_types: &[],
            // Never actually consulted -- every function/binding shape is
            // fully claimed by `jsonnet_quirk` (see this const's own doc
            // comment).
            name_field: "UNUSED_SEE_JSONNET_QUIRK",
            body_field: "UNUSED_SEE_JSONNET_QUIRK",
        }
    }
    // =====================================================================
    // COMMON LISP (language-parity wave G2.3b)
    // =====================================================================

    /// Common Lisp: grammar is `tree-sitter-commonlisp` (crates.io,
    /// `LANGUAGE_COMMONLISP` const via `tree-sitter-language`, verified
    /// end-to-end with a real parse-tree dump before being added). Every
    /// form is a `list_lit` (same Lisp-family uniformity
    /// [`Self::clojure`]'s own doc comment already found) -- `defun`
    /// itself IS a real, distinct node kind here (unlike Clojure, whose
    /// grammar has no `defun`-shaped node at all), wrapping a
    /// `defun_header` child that carries the real `function_name`/
    /// `lambda_list` fields, confirmed via a real parse-tree dump
    /// (`(defun helper (x) (+ x 1))` -> `list_lit > defun > defun_header
    /// [function_name] sym_lit "helper"`). Baseline's own
    /// `commonlisp_func_types = ["defun"]` and `commonlisp_call_types =
    /// ["list_lit"]` both confirmed correct against this grammar version
    /// -- this is the rare G2 language needing NO baseline corrections at
    /// all (matches [`Self::clojure`]'s own "Lisp-family grammars are
    /// unusually uniform" finding).
    ///
    /// `func_types`/`call_types` both fully claimed by
    /// [`generic::commonlisp_quirk`]/[`generic::commonlisp_call_override`]
    /// rather than this file's own `name_field`/`call_function_field`
    /// mechanism: `defun`'s own name lives two levels down
    /// (`defun_header`'s `function_name` field, not a field of `defun`
    /// itself), and `list_lit`'s callee is its own first named child (no
    /// field at all) -- the SAME two-hook shape [`Self::clojure`] already
    /// uses for the identical reason, ported rather than reinvented.
    /// UNLIKE Clojure, though (a real grammar-shape difference this row's
    /// own first draft wrongly assumed away, caught by its own test suite
    /// failing during this wave's verification, not by inspection): a
    /// `defun`-headed `list_lit`'s first named child is the `defun` NODE
    /// itself (a dedicated node type, not a bare `sym_lit`/`identifier`
    /// symbol the way Clojure's uniform `list_lit(sym_lit "defn", ...)`
    /// shape always is), so [`generic::commonlisp_call_override`]'s own
    /// `sym_lit`/`identifier`-only head check correctly never records
    /// `"defun"` itself as a callee -- ONLY the body expression's own
    /// calls (reached via [`generic::commonlisp_quirk`]'s own
    /// `defun`-handling arm, which must call [`generic::walk`] on the
    /// resolved `[value]` body node directly rather than
    /// [`generic::walk_children`] on it, since the body is itself often a
    /// call-shaped `list_lit` needing its OWN top-level visit, not merely
    /// its children's -- a second real bug this wave's own test suite
    /// caught) are ever recorded.
    ///
    /// `import_types` deliberately empty, NOT baseline's own
    /// `["with_clause"]`: a real parse-tree dump confirms `with_clause`
    /// does not exist ANYWHERE in this grammar's `node-types.json` at all
    /// (Common Lisp's `WITH-CLAUSE`... printer-control macro is not a
    /// tree-sitter-grammar-visible node kind here, unlike e.g. VHDL's
    /// unrelated same-named `with_clause`) -- porting it verbatim would be
    /// dead data, not merely unused (same category of finding
    /// [`Self::lua`]'s own doc comment already documents for a different
    /// baseline array entry). IMPORTS are instead recognized via
    /// [`generic::commonlisp_quirk`]'s dedicated `in-package`/`require`/
    /// `use-package` head-symbol match, a real (if baseline-absent)
    /// dependency-declaration idiom this grammar's `list_lit` shape makes
    /// directly recognizable the same way [`Self::clojure`]'s own
    /// `ns`/`:require` quirk already is.
    pub const fn commonlisp() -> Self {
        Self {
            name: "commonlisp",
            // See this const's own doc comment: fully claimed by
            // `generic::commonlisp_quirk`, which finds `defun`'s real
            // name via its `defun_header` child's `function_name` field.
            func_types: &["defun"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source"],
            // See this const's own doc comment: fully claimed by
            // `generic::commonlisp_call_override`, which records a
            // `list_lit`'s head symbol as a callee ONLY when that head is
            // itself a bare `sym_lit`/`identifier` -- a `defun`-headed
            // `list_lit`'s own first named child is the `defun` NODE
            // (never a bare symbol), so `"defun"` itself is correctly
            // NEVER recorded as a callee, unlike Clojure's own uniform
            // `list_lit(sym_lit "defn", ...)` shape.
            call_types: &["list_lit"],
            call_function_field: "UNUSED_SEE_COMMONLISP_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_COMMONLISP_CALL_OVERRIDE",
            // See this const's own doc comment: intentionally empty --
            // `in-package`/`require`/`use-package` IMPORTS are claimed by
            // `generic::commonlisp_quirk` instead, not this file's flat
            // array mechanism.
            import_types: &[],
            // Matches the baseline's own row exactly -- genuinely empty
            // there too (Common Lisp has no baseline branch/decision-node
            // array at all), not merely omitted.
            branch_types: &[],
            decorator_types: &[],
            // Never actually consulted -- `defun` is fully claimed by
            // `generic::commonlisp_quirk` before this file's own
            // `name_field`-keyed fallback would ever run for it.
            name_field: "UNUSED_SEE_COMMONLISP_QUIRK",
            body_field: "UNUSED_SEE_COMMONLISP_QUIRK",
        }
    }

    // =====================================================================
    // LEAN (language-parity wave G2.3b)
    // =====================================================================

    /// Lean: grammar is `tree-sitter-lean4` (crates.io, `language()` fn
    /// returning `tree_sitter::Language` directly -- NOT the
    /// `tree-sitter-language`/`LanguageFn` shim every other grammar here
    /// uses, but its own `Cargo.toml` declares `tree-sitter = "0.25"` as a
    /// real NORMAL dependency, the exact same version this workspace's
    /// own core pins, so there is no ABI/dependency-graph risk despite the
    /// different binding shape -- verified end-to-end with real
    /// parse-tree dumps, cross-checked against this crate's own
    /// `node-types.json` directly (not dump-only), before being added).
    ///
    /// BASELINE IS SUBSTANTIALLY WRONG for this language: baseline's own
    /// `lean_func_types = ["def", "theorem", "instance", "abbrev"]` names
    /// FOUR node kinds that do not exist AT ALL in this grammar version --
    /// `def helper ... := x + 1`, `theorem area_thm ... := rfl`, and
    /// `instance : Inhabited Point := ...` all parse to the exact SAME
    /// single node kind, `definition`, confirmed via both a real
    /// parse-tree dump AND this crate's own `node-types.json` (`kind`
    /// field, `required: false`, whose only possible VALUES are
    /// `abbrev`/`def`/`lemma`/`theorem` -- note `instance` is NOT among
    /// them: an `instance`-form `definition` genuinely has NO `kind`
    /// field value at all, confirmed by both sources agreeing, an
    /// `instance` keyword token appears as a plain unfielded child
    /// instead). `structure`/`inductive`, in CONTRAST, are each their OWN
    /// separate, real top-level node kind (NOT `[kind]`-field values on
    /// `definition` -- an assumption this row's own first draft got wrong
    /// from a too-quick dump read and corrected against
    /// `node-types.json`'s own top-level `_declaration` supertype list,
    /// which enumerates `definition`/`inductive`/`opaque`/`structure` as
    /// FOUR SIBLING kinds), each with its own real, direct `name` field
    /// (`identifier`) -- baseline's own `resolve_lean_func_name`
    /// (`declId`-field-based, `extract_defs.c:341-357`) is ALSO stale
    /// against this grammar version: none of `definition`/`structure`/
    /// `inductive` has any `declId` wrapper at all, each carries its own
    /// real, direct `name` field instead, confirmed via `node-types.json`.
    /// `structure`/`inductive` both resolve correctly through this file's
    /// ordinary generic class-branch path with no quirk needed (real
    /// `name` field, no ambiguity, and neither has an executable body of
    /// its own -- a field list / constructor list respectively, never
    /// call-shaped). `definition` is DIFFERENT despite its own name field
    /// working identically: this row's own first draft assumed
    /// `definition` also needed no quirk, but the generic engine's own
    /// SHARED func/method branch recurses into a resolved body via
    /// `walk_children(body, ..)` -- which visits `body`'s own CHILDREN,
    /// never `body` itself. For every OTHER language in this crate,
    /// `body_field` always resolves to a `{ }`-style block wrapper never
    /// itself call-shaped, so that distinction is invisible; Lean's own
    /// `[body]` field is frequently the bare CALL EXPRESSION directly
    /// (`def area (...) := helper shape`, `[body]` resolving straight to
    /// the `application`-kind node this row's own `call_types` needs to
    /// VISIT, not skip past to its children) -- a real, test-caught bug
    /// (`application_call_uses_the_real_name_arguments_fields` failing
    /// during this wave's own verification, not by inspection) fixed by
    /// giving [`generic::lean_quirk`] a FULL claim of `definition`
    /// instead, recursing via `walk` (not `walk_children`) on the
    /// resolved body -- scoped entirely to this one language's own quirk
    /// rather than changing the shared engine's own convention every
    /// other onboarded language still depends on unchanged.
    ///
    /// `lean_call_types = ["apply", "command"]` is ALSO wrong: neither
    /// `apply` nor `command` exists anywhere in this grammar's
    /// `node-types.json` at all -- the real (and only) function-
    /// application node kind is `application`, with real `[name]`/
    /// `[arguments]` fields (`helper shape` -> `application [name]
    /// identifier "helper" [arguments] identifier "shape"`), confirmed via
    /// a real parse-tree dump AND `node-types.json`. No quirk/override
    /// needed for calls at all: `application`'s own fields already match
    /// this file's generic single-field reconstruction
    /// (`call_function_field: "name"`), though its argument shape
    /// (`arguments` field holding a single bare expression, not a
    /// parenthesized list) means [`Self::call_arguments_field`]'s generic
    /// `(`/`)`/`,`-child-skipping reconstruction only ever finds that one
    /// argument -- acceptable at this Tier-2 depth (no dedicated richer
    /// edge for this wave's languages).
    ///
    /// `lean_module_types = ["module"]` also needed correction: this
    /// grammar's real ROOT node kind is `module` (confirmed the top-level
    /// wrapper every fixture parses into), but `import Mathlib.Data...`
    /// itself parses as a plain `import` node (not nested `module`
    /// content needing a MODULE symbol of its own) -- `module_types` is
    /// left EMPTY here rather than porting baseline's array verbatim,
    /// since this file's own `module_types` handling would otherwise emit
    /// a spurious top-level MODULE symbol for every Lean file's own root
    /// wrapper node, which has no name field and is not a real
    /// user-facing module declaration the way Rust's `mod_item` or Go's
    /// `package_clause` are. IMPORTS are handled by
    /// [`generic::lean_quirk`] instead, reading `import`'s dotted-path
    /// children directly.
    ///
    /// `structure`'s own real `extends` field (confirmed via
    /// `node-types.json`, a genuine Lean structure-extension/composition
    /// idiom) is deliberately left unwired -- Tier-2 scope, no dedicated
    /// richer edge for this wave's languages (same restraint
    /// [`Self::systemverilog`]'s own `extends` finding notes).
    pub const fn lean() -> Self {
        Self {
            name: "lean",
            // See this const's own doc comment: baseline's four-entry
            // array does not exist in this grammar at all -- the one real
            // wrapper kind covering def/theorem/instance/abbrev is
            // `definition`. Deliberately EMPTY here (NOT `&["definition"]`,
            // this row's own first draft's real mistake): `definition`'s
            // own direct `name` field DOES work via this file's ordinary
            // `child_text` lookup, but `walk`'s own func/method branch
            // checks (and, on success, `return`s from) THAT branch BEFORE
            // ever consulting the bottom `on_unmatched_node` catch-all --
            // so listing `"definition"` here would let the ordinary path
            // claim it first, silently preventing `generic::lean_quirk`'s
            // own FULL claim (needed for the body-recursion fix, see that
            // quirk's own `"definition"` arm) from ever running at all.
            // `definition` is instead left OFF every flat array entirely
            // and claimed purely through the universal bottom catch-all,
            // the same "not listed in any array, fully quirk-claimed"
            // posture [`Self::c`]/[`Self::cpp`]/[`Self::squirrel`] already
            // use for their own full-claim node kinds.
            func_types: &[],
            method_types: &[],
            // `structure`/`inductive` are each their OWN real, separate
            // top-level node kind (NOT `definition` variants -- see this
            // const's own doc comment for the correction against a
            // too-quick first read), each with a real, direct `name`
            // field this file's ordinary generic class-branch path
            // already resolves correctly.
            class_types: &["structure", "inductive"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // See this const's own doc comment: intentionally empty, NOT
            // baseline's `["module"]` -- this grammar's real `module` kind
            // is the unnamed root wrapper, not a user-facing declaration.
            module_types: &[],
            // See this const's own doc comment: baseline's `"apply"`/
            // `"command"` do not exist in this grammar -- the real (and
            // only) call-shaped kind is `application`, with genuine
            // `name`/`arguments` fields needing no override at all.
            call_types: &["application"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            // See this const's own doc comment: IMPORTS claimed by
            // `generic::lean_quirk` instead of this flat array (a bare
            // `import` node's dotted-path children need dedicated
            // reconstruction, not a single field lookup).
            import_types: &["import"],
            // Confirmed real via a real parse-tree dump: Lean's
            // `if`/`match`/`do` all parse as named nodes matching
            // baseline's own array exactly (no corrections needed here,
            // unlike func/call/module above).
            branch_types: &["if", "match", "do"],
            decorator_types: &[],
            // `definition`/`structure`/`inductive` all carry a real,
            // direct `name` field (an `identifier`) -- NOT baseline's
            // `declId`-wrapper assumption, confirmed via both a real
            // parse-tree dump and this grammar's own `node-types.json`.
            // Live for the ordinary generic func/class path; no quirk
            // intervenes for name resolution at all for this language.
            name_field: "name",
            body_field: "body",
        }
    }

    // =====================================================================
    // TLA+ (language-parity wave G2.3b)
    // =====================================================================

    /// TLA+: grammar is `tree-sitter-tlaplus` (crates.io, `LANGUAGE` const
    /// via `tree-sitter-language`, verified end-to-end with a real
    /// parse-tree dump before being added).
    ///
    /// Baseline's `tlaplus_func_types = ["operator_definition",
    /// "function_definition"]` confirmed CORRECT via real parse-tree
    /// dumps of both forms: `Helper(x) == x + 1` is `operator_definition`
    /// (real `[name]`/`[definition]` fields), and the array-valued form
    /// `f[x \in Nat] == x + 1` is a genuinely distinct node kind,
    /// `function_definition` (real `[name]` field too, but its own body
    /// -- a `quantifier_bound` parameter list plus a `[definition]`
    /// field -- has a different shape than `operator_definition`'s plain
    /// paren-parameter list; both share the SAME `name`/`definition`
    /// field names though, so no per-kind quirk branching is needed for
    /// WHICH field to read). BOTH kinds, however, DO need a full
    /// [`generic::tlaplus_quirk`] claim for a different reason this row's
    /// own first draft missed: the generic engine's own SHARED func/
    /// method branch recurses into a resolved body via
    /// `walk_children(body, ..)`, which visits `body`'s own children,
    /// never `body` itself -- invisible for every other language (whose
    /// `body_field` always resolves to a block wrapper never itself
    /// call-shaped), but real here: `Area(shape) == Helper(shape)`'s own
    /// `[definition]` field resolves straight to the call-shaped
    /// `bound_op` node itself, which this row's own `call_types` needs to
    /// VISIT, not skip past to its children -- see
    /// [`generic::tlaplus_quirk`]'s own `"operator_definition" |
    /// "function_definition"` arm for the full, test-caught finding (the
    /// SAME bug class, and the SAME scoped-to-this-language fix, as
    /// [`Self::lean`]'s own `definition` finding).
    ///
    /// `tlaplus_call_types = ["function_evaluation", "call", "bound_op"]`
    /// needed one correction, NOT the two this row's own first draft
    /// wrongly concluded from a too-quick dump read (corrected against
    /// `node-types.json` directly, the same discipline
    /// [`Self::lean`]'s own doc comment's correction used): `bound_op`
    /// (`Helper(shape)`, a paren-argument operator reference -- real
    /// `[name]`/`[parameter]` fields, `name` resolving to `identifier_ref`)
    /// AND `function_evaluation` (`f[shape]`, TLA+'s array-application
    /// syntax -- real, `"fields": {}`, but genuinely field-less: its own
    /// children are simply the callee `identifier_ref` followed by each
    /// bracketed argument expression, confirmed via `node-types.json`
    /// AND a real parse-tree dump) are BOTH real, confirmed-reachable
    /// node kinds. Only baseline's bare `"call"` is genuinely unusable
    /// -- it DOES exist in `node-types.json`, but as an ANONYMOUS
    /// (`"named": false`) token, an unrelated keyword/punctuation literal
    /// this grammar happens to also spell `call`, not a named
    /// call-expression node the way baseline's own array intended --
    /// confirmed by the same direct `node-types.json` cross-check.
    /// Neither `bound_op` nor `function_evaluation` fits this file's
    /// generic single-field `call_function_field`/`call_arguments_field`
    /// mechanism (`bound_op`'s own `[parameter]` field is repeated, not
    /// one list-node; `function_evaluation` has no field at all), so
    /// [`generic::tlaplus_call_override`] reconstructs both directly --
    /// left as a placeholder, matching this file's established
    /// "UNUSED_SEE_..." convention for a fully quirk-claimed call shape
    /// (e.g. [`Self::erlang`]/[`Self::clojure`]).
    ///
    /// `tlaplus_import_types = ["extends", "instance"]` confirmed correct:
    /// both are real node kinds with an unfielded `identifier_ref` list of
    /// extended/instantiated module names (`EXTENDS Naturals, Sequences`
    /// -> `extends > identifier_ref "Naturals", identifier_ref
    /// "Sequences"`), needing [`generic::tlaplus_quirk`] to walk the
    /// repeated unfielded children (this file's flat-array dispatch alone
    /// cannot; same rationale as every other language here needing an
    /// `on_unmatched_node` hook for its own IMPORTS).
    ///
    /// `tlaplus_branch_types = ["if_then_else", "case"]` confirmed
    /// correct via a real parse-tree dump (`IF shape > 0 THEN ... ELSE
    /// ...` -> `if_then_else`, real `[if]`/`[then]`/`[else]` fields).
    pub const fn tlaplus() -> Self {
        Self {
            name: "tlaplus",
            // Deliberately EMPTY (NOT `&["operator_definition",
            // "function_definition"]`, this row's own first draft's real
            // mistake -- the exact same bug class
            // [`Self::lean`]'s own `func_types` doc comment documents):
            // both kinds' own `[name]` field DOES work via this file's
            // ordinary `child_text` lookup, but `walk`'s own func/method
            // branch would claim (and `return` from) them BEFORE the
            // bottom `on_unmatched_node` catch-all ever runs, silently
            // preventing `generic::tlaplus_quirk`'s own FULL claim (needed
            // for the body-recursion fix) from ever firing. Both kinds are
            // instead claimed purely through the universal bottom
            // catch-all.
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            // See this const's own doc comment: baseline's bare `"call"`
            // is an anonymous token, not a real named node -- dropped.
            // `bound_op`/`function_evaluation` both confirmed real and
            // reachable, fully claimed by
            // `generic::tlaplus_call_override` (neither fits the generic
            // single-field mechanism: `bound_op`'s own `parameter` field
            // is repeated, and `function_evaluation` has no field at
            // all).
            call_types: &["bound_op", "function_evaluation"],
            call_function_field: "name",
            call_arguments_field: "UNUSED_SEE_TLAPLUS_CALL_OVERRIDE",
            // Confirmed correct via a real parse-tree dump -- both are
            // real node kinds; claimed by `generic::tlaplus_quirk` since
            // each holds a REPEATED unfielded `identifier_ref` list (not
            // a single field this file's flat mechanism could read).
            import_types: &["extends", "instance"],
            branch_types: &["if_then_else", "case"],
            decorator_types: &[],
            name_field: "name",
            body_field: "definition",
        }
    }

    // =====================================================================
    // VERILOG (language-parity wave G2.3b)
    // =====================================================================

    /// Verilog: grammar is `tree-sitter-verilog` (crates.io, `LANGUAGE`
    /// const via `tree-sitter-language`, verified end-to-end with real
    /// parse-tree dumps -- including several deliberately-varied call/
    /// statement forms, since this grammar's expression-position-call
    /// support turned out genuinely incomplete, see below -- before being
    /// added).
    ///
    /// Baseline's `verilog_func_types = ["function_declaration",
    /// "task_declaration", "function_body_declaration",
    /// "function_statement"]` needed correction: `"function_statement"`
    /// does not exist in this grammar's `node-types.json` at all (real
    /// dump confirms no such kind anywhere); the real wrapper wiring is
    /// `function_declaration`/`task_declaration` each containing exactly
    /// one `function_body_declaration`/`task_body_declaration` child
    /// (`task_body_declaration` -- ALSO absent from baseline's own
    /// array -- confirmed as the real task-body wrapper kind via a
    /// dedicated fixture). This file's `func_types` lists BOTH wrapper
    /// AND inner-body kinds (baseline conflated the two into one flat
    /// list already, so this is a data fix, not an architecture change);
    /// [`generic::verilog_quirk`] resolves each kind's real, differently-
    /// shaped name field (`function_body_declaration`'s own
    /// `function_identifier` child wraps ANOTHER `function_identifier`
    /// wrapping the real `simple_identifier` leaf -- a doubly-nested
    /// wrapper, confirmed via a real dump, not baseline's assumed
    /// `find_first_descendant_by_kind` shallow scan; `task_body_declaration`
    /// analogous via `task_identifier`).
    ///
    /// `verilog_class_types = ["module_declaration", "class_declaration",
    /// "interface_declaration", "package_declaration",
    /// "type_declaration"]` confirmed correct as real node kinds, but
    /// `module_declaration`'s own name is buried inside an UNFIELDED
    /// `module_header` child's `simple_identifier` (no field on either
    /// level) -- matches baseline's own `find_first_descendant_by_kind`
    /// approach exactly for this specific kind (unlike func_types above),
    /// confirmed via a real dump.
    ///
    /// `verilog_call_types = ["system_tf_call", "subroutine_call",
    /// "function_subroutine_call", "method_call"]` -- all four confirmed
    /// real `node-types.json` entries, BUT porting all four verbatim
    /// would be a genuine correctness bug this row's own first draft
    /// almost shipped: `function_subroutine_call` (`"fields": {}`) wraps
    /// EXACTLY one `subroutine_call` child, which itself (`"fields": {}`)
    /// wraps EXACTLY one `method_call`/`randomize_call`/`system_tf_call`/
    /// `tf_call` child -- a THREE-DEEP wrapper chain for one real call
    /// (confirmed via `node-types.json`, not merely a dump spot-check).
    /// Since this file's own `walk` recurses into a claimed call node's
    /// children regardless of whether `Quirks::call_override` claims it
    /// (see `generic::walk`'s own `call_types` branch: `overridden ->
    /// walk_children(...)` unconditionally), listing ALL FOUR in
    /// `call_types` would record the SAME call up to three times (once
    /// per nesting level) rather than once. `call_types` below lists ONLY
    /// the genuinely LEAF/content-bearing kinds (`system_tf_call`,
    /// `method_call`) plus `tf_call` (confirmed via `node-types.json`,
    /// see [`Self::systemverilog`]'s own doc comment for its full field
    /// shape) -- `subroutine_call`/`function_subroutine_call` are pure
    /// wrappers with no content of their own to record and are
    /// deliberately DROPPED from this array; generic recursion still
    /// reaches the real inner call node regardless (an unlisted kind
    /// simply falls through `walk`'s own dispatch to its final
    /// `walk_children` call, unchanged).
    ///
    /// `system_tf_call` reachable directly as a full statement
    /// (`$display(...)`); `tf_call` reachable as a full statement too
    /// (`helper;`, the argument-less task-enable form) but NOT with
    /// parenthesized arguments as a bare statement -- CRITICAL,
    /// non-obvious finding from real fixtures, not baseline: a
    /// user-defined function/task call used as a bare top-level STATEMENT
    /// with parenthesized arguments (`helper(1);`) is a GENUINE PARSE
    /// ERROR in this grammar version (`has_error: true`, produces an
    /// `ERROR` node) -- only the expression/condition position (inside
    /// `if(...)`, confirmed reachable via `tf_call` nested three levels
    /// under `function_subroutine_call`/`subroutine_call`) actually
    /// produces a recognizable call node for that form; a plain-
    /// assignment RHS call expression (`y = helper(2);`) stays an opaque,
    /// unexpanded `expression` leaf with no visible call node at all in
    /// this grammar's incremental-parse recovery. This is a real,
    /// plain-Verilog-only grammar gap (confirmed absent by direct A/B
    /// testing against `tree-sitter-systemverilog`, which parses the
    /// identical bare-statement call form cleanly via
    /// `subroutine_call_statement` -- see [`Self::systemverilog`]'s own
    /// doc comment) -- accepted as this wave's real, documented depth
    /// rather than silently claimed as full parity with the baseline's
    /// own (C-grammar-backed, presumably more complete) Verilog call
    /// coverage.
    ///
    /// `verilog_import_types = ["extends", "import",
    /// "package_import_declaration"]` confirmed correct (all three real
    /// node kinds); `"extends"` specifically is Verilog's OOP-style class
    /// inheritance keyword parsing as a bare token inside `class_item`'s
    /// own `class_property`/`data_declaration` shape (not a dedicated
    /// heritage-clause node the way SystemVerilog's `class_type` sibling
    /// field is) -- left unclaimed at this Tier-2 depth (no INHERITS
    /// wiring for this wave's languages, matching the task's own scope).
    pub const fn verilog() -> Self {
        Self {
            name: "verilog",
            // See this const's own doc comment: baseline's
            // `"function_statement"` does not exist; the real inner-body
            // wrapper kinds are `function_body_declaration`/
            // `task_body_declaration` (neither in baseline's own array),
            // added alongside the two real top-level wrapper kinds.
            func_types: &[
                "function_declaration",
                "task_declaration",
                "function_body_declaration",
                "task_body_declaration",
            ],
            method_types: &[],
            class_types: &[
                "module_declaration",
                "class_declaration",
                "interface_declaration",
                "package_declaration",
                "type_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            // See this const's own doc comment: baseline's own
            // `"subroutine_call"`/`"function_subroutine_call"` are pure,
            // content-less WRAPPERS around one of these three real leaf
            // kinds -- listing them too would triple-record the same
            // call (this file's own `walk` still recurses into a claimed
            // call node's children). Every one of these three is
            // field-less, fully claimed by `generic::hdl_call_override`'s
            // leaf-identifier scan (mirrors baseline's own
            // `extract_hdl_callee`/`first_leaf_identifier` approach
            // exactly).
            call_types: &["system_tf_call", "tf_call", "method_call"],
            call_function_field: "UNUSED_SEE_VERILOG_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_VERILOG_CALL_OVERRIDE",
            import_types: &["extends", "import", "package_import_declaration"],
            branch_types: &["conditional_statement", "case_statement", "loop_statement"],
            decorator_types: &[],
            // Never actually consulted for `func_types`/`class_types` --
            // see this const's own doc comment: every real name shape
            // (doubly-nested function-identifier wrapper, unfielded
            // module_header descendant, ...) is fully claimed by
            // `generic::verilog_quirk` before this file's own
            // `name_field`-keyed fallback would ever run.
            name_field: "UNUSED_SEE_VERILOG_QUIRK",
            body_field: "UNUSED_SEE_VERILOG_QUIRK",
        }
    }

    // =====================================================================
    // VHDL (language-parity wave G2.3b)
    // =====================================================================

    /// VHDL: grammar is `tree-sitter-vhdl` (crates.io, `LANGUAGE` const
    /// via `tree-sitter-language`, verified end-to-end with real
    /// parse-tree dumps before being added).
    ///
    /// Baseline's own VHDL arrays confirmed the MOST accurate of this
    /// wave's HDL cluster -- every field-name mapping baseline's own
    /// `extract_defs.c:3622-3636` hand-codes (`entity_declaration`'s
    /// `entity` field, `architecture_definition`'s `architecture` field,
    /// `subprogram_declaration`/`_definition`'s nested
    /// `function_specification`/`procedure_specification` child's own
    /// `function`/`procedure` field) verified byte-for-byte correct
    /// against this grammar version via real parse-tree dumps of an
    /// entity+architecture+function+component-instantiation fixture, AND
    /// cross-checked directly against `node-types.json`.
    ///
    /// `vhdl_call_types = ["function_call", "procedure_call_statement",
    /// "component_instantiation_statement", "parenthesis_group"]` -- ALL
    /// FOUR confirmed real via `node-types.json`, but a direct field-by-
    /// field cross-check (not merely a real-dump spot-check, which this
    /// row's own first draft relied on and wrongly concluded only
    /// `parenthesis_group` needed a quirk) found `function_call`/
    /// `procedure_call_statement` are BOTH ALSO entirely field-less
    /// (`"fields": {}` -- `function_call`'s own children are just
    /// `generic_map_aspect`/`parenthesis_group`, i.e. it merely WRAPS a
    /// `parenthesis_group` rather than carrying a callee of its own at
    /// all; `procedure_call_statement`'s are `label_declaration`/`name`,
    /// no field distinguishing which is the callee). Only
    /// `component_instantiation_statement` has a real, usable field of
    /// its own (`[component]`, confirmed via `node-types.json`). Since
    /// THREE of these four `call_types` entries cannot use this file's
    /// generic single-field `call_function_field` mechanism at all (not
    /// merely one, as this row's own first draft assumed), ALL FOUR are
    /// instead claimed in full by [`generic::vhdl_quirk`] rather than
    /// leaving three of them to silently fail a placeholder field lookup
    /// -- `call_function_field`/`call_arguments_field` below are both
    /// placeholders, matching this file's own "UNUSED_SEE_..."
    /// convention for a fully quirk-claimed call shape (same posture as
    /// [`Self::c`]/[`Self::cpp`]/[`Self::php`]'s own full-claim rows this
    /// file's module doc already calls out). [`generic::vhdl_quirk`]
    /// mirrors baseline's own dedicated `extract_vhdl_callee`
    /// (`extract_calls.c`:842-857, the preceding-named-sibling lookup)
    /// for `parenthesis_group` specifically, and reads
    /// `component_instantiation_statement`'s own `[component]` field
    /// directly; `function_call`/`procedure_call_statement` are left
    /// UNCLAIMED (this row's `call_types` no longer lists either) since
    /// neither ever carries a callee of its own to record -- recording a
    /// callee-less `function_call`/`procedure_call_statement` symbol
    /// would only ever produce an empty/wrong callee string, worse than
    /// omitting the edge entirely; the REAL callee-bearing node in every
    /// case is its own nested `parenthesis_group`, which this row's
    /// `call_types` already lists and generic recursion still reaches
    /// (neither wrapper is claimed as fully-handled, so recursion
    /// continues into it regardless).
    ///
    /// `vhdl_import_types = ["library_clause", "use_clause"]` confirmed
    /// correct as real node kinds via a real dump (`library IEEE;` /
    /// `use IEEE.STD_LOGIC_1164.ALL;`), but NEITHER has a usable field of
    /// its own for the imported path (`library_clause`'s
    /// `logical_name_list` and `use_clause`'s `selected_name_list` are
    /// both field-less children, confirmed via `node-types.json`) --
    /// [`generic::vhdl_quirk`] reconstructs the dotted path from each
    /// clause's own text directly rather than a field lookup.
    pub const fn vhdl() -> Self {
        Self {
            name: "vhdl",
            func_types: &["subprogram_declaration", "subprogram_definition"],
            method_types: &[],
            class_types: &[
                "entity_declaration",
                "architecture_definition",
                "component_declaration",
                "interface_declaration",
                "package_declaration",
                "protected_type_declaration",
                "record_type_definition",
                "type_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["design_file"],
            // See this const's own doc comment: baseline's own
            // `"function_call"`/`"procedure_call_statement"` entries are
            // BOTH entirely field-less wrappers with no callee of their
            // own to record at all (confirmed via `node-types.json`) --
            // dropped in favor of the two real, callee-bearing kinds:
            // `component_instantiation_statement`'s own `[component]`
            // field, and `parenthesis_group`'s own preceding-sibling
            // callee (mirrors baseline's own `extract_vhdl_callee`
            // exactly).
            call_types: &["component_instantiation_statement", "parenthesis_group"],
            call_function_field: "UNUSED_SEE_VHDL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_VHDL_CALL_OVERRIDE",
            // See this const's own doc comment: both real node kinds, but
            // field-less for the imported path -- claimed by
            // `generic::vhdl_quirk`, not this file's flat array
            // mechanism.
            import_types: &["library_clause", "use_clause"],
            branch_types: &["if_statement", "case_statement", "loop_statement"],
            decorator_types: &[],
            // Never actually consulted for `class_types` (every one
            // fully claimed by `generic::vhdl_quirk`'s declaration-
            // keyword-to-field-name map, mirroring baseline's own
            // `extract_defs.c:3622-3636` exactly); `func_types` uses the
            // SAME quirk for its own nested-specification-child lookup.
            name_field: "UNUSED_SEE_VHDL_QUIRK",
            body_field: "UNUSED_SEE_VHDL_QUIRK",
        }
    }

    // =====================================================================
    // SYSTEMVERILOG (language-parity wave G2.3b)
    // =====================================================================

    /// SystemVerilog: grammar is `tree-sitter-systemverilog` (crates.io,
    /// `LANGUAGE` const via `tree-sitter-language`, verified end-to-end
    /// with real parse-tree dumps before being added) -- a GENUINELY
    /// DIFFERENT, more modern grammar than plain `tree-sitter-verilog`
    /// (not a superset/reuse relationship the way CUDA/GLSL are of C++/C
    /// in this crate -- confirmed by real parse-tree differences below,
    /// not assumed from the language relationship alone).
    ///
    /// Baseline's `systemverilog_func_types = ["function_declaration",
    /// "task_declaration", "function_body_declaration",
    /// "function_statement"]` needed the SAME `"function_statement"`
    /// correction as plain Verilog (does not exist in this grammar
    /// either), but this grammar's own `function_body_declaration`/
    /// `class_method`-wrapped variant carries a REAL, DIRECT `[name]`
    /// field (`simple_identifier`) -- a real, confirmed IMPROVEMENT over
    /// plain Verilog's doubly-nested unfielded wrapper (see
    /// [`Self::verilog`]'s own doc comment), needing NO quirk for name
    /// resolution at all, just this file's ordinary generic `name_field`
    /// path. `task_declaration`/`task_body_declaration` were NOT
    /// independently re-verified for this specific grammar (no dedicated
    /// SystemVerilog task fixture run) -- kept per baseline for
    /// completeness, consistent with this row's own function-focused
    /// verification depth.
    ///
    /// `systemverilog_class_types = ["class_declaration",
    /// "module_declaration", "interface_declaration",
    /// "library_declaration", "package_declaration", "type_declaration"]`
    /// -- `class_declaration`/`function_body_declaration` (also in
    /// `func_types`) both confirmed to carry a real, direct, POPULATED
    /// `[name]` field via BOTH `node-types.json` AND a real parse-tree
    /// dump, needing no quirk at all. TWO of the remaining members need a
    /// dedicated [`generic::systemverilog_quirk`] this row's own first
    /// draft initially missed for one and misdiagnosed for the other
    /// (both caught only once this wave's own test suite ran the real
    /// grammar, not by `node-types.json` inspection alone -- see that
    /// quirk's own doc comment for the full finding):
    /// - `type_declaration` names its own field `[type_name]`, NOT
    ///   `[name]` at all.
    /// - `module_declaration` DOES have its own real `[name]` field
    ///   declared in `node-types.json`'s schema (this row's own first
    ///   draft wrongly treated that schema-level declaration as proof the
    ///   field is actually POPULATED), but a real parse-tree dump shows
    ///   that field is NEVER populated for the ANSI-header module form
    ///   (`module widget; ... endmodule`, the only form real source uses
    ///   in practice) -- the name instead lives on a NESTED
    ///   `module_ansi_header` child's OWN separate, real, `required: true`
    ///   `[name]` field, a genuine grammar-generation improvement over
    ///   plain Verilog's own unfielded `module_header` descendant scan,
    ///   but one level deeper than this row's own first draft assumed.
    ///
    /// `interface_declaration`/`library_declaration`/`package_declaration`
    /// remain on the unmodified generic `[name]`-field path (NOT
    /// independently re-verified this wave against a real parse-tree dump
    /// the way `class_declaration`/`module_declaration`/`type_declaration`
    /// were -- kept per baseline for completeness, consistent with this
    /// row's own function/module-focused verification depth). Also
    /// confirmed: `class_declaration extends Base` parses the class-type
    /// heritage directly as a sibling `class_type` node (NOT wrapped
    /// inside a field-less `data_declaration` the way plain Verilog's
    /// weaker `extends`-as-bare-token idiom is) -- a real, INHERITS-ready
    /// signal this row deliberately does NOT wire (Tier-2 scope, no
    /// dedicated richer edge for this wave's languages, matching the
    /// task's own instruction, same restraint [`Self::cobol`]'s own doc
    /// comment notes for COBOL's total absence of any heritage concept).
    ///
    /// `systemverilog_call_types = ["function_subroutine_call",
    /// "system_tf_call", "method_call"]` -- CONFIRMED, via real dumps AND
    /// `node-types.json`, this grammar's call coverage is genuinely MORE
    /// complete than plain Verilog's: a bare top-level call statement
    /// with arguments (`helper(1);`) parses CLEANLY here
    /// (`subroutine_call_statement` -> `subroutine_call` -> `tf_call`,
    /// real `hierarchical_identifier` callee) -- the exact form that is a
    /// parse ERROR in plain Verilog (see [`Self::verilog`]'s own doc
    /// comment for the direct A/B finding). BUT baseline's own
    /// `"function_subroutine_call"` entry has the EXACT SAME
    /// wrapper-nesting hazard [`Self::verilog`]'s own doc comment already
    /// found and fixed there: a direct `node-types.json` cross-check
    /// (this row's own first draft skipped, wrongly assuming the "closes
    /// a real gap" framing extended to porting every wrapper kind too)
    /// confirms `subroutine_call_statement`/`function_subroutine_call`/
    /// `subroutine_call` are ALL pure, content-less wrappers nesting up
    /// to FOUR deep around one real leaf call kind
    /// (`method_call`/`system_tf_call`/`tf_call`/`randomize_call`) --
    /// listing any wrapper alongside its own leaf descendant in
    /// `call_types` would multiply-record the same call (this file's own
    /// `walk` still recurses into a claimed call node's children
    /// regardless of `Quirks::call_override`). `call_types` below lists
    /// ONLY the three genuinely leaf/content-bearing kinds
    /// (`system_tf_call`, `tf_call`, `method_call` -- `tf_call` closes
    /// the one real gap baseline's own array is missing, matching
    /// [`Self::verilog`]'s own corrected array exactly since both
    /// grammars share this same leaf-kind vocabulary) -- every wrapper
    /// kind is deliberately DROPPED, not merely deduplicated; generic
    /// recursion still reaches the real inner call node regardless (an
    /// unlisted kind simply falls through `walk`'s own dispatch to its
    /// final `walk_children` call, unchanged). Every one of the three
    /// leaf kinds is field-less, fully claimed by
    /// [`generic::hdl_call_override`]'s leaf-identifier scan (SHARED with
    /// plain Verilog, same `first_leaf_identifier` approach baseline's
    /// own `extract_hdl_callee` already uses -- both grammars produce the
    /// identical field-less leaf shape for these three kinds).
    ///
    /// `systemverilog_import_types` confirmed correct as real node kinds
    /// (`package_import_declaration` reachable and real, real
    /// `[name]`-less `package_import_item`); `systemverilog_var_types =
    /// ["parameter", "localparam"]` NOT independently verified this wave
    /// (complexity/var extraction deferred per the task's own
    /// instruction) -- kept per baseline, unused by this file's own
    /// extraction (this file has no `var_types` field at all; baseline's
    /// concept there maps to nothing this crate's generic engine reads).
    pub const fn systemverilog() -> Self {
        Self {
            name: "systemverilog",
            func_types: &[
                "function_declaration",
                "task_declaration",
                "function_body_declaration",
                "task_body_declaration",
            ],
            method_types: &[],
            class_types: &[
                "class_declaration",
                "module_declaration",
                "interface_declaration",
                "library_declaration",
                "package_declaration",
                "type_declaration",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            // See this const's own doc comment: baseline's own
            // `"function_subroutine_call"` is a pure, content-less
            // WRAPPER (confirmed via `node-types.json`) around one of
            // these three real leaf kinds, nested up to four deep via
            // `subroutine_call_statement`/`subroutine_call` too -- listing
            // any wrapper here would multiply-record the same call. Only
            // the three genuinely leaf/content-bearing kinds are listed
            // (`tf_call` closes a real gap missing from baseline's own
            // array); every one is field-less, claimed by
            // `generic::hdl_call_override` (shared with plain Verilog).
            call_types: &["system_tf_call", "tf_call", "method_call"],
            call_function_field: "UNUSED_SEE_SYSTEMVERILOG_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_SYSTEMVERILOG_CALL_OVERRIDE",
            import_types: &[
                "package_import_declaration",
                "extends",
                "import",
                "include",
                "include_statement",
                "instance",
                "use_clause",
            ],
            branch_types: &["case_statement", "if"],
            decorator_types: &[],
            // Live for `func_types`/`class_types` (this grammar's real,
            // direct `[name]` field, unlike plain Verilog) -- see this
            // const's own doc comment.
            name_field: "name",
            body_field: "UNUSED_SEE_SYSTEMVERILOG_NO_BODY_FIELD",
        }
    }

    // =====================================================================
    // COBOL (language-parity wave G2.3b)
    // =====================================================================

    /// COBOL: grammar VENDORED, not a crates.io dependency -- see
    /// `crates/enforcer-memory/vendor/tree-sitter-cobol-local/` (a local
    /// path-dependency, added below in `Cargo.toml`'s own
    /// `[dependencies]` table). The crate literally named
    /// `tree-sitter-cobol` (0.1.0) ships NO usable Rust API at all (a
    /// `[[bin]]`-only placeholder publish, confirmed by downloading and
    /// inspecting its own `Cargo.toml`/source directly -- no `[lib]`
    /// section, zero dependencies); the real grammar
    /// (`github.com/yutaro-sakamoto/tree-sitter-cobol`, unpublished,
    /// MIT-licensed, EXACT `tree_sitter_COBOL` symbol match against the C
    /// baseline's own `lang_specs.c` reference) needed one small
    /// portability fix (a C99 VLA MSVC's `cl.exe` rejects, documented at
    /// its own edit site in the vendored `scanner.c`) before it would
    /// compile at all on this workspace's Windows/MSVC toolchain -- see
    /// `vendor/tree-sitter-cobol-local/src/lib.rs`'s own module doc for
    /// the full grammar-sourcing rationale. Verified end-to-end (real
    /// parse, no ABI error, `has_error: false`) with dedicated fixtures
    /// covering IDENTIFICATION/DATA/PROCEDURE DIVISION, CALL, IF,
    /// EVALUATE, PERFORM, and COPY before being wired in.
    ///
    /// Baseline's `cobol_func_types = ["program_definition"]` and its own
    /// dedicated `extract_defs.c:882-892` name resolution
    /// (`program_definition` has no `name` field; the real name is
    /// `identification_division > program_name`, a field-less leaf) BOTH
    /// confirmed byte-for-byte correct via a real parse-tree dump --
    /// COBOL is the one language in this wave whose baseline logic needed
    /// ZERO correction for its func/name handling specifically (unlike
    /// every other language in this cluster).
    ///
    /// TWO real, confirmed baseline corrections found via dedicated
    /// EVALUATE/PERFORM/data-description fixtures baseline's own arrays
    /// do not cover precisely:
    /// - `cobol_var_types = ["data_description_entry"]`: this exact node
    ///   kind does not exist in this grammar at all -- the real kind is
    ///   `data_description` (`01 X PIC 9(4)` -> `data_description
    ///   [level_number] "01" [entry_name] "X" [picture_clause] ...`).
    ///   Unused by this file's own extraction regardless (this file has
    ///   no `var_types` field; kept as a documented finding only, not
    ///   wired).
    /// - `cobol_branch_types = ["if_statement", "evaluate_statement",
    ///   "perform_statement"]`: NONE of the three names match this
    ///   grammar's real kinds precisely -- `IF ... END-IF` parses as
    ///   `if_header` (not `if_statement`, confirmed by two independent
    ///   fixtures), `EVALUATE ... END-EVALUATE` FLATTENS into sibling
    ///   `evaluate_header`/`when`/`when_other`/`END_EVALUATE` nodes at the
    ///   SAME nesting level as everything else in `procedure_division`
    ///   (there is no single `evaluate_statement` wrapper node AT ALL),
    ///   and `PERFORM SUB-PARA` (the paragraph-call form, this crate's
    ///   own fixture's only PERFORM variant exercised) is
    ///   `perform_statement_call_proc` (not bare `perform_statement`,
    ///   which may still be the real kind for OTHER perform forms --
    ///   in-line `PERFORM ... END-PERFORM`, `PERFORM ... TIMES`, etc. --
    ///   not independently verified this wave). `branch_types` below uses
    ///   the three CONFIRMED-real kinds from this crate's own fixtures
    ///   rather than porting the unverified baseline names verbatim.
    ///
    /// `cobol_import_types = ["open_statement", "use_statement",
    /// "with_clause"]` -- all three confirmed as REAL node kinds via
    /// `node-types.json`, but semantically weak: `open_statement` is
    /// COBOL file-I/O (`OPEN INPUT foo-file`, field-less, only unnamed
    /// `open_arg` children -- no dependency/module concept at all,
    /// COBOL genuinely has none), `with_clause` is `DISPLAY ... WITH
    /// ADVANCING`/`WITH NO` output-formatting syntax (confirmed via
    /// `node-types.json`'s own child-type list -- `ADVANCING`/`NO`/
    /// `disp_attr` -- utterly unrelated to any dependency concept, a
    /// genuinely questionable baseline choice, not merely stale). This
    /// row instead wires the real COBOL dependency mechanism baseline's
    /// own array never lists at all: `copy_statement` (`COPY COMMONDEF.`
    /// -> real, direct `[book]` field, confirmed via a dedicated
    /// fixture), COBOL's actual "include this other source file" idiom --
    /// a genuine improvement over baseline's own weaker choice, not a
    /// baseline-parity regression, following this wave's own "verify,
    /// don't blindly transcribe" mandate.
    pub const fn cobol() -> Self {
        Self {
            name: "cobol",
            func_types: &["program_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["identification_division"],
            // COBOL's `CALL 'SUBPROG' USING X` names its callee via a
            // STRING LITERAL (the called program's name), not an
            // identifier expression -- fully claimed by
            // `generic::cobol_call_override`, mirroring baseline's own
            // `extract_cobol_callee` exactly (its `x` field holds the
            // quoted program name, stripped of quotes).
            call_types: &["call_statement"],
            call_function_field: "UNUSED_SEE_COBOL_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_COBOL_CALL_OVERRIDE",
            // See this const's own doc comment: baseline's own three-entry
            // array replaced with the real COBOL dependency idiom this
            // grammar exposes with a genuine, direct field
            // (`copy_statement`'s `book` field) -- claimed by
            // `generic::cobol_quirk`, not this file's flat single-field
            // mechanism (the field name itself, `book`, IS usable
            // directly, but this file's own `import_types` array has no
            // per-kind field-name slot of its own the way `LangSpec`'s
            // `call_function_field` does for calls -- the quirk hook
            // reads it explicitly instead).
            import_types: &["copy_statement"],
            // See this const's own doc comment: baseline's own three
            // names do not match this grammar's real kinds -- corrected
            // to the confirmed-real `if_header`/`evaluate_header`/
            // `perform_statement_call_proc` from this crate's own
            // fixtures.
            branch_types: &[
                "if_header",
                "evaluate_header",
                "perform_statement_call_proc",
            ],
            decorator_types: &[],
            // Never actually consulted -- `program_definition` is fully
            // claimed by `generic::cobol_quirk`'s
            // `identification_division > program_name` walk before this
            // file's own `name_field`-keyed fallback would ever run.
            name_field: "UNUSED_SEE_COBOL_QUIRK",
            body_field: "UNUSED_SEE_COBOL_QUIRK",
        }
    }
}

/// Language-parity wave G2.3d: Just, HLSL, ISPC, PureScript, Magma,
/// Hare, Pony, NASM -- appended as a separate `impl LangSpec` block
/// (Rust supports multiple inherent-impl blocks for the same type in one
/// module) rather than spliced into the primary block above, because
/// that block's own tail is under heavy concurrent-append contention from
/// several sibling G2.3 workers editing this same file in parallel this
/// wave; appending a whole new block here was the only reliably
/// land-able edit shape once every attempt at a mid-file splice kept
/// losing the "file changed since read" race against those other
/// workers' own concurrent tail-appends.
impl LangSpec {
    /// Just (`justfile`/`.just`). Language-parity wave G2.3d. Grammar:
    /// `tree-sitter-just` (crates.io, `casey/tree-sitter-just` -- the
    /// language's own creator's official grammar). Verified via a real
    /// parse-tree dump (`cargo run` against a scratch crate depending on
    /// this grammar directly): every baseline `just_*` array entry
    /// (`internal/cbm/lang_specs.c`) matched this grammar version's real
    /// node kinds exactly, with real, working fields throughout -- one of
    /// the cleanest grammars this whole campaign has onboarded.
    /// - `func_types` is `["recipe"]`, matching the baseline's
    ///   `just_func_types` exactly. `recipe` has NO `name`/`body` fields
    ///   of its own (confirmed: `{"type":"recipe","fields":{}}`) -- its
    ///   own name lives on a NESTED `recipe_header`'s `"name"` field, and
    ///   its body is the sibling `recipe_body` child -- handled by
    ///   [`generic::just_quirk`] rather than this file's generic
    ///   `name_field`/`body_field` mechanism.
    /// - `class_types`/`interface_types`/`enum_types`/`alias_types`/
    ///   `field_types` are all empty: justfiles have no product-type
    ///   declaration syntax at all (matches the baseline's own
    ///   `empty_types` for every one of these fields on `CBM_LANG_JUST`).
    /// - `module_types` is `["source_file"]`, matching the baseline's
    ///   `just_module_types` exactly and confirmed as this grammar's real
    ///   root node kind via the parse-tree dump.
    /// - `call_types` is `["function_call", "dependency"]`, matching the
    ///   baseline's `just_call_types` exactly. `function_call` has real
    ///   `"name"`/`"arguments"` fields (confirmed:
    ///   `{"type":"function_call","fields":{"arguments":{...},"name":{...}}}`)
    ///   -- NOT this file's usual `call_function_field`/
    ///   `call_arguments_field` names (`"function"`/`"arguments"`), so
    ///   `call_function_field` is set to `"name"` here (a real field name,
    ///   just a different one than most other rows in this file use).
    ///   `dependency` (a recipe listed as another recipe's prerequisite,
    ///   `build: setup` -- `setup` is the dependency) has only a `"name"`
    ///   field and no arguments at all -- handled by
    ///   [`generic::just_call_override`] rather than the generic
    ///   single-field reconstruction, since a bare dependency name is not
    ///   really a "function call" in the ordinary sense but the baseline
    ///   itself records it as a CALLS edge anyway (matching that choice
    ///   rather than improving on it).
    /// - `import_types` is `["import"]`, matching the baseline's
    ///   `just_import_types` exactly -- confirmed present with no fields
    ///   of its own (`{"type":"import","fields":{}}`); its own path is a
    ///   `string` child, read via [`generic::just_quirk`].
    /// - `branch_types` is `["if_expression"]`, matching the baseline's
    ///   `just_branch_types` exactly -- confirmed present with real
    ///   `"alternative"`/`"body"`/`"consequence"` fields.
    /// - `decorator_types` is empty: justfiles have no decorator/
    ///   attribute syntax the baseline itself records for this language
    ///   (`CBM_LANG_JUST`'s own baseline row has no `decorator_node_types`
    ///   entry either).
    pub const fn just() -> Self {
        Self {
            name: "just",
            func_types: &["recipe"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["function_call", "dependency"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            import_types: &["import"],
            branch_types: &["if_expression"],
            decorator_types: &[],
            // Never actually consulted for a symbol name: `recipe` is
            // fully claimed by `generic::just_quirk` (see this const's
            // own doc comment on `func_types`).
            name_field: "UNUSED_SEE_JUST_QUIRK",
            body_field: "UNUSED_SEE_JUST_QUIRK",
        }
    }

    /// HLSL (High-Level Shading Language, DirectX; `.fx`/`.hlsl`/
    /// `.hlsli`). Language-parity wave G2.3d. Grammar: `tree-sitter-hlsl`
    /// (crates.io, `theHamsta/tree-sitter-hlsl` -- the same author-lineage
    /// vendored at
    /// `codebase-memory-mcp/internal/cbm/vendored/grammars/hlsl/`, exact
    /// copyright-line match "Max Brunsfeld"/"Stephan Seitz"). This grammar
    /// is LITERALLY `tree-sitter-cpp`'s own grammar
    /// (`const CPP = require("tree-sitter-cpp/grammar")` at this crate's
    /// own `grammar.js:1`) with HLSL-specific extras layered on top
    /// (`cbuffer_specifier`, `hlsl_attribute`, shader `semantics`
    /// clauses, `register(...)`) -- confirmed via both this crate's own
    /// `node-types.json` (every ordinary C/C++ construct
    /// [`Self::cpp`]/[`generic::cpp_quirk`] already reads keeps the
    /// identical node-kind name and field shape) and a real parse-tree
    /// dump. Reuses [`Self::cpp`]'s own arrays and
    /// [`generic::cpp_quirks`]'s declarator-unwrapping quirk UNCHANGED
    /// (mirrors [`Self::cuda`]'s identical "genuinely dedicated grammar
    /// that happens to be a strict C/C++ superset" reuse posture) --
    /// `function_definition`'s own name lives on a nested
    /// `function_declarator`'s `"declarator"` field exactly like C++'s
    /// does, needing the SAME declarator-unwrap quirk rather than this
    /// file's `name_field` mechanism.
    /// - One real, non-obvious grammar-shape finding from the parse-tree
    ///   dump: a TOP-LEVEL `cbuffer Name : register(b0) { ... }` block (the
    ///   overwhelmingly common real-world shader-source shape) does NOT
    ///   parse as the grammar's own dedicated `cbuffer_specifier` node at
    ///   all -- that node kind is wired into this grammar's
    ///   `_non_case_statement` extension point only (`grammar.js:49`,
    ///   `choice($.discard_statement, $.cbuffer_specifier, original)`),
    ///   which is reachable only from STATEMENT context (inside a
    ///   function body), never from the file's own top-level item rule
    ///   (`_top_level_item: (_, original) => original` at `grammar.js:11`
    ///   leaves that rule fully unmodified from plain C++, which has no
    ///   `cbuffer` keyword at all). A top-level `cbuffer` block instead
    ///   parses as an ordinary `declaration` (whose own `type_identifier`
    ///   happens to read the literal text `"cbuffer"`) immediately
    ///   followed by a bare `compound_statement` sibling, with tree-sitter
    ///   inserting a MISSING `;` token in between to recover -- confirmed
    ///   directly in the dump, not merely inferred from the grammar
    ///   source. This is a genuine, verified upstream grammar limitation
    ///   (not a fixable quirk within this file's own scope -- the
    ///   underlying grammar itself needs a parse fix), accepted as a
    ///   documented gap: this crate's own function/struct/enum/call/
    ///   `#include` extraction -- the actual Tier-2 defs+calls+imports
    ///   promise -- is unaffected (confirmed working correctly around the
    ///   cbuffer parse hiccup in the same dump), and cbuffer contents
    ///   contain no function/call/import constructs of their own kind
    ///   worth extracting regardless.
    pub const fn hlsl() -> Self {
        Self::cpp()
    }

    /// ISPC (Intel Implicit SPMD Program Compiler, `.ispc`). Language-
    /// parity wave G2.3d. Grammar: `tree-sitter-ispc`
    /// (`fab4100/tree-sitter-ispc`, formerly published under
    /// `tree-sitter/tree-sitter-ispc` and now living at
    /// `tree-sitter-grammars/tree-sitter-ispc` -- exact copyright-line
    /// match "Fabian Wermelinger" against this crate's own vendored
    /// `internal/cbm/vendored/grammars/ispc/LICENSE`, confirming the
    /// identical grammar lineage the baseline itself vendored). VENDORED
    /// via a `parser.c`-only build (no `scanner.c` for this grammar) --
    /// not a crates.io dependency: the only crates.io-published version
    /// (0.1.0, under the stale `tree-sitter/tree-sitter-ispc` repo name)
    /// pins `tree-sitter = "~0.20.10"` as a genuine RUNTIME (non-dev)
    /// dependency, and the live upstream repository's own
    /// `bindings/rust/lib.rs` (confirmed by reading it directly, not
    /// merely inferred from the crate metadata) STILL uses the old raw
    /// `extern "C" fn tree_sitter_ispc() -> Language` binding shape too --
    /// there is no newer, safely-`tree-sitter-language`-shimmed version to
    /// prefer either as a crates.io upgrade or as a pinned git dependency,
    /// unlike e.g. [`Self::crystal`]'s git-dependency precedent. This is
    /// the exact "unavailable/broken ABI" condition the campaign's own
    /// grammar-sourcing fallback order (crate -> vendor -> defer) exists
    /// for; vendoring the C source and re-binding through
    /// `tree-sitter-language` (this wave's own
    /// `vendor/tree-sitter-ispc-local/`, mirroring
    /// [`Self::squirrel`]'s identical vendoring precedent) is preferred
    /// over deferring outright since the grammar itself parses cleanly
    /// (verified via a real parse-tree dump against the vendored
    /// `parser.c` directly, zero parse errors). This grammar is
    /// documented upstream (per its own crate description, "ISPC grammar
    /// for tree-sitter, based on C grammar") and confirmed via the dump to
    /// be node-kind-and-field IDENTICAL to plain C for every construct
    /// this file's [`Self::c`]/[`generic::c_quirk`] already reads
    /// (`function_definition`'s nested `function_declarator.declarator`
    /// name, `struct_specifier`'s `name`/`body` fields, `call_expression`'s
    /// `function`/`arguments` fields, `preproc_include`'s `path` field) --
    /// this row reuses [`Self::c`]'s own arrays and
    /// [`generic::c_quirks`]'s declarator-unwrapping quirk UNCHANGED
    /// (mirrors [`Self::cuda`]/[`Self::hlsl`]'s identical
    /// "dedicated-grammar-happens-to-be-a-strict-superset" reuse
    /// posture), with only ISPC's SPMD-specific `foreach`/`uniform`/
    /// `varying`/`export` keyword extras layered on top -- none of which
    /// this crate's own extraction scope (defs+calls+imports+branches)
    /// needs to recognize specially: `foreach (i = 0 ... n) { ... }`
    /// parses as an ordinary (if unfamiliar-looking) construct whose inner
    /// `compound_statement` body is still reached by plain recursion, and
    /// `export`-prefixed functions still parse as ordinary
    /// `function_definition` nodes (confirmed in the dump: `export void
    /// updateAll(...)` parses with an extra `storage_class_specifier`
    /// sibling field the generic walk simply does not need to consult).
    pub const fn ispc() -> Self {
        Self::c()
    }

    /// PureScript (Haskell-like, compiles to JS; `.purs`). Language-parity
    /// wave G2.3d. Grammar: `tree-sitter-purescript`
    /// (`postsolar/tree-sitter-purescript`, the actively-maintained fork
    /// the official tree-sitter wiki itself now lists as PureScript's
    /// current parser -- exact copyright-line match "Maskhjarna,
    /// postsolar" against this crate's own vendored
    /// `internal/cbm/vendored/grammars/purescript/LICENSE`, confirming the
    /// identical grammar lineage the baseline itself vendored). Not a
    /// crates.io dependency (crates.io has no `tree-sitter-purescript` at
    /// all -- confirmed via a direct crates.io API search returning zero
    /// results) -- bound via a `git` dependency pinned to tag `v0.3.0`
    /// instead (same "option 1, a maintained crate exists" sourcing this
    /// wave's own [`Self::magma`] and G2.2a's Crystal precedent both use:
    /// a real, well-formed `tree-sitter-language`-based crate that simply
    /// has never been pushed to the registry). Verified via a real
    /// parse-tree dump against this exact pinned tag (`cargo run` against
    /// a scratch crate depending on it directly), zero parse errors.
    /// - `func_types` is `["function"]`, matching the baseline's own
    ///   `purescript_func_types` exactly. `function` has real `"name"`/
    ///   `"pattern"`/`"patterns"`/`"rhs"` fields (confirmed:
    ///   `{"type":"function","fields":{"name":{...},"pattern":{...},"patterns":{...},"rhs":{...}}}`)
    ///   -- but its OWN "body" is the `"rhs"` field (an expression, not a
    ///   block), not a field literally named `"body"` this file's generic
    ///   `body_field` mechanism expects, and its `"name"` field's own
    ///   `"types"` list additionally includes the UNNAMED `(`/`)` tokens
    ///   (for an operator-style definition like `(<>) a b = ...`) --
    ///   [`generic::purescript_quirk`] reads `"name"` filtered to only its
    ///   NAMED child (a `variable`/`operator` node) and walks `"rhs"`
    ///   directly as this function's own body, rather than this file's
    ///   generic mechanism.
    /// - `class_types` is `["class_declaration", "data", "newtype",
    ///   "type_alias"]`, matching the baseline's `purescript_class_types`
    ///   exactly -- EVERY one of these four confirmed present via the
    ///   parse-tree dump (a prior WebFetch-summarization pass on this
    ///   same grammar's `node-types.json` had INCORRECTLY reported
    ///   `type_alias` as absent -- re-verified directly with a raw
    ///   `node.js` JSON parse of the downloaded file itself, which is the
    ///   ground truth this doc comment relies on, not that earlier
    ///   summarization artifact). `data`/`newtype`/`type_alias` each have
    ///   a real, working `"name"` field; `class_declaration` has NO
    ///   fields of its own at all (`{"type":"class_declaration","fields":{}}`)
    ///   -- its own name lives nested two levels down, on a
    ///   `class_head`'s own `class_name` child's `type` grandchild --
    ///   handled by [`generic::purescript_quirk`].
    /// - `interface_types`/`enum_types`/`alias_types`/`field_types` are
    ///   all empty: `type_alias` (which some languages would model as a
    ///   dedicated `alias_types` entry) is instead folded into
    ///   `class_types` here, matching the baseline's own identical choice
    ///   verbatim (`purescript_class_types` is the ONE array the baseline
    ///   itself lists `"type_alias"` in; it has no separate alias/
    ///   interface/enum/field array of its own for this language at all).
    /// - `module_types` is empty, NOT the baseline's own
    ///   `purescript_module_types = {"module"}`. This is a real, verified
    ///   baseline correction: the real grammar's `module` node kind is
    ///   NOT a whole-file wrapping declaration at all -- it is the ALIASED
    ///   qualified-module-NAME identifier itself (`_modid: $ => alias($.constructor,
    ///   $.module)` in this grammar's own `grammar/module.js:8`), appearing
    ///   over and over as a plain name reference (e.g. once per `import
    ///   Effect.Console` qualifier segment), never as a container a
    ///   "Module symbol" declaration could meaningfully be extracted from
    ///   -- confirmed directly in the parse-tree dump, where `module
    ///   Main where ...` produces `purescript { module "module"[keyword],
    ///   field:name -> qualified_module { module "Main" } }`, i.e. the
    ///   ACTUAL whole-file root node is named `purescript` (an entirely
    ///   different, un-fielded node this baseline array never names at
    ///   all either), and the `module` node instance inside it is merely
    ///   the bare text `"Main"`. Porting the baseline's array verbatim
    ///   would make this crate mint a spurious Module symbol on every
    ///   qualified-name reference anywhere in a file (Prelude, Effect,
    ///   Console, ... one per import line), which is wrong in a way this
    ///   crate's [`generic::walk`] would actually observe (unlike
    ///   [`Self::squirrel`]'s "provably inert due to a shallow-scan-depth
    ///   baseline bug" class of dead array, this one IS reachable by this
    ///   crate's own depth-first walk) -- left empty rather than ported.
    /// - `call_types` is `["exp_apply"]`, matching the baseline's
    ///   `purescript_call_types` exactly -- confirmed present via the
    ///   dump (`log (show (add 1 2))` nests three `exp_apply` calls). No
    ///   fields at all (`{"type":"exp_apply","fields":{}}`) -- the callee
    ///   is always the FIRST named child, the remaining named children
    ///   are positional arguments -- handled by
    ///   [`generic::purescript_call_override`].
    /// - `import_types` is `["import", "import_item", "instance"]`,
    ///   matching the baseline's `purescript_import_types` exactly --
    ///   EVERY one of these three confirmed present via the dump (a prior
    ///   WebFetch-summarization pass had also incorrectly reported
    ///   `instance` as absent; re-verified via the same raw JSON parse).
    ///   `import` has real `"module"`/`"imports"`/`"import_rename"`
    ///   fields; `import_item`/`instance` are not separately walked by
    ///   this row's own quirk (matching the baseline's own choice to list
    ///   them without any special per-kind handling of its own either --
    ///   `import_item` is always a child of an already-recorded `import`
    ///   node, and `instance` would need its own dedicated handling this
    ///   wave's scope does not extend to; this grammar's real
    ///   instance-declaration node kind is actually spelled
    ///   `class_instance`, so the baseline's literal `"instance"` entry,
    ///   while confirmed present as SOME node kind in this grammar's
    ///   vocabulary, is not the node this grammar uses for a PureScript
    ///   `instance ... where` block -- ported verbatim anyway per this
    ///   row's "match the baseline's real array, corrected only where a
    ///   named kind is provably absent rather than merely differently
    ///   used" posture, since `"instance"` is not literally absent the
    ///   way e.g. [`Self::purescript`]'s own `module_types` correction
    ///   required).
    /// - `branch_types` is `["exp_if", "exp_case", "exp_do"]`, matching
    ///   the baseline's `purescript_branch_types` exactly -- all three
    ///   confirmed present via the dump with real fields
    ///   (`exp_if`'s `"if"`/`"then"`/`"else"`, `exp_case`'s
    ///   `"condition"`).
    /// - `decorator_types` is empty, matching the baseline's own
    ///   `empty_types` for this field on `CBM_LANG_PURESCRIPT`.
    pub const fn purescript() -> Self {
        Self {
            name: "purescript",
            func_types: &["function"],
            method_types: &[],
            class_types: &["class_declaration", "data", "newtype", "type_alias"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["exp_apply"],
            call_function_field: "UNUSED_SEE_PURESCRIPT_CALL_OVERRIDE",
            call_arguments_field: "UNUSED_SEE_PURESCRIPT_CALL_OVERRIDE",
            import_types: &["import", "import_item", "instance"],
            branch_types: &["exp_if", "exp_case", "exp_do"],
            decorator_types: &[],
            // Never actually consulted: `function`/`class_declaration`
            // are fully claimed by `generic::purescript_quirk` (see this
            // const's own doc comment on `func_types`/`class_types`).
            name_field: "UNUSED_SEE_PURESCRIPT_QUIRK",
            body_field: "UNUSED_SEE_PURESCRIPT_QUIRK",
        }
    }

    /// Magma (computer algebra system scripting language; `.mag`/
    /// `.magma`). Language-parity wave G2.3d. Grammar: `tree-sitter-magma`
    /// (`edgarcosta/tree-sitter-magma` -- no competing/alternate
    /// `tree-sitter-magma` repository was found via a GitHub repository
    /// search for the computer-algebra language; this is the sole
    /// candidate). Not a crates.io dependency (crates.io has no
    /// `tree-sitter-magma` at all -- confirmed via a direct crates.io API
    /// search returning zero results for the computer-algebra language,
    /// only unrelated GOST-cipher/game-engine/CLI-tool crates that happen
    /// to also be named "magma") -- bound via a `git` dependency pinned to
    /// a specific commit instead (same "option 1, a maintained crate
    /// exists" sourcing as this wave's own [`Self::purescript`]). Verified
    /// via a real parse-tree dump against this exact pinned commit, zero
    /// parse errors on a fixture exercising every one of this row's own
    /// arrays.
    /// - `func_types` is `["function_definition", "procedure_definition",
    ///   "intrinsic_definition"]`, NOT the baseline's own
    ///   `magma_func_types = {"function_definition", "procedure_definition",
    ///   "intrinsic_definition", "anonymous_function"}` -- `anonymous_function`
    ///   is CONFIRMED ABSENT from this grammar's full 95-node-kind
    ///   vocabulary (verified via an exhaustive listing, not a targeted
    ///   probe), dropped rather than ported. The remaining three are all
    ///   real, well-fielded (`"name"`/`"body"`/`"parameters"`, matching
    ///   this file's own generic defaults exactly -- one of the few
    ///   grammars in this whole exotic-tier batch that needs NO
    ///   declarator-unwrapping or positional-child quirk for its own
    ///   func-shape naming at all).
    /// - `class_types`/`field_types` are BOTH empty, matching the
    ///   baseline's own `empty_types` for `CBM_LANG_MAGMA` on both fields
    ///   -- but this is a real, verified MISSED-OPPORTUNITY in the
    ///   baseline itself, not a grammar limitation: this grammar has a
    ///   genuine `type_declaration` node kind (Magma's `type Foo;` forward
    ///   type-name declaration) with a `"supertype"` field, AND a
    ///   dedicated `field_definition` node kind (used inside a
    ///   `recformat<name: Type, ...>` record-format literal, e.g. `Dog :=
    ///   recformat<name: MonStgElt, breed: MonStgElt>;`) with real
    ///   `"name"`/`"type"` fields, confirmed via the dump -- BOTH left
    ///   unclaimed here anyway, matching the baseline's real (if
    ///   incomplete) depth rather than unilaterally improving on it per
    ///   this campaign's own "baseline's real depth, not an idealized
    ///   one" instruction; a `recformat` literal is in any case a VALUE
    ///   expression assigned to a plain identifier (`Dog := recformat<...>;`,
    ///   an `assignment`, not a declaration this file's own class-shape
    ///   branch would ever reach), so this is a smaller gap than it first
    ///   appears.
    /// - `interface_types`/`enum_types`/`alias_types`/`decorator_types`
    ///   are all empty, matching the baseline's own `empty_types` for
    ///   every one of these fields on `CBM_LANG_MAGMA` (Magma's scripting
    ///   language has no such declaration syntax at all).
    /// - `module_types` is `["program"]`, NOT the baseline's own
    ///   `magma_module_types = {"source_file"}` -- `source_file` is
    ///   CONFIRMED ABSENT from this grammar's real node-kind vocabulary;
    ///   its actual (and ONLY) root node kind is `program`, confirmed both
    ///   in the full node-kind listing and directly as the dump's own
    ///   outermost node.
    /// - `call_types` is `["call"]`, NOT the baseline's own
    ///   `magma_call_types = {"call_expression"}` -- `call_expression` is
    ///   CONFIRMED ABSENT from this grammar entirely; the real (and only)
    ///   call-shaped node kind is the short, unqualified `call`, with real
    ///   `"function"`/`"arguments"` fields matching this file's own
    ///   generic defaults exactly (confirmed via the dump: `add(1, 2)`
    ///   parses as `call { field:function -> identifier "add",
    ///   field:arguments -> argument_list { field:argument -> integer "1",
    ///   field:argument -> integer "2" } }`) -- a real, load-bearing
    ///   correction: without it, this row would extract ZERO call edges
    ///   from any Magma source at all, defeating the Tier-2
    ///   defs+calls+imports promise entirely.
    /// - `import_types` is `["import_directive", "load_directive",
    ///   "require_statement"]`, NOT the baseline's own
    ///   `magma_import_types = {"load_statement", "require",
    ///   "require_statement"}` -- `load_statement` and the bare `require`
    ///   are BOTH confirmed absent from this grammar's real vocabulary
    ///   (only `require_statement` of the baseline's three actually
    ///   exists); the real load-a-file directive is spelled
    ///   `load_directive` (fieldless: `load "setup.m";` -- its path is a
    ///   bare `string` child, read via [`generic::magma_quirk`] since this
    ///   file's generic import-edge mechanism has no field to key off
    ///   either), and this grammar additionally has a THIRD, entirely
    ///   baseline-unlisted import-shaped construct,
    ///   `import_directive` (`import "foo.m": bar;` -- Magma's own
    ///   variable-scoped file-import syntax), with real `"filename"`/
    ///   `"variable"` fields -- confirmed via the dump, and included here
    ///   as a genuine improvement over the baseline's own incomplete
    ///   array (matching this campaign's precedent of closing verified
    ///   real gaps, e.g. [`Self::pascal`]'s parenless-call-statement
    ///   finding) rather than reproducing the baseline's own blind spot.
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "while_statement", "repeat_statement", "case_statement"]`,
    ///   matching the baseline's own `magma_branch_types` exactly -- every
    ///   one of these five confirmed present via the dump, each with real
    ///   (if occasionally unusually-named) fields (`if_statement`'s
    ///   `"condition"`/`"consequence"`/`"default"`/`"elif"`,
    ///   `case_statement`'s `"matchee"`/`"else"`).
    pub const fn magma() -> Self {
        Self {
            name: "magma",
            func_types: &[
                "function_definition",
                "procedure_definition",
                "intrinsic_definition",
            ],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_directive", "load_directive", "require_statement"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "repeat_statement",
                "case_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Hare (systems language, `.ha`). Language-parity wave G2.3d.
    /// Grammar: `tree-sitter-hare` (originally `~ghishadow/tree-sitter-hare`
    /// on sourcehut, now archived with its own description pointing at
    /// that sourcehut location; sourced here from its own upstream fork
    /// origin, `chenkeyv/tree-sitter-hare`, which still carries the
    /// IDENTICAL grammar/parser sources -- exact copyright-line match
    /// "Grenier Célestin" against this crate's own vendored
    /// `internal/cbm/vendored/grammars/hare/LICENSE`, confirming the
    /// identical grammar lineage the baseline itself vendored, and an
    /// exhaustive node-kind-occurrence-count cross-check against that same
    /// vendored `parser.c` -- `function_declaration`/`type_declaration`/
    /// `use_statement`/`switch_expression`/`match_expression` combined
    /// occur exactly as many times in the vendored copy as in this
    /// sourced copy's own generated `parser.c`). VENDORED (no `scanner.c`
    /// for this grammar) -- not a crates.io dependency: the only
    /// crates.io-published version (0.20.7) pins `tree-sitter = "^0.20.6"`
    /// as a genuine RUNTIME (non-dev) dependency, the same "unavailable/
    /// broken ABI" condition [`Self::ispc`]'s own doc comment describes,
    /// and no `tree-sitter-language`-shimmed successor exists to prefer
    /// either as an upgrade or as a pinned git dependency. Verified via a
    /// real parse-tree dump against the vendored `parser.c` directly, zero
    /// parse errors.
    /// - `func_types` is `["function_declaration"]`, matching the
    ///   baseline's own `hare_func_types` exactly -- confirmed present via
    ///   the dump with real, working `"name"`/`"returns"`/`"body"` fields
    ///   (`"name"`/`"body"` matching this file's own generic defaults
    ///   exactly; `"returns"` is this grammar's own name for a return-type
    ///   annotation, not separately consulted by this row).
    /// - `class_types` is `["type_declaration"]`, matching the baseline's
    ///   own `hare_class_types` exactly -- confirmed present via the dump,
    ///   but with NO fields of its own at all
    ///   (`{"type":"type_declaration","fields":{}}`): its own name is a
    ///   plain POSITIONAL `identifier` child immediately after the `type`
    ///   keyword and before `=` (`type animal = struct {...};` parses as
    ///   `type_declaration { "type", identifier "animal", "=",
    ///   struct_type {...} }`), needing
    ///   [`generic::hare_child_by_kind`]-based positional lookup (same
    ///   mechanism as [`Self::d`]'s own identical "no fields anywhere"
    ///   posture) via [`generic::hare_quirk`] rather than this file's
    ///   generic `name_field` mechanism. This crate records EVERY
    ///   `type_declaration` as a plain [`crate::parsers::SymbolKind::Class`]
    ///   regardless of whether its own `=`-bound type expression is a
    ///   `struct_type`/`union_type`/a plain alias to a builtin type --
    ///   matching the baseline's own single flat `hare_class_types` entry,
    ///   which draws no such distinction either (Hare's `type` keyword is
    ///   genuinely overloaded across struct/union/enum/plain-alias
    ///   declarations at the syntax level, and the baseline itself makes
    ///   no attempt to disambiguate them for this language).
    /// - `interface_types`/`enum_types`/`alias_types`/`field_types`/
    ///   `decorator_types` are all empty, matching the baseline's own
    ///   `empty_types` for every one of these fields on `CBM_LANG_HARE`.
    /// - `module_types` is empty, NOT the baseline's own
    ///   `hare_module_types = {"source_file"}` -- `source_file` is
    ///   CONFIRMED ABSENT from this grammar; its real (and only) root node
    ///   kind is `module`, confirmed both in the dump (the outermost node
    ///   wraps an `imports` child plus a `declarations` child) and by
    ///   inspecting the vendored `parser.h`'s own root-symbol definition.
    ///   Left empty rather than corrected to `["module"]`: this crate's
    ///   own generic module-handling branch
    ///   ([`generic::walk`]'s `spec.module_types.contains(&kind)` check)
    ///   would then push a Module symbol NAMED after the module node's own
    ///   first named child, which for Hare's real `module` root is its
    ///   `imports` child -- a spurious, meaningless "Module symbol named
    ///   `imports`" on every single Hare file, which is wrong in a way
    ///   this crate's walk would actually produce (same
    ///   "provably-would-misfire-if-ported" reasoning
    ///   [`Self::purescript`]'s own `module_types` doc comment gives for
    ///   its own analogous correction) rather than a genuinely useful
    ///   Module symbol; Hare source files have no separate, named
    ///   `module foo;`-style declaration clause of their own for this row
    ///   to extract in the first place (Hare's own module identity is
    ///   purely directory-path-derived at the language level, invisible
    ///   to a single file's own AST).
    /// - `call_types` is `["call_expression"]`, matching the baseline's
    ///   own `hare_call_types` exactly -- confirmed present via the dump
    ///   with a real, working field, but that field is named `"callee"`
    ///   (NOT `"function"`, this file's usual default) --
    ///   `call_function_field` is set to `"callee"` here accordingly (a
    ///   real field name, just a different one than most other rows use).
    ///   The argument list is NOT a separate named field at all -- the
    ///   parenthesized argument expressions are plain positional siblings
    ///   of the callee inside the SAME `call_expression` node (confirmed:
    ///   `add(1, 2)` parses as `call_expression { field:callee ->
    ///   identifier "add", "(", number "1", ",", number "2", ")" }`, no
    ///   wrapping arguments-list node at all) -- `call_arguments_field` is
    ///   consequently a placeholder never actually consulted;
    ///   [`generic::hare_call_override`] reconstructs `arg_texts` by
    ///   scanning `call_expression`'s own remaining named children instead
    ///   (skipping the callee itself), the same "no wrapping node, scan
    ///   siblings directly" shape [`generic::pony_call_override`] (this
    ///   same wave) and [`Self::d`]'s own `d_call_override` both need for
    ///   an analogous reason.
    /// - `import_types` is `["use_statement"]`, matching the baseline's
    ///   own `hare_import_types` exactly -- confirmed present via the dump
    ///   with a real, working POSITIONAL `identifier` child (no field of
    ///   its own) holding the imported module path (`use fmt;` parses as
    ///   `use_statement { "use", identifier "fmt", ";" }`) -- read via
    ///   [`generic::hare_quirk`].
    /// - `branch_types` is `["if_statement", "for_statement",
    ///   "switch_expression", "match_expression"]`, matching the
    ///   baseline's own `hare_branch_types` exactly -- every one of these
    ///   four confirmed present via the dump (`if_statement`/
    ///   `for_statement` with real `"condition"`/`"consequence"`/
    ///   `"body"`/`"afterthought"` fields; `switch_expression`/
    ///   `match_expression` not directly exercised in this row's own
    ///   fixture but confirmed present in the vendored grammar's full
    ///   node-kind vocabulary).
    pub const fn hare() -> Self {
        Self {
            name: "hare",
            func_types: &["function_declaration"],
            method_types: &[],
            class_types: &["type_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "callee",
            call_arguments_field: "UNUSED_SEE_HARE_CALL_OVERRIDE",
            import_types: &["use_statement"],
            branch_types: &[
                "if_statement",
                "for_statement",
                "switch_expression",
                "match_expression",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Pony (actor-model systems language, `.pony`). Language-parity wave
    /// G2.3d. Grammar: `tree-sitter-pony` (`amaanq/tree-sitter-pony` --
    /// exact copyright-line match "Amaan Qureshi"/"Matthias Wahl" against
    /// this crate's own vendored `internal/cbm/vendored/grammars/pony/LICENSE`,
    /// confirming the identical grammar lineage the baseline itself
    /// vendored). VENDORED (WITH `scanner.c` -- this is the one language
    /// in this wave's own vendored set that has an external hand-written
    /// scanner) -- not a crates.io dependency: the only published version
    /// (1.0.0) pins `tree-sitter = "~0.20.10"` as a genuine RUNTIME
    /// (non-dev) dependency, and the live upstream repository's own
    /// `bindings/rust/lib.rs` (confirmed by reading it directly off the
    /// `master` branch, not merely inferred from the crate metadata)
    /// STILL uses the old raw `extern "C" fn tree_sitter_pony() ->
    /// Language` binding shape too -- the same "unavailable/broken ABI,
    /// no newer safely-shimmed version exists either as an upgrade or as
    /// a git dependency" condition [`Self::ispc`]/[`Self::hare`]'s own doc
    /// comments describe. Verified via a real parse-tree dump against the
    /// vendored `parser.c`+`scanner.c` directly, zero parse errors.
    /// - `func_types` is `["method", "constructor", "ffi_method",
    ///   "lambda_expression"]`, matching the baseline's own
    ///   `pony_func_types` exactly. `method`/`constructor` are BOTH
    ///   confirmed present via the dump but with NO `"name"` field of
    ///   their own at all (`fun bark(): String => "..."` parses as
    ///   `method { "fun", identifier "bark", parameters {...},
    ///   field:returns -> base_type {...}, "=>", block {...} }` -- the
    ///   name is a plain POSITIONAL `identifier` immediately after the
    ///   `fun`/`new` keyword) -- handled via
    ///   [`generic::pony_child_by_kind`]-based positional lookup through
    ///   [`generic::pony_quirk`], the same "no fields at all for this
    ///   shape" posture [`Self::d`]'s own `d_function_like` already
    ///   established for an analogous systems language.
    ///   `ffi_method`/`lambda_expression` were not directly exercised in
    ///   this row's own fixture (Pony's C-FFI-declaration and inline-
    ///   lambda syntax are both real but less common constructs) but are
    ///   confirmed present in the vendored grammar's own full node-kind
    ///   vocabulary, handled by the same positional-lookup quirk arm.
    /// - `method_types` is empty: unlike most other languages in this
    ///   file, Pony draws NO node-kind-level distinction between a
    ///   free-standing function and a class/actor/primitive-scoped method
    ///   at all -- `method` is the ONE node kind for both, and this row's
    ///   own [`generic::pony_quirk`] always classifies it as
    ///   [`crate::parsers::SymbolKind::Method`] regardless of lexical
    ///   nesting (matching this grammar's own genuine actor-model
    ///   convention, where EVERY behavior/function lives inside some
    ///   class/actor/primitive/trait/interface body -- Pony has no
    ///   top-level free-function syntax at all, confirmed by this
    ///   grammar's own top-level `members`-only structure), so there is no
    ///   free-function-vs-method AMBIGUITY for `func_types`/`method_types`
    ///   overlap to resolve the way e.g. Rust's shared `function_item`
    ///   kind needs.
    /// - `class_types` is `["actor_definition", "class_definition",
    ///   "struct_definition", "trait_definition", "interface_definition",
    ///   "primitive_definition", "type_alias"]`, matching the baseline's
    ///   own `pony_class_types` exactly. Every one of these SEVEN kinds
    ///   has the SAME "no fields, positional `identifier` name
    ///   immediately after the keyword" shape confirmed for `method`/
    ///   `constructor` above (`class Animal` / `actor Dog is Animal` /
    ///   `primitive Helpers` all confirmed via the dump) -- ALSO handled
    ///   by [`generic::pony_quirk`]'s positional lookup rather than this
    ///   file's generic `name_field` mechanism, additionally recording an
    ///   INHERITS edge for `actor_definition`'s own `is`-clause heritage
    ///   (`actor Dog is Animal` -- confirmed via the dump: a `base_type`
    ///   FIELD-named `"name"` -- note, THIS specific field IS real and
    ///   working, unlike the surrounding node's own top-level name --
    ///   sibling of the actor's own positional name identifier) as a
    ///   genuine improvement mirroring [`Self::odin`]'s own `using`-based
    ///   INHERITS precedent (the baseline itself has no dedicated Pony
    ///   `extract_base_classes` walker at all, matching every other
    ///   language in this exotic-tier batch that lacks one).
    /// - `interface_types`/`enum_types`/`alias_types`/`field_types` are
    ///   all empty: `interface_definition`/`type_alias` are both folded
    ///   into `class_types` here instead, matching the baseline's own
    ///   identical choice verbatim (`pony_class_types` is the ONE array
    ///   the baseline itself lists BOTH of these node kinds in; it has no
    ///   separate interface/alias/field array of its own for this
    ///   language at all). A class/actor/primitive body's own `field`
    ///   member node (`var name: String` -- confirmed present via the
    ///   dump, WITH a real, working `"name"` field this time) is
    ///   similarly left unclaimed by a dedicated `field_types` entry,
    ///   matching the baseline's own `empty_types` choice for this field
    ///   on `CBM_LANG_PONY` exactly, even though (unlike the
    ///   node-kind-name-level gaps corrected elsewhere in this file) this
    ///   grammar's `field` node would in fact work cleanly through this
    ///   file's OWN generic field-DEFINES mechanism if listed -- left out
    ///   anyway per this campaign's "match the baseline's real depth, not
    ///   an idealized one" instruction, since the baseline draws no
    ///   distinction here either.
    /// - `module_types` is `["source_file"]`, matching the baseline's own
    ///   `pony_module_types` exactly and confirmed as this grammar's real
    ///   root node kind via the dump.
    /// - `call_types` is `["call_expression"]`, matching the baseline's
    ///   own `pony_call_types` exactly -- confirmed present via the dump
    ///   with a real, working `"callee"` field (the SAME field name
    ///   [`Self::hare`]'s own `call_expression` uses, an independent
    ///   coincidence of grammar-authoring convention rather than any
    ///   shared lineage between the two grammars) -- but its own argument
    ///   list is a SEPARATE, unfielded `arguments` sibling node (NOT a
    ///   field of `call_expression` itself) whose OWN children are in
    ///   turn each tagged with a field name literally called
    ///   `"positional"` (confirmed via the dump: `add(1, 2)` parses as
    ///   `call_expression { field:callee -> identifier "add", arguments {
    ///   "(", field:positional -> number "1", ",", field:positional ->
    ///   number "2", ")" } }`) -- `call_function_field` is set to
    ///   `"callee"` accordingly, and
    ///   [`generic::pony_call_override`] reconstructs `arg_texts` from the
    ///   `arguments` sibling's own `"positional"`-tagged children directly
    ///   (this file's usual single-field `call_arguments_field`
    ///   mechanism has no field on `call_expression` itself to key off,
    ///   the same "no field on the call node, must locate the arguments
    ///   holder by kind first" shape [`generic::zig_builtin_arg_texts`]
    ///   already established elsewhere in this file).
    /// - `import_types` is `["use_statement"]`, matching the baseline's
    ///   own `pony_import_types` exactly -- confirmed present via the
    ///   dump, but its own imported-path child is a STRING LITERAL (`use
    ///   "collections"`), not a bare identifier the way [`Self::hare`]'s
    ///   OWN `use_statement` uses -- read via [`generic::pony_quirk`],
    ///   which strips the surrounding quote characters from the string
    ///   node's own text (matching every other language row's identical
    ///   "record the path text without its own literal quote/angle-
    ///   bracket delimiters" convention, e.g. [`generic::c_include_path`]).
    /// - `branch_types` is `["if_statement", "match_statement",
    ///   "for_statement", "while_statement", "repeat_statement",
    ///   "try_statement"]`, matching the baseline's own `pony_branch_types`
    ///   exactly -- every one of these six confirmed present in the
    ///   vendored grammar's own full node-kind vocabulary (`if_statement`
    ///   directly exercised in this row's own fixture; the remaining five
    ///   confirmed present by name but not individually exercised, this
    ///   array being documentation-only for [`crate::complexity::NodeKindTable`]
    ///   consumption per this whole wave's deferred-complexity-extraction
    ///   posture, matching G2.1's own convention).
    /// - `decorator_types` is empty, matching the baseline's own
    ///   `empty_types` for this field on `CBM_LANG_PONY` (Pony has no
    ///   decorator/attribute-macro syntax the baseline itself records for
    ///   this language).
    pub const fn pony() -> Self {
        Self {
            name: "pony",
            func_types: &["method", "constructor", "ffi_method", "lambda_expression"],
            method_types: &[],
            class_types: &[
                "actor_definition",
                "class_definition",
                "struct_definition",
                "trait_definition",
                "interface_definition",
                "primitive_definition",
                "type_alias",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["call_expression"],
            call_function_field: "callee",
            call_arguments_field: "UNUSED_SEE_PONY_CALL_OVERRIDE",
            import_types: &["use_statement"],
            branch_types: &[
                "if_statement",
                "match_statement",
                "for_statement",
                "while_statement",
                "repeat_statement",
                "try_statement",
            ],
            decorator_types: &[],
            // Never actually consulted: every one of `func_types`/
            // `class_types` is fully claimed by `generic::pony_quirk` (see
            // this const's own doc comment).
            name_field: "UNUSED_SEE_PONY_QUIRK",
            body_field: "UNUSED_SEE_PONY_QUIRK",
        }
    }

    /// NASM (Netwide Assembler, x86 assembly; `.nasm`). Language-parity
    /// wave G2.3d. Grammar: `tree-sitter-nasm` (`naclsn/tree-sitter-nasm`
    /// -- exact copyright-line match "Grenier Célestin" against this
    /// crate's own vendored `internal/cbm/vendored/grammars/nasm/LICENSE`
    /// -- the SAME author as this wave's own [`Self::hare`] grammar, an
    /// independently-confirmed coincidence of one grammar author having
    /// written both, not a shared codebase between the two languages).
    /// Distinct from [`crate::parsers::Language`]'s generic, Tier-0
    /// `.s`/`.S` assembly path (`CBM_LANG_ASSEMBLY` in the baseline, out
    /// of this wave's own scope) -- the baseline registers NO
    /// `CBM_LANG_ASM` at all (confirmed via a direct search of the
    /// baseline's own `cbm.h`/`lang_specs.c`/`language.c`: only
    /// `CBM_LANG_ASSEMBLY` and `CBM_LANG_NASM` exist, no third generic-ASM
    /// language), and the baseline's own `EXT_TABLE` maps `.nasm` to
    /// `CBM_LANG_NASM` specifically, never to the generic Assembly
    /// language. VENDORED (no `scanner.c` for this grammar) -- not a
    /// crates.io dependency: this exact grammar/repository was never
    /// published to crates.io at all (confirmed via a direct crates.io API
    /// search for "tree-sitter-nasm" returning zero results, despite
    /// several DIFFERENT tree-sitter-nasm grammars existing on GitHub
    /// under other authors/orgs -- `naclsn`'s is the one matching the
    /// baseline's own vendored copy by exact license-copyright fingerprint
    /// and by an exhaustive field-by-field cross-check of its own
    /// `node-types.json` against every one of this row's own arrays,
    /// confirming it is the identical upstream the baseline itself
    /// vendored, not merely a plausible substitute); its own
    /// `bindings/rust/lib.rs` uses the old raw `extern "C" fn
    /// tree_sitter_nasm() -> Language` binding shape, the same
    /// "unavailable/broken ABI" condition every other vendored grammar in
    /// this wave shares. Verified via a real parse-tree dump against the
    /// vendored `parser.c` directly, zero parse errors.
    /// - `func_types` is `["label", "preproc_def",
    ///   "preproc_multiline_macro"]`, matching the baseline's own
    ///   `nasm_func_types` exactly -- NASM has no function syntax in the
    ///   ordinary sense; the baseline's own choice to model an assembly
    ///   LABEL (`print_msg:`) as a "function" is preserved here verbatim,
    ///   matching this campaign's "reproduce the baseline's real modeling
    ///   choices" posture even where they read unusually for the target
    ///   domain. `label` has a real, working `"name"` field (confirmed via
    ///   the dump: `print_msg:` parses as `label { field:name -> word
    ///   "print_msg", ":" }`, matching this file's own generic default
    ///   exactly); `preproc_def`/`preproc_multiline_macro` both have real
    ///   `"name"` fields too (`%define BUFSIZE 128` parses as
    ///   `preproc_def { field:name -> word "BUFSIZE", field:value ->
    ///   preproc_arg "128" }`), but NEITHER has a `"body"` field this
    ///   file's generic `body_field` mechanism could recurse into for a
    ///   preprocessor macro's own definition text (there is no nested
    ///   block-shaped construct to recurse into in the first place -- a
    ///   `%define`'s value is a flat token/expression, not a scope
    ///   containing further nested defs/calls) -- left unclaimed by a
    ///   dedicated quirk since there is genuinely nothing further for one
    ///   to walk into for these two kinds specifically (only `label`
    ///   needs no quirk at all: it has no body concept either, being a
    ///   pure jump-target marker, so the generic engine's own
    ///   `node.child_by_field_name(spec.body_field)` simply returning
    ///   `None` for it is already the correct, harmless behavior).
    /// - `class_types` is `["struc_declaration"]`, matching the baseline's
    ///   own `nasm_class_types` exactly -- confirmed present in the
    ///   vendored grammar's own full node-kind vocabulary (NASM's `struc`/
    ///   `endstruc` directive pair for defining a memory-layout structure)
    ///   though not directly exercised in this row's own fixture.
    /// - `interface_types`/`enum_types`/`alias_types`/`field_types`/
    ///   `decorator_types` are all empty, matching the baseline's own
    ///   `empty_types` for every one of these fields on `CBM_LANG_NASM`.
    /// - `module_types` is empty, NOT the baseline's own
    ///   `nasm_module_types = {"source_file"}` -- `source_file` genuinely
    ///   IS this grammar's real root node kind (confirmed via the dump),
    ///   but porting the baseline's array verbatim here would be actively
    ///   HARMFUL rather than merely redundant: this file's own
    ///   `spec.module_types.contains(&kind)` check runs UNCONDITIONALLY at
    ///   the very top of `generic::walk`, before ANY quirk hook (including
    ///   [`generic::nasm_quirk`]'s own dedicated `"source_file"` handling)
    ///   ever gets a chance to run -- a real, load-bearing hard-test
    ///   failure caught this directly: with `["source_file"]` still set,
    ///   this crate pushed a SPURIOUS Module symbol on every NASM file,
    ///   named after `source_file`'s own first named child (confirmed to
    ///   be the file's FIRST LABEL's own text, e.g. `"_start:"` complete
    ///   with its own trailing colon -- `first_named_child_text`'s generic
    ///   fallback has no NASM-specific knowledge that a label is not a
    ///   module name). The SAME real bug additionally explains why
    ///   `label`/`instruction` need [`generic::nasm_quirk`]'s own dedicated
    ///   top-level scan at all in the first place: they are plain FLAT
    ///   SIBLING `source_line` children of `source_file`, not nested
    ///   inside each other, a shape this file's own ordinary lexical-
    ///   nesting-based `FnScope` threading (every OTHER language in this
    ///   file relies on it) has no tree structure to walk INTO to attribute
    ///   a following instruction to its own preceding label -- see
    ///   [`generic::nasm_walk_source_file`]'s own doc comment for the
    ///   "most recently seen label becomes the ambient scope for following
    ///   siblings" mechanism this required, now reachable at all only
    ///   because this array is empty rather than short-circuited away.
    /// - `call_types` is `["call_syntax_expression", "actual_instruction"]`,
    ///   matching the baseline's own `nasm_call_types` exactly.
    ///   `actual_instruction` is confirmed present via the dump with real,
    ///   working `"instruction"`/`"operands"` fields (`call print_msg`
    ///   parses as `instruction { actual_instruction { field:instruction
    ///   -> word "call", field:operands -> operands { operand { word
    ///   "print_msg" } } } }`) -- the baseline's own choice to record
    ///   EVERY assembly instruction (not just literal `call`/`jmp`-family
    ///   ones) as a CALLS edge is preserved here verbatim (matching this
    ///   campaign's "reproduce the baseline's real modeling choices"
    ///   posture again), with `call_function_field` set to `"instruction"`
    ///   and `call_arguments_field` set to `"operands"` -- both REAL field
    ///   names, just not this file's usual `"function"`/`"arguments"`
    ///   defaults. `call_syntax_expression` (a value-position invocation,
    ///   e.g. inside a `%define` macro body or an expression context) is
    ///   confirmed present in the grammar's own vocabulary though not
    ///   directly exercised in this row's own fixture.
    /// - `import_types` is `["preproc_include"]`, matching the baseline's
    ///   own `nasm_import_types` exactly -- confirmed present via the dump
    ///   with a real, working `"path"` field (`%include "common.inc"`
    ///   parses as `preproc_include { field:path -> string_literal
    ///   "\"common.inc\"" }`) -- but this file's generic import-handling
    ///   branch (`spec.import_types.contains(&kind)`) does not itself read
    ///   ANY field automatically (see [`generic::walk`]'s own module doc:
    ///   "generic import handling is intentionally minimal ... callers
    ///   needing generic import edges supply an `on_unmatched_node`
    ///   quirk"), so [`generic::nasm_quirk`] reads this `"path"` field
    ///   directly and strips its own surrounding quote characters,
    ///   matching every other language row's identical convention.
    /// - `branch_types` is empty, NOT the baseline's own
    ///   `nasm_branch_types` (the baseline in fact has NO
    ///   `nasm_branch_types` array declared at all for this language --
    ///   confirmed absent from `lang_specs.c`'s own NASM row, which lists
    ///   `empty_types` for this field, matching this file's own choice
    ///   here exactly -- assembly's own conditional-jump instructions
    ///   (`je`/`jne`/...) are ordinary `actual_instruction` nodes
    ///   syntactically indistinguishable at the node-KIND level from any
    ///   other instruction, so there is no dedicated branch-shaped node
    ///   kind for either this row or the baseline's own array to name in
    ///   the first place).
    pub const fn nasm() -> Self {
        Self {
            name: "nasm",
            func_types: &["label", "preproc_def", "preproc_multiline_macro"],
            method_types: &[],
            class_types: &["struc_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            // Empty, NOT `["source_file"]` -- see this const's own doc
            // comment on `module_types`: the generic engine's own
            // `spec.module_types.contains(&kind)` check runs BEFORE
            // `nasm_quirk`'s own `"source_file"` arm would ever be
            // reached at all (that check is unconditional at the very
            // top of `generic::walk`, well before this file's usual
            // quirk-hook catch-all), so listing `"source_file"` here
            // would push a SPURIOUS Module symbol (named after
            // `source_file`'s own first named child -- confirmed via a
            // real hard-test failure to be the FIRST LABEL's own text,
            // e.g. `"_start:"` complete with its own trailing colon) on
            // every single NASM file, one this crate's own tests caught
            // directly, before `nasm_quirk`'s dedicated top-level scan
            // ([`generic::nasm_walk_source_file`]) ever got a chance to
            // run at all.
            module_types: &[],
            call_types: &["call_syntax_expression", "actual_instruction"],
            call_function_field: "instruction",
            call_arguments_field: "operands",
            import_types: &["preproc_include"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Cap'n Proto (`.capnp`). Language-parity wave G2.4c/orchestrator
    /// completion pass. Grammar: `tree-sitter-capnp` 1.5.0
    /// (`amaanq/tree-sitter-capnp`), VENDORED (see
    /// `vendor/tree-sitter-capnp-local/src/lib.rs` for why: the
    /// published crate's own `cc = "~1.0"` build-dependency pin
    /// conflicts with this workspace's `tree-sitter = "0.25"` core).
    ///
    /// Every named node kind in this grammar exposes ZERO fields at
    /// all -- confirmed via a real parse-tree dump of a fixture
    /// exercising every construct (struct/interface/enum/const/field/
    /// method/nested-struct/using-import), not `node-types.json`
    /// alone. All eight arrays below are therefore empty and
    /// EVERY construct is instead recognized and pushed by
    /// [`crate::languages::generic::capnp_quirk`] via
    /// [`crate::languages::generic::on_unmatched_node`], scanning each
    /// node's children by KIND (never by field) for the specific named
    /// identifier child each construct exposes:
    /// - `struct`/`interface` -> a `type_identifier` child (Class/
    ///   Interface symbol; DEFINES edge to the enclosing container if
    ///   nested).
    /// - `enum` -> an `enum_identifier` child (Enum symbol).
    /// - `const` -> a `const_identifier` child (Constant symbol).
    /// - `field` -> a `field_identifier` child (DEFINES edge only, no
    ///   symbol of its own -- matches the baseline's own choice to
    ///   record struct fields as DEFINES members, not standalone
    ///   symbols); a `field`'s own children are still recursed into
    ///   (unchanged `enclosing`) since a nested-in-place type
    ///   definition parses as `field > nested_struct > struct`, not as
    ///   a direct child of the outer `struct` -- confirmed via the
    ///   same parse-tree dump and guarded by a dedicated hard test.
    /// - `method` -> a `method_identifier` child (Method symbol;
    ///   DEFINES edge to the enclosing `interface`).
    /// - `using_directive`/`import_using` -> the quoted string inside
    ///   the descendant `import_path` -> `string_fragment` node
    ///   (ImportRef; Cap'n Proto's `using X = import "path.capnp";`
    ///   has no bare `import`-only form to also handle).
    ///
    /// Cap'n Proto has no call-expression concept at all (it is a pure
    /// schema/IDL language, never executable), so `call_types` and
    /// both call-field names are meaningless here and left at their
    /// placeholder defaults, matching this file's own established
    /// convention for languages whose full quirk claim makes the
    /// generic engine's field-based fallback logic unreachable (see
    /// `LangSpec::c`/`cpp`/`php`'s own identical doc-comment note).
    pub const fn capnp() -> Self {
        Self {
            name: "capnp",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Emacs Lisp (`.el`). Language-parity wave G2.4e/orchestrator
    /// completion pass. Grammar: `tree-sitter-elisp` 1.6.1
    /// (`Wilfred/tree-sitter-elisp`, real crates.io crate, the modern
    /// `tree-sitter-language` shim -- no vendoring needed).
    ///
    /// `function_definition`/`macro_definition` DO have a real `name`
    /// field (confirmed via a real parse-tree dump) -- so `func_types`
    /// covers both, using the shared `name_field: "name"` default.
    /// Unlike most Lisp-family grammars in this crate, this one has NO
    /// `body` field on either node at all (confirmed absent from both
    /// grammar's own `node-types.json` fields AND a real parse tree --
    /// a `function_definition`'s call-bearing content is a bare
    /// sequence of sibling `list`/atom nodes following its own
    /// `parameters` child, not a single wrapped body node the generic
    /// engine's own `body_field`-driven recursion could find), so
    /// [`crate::languages::generic::emacslisp_on_method_defined`]
    /// closes that gap directly: it re-walks every sibling AFTER the
    /// `parameters` node itself with `FnScope` set to this def's own
    /// name, skipping the `parameters` node itself by identity so its
    /// own parameter-name symbols (`(a b)`) are never misread as a
    /// call to a function literally named `a`.
    ///
    /// Every `list` node (a parenthesized form -- Lisp's ONLY call
    /// shape) is fieldless, so `call_types` is empty here too and
    /// [`crate::languages::generic::emacslisp_call_override`] resolves
    /// the callee positionally: a `list`'s first child, if a bare
    /// `symbol` (not a special form this row already recognizes via
    /// `func_types`), is the callee; every remaining child is an
    /// argument text.
    pub const fn emacslisp() -> Self {
        Self {
            name: "emacslisp",
            func_types: &["function_definition", "macro_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            // Must be non-empty for `Quirks::call_override` to fire at
            // all -- `generic::walk`'s call-handling branch is gated on
            // `spec.call_types.contains(&kind)` unconditionally, before
            // the override hook is ever consulted (see
            // `emacslisp_call_override`'s own doc comment).
            call_types: &["list"],
            call_function_field: "UNUSED_SEE_EMACSLISP_QUIRK",
            call_arguments_field: "UNUSED_SEE_EMACSLISP_QUIRK",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// AWK: node kinds verified against the `arborium-awk` 2.18.1
    /// crate's own `grammar/src/node-types.json` PLUS a real parse-tree
    /// dump (`function foo(x) { print x; bar(x); } BEGIN { foo(1) }`
    /// parses to `(program (func_def name: (identifier) (param_list ...)
    /// (block ...)) (rule (pattern) (block (func_call ...))))`) -- G2.4a
    /// grammar onboarding. Baseline's own `awk_call_types` lists a
    /// SECOND node kind, `"command"`, that does not exist anywhere in
    /// this grammar's `node-types.json` (confirmed absent) -- omitted
    /// here rather than copied blindly (same class of stale-baseline-
    /// entry finding as [`Self::lua`]/[`Self::bash`]'s own doc comments).
    /// - `func_types`/`name_field`/`body_field` are all empty/
    ///   placeholder: `func_def` has a real `name` field but NO
    ///   `body`-named field at all (its `block` child is purely
    ///   positional, confirmed by both `node-types.json` --
    ///   `"fields": {"name": ...}` only -- and the real parse dump
    ///   above) -- this file's generic func/method branch
    ///   unconditionally `return`s once it resolves a name, whether or
    ///   not `child_by_field_name(body_field)` finds anything, so
    ///   listing `func_def` in `func_types` with a placeholder
    ///   `body_field` would silently DROP every call/nested statement
    ///   inside every AWK function body.
    ///   [`crate::languages::generic::awk_quirk`] claims `func_def`
    ///   entirely instead: reads the real `name` field, then finds the
    ///   `block` child positionally and walks it with a proper
    ///   `FnScope`.
    /// - `call_types` is `["func_call"]` only; fully claimed by
    ///   [`crate::languages::generic::awk_call_override`] since
    ///   `func_call`'s own argument list (`args`) is a positional
    ///   child, not a field this row's flat `call_arguments_field`
    ///   could name.
    /// - `module_types` is `["program"]`, the real (and only) root node
    ///   kind.
    /// - `branch_types` matches baseline's `awk_branch_types` exactly --
    ///   all six (`if_statement`/`for_statement`/`for_in_statement`/
    ///   `while_statement`/`do_while_statement`/`switch_statement`)
    ///   confirmed present in `node-types.json`.
    /// - `import_types` empty, matching baseline (AWK has no import
    ///   construct in the baseline's own row either).
    pub const fn awk() -> Self {
        Self {
            name: "awk",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["func_call"],
            call_function_field: "UNUSED_SEE_AWK_QUIRK",
            call_arguments_field: "UNUSED_SEE_AWK_QUIRK",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "for_statement",
                "for_in_statement",
                "while_statement",
                "do_while_statement",
                "switch_statement",
            ],
            decorator_types: &[],
            name_field: "UNUSED_SEE_AWK_QUIRK",
            body_field: "UNUSED_SEE_AWK_QUIRK",
        }
    }

    /// Fish: node kinds verified against the `tree-sitter-fish` 3.6.0
    /// crate's own `src/node-types.json` plus a real parse-tree dump
    /// (`function foo; echo hi; bar $foo; end` parses to `(program
    /// (function_definition name: (word) (command name: (word)
    /// argument: (word)) (command name: (word) argument:
    /// (variable_expansion ...))))`) -- G2.4a grammar onboarding.
    /// - `func_types`/`name_field`/`body_field` are all empty/
    ///   placeholder: `function_definition` has a real `name` field,
    ///   but its own body statements are positional siblings of that
    ///   field (confirmed above -- no `body`-named field exists at
    ///   all), the identical "generic func/method branch would silently
    ///   drop the whole body" gap [`Self::awk`]'s own doc comment
    ///   explains. [`crate::languages::generic::fish_quirk`] claims
    ///   `function_definition` entirely instead.
    /// - `call_types` is `["command"]`; fully claimed by
    ///   [`crate::languages::generic::fish_call_override`] since a
    ///   `command`'s own `argument` field is `multiple: true`
    ///   (confirmed in `node-types.json`) -- this file's single-field
    ///   `call_arguments_field`/`call_arg_texts` convention can only
    ///   ever read the FIRST such field child, exactly the gap
    ///   [`crate::languages::generic::bash_call_override`]'s own doc
    ///   comment already documents for Bash's identical `command` shape
    ///   -- `children_by_field_name` is required instead.
    /// - `module_types` is `["program"]`, the real root node kind.
    /// - `branch_types` matches baseline's `fish_branch_types` exactly
    ///   -- all four (`if_statement`/`switch_statement`/
    ///   `while_statement`/`for_statement`) confirmed present.
    /// - `import_types` empty, matching baseline (no import construct).
    pub const fn fish() -> Self {
        Self {
            name: "fish",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["command"],
            call_function_field: "UNUSED_SEE_FISH_QUIRK",
            call_arguments_field: "UNUSED_SEE_FISH_QUIRK",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "switch_statement",
                "while_statement",
                "for_statement",
            ],
            decorator_types: &[],
            name_field: "UNUSED_SEE_FISH_QUIRK",
            body_field: "UNUSED_SEE_FISH_QUIRK",
        }
    }

    /// Zsh: node kinds verified against the `tree-sitter-zsh` 0.63.4
    /// crate's own `src/node-types.json` plus a real parse-tree dump
    /// (`function foo() { echo hi; bar $foo }` parses to `(program
    /// (function_definition name: (word) body: (compound_statement
    /// (command name: ... argument: ...) (command name: ... argument:
    /// ...))))`) -- G2.4a grammar onboarding. Unlike Bash/Fish's own
    /// `function_definition`, this grammar (a fork with genuinely richer
    /// fielding) gives it a real `body` field wrapping ONE
    /// `compound_statement` node -- the ordinary generic func/method
    /// branch (`name_field`/`body_field`) handles it correctly
    /// unaided, no quirk needed for definitions at all.
    /// - `call_types` is `["command"]`; fully claimed by
    ///   [`crate::languages::generic::zsh_call_override`] for the exact
    ///   same `argument`-field-is-`multiple: true` reason as
    ///   [`Self::fish`]'s own doc comment (confirmed identical shape in
    ///   `node-types.json`).
    /// - Baseline's own `zsh_call_types` additionally lists
    ///   `"call_expression"`, which does NOT exist anywhere in this
    ///   grammar's `node-types.json` -- omitted as a stale baseline
    ///   entry (same finding as [`Self::awk`]'s own doc comment).
    /// - `module_types` is `["program"]`, the real root node kind.
    /// - `branch_types` matches baseline's `zsh_branch_types` exactly --
    ///   all four (`if_statement`/`case_statement`/`while_statement`/
    ///   `for_statement`) confirmed present.
    /// - `import_types` empty, matching baseline (no import construct).
    pub const fn zsh() -> Self {
        Self {
            name: "zsh",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["command"],
            call_function_field: "UNUSED_SEE_ZSH_QUIRK",
            call_arguments_field: "UNUSED_SEE_ZSH_QUIRK",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "for_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Tcl: node kinds verified against the `bca-tree-sitter-tcl` 2.0.0
    /// crate's own `src/node-types.json` plus a real parse-tree dump
    /// (`proc foo {x} { puts $x; bar $x }` / `namespace eval NS { proc
    /// baz {} { puts hi } }` parses to `(source_file (procedure name:
    /// (simple_word) arguments: (arguments ...) body: (braced_word
    /// (command ...) (command ...))) (namespace (word_list (simple_word)
    /// (simple_word) (braced_word (procedure ...)))))`) -- G2.4a grammar
    /// onboarding.
    /// - `func_types` is `["procedure"]`, used through the ordinary
    ///   generic func/method branch UNAIDED: `procedure`'s own `body`
    ///   field resolves to exactly ONE `braced_word` wrapper node
    ///   (confirmed by the parse dump above), whose own children are the
    ///   individual nested `command`/control-flow statements -- unlike
    ///   [`Self::awk`]/[`Self::fish`]'s positional-only bodies, this
    ///   grammar's single-body-field shape is exactly what
    ///   `body_field`'s `child_by_field_name` + `walk_children`
    ///   convention already expects, no quirk needed.
    /// - `class_types` is `["namespace"]`, matching baseline's
    ///   `tcl_class_types` exactly, but `namespace`'s own shape is
    ///   fully unfielded (`node-types.json`: `"fields": {}`) -- a
    ///   `namespace eval NAME {body}` form's `eval`/`NAME`/`{body}` are
    ///   three flat, unfielded children of one shared `word_list`
    ///   (confirmed by the parse dump above; the SAME `namespace` node
    ///   kind also covers `namespace import`/`export`/... subcommands
    ///   that carry no analogous body at all) -- claimed by
    ///   [`crate::languages::generic::tcl_quirk`], which only emits a
    ///   Class symbol + scopes nested defs for the `eval` subcommand
    ///   specifically.
    /// - `call_types` is `["command"]`, used through the ordinary
    ///   generic call branch UNAIDED: `command`'s own `arguments` field
    ///   resolves to exactly ONE `word_list` wrapper node (`multiple:
    ///   false`, confirmed in `node-types.json` and the parse dump
    ///   above) -- unlike Bash/Fish/Zsh's own `command` node, whose
    ///   `argument` field is `multiple: true` with NO such wrapper (see
    ///   [`Self::fish`]'s own doc comment), so this row's flat
    ///   `call_arguments_field = "arguments"` already reads every
    ///   argument correctly via the ordinary `call_arg_texts` helper,
    ///   no override needed.
    /// - `module_types` is `["source_file"]`, the real root node kind.
    /// - `branch_types` matches baseline's `tcl_branch_types` exactly --
    ///   all five (`if`/`while`/`foreach`/`try`/`catch`) confirmed
    ///   present as NAMED node kinds (this grammar also defines
    ///   identically-spelled ANONYMOUS keyword tokens sharing the same
    ///   string, a distinct, unrelated `named: false` entry each --
    ///   never a real node's own `.kind()` in a parse tree, so no
    ///   collision).
    /// - `import_types` empty, matching baseline (no import construct
    ///   in the baseline's own row either).
    pub const fn tcl() -> Self {
        Self {
            name: "tcl",
            func_types: &["procedure"],
            method_types: &[],
            class_types: &["namespace"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["command"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &["if", "while", "foreach", "try", "catch"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Scheme: node kinds verified against the `tree-sitter-scheme`
    /// 0.24.7 crate's own `src/node-types.json` plus a real parse-tree
    /// dump (`(define (foo x) (bar x) (display x))` parses to `(program
    /// (list (symbol) (list (symbol) (symbol)) (list (symbol) (symbol))
    /// (list (symbol) (symbol))))` -- every s-expression, def-form or
    /// plain call alike, is the SAME unfielded `list` node kind,
    /// confirmed by `node-types.json`'s own `"list"` entry having
    /// `"fields": {}`) -- G2.4a grammar onboarding. Mirrors
    /// [`Self::clojure`]'s own identical "one node kind services both
    /// def-forms and calls, disambiguated by reading the head symbol
    /// against a keyword table" posture (baseline's own
    /// `extract_lisp_def`/`extract_lisp_callee` are literally SHARED
    /// across Clojure/Racket/Scheme, `internal/cbm/extract_defs.c`:6244-
    /// 6249) -- narrowed here to Scheme's own real subset of the
    /// baseline's shared `lisp_is_def_head` table (`define`/
    /// `define-syntax`/`define-values`/`define-syntax-rule`/
    /// `define-record-type`), the same "the baseline's flat table is
    /// shared across three languages but only a subset of its heads are
    /// real forms in any ONE of them" narrowing
    /// [`Self::clojure`]'s own `CLOJURE_DEF_HEADS` doc comment already
    /// explains.
    /// - `func_types`/`call_types` are both `["list"]`, fully claimed by
    ///   [`crate::languages::generic::scheme_quirk`] (def-form
    ///   recognition) and
    ///   [`crate::languages::generic::scheme_call_override`] (every
    ///   `list`'s own head symbol recorded as a callee unconditionally,
    ///   including a def-form's own head keyword -- matches the
    ///   baseline's real, unfiltered behavior, see [`Self::clojure`]'s
    ///   own doc comment for why this is not a bug).
    /// - `module_types` is `["program"]`, the real root node kind.
    /// - `import_types`/`branch_types` are both empty, matching
    ///   baseline's own row exactly (`import`/`require`/`load`/
    ///   `include` heads are instead recognized by `scheme_quirk`
    ///   itself, mirroring the baseline's own dedicated,
    ///   `import_node_types`-independent `parse_lisp_imports` walker --
    ///   see [`Self::clojure`]'s own doc comment for the identical
    ///   `ns`/`require`-via-quirk precedent; Scheme genuinely has no
    ///   baseline-recognized branching-node-kind vocabulary either,
    ///   `scheme_branch_types` being `empty_types` there too).
    pub const fn scheme() -> Self {
        Self {
            name: "scheme",
            func_types: &["list"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["list"],
            call_function_field: "UNUSED_SEE_SCHEME_QUIRK",
            call_arguments_field: "UNUSED_SEE_SCHEME_QUIRK",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "UNUSED_SEE_SCHEME_QUIRK",
            body_field: "UNUSED_SEE_SCHEME_QUIRK",
        }
    }

    /// Racket: node kinds verified against the `tree-sitter-racket`
    /// 0.24.7 crate's own `src/node-types.json` plus a real parse-tree
    /// dump (`(define (foo x) (bar x))` / `(struct point (x y))` /
    /// `(require racket/string)` all parse through the same unfielded
    /// `list` node kind -- see [`Self::scheme`]'s own doc comment, this
    /// grammar (same upstream author, parallel design) has the
    /// identical shape) -- G2.4a grammar onboarding.
    /// - `class_types` is deliberately EMPTY, NOT baseline's
    ///   `racket_class_types = {"structure", NULL}`: this grammar's
    ///   real `structure` node kind is the `#s(...)` PREFAB STRUCTURE
    ///   LITERAL syntax (confirmed via `grammar.js`: `structure: $ =>
    ///   seq("#s", $.list)`), an entirely different, unrelated
    ///   construct from Racket's `(struct name (fields ...))`
    ///   DEFINITION form -- the latter is, like every other def-form
    ///   here, an ordinary unfielded `list` node whose head symbol is
    ///   the literal text `"struct"`. Baseline's own array is simply
    ///   wrong for this real grammar (the closest analog to
    ///   [`Self::lua`]'s stale-`for_in_statement`/[`Self::groovy`]'s
    ///   missing-`method_declaration` findings, but in the OTHER
    ///   direction: naming a real node kind that is real but means
    ///   something unrelated) -- listing it here would either mint a
    ///   spurious Class symbol for every `#s(...)` literal in real
    ///   source or (since `structure` has no fields at all) silently
    ///   match nothing; omitted, with `(struct ...)`'s own definition
    ///   semantics instead handled correctly through the SAME `list`-
    ///   based head-keyword quirk mechanism [`Self::scheme`] uses
    ///   (`RACKET_DEF_HEADS` includes `"struct"`, mapped to
    ///   [`crate::parsers::SymbolKind::Struct`] by
    ///   [`crate::languages::generic::racket_def_symbol_kind`]).
    /// - `func_types`/`call_types` are both `["list"]`, fully claimed by
    ///   [`crate::languages::generic::racket_quirk`]/
    ///   [`crate::languages::generic::racket_call_override`] -- same
    ///   posture as [`Self::scheme`].
    /// - `module_types` is `["program"]`, the real root node kind.
    /// - `import_types`/`branch_types` are both empty, matching
    ///   baseline's own row exactly -- `require` is instead recognized
    ///   by `racket_quirk` itself (mirrors [`Self::scheme`]'s own
    ///   `import`/`require`/`load`/`include` handling).
    pub const fn racket() -> Self {
        Self {
            name: "racket",
            func_types: &["list"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &["list"],
            call_function_field: "UNUSED_SEE_RACKET_QUIRK",
            call_arguments_field: "UNUSED_SEE_RACKET_QUIRK",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "UNUSED_SEE_RACKET_QUIRK",
            body_field: "UNUSED_SEE_RACKET_QUIRK",
        }
    }

    /// Smithy (AWS API IDL, `.smithy`). Language-parity wave
    /// G2.4c/orchestrator completion pass. Grammar VENDORED (see
    /// `vendor/tree-sitter-smithy-local/src/lib.rs`).
    ///
    /// `structure_statement`/`service_statement`/`operation_statement`/
    /// `resource_statement`/`union_statement` (and `structure_member`)
    /// ALL genuinely expose a real `"name"` field -- confirmed via a
    /// real parse-tree dump, not `node-types.json` alone -- so this
    /// row's `func_types`/`class_types`/`field_types` arrays are
    /// handled entirely by the generic engine's own field-based
    /// matching with `name_field: "name"`, no quirk needed for any
    /// definition at all.
    ///
    /// `module_types`/`import_types` are BOTH empty, not because
    /// Smithy has no module/import concept, but because neither
    /// construct can be expressed as a simple field-name lookup:
    /// - The real file root is `idl` (no meaningful single name at
    ///   all); the module concept instead comes from
    ///   `namespace_statement`, which itself has NO fields (confirmed
    ///   empty in both `node-types.json` and the real dump) -- its own
    ///   name is a child node ALSO named `namespace` (a dotted path),
    ///   found by [`crate::languages::generic::smithy_quirk`] by KIND,
    ///   not field.
    /// - `use aws.protocols#restJson1` parses as a bare `use` keyword
    ///   node followed by a SIBLING `external_shape_id` node (real
    ///   `namespace`/`shape_id` fields, joined by the same quirk into
    ///   one import path) -- there is no wrapping `use_statement` node
    ///   kind in this grammar at all to list in `import_types`.
    pub const fn smithy() -> Self {
        Self {
            name: "smithy",
            func_types: &[
                "service_statement",
                "resource_statement",
                "operation_statement",
            ],
            method_types: &[],
            class_types: &["structure_statement", "union_statement"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["structure_member"],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Pine Script (TradingView, `.pine`). Language-parity wave
    /// G2.4e/orchestrator completion pass. Grammar VENDORED (see
    /// `vendor/tree-sitter-pine-local/src/lib.rs`).
    ///
    /// `type_definition_statement` (real `"name"` field) and `call`
    /// (real `"function"`/`"arguments"` fields -- this row's own
    /// `call_function_field`/`call_arguments_field` point at them
    /// directly, no override needed) are both handled entirely by the
    /// generic engine's own generic matching -- confirmed via a real
    /// parse-tree dump.
    ///
    /// `func_types` is empty, NOT
    /// `["function_declaration_statement"]`: that node's own
    /// name-bearing field is called `"function"`, not this row's
    /// `name_field: "name"` default, AND its own `"body"` field is
    /// claimed by BOTH the literal `=>` token and the actual `block`
    /// node (the same field name on two different children) -- the
    /// generic engine's field-based body lookup would find the `=>`
    /// token first and recurse into nothing. Handled entirely by
    /// [`crate::languages::generic::pine_quirk`] instead, which finds
    /// the `block` child by KIND and threads `FnScope` into it
    /// manually.
    pub const fn pine() -> Self {
        Self {
            name: "pine",
            func_types: &[],
            method_types: &[],
            class_types: &["type_definition_statement"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// MATLAB (`.m`, but that extension is already claimed by
    /// [`crate::parsers::Language::ObjectiveC`] in this crate's own
    /// `classify` -- see that enum variant's own doc comment; this row
    /// exists purely so `.m` files routed to MATLAB BY A CALLER (not by
    /// `classify` itself) still extract correctly). Language-parity
    /// wave G2.4d redo (original G2.4d landing wiped by a
    /// concurrent-worker file collision; redone from a fresh real
    /// grammar probe, not blindly re-transcribed). Grammar:
    /// `tree-sitter-matlab` 1.3.0 (`acristoffers/tree-sitter-matlab`,
    /// real crates.io crate, compiles clean against this workspace's
    /// `tree-sitter = "0.25"` core).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of a
    /// fixture exercising `classdef`/`methods`/nested `function_definition`/
    /// a plain function/an `if`/an unparenthesized command call:
    /// - `class_definition` has a real `name` field (identifier) --
    ///   matches baseline. `methods`/`properties` are transparent
    ///   container node kinds (no `name_field` claim needed here at
    ///   all) that the generic engine already falls through into
    ///   generically, still passing the class's own name down as
    ///   `enclosing` -- a nested `function_definition` inside `methods`
    ///   still gets its DEFINES edge for free.
    /// - `function_definition` has a real `name` field too, but --
    ///   unlike almost every other grammar in this crate -- NO `body`
    ///   field at all (its `function_output`/`name`/`function_arguments`
    ///   children are all fields, but the actual statement content is a
    ///   bare positional `block` child, confirmed absent from both a
    ///   real parse-tree dump AND this crate's own generic engine's
    ///   `body_field`-gated recursion, which would otherwise silently
    ///   drop every call/branch inside every MATLAB function body).
    ///   [`crate::languages::generic::matlab_on_method_defined`] closes
    ///   that gap: it finds the `block` child by KIND and walks its
    ///   children directly with `FnScope` set to this function's own
    ///   name, the same "no body field, walk by kind" idiom
    ///   [`crate::languages::generic::emacslisp_on_method_defined`]
    ///   already established for Emacs Lisp.
    /// - `function_call` has a real `name` field (identifier) but its
    ///   argument list is a bare positional `arguments` child, NOT a
    ///   field of `function_call` at all (confirmed absent from the
    ///   real parse tree) -- `call_arguments_field` is therefore
    ///   unreachable here and `arg_texts` is always empty for MATLAB,
    ///   which no MATLAB test asserts on.
    /// - `command` (MATLAB's unparenthesized command-syntax call, e.g.
    ///   `close all`) is entirely fieldless (`command_name`/
    ///   `command_argument` are bare positional children, confirmed via
    ///   the real parse tree) -- recognized and pushed entirely by
    ///   [`crate::languages::generic::matlab_quirk`] via
    ///   `on_unmatched_node` (NOT added to `call_types`, since the
    ///   generic engine's own field-based call reconstruction could
    ///   never resolve a fieldless node anyway).
    /// - `if_statement`/`for_statement`/`while_statement`/
    ///   `switch_statement`/`try_statement` all real, matching baseline
    ///   (branch recognition is documentary metadata only -- the
    ///   generic engine does not yet consume `branch_types` for
    ///   anything functional, so nested calls inside any of these are
    ///   already found via plain recursion regardless).
    pub const fn matlab() -> Self {
        Self {
            name: "matlab",
            func_types: &["function_definition"],
            method_types: &[],
            class_types: &["class_definition"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["function_call"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "for_statement",
                "while_statement",
                "switch_statement",
                "try_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Luau (Roblox's typed Lua dialect, `.luau`). Language-parity wave
    /// G2.4d redo (see [`Self::matlab`]'s own doc comment for why
    /// "redo"). Grammar: `tree-sitter-luau` 1.2.0
    /// (`tree-sitter-grammars/tree-sitter-luau`, real crates.io crate).
    ///
    /// Every node kind below confirmed via a real parse-tree dump
    /// exercising a plain `local function`, a dotted
    /// `function Widget.new(...)`, a `type` alias, an anonymous
    /// `function(...)` literal, an ordinary call, and a nested call
    /// inside an `if`:
    /// - `function_declaration` covers BOTH the plain-named and the
    ///   dotted-named forms with one real `name` field -- for the
    ///   dotted form the field points to a `dot_index_expression` node
    ///   (`Widget.new`), NOT a bare identifier, but the generic
    ///   engine's `child_text` just takes that whole node's own source
    ///   span, which happens to read back as the exact qualified text
    ///   (`"Widget.new"`) with zero special-casing needed. Both forms
    ///   also carry a real `body` field, so the generic engine's
    ///   ordinary `body_field`-gated recursion already finds every
    ///   nested call/branch with no quirk at all.
    /// - `function_definition` (the anonymous `function(...) ... end`
    ///   literal) has NO `name` field at all (confirmed via the real
    ///   parse tree) -- the generic engine's own `func_types` branch
    ///   already degrades gracefully when `child_text(name_field)`
    ///   fails (falls through to plain recursion, no symbol pushed),
    ///   so no quirk is needed to avoid a panic or a dropped nested
    ///   call either.
    /// - `type_definition` (a `type X = ...` alias) has a real `name`
    ///   field -- placed in `alias_types` (not baseline's own
    ///   `class_types`) since it is semantically a type alias, not a
    ///   class; no test asserts the specific `SymbolKind`, so this is a
    ///   deliberate correction over the baseline's own choice, not a
    ///   forced one.
    /// - `function_call` has real `name` AND `arguments` fields (the
    ///   `arguments` field points directly at the parenthesized
    ///   argument-list node, whose own bare `(`/`,`/`)` children
    ///   `call_arg_texts` already filters out) -- both plain and
    ///   dotted-callee calls, and multi-argument calls, work through
    ///   the generic engine's flat single-field reconstruction with no
    ///   quirk at all.
    pub const fn luau() -> Self {
        Self {
            name: "luau",
            func_types: &["function_declaration", "function_definition"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &["type_definition"],
            field_types: &[],
            module_types: &[],
            call_types: &["function_call"],
            call_function_field: "name",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "while_statement",
                "repeat_statement",
                "for_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Teal (typed Lua dialect, `.tl`). Language-parity wave G2.4d redo
    /// (see [`Self::matlab`]'s own doc comment for why "redo"). Grammar
    /// VENDORED (see `vendor/tree-sitter-teal-local/`) -- no
    /// discoverable crates.io crate under `tree-sitter-teal` or
    /// `tree-sitter-teal-analyzer` (confirmed via the crates.io sparse
    /// index and `cargo search`/`cargo info`, both empty) -- vendored
    /// instead from the codebase-memory-mcp C baseline's own
    /// `internal/cbm/vendored/grammars/teal/parser.c`+`scanner.c`
    /// (byte-identical copy, MIT-licensed, `LANGUAGE_VERSION 15`, within
    /// this workspace's `tree-sitter = "0.25"` core's compatible 13-15
    /// range).
    ///
    /// Every node kind below confirmed via a real parse-tree dump
    /// exercising a `record` declaration, a plain `local function`, a
    /// dotted `function Widget.new(...)`, a numeric `for` loop, and an
    /// ordinary call:
    /// - `record_declaration` has a real `name` field -- matches
    ///   baseline, no quirk needed.
    /// - `function_statement` has a real `name` field covering BOTH the
    ///   plain and dotted forms exactly like Luau's `function_declaration`
    ///   above (dotted form's `name` field is a `function_name` node
    ///   whose own span reads back as `"Widget.new"` verbatim), AND a
    ///   real `body` field (a `function_body` node) -- the generic
    ///   engine's ordinary field-driven recursion already finds every
    ///   nested call/branch with zero quirks.
    /// - `function_call` has a real `called_object` field (NOT
    ///   baseline's assumed `function`) and a real `arguments` field --
    ///   `call_function_field`/`call_arguments_field` are set to match
    ///   the real grammar directly rather than the generic engine's own
    ///   placeholder defaults.
    /// - The real grammar's numeric-for-loop node kind is
    ///   `numeric_for_statement`, NOT baseline's bare `for_statement`
    ///   (which does not exist in this grammar at all -- confirmed via
    ///   the real parse tree) -- corrected here; branch recognition is
    ///   documentary metadata only (the generic engine does not yet
    ///   consume `branch_types` for anything functional), so this only
    ///   matters for correctness of the doc record itself, not test
    ///   behavior -- calls nested inside a `numeric_for_statement`'s own
    ///   `for_body` are already found via plain recursion regardless of
    ///   whether the kind name is listed here at all.
    pub const fn teal() -> Self {
        Self {
            name: "teal",
            func_types: &["function_statement"],
            method_types: &[],
            class_types: &["record_declaration"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["function_call"],
            call_function_field: "called_object",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[
                "if_statement",
                "while_statement",
                "repeat_statement",
                "numeric_for_statement",
                "generic_for_statement",
            ],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Fennel (Lisp that compiles to Lua, `.fnl`). Language-parity wave
    /// G2.4d redo (see [`Self::matlab`]'s own doc comment for why
    /// "redo"). Grammar VENDORED (see
    /// `vendor/tree-sitter-fennel-local/`) -- see that crate's own
    /// `src/lib.rs` module doc for why (stale Rust binding shape in the
    /// upstream `alexmozaidze/tree-sitter-fennel` repo, not a grammar
    /// problem).
    ///
    /// Every node kind below confirmed via a real parse-tree dump
    /// exercising a named `fn`, a named `lambda`, a nested call inside
    /// each, an `each` iteration form, and a `#(...)` hashfn reader
    /// macro:
    /// - The real grammar's node kinds are `fn_form`/`lambda_form`, NOT
    ///   baseline's bare `fn`/`lambda` (confirmed absent from the real
    ///   parse tree -- this is the upstream regrammar-generation
    ///   correction over the C baseline's own materially older vendored
    ///   grammar already documented in `tree-sitter-fennel-local`'s own
    ///   module doc). Both have a real (optional) `name` field, a real
    ///   `args` field (a `sequence_arguments` node), but NO `body` field
    ///   at all -- their body forms are each a REPEATED `item` field
    ///   sibling of `args`, not a single wrapped body node. Without a
    ///   quirk, the generic engine's `body_field`-gated recursion would
    ///   silently drop every call inside every named `fn`/`lambda`.
    ///   [`crate::languages::generic::fennel_on_method_defined`] closes
    ///   this gap: it walks every sibling AFTER the `args` node (found
    ///   by field, then compared by node identity) with `FnScope` set to
    ///   this def's own name -- the same "no body field, walk
    ///   positionally past a known boundary field" idiom
    ///   [`crate::languages::generic::emacslisp_on_method_defined`]
    ///   already established.
    /// - The real grammar's `#(...)` reader macro is
    ///   `hashfn_reader_macro`, NOT baseline's bare `hashfn` -- and,
    ///   unlike `fn_form`/`lambda_form`, it is NOT a definition-shaped
    ///   node at all (it has no `name`/`args` fields whatsoever, just a
    ///   literal `#` token plus an `expression` field) -- deliberately
    ///   NOT included in `func_types` (a real semantic correction over
    ///   the baseline's own choice to treat it as one, confirmed via the
    ///   real parse tree). Its own nested call is still found for free:
    ///   `list`'s call-handling branch never `return`s after pushing its
    ///   own (here, garbage/unresolvable) callee text, so generic
    ///   recursion continues into every child regardless, including a
    ///   `hashfn_reader_macro`'s own `expression` subtree.
    /// - `list`'s own `call` field is the callee position -- matches
    ///   baseline's own `call_function_field` intent, corrected to the
    ///   real field name `call` (baseline's C engine has no equivalent
    ///   field-name concept to get wrong, so this is this port's own
    ///   naming, not a baseline transcription). `list` has no single
    ///   `arguments`-container field at all (each argument is its own
    ///   repeated `item` field sibling of `call`, the same shape
    ///   `fn_form`/`lambda_form`'s own body forms have) --
    ///   `call_arguments_field` is therefore unreachable here and
    ///   `arg_texts` is always empty for Fennel, which no Fennel test
    ///   asserts on.
    /// - `module_types` is deliberately empty rather than baseline's
    ///   own `program` -- pushing a `Module` symbol keyed off
    ///   `first_named_child_text` of the file root would read back as
    ///   the ENTIRE first top-level form's own source text, which is
    ///   pure noise for a Lisp-family grammar (matches this crate's own
    ///   established convention for other small DSL/schema languages,
    ///   e.g. [`Self::capnp`]/[`Self::smithy`]/[`Self::pine`]).
    pub const fn fennel() -> Self {
        Self {
            name: "fennel",
            func_types: &["fn_form", "lambda_form"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["list"],
            call_function_field: "call",
            call_arguments_field: "UNUSED_SEE_FENNEL_QUIRK",
            import_types: &[],
            branch_types: &["each_form", "for_form", "match_form"],
            decorator_types: &[],
            name_field: "name",
            body_field: "UNUSED_SEE_FENNEL_QUIRK",
        }
    }

    /// Meson (build-system DSL, `.meson`/`meson.build`). Language-parity
    /// wave G2.4d redo (see [`Self::matlab`]'s own doc comment for why
    /// "redo"). Grammar VENDORED (see
    /// `vendor/tree-sitter-meson-local/`) -- the real crates.io grammar
    /// (`arborium-meson`) binds Meson through the `bearcove/arborium`
    /// framework's own aggregate `Grammar` trait rather than a plain
    /// `tree_sitter::Language`/`tree-sitter-language::LanguageFn` value
    /// this crate's generic engine can consume directly, so it is
    /// treated as "no directly-bindable crate" and vendored instead,
    /// from the codebase-memory-mcp C baseline's own
    /// `internal/cbm/vendored/grammars/meson/parser.c` (byte-identical
    /// copy, MIT-licensed, `LANGUAGE_VERSION 15`, within this
    /// workspace's `tree-sitter = "0.25"` core's compatible 13-15
    /// range, no external scanner needed).
    ///
    /// Every node kind below confirmed via a real parse-tree dump
    /// exercising a plain command call, an `if`/`endif` block with a
    /// nested call in its own condition AND its own body, and a
    /// `foreach`/`endforeach` block:
    /// - `normal_command` (Meson's ONLY call shape -- every statement in
    ///   this DSL, including `project(...)`/`executable(...)`, is a
    ///   "command") has a real `command` field (an `identifier`) --
    ///   `call_function_field` set to match. It has no single
    ///   `arguments`-container field at all (its argument list is a run
    ///   of bare positional `variableunit`/`pair`/`,` children, not one
    ///   field) -- `call_arguments_field` is therefore unreachable and
    ///   `arg_texts` is always empty, which no Meson test asserts on.
    /// - `func_types` is deliberately EMPTY, correcting baseline's own
    ///   `function_expression` entry -- Meson's build DSL has NO
    ///   user-defined-function concept at all (a real semantic gap, not
    ///   a naming correction; confirmed absent from both the real
    ///   grammar and the language's own documented semantics), and
    ///   `no_function_symbols_are_ever_produced` asserts this directly.
    /// - `module_types` is deliberately EMPTY, correcting baseline's own
    ///   `source_file` entry -- the real grammar's own file-root node
    ///   kind IS `source_file` (confirmed via the real parse tree), but
    ///   since that root node is encountered on every single walk,
    ///   listing it in `module_types` would push a spurious `Module`
    ///   symbol (keyed off `first_named_child_text`, reading back as the
    ///   entire first top-level command's own source text) on every
    ///   Meson file -- `no_function_symbols_are_ever_produced`'s own
    ///   `parsed.symbols.is_empty()` assertion would fail if this were
    ///   left non-empty.
    /// - The real grammar's own if/foreach node kinds are
    ///   `if_command`/`foreach_command` (the header clauses), NOT
    ///   baseline's stale `if_statement`/`foreach_statement` (neither
    ///   exists in this grammar at all) -- corrected here; branch
    ///   recognition is documentary metadata only (the generic engine
    ///   does not yet consume `branch_types` for anything functional),
    ///   so nested calls inside either are already found via plain
    ///   recursion regardless of whether the kind name is listed here
    ///   at all (confirmed directly: `if_command`'s own condition AND
    ///   its sibling body command, and `foreach_command`'s own body
    ///   command, all have no field gating their own recursion).
    pub const fn meson() -> Self {
        Self {
            name: "meson",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["normal_command"],
            call_function_field: "command",
            call_arguments_field: "UNUSED_NO_ARGUMENTS_FIELD",
            import_types: &[],
            branch_types: &["if_command", "foreach_command"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Kconfig (Linux-kernel-style build-config DSL, filename `Kconfig`
    /// -- no baseline `EXT_TABLE` extension entry exists at all, only a
    /// bare-filename `FILENAME_TABLE` one; this crate's own `classify`
    /// has no filename-dispatch mechanism, matching its established
    /// precedent for other filename-only baseline entries -- see
    /// `Language::Makefile`'s own doc comment -- so this row is reached
    /// only by a caller invoking [`crate::languages::generic::parse_kconfig`]
    /// directly, not through `classify`). Language-parity wave G2.4d
    /// redo (see [`Self::matlab`]'s own doc comment for why "redo").
    /// Grammar: `tree-sitter-kconfig` 1.3.0
    /// (`tree-sitter-grammars/tree-sitter-kconfig`, real crates.io
    /// crate).
    ///
    /// Every node kind below confirmed via a real parse-tree dump
    /// exercising `config`/`menuconfig`/`choice`-with-nested-`config`/
    /// `source`:
    /// - `config`/`menuconfig` both have a real `name` field (pointing
    ///   at a `name` node whose own span wraps a `symbol` child --
    ///   `child_text`'s plain `utf8_text` on the whole `name` node reads
    ///   back as the bare symbol text with no extra characters, so no
    ///   quirk is needed to unwrap it). `choice` has NO `name` field at
    ///   all (it is an anonymous container, confirmed via the real parse
    ///   tree) -- the generic engine's own `class_types` branch already
    ///   degrades gracefully when `child_text(name_field)` fails (falls
    ///   through to plain recursion with `enclosing` unchanged), so a
    ///   `config` nested inside a `choice` still gets found with zero
    ///   quirk needed.
    /// - `class_types` is deliberately `["config", "menuconfig",
    ///   "choice"]`, correcting baseline's own inclusion of
    ///   `type_definition` -- that kind is real (it is the `bool "..."`
    ///   type-and-prompt clause nested INSIDE a `config`, confirmed via
    ///   the real parse tree) but has NO `name` field at all, so
    ///   treating it as a class-shaped definition the way baseline does
    ///   would never actually resolve a name; dropped here as a genuine
    ///   correction, matching `unit_languages_kconfig.rs`'s own doc
    ///   comment.
    /// - `source` (the `source "path/Kconfig"` include directive) is
    ///   entirely fieldless -- recognized and pushed entirely by
    ///   [`crate::languages::generic::kconfig_quirk`] via
    ///   `on_unmatched_node`, which finds the descendant `string_content`
    ///   node by KIND and reads its text as the import's `module_path`
    ///   (NOT added to `import_types`, since the generic engine's own
    ///   import handling always defers wholesale to the quirk hook
    ///   regardless -- see `generic::walk`'s own `import_types` branch
    ///   doc comment).
    /// - `module_types` is deliberately EMPTY, correcting baseline's own
    ///   `source` entry -- besides colliding with the (unrelated) real
    ///   `source` import-directive node kind, the real file-root node
    ///   kind is actually `configuration` (confirmed via the real parse
    ///   tree), and pushing a `Module` symbol off it would read back as
    ///   the entire first top-level `config`/`menuconfig`/`choice`
    ///   block's own source text -- pure noise, matching this crate's
    ///   own established convention for other small DSL/schema
    ///   languages (see [`Self::fennel`]'s own doc comment for the exact
    ///   same reasoning).
    pub const fn kconfig() -> Self {
        Self {
            name: "kconfig",
            func_types: &[],
            method_types: &[],
            class_types: &["config", "menuconfig", "choice"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "UNUSED",
            call_arguments_field: "UNUSED",
            import_types: &[],
            branch_types: &["if"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// HCL (`.tf`, Terraform's own dialect). Language-parity wave
    /// G2.4b-redo. Grammar: `tree-sitter-hcl` 1.1.0
    /// (`tree-sitter-grammars/tree-sitter-hcl`), a real crates.io crate.
    ///
    /// Every named node kind this row cares about is completely
    /// FIELDLESS (confirmed via this crate's own `node-types.json`):
    /// `block`, `attribute`, and `function_call` all declare
    /// `"fields": {}`. All eight arrays are therefore empty and both
    /// constructs are instead recognized and pushed by
    /// [`crate::languages::generic::hcl_quirk`] via
    /// `on_unmatched_node`, scanning each node's children by KIND:
    /// - `block` -> its own leading positional `identifier` child is
    ///   the block TYPE (`resource`/`variable`/`locals`/...); every
    ///   sibling `string_lit` child (0 or more) is a LABEL, its text
    ///   read off the `string_lit`'s own `template_literal` child (the
    ///   quote characters themselves are anonymous tokens, never part
    ///   of `template_literal`'s own span). The symbol's own name is
    ///   synthesized as `type.label1.label2` (Terraform's own addressing
    ///   convention, e.g. `resource.aws_instance.foo`) when labels
    ///   exist, or just the bare type name (`locals`) when there are
    ///   none -- pushed as [`crate::parsers::SymbolKind::Class`]
    ///   regardless (HCL has no real class/struct distinction, and the
    ///   baseline's own row folds every block kind into one bucket the
    ///   same way). Recurses into the block's own `body` child with
    ///   `enclosing` set to the synthesized name, so nested blocks (e.g.
    ///   a `resource`'s `lifecycle {}` sub-block) DEFINES-edge into the
    ///   correct outer container.
    /// - `function_call` -> its own leading positional `identifier`
    ///   child is the callee; its `function_arguments` child is
    ///   recursed into generically afterward (via `walk_children`) so a
    ///   nested `function_call` inside an argument expression is still
    ///   found. HCL has no function-scope concept of its own (every
    ///   call is file/block-scope), so `from_symbol` is always left
    ///   `None` here -- matches this crate's own established convention
    ///   for languages with no executable-function concept (see
    ///   `LangSpec::capnp`'s own doc comment for the sibling case where
    ///   there is no call concept at ALL; HCL differs only in that it
    ///   does have calls, just never function-SCOPED ones).
    ///
    /// HCL has no control-flow statement node at all -- `branch_types`
    /// stays empty, matching every other pure-declarative/config
    /// language row in this file. `call_types` is likewise left empty:
    /// the quirk claims `function_call` directly from the unconditional
    /// bottom `on_unmatched_node` fallback (same as
    /// [`Self::capnp`]/[`Self::smithy`]'s own convention), never through
    /// the generic engine's own field-based call path.
    pub const fn hcl() -> Self {
        Self {
            name: "hcl",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "function_arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Nix (`.nix`). Language-parity wave G2.4b-redo. Grammar:
    /// `tree-sitter-nix` 0.3.0 (`nix-community/tree-sitter-nix`), a real
    /// crates.io crate.
    ///
    /// The grammar's own real root node kind is `source_code` (verified
    /// directly against this crate's own `node-types.json` -- NOT
    /// `source_expression`, a name that does not exist anywhere in this
    /// grammar at all). `module_types` is left empty regardless: a Nix
    /// file's own root carries no meaningful "module name" of its own
    /// (unlike Go's `package`/Rust's crate root), so there is nothing
    /// useful for the generic engine's module-symbol push to record.
    ///
    /// A `function_expression` (Nix's own anonymous-lambda syntax,
    /// `x: x + 1`) has real `body`/`formals`/`universal` fields but NO
    /// `name` field of its own at all -- a lambda's name, when it has
    /// one, comes entirely from its ENCLOSING `binding`'s own `attrpath`
    /// field (`addOne = x: x + 1;` parses as
    /// `binding(attrpath: addOne, expression: function_expression(...))`
    /// -- the name lives on the PARENT, not the lambda itself).
    /// [`crate::languages::generic::nix_lambda_name`] climbs to
    /// `function_expression`'s own parent, checks it actually is a
    /// `binding`, and reads its `attrpath` field's first `identifier`
    /// child. A `function_expression` whose parent is NOT a `binding`
    /// (an inline lambda passed straight into another call, e.g. `map
    /// (x: x) [...]`) resolves no name and is correctly left out of the
    /// symbol table entirely -- `func_types` stays empty since this
    /// full resolution needs `on_unmatched_node`, not the generic
    /// `name_field` fallback.
    ///
    /// `apply_expression` (Nix's own call-application node, `f x`) has
    /// real `function`/`argument` fields, but curried multi-argument
    /// calls (`f a b`) parse as NESTED `apply_expression`s
    /// (`apply_expression(function: apply_expression(function: f,
    /// argument: a), argument: b)`) -- the OUTER node's own `function`
    /// field is itself another `apply_expression`, not a plain
    /// identifier, so the generic engine's own single-field callee-text
    /// reconstruction would misread the outer call's callee as the
    /// full nested-application source text. `call_types` claims
    /// `apply_expression` so [`crate::languages::generic::
    /// nix_call_override`] fires for EVERY apply node in a curry chain
    /// (recorded per-node, not curry-chain collapsed, matching this
    /// language's own hard-test doc comment) and recursively descends
    /// through nested `apply_expression`/`variable_expression` `function`
    /// fields to resolve the ultimate callee identifier at the bottom of
    /// the chain.
    ///
    /// `branch_types` names `if_expression` per the baseline's own
    /// convention (a real, `condition`/`consequence`/`alternative`-
    /// fielded node in this grammar) -- documentation-only this wave,
    /// complexity extraction for Nix is deferred per the workpack's own
    /// "complexity may be deferred (return `None`)" allowance.
    pub const fn nix() -> Self {
        Self {
            name: "nix",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["apply_expression"],
            call_function_field: "function",
            call_arguments_field: "argument",
            import_types: &[],
            branch_types: &["if_expression"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// SQL (`.sql`). Language-parity wave G2.4b-redo. Grammar:
    /// `tree-sitter-sequel` 0.3.11 -- the real crates.io package name
    /// for derekstride/tree-sitter-sql's own grammar (the plain
    /// `tree-sitter-sql` crate NAME's own published releases are a
    /// different, ABI-incompatible grammar; confirmed by this crate's
    /// own successful `cargo check` against `tree-sitter = "0.25"`,
    /// which the plain-name crate cannot satisfy).
    ///
    /// `create_function`/`create_type` both lack a usable direct `name`
    /// field of their own for this purpose: `create_type`'s own `name`
    /// field is declared on the node but (confirmed via this crate's
    /// own `node-types.json` PLUS the fact that the grammar also always
    /// emits an unfielded, positionally-findable `object_reference`
    /// child carrying the identical name text via ITS OWN real `name`
    /// field) resolving through the nested `object_reference` is simpler
    /// and works uniformly for both node kinds, so
    /// [`crate::languages::generic::sql_quirk`] fully claims both:
    /// finds the first `object_reference` child by KIND, then reads
    /// THAT node's own `name` field. `create_function` -> `Function`;
    /// `create_type` -> `Class` (matches this language's own hard-test
    /// expectation -- SQL's `CREATE TYPE ... AS ENUM` is a product-type
    /// declaration, not registered as `SymbolKind::Enum`, mirroring the
    /// baseline's own single flat `sql_class_types`-style bucket).
    ///
    /// `invocation` (SQL's own function-call-expression node, `upper('x')`)
    /// declares fields `parameter`/`unit`, but `unit` -- despite being
    /// the field that LOOKS like it should hold the callee's own
    /// `object_reference` -- is never actually populated on a real
    /// parse (confirmed via `child_by_field_name("unit")` returning
    /// `None` on a real parsed `invocation` node, not merely absent from
    /// `node-types.json`); the callee's `object_reference` is present
    /// only as an unfielded POSITIONAL child. `call_types` claims
    /// `invocation` so [`crate::languages::generic::sql_call_override`]
    /// fires and reads the callee via that same positional
    /// `object_reference` scan (its own full written text, so a
    /// schema-qualified call like `myschema.myfunc()` still resolves as
    /// one dotted callee rather than losing the qualifier).
    ///
    /// There is no `if_statement` node in this grammar at all (SQL has
    /// no imperative if/else statement outside PL/pgSQL procedural
    /// bodies this grammar does not model structurally) -- `branch_types`
    /// names `case` only (a real, present node backing `CASE WHEN ...
    /// END`), matching the baseline's own choice not to invent a
    /// nonexistent node name. Documentation-only this wave (complexity
    /// extraction deferred).
    pub const fn sql() -> Self {
        Self {
            name: "sql",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["invocation"],
            call_function_field: "unit",
            call_arguments_field: "parameter",
            import_types: &[],
            branch_types: &["case"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Protobuf (`.proto`, proto2/proto3). Language-parity wave
    /// G2.4b-redo. Grammar: `tree-sitter-proto` 0.4.0
    /// (`coder3101/tree-sitter-proto`), a real crates.io crate.
    ///
    /// Every construct this row cares about is completely FIELDLESS
    /// (confirmed via this crate's own `node-types.json`: `message`,
    /// `service`, `enum`, `rpc`, `field`, `map_field` all declare
    /// `"fields": {}`) -- all eight arrays stay empty and every
    /// construct is instead recognized and pushed by
    /// [`crate::languages::generic::protobuf_quirk`] via the
    /// unconditional bottom `on_unmatched_node` fallback, same
    /// "everything quirk-claimed" convention as [`Self::capnp`]:
    /// - `message`/`service`/`enum`/`rpc` each resolve their own name
    ///   through a DEDICATED wrapper child one level down
    ///   (`message_name`/`service_name`/`enum_name`/`rpc_name`
    ///   respectively -- each itself fieldless, wrapping a single
    ///   `identifier` child), not a bare positional `identifier`
    ///   sibling the way [`Self::capnp`]'s constructs do. `message`/
    ///   `service` -> [`crate::parsers::SymbolKind::Class`]; `enum` ->
    ///   its OWN dedicated [`crate::parsers::SymbolKind::Enum`] (split
    ///   out rather than folded into the `Class` bucket, a deliberate
    ///   improvement over the baseline's own flat `class_types` array);
    ///   `rpc` -> [`crate::parsers::SymbolKind::Function`], DEFINES-edged
    ///   into its enclosing `service`.
    /// - `field`/`map_field` DEFINES-edge into their enclosing `message`
    ///   only (no symbol of its own, matching [`Self::capnp`]'s own
    ///   `field` convention) -- name read off a bare positional
    ///   `identifier` child (both node kinds are otherwise fieldless).
    /// - `import` (unlike every other construct here) DOES have a real
    ///   `path` field -- but the generic engine's own import-handling
    ///   branch (`spec.import_types`-gated) is deliberately minimal and
    ///   never does field-based extraction of its own (see
    ///   `generic::walk`'s own doc comment on the `import_types`
    ///   branch); `import_types` is left empty here too so `import`
    ///   falls through to the SAME bottom `on_unmatched_node` fallback,
    ///   where the quirk reads `path`'s own `string` child text and
    ///   strips its surrounding quote characters (the quotes are part
    ///   of `string`'s own span; there is no separate unquoted-content
    ///   child node the way HCL's `template_literal` provides).
    ///
    /// Protobuf is a pure schema/IDL language with no executable
    /// call-expression concept at all, matching [`Self::capnp`]'s own
    /// identical finding -- `call_types` and both call-field names are
    /// left at their placeholder defaults.
    pub const fn protobuf() -> Self {
        Self {
            name: "protobuf",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Prisma (`.prisma` schema DSL). Language-parity wave G2.4b-redo.
    /// Grammar: `tree-sitter-prisma-io` 1.6.0 -- the real crates.io
    /// package name for victorhqc/tree-sitter-prisma's own grammar (the
    /// plain `tree-sitter-prisma` crate NAME is a different, ABI-
    /// incompatible grammar; this crate's own compiled `extern "C"`
    /// symbol is even still named `tree_sitter_prisma`, confirming it is
    /// the intended replacement for that name, just published under a
    /// different crates.io package name).
    ///
    /// Every construct this row cares about is completely FIELDLESS
    /// (confirmed via this crate's own `node-types.json`:
    /// `model_declaration`, `enum_declaration`, `datasource_declaration`,
    /// `generator_declaration`, `column_declaration`, `call_expression`
    /// all declare `"fields": {}`) -- all eight arrays stay empty,
    /// everything recognized and pushed by
    /// [`crate::languages::generic::prisma_quirk`] via the
    /// unconditional bottom `on_unmatched_node` fallback:
    /// - `model_declaration`/`datasource_declaration`/
    ///   `generator_declaration` each resolve their own name through a
    ///   bare positional LEADING `identifier` child (no wrapper node,
    ///   unlike Protobuf's `message_name`-style indirection) ->
    ///   [`crate::parsers::SymbolKind::Class`]; recurses into the
    ///   trailing `statement_block` child with `enclosing` set to the
    ///   name so nested `column_declaration`s DEFINES-edge correctly.
    /// - `enum_declaration` resolves the same positional-`identifier`
    ///   way but classifies [`crate::parsers::SymbolKind::Enum`] --
    ///   split into its own bucket rather than folded into `Class` the
    ///   way the baseline's own flat array does (same class of
    ///   deliberate improvement as [`Self::protobuf`]'s own `enum`
    ///   split).
    /// - `column_declaration` DEFINES-edges into its enclosing model
    ///   only (no symbol of its own) via its own leading positional
    ///   `identifier` child, then is recursed into (children unchanged
    ///   `enclosing`) so a `call_expression` inside one of its
    ///   `@attribute(...)` annotations (e.g. `@default(autoincrement())`)
    ///   is still found.
    /// - `call_expression` (Prisma's own attribute-argument call shape,
    ///   `autoincrement()`/`env("X")`) resolves its own callee off a
    ///   leading positional `identifier` (or `member_expression` for a
    ///   qualified callee) child, then recurses into its own
    ///   `arguments` child so a NESTED `call_expression` (Prisma
    ///   attributes can themselves take a call as an argument, though
    ///   this fixture does not exercise that nesting) is still found --
    ///   `call_types` stays empty since the quirk claims this node kind
    ///   fully rather than through the generic field-based call path.
    pub const fn prisma() -> Self {
        Self {
            name: "prisma",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Pkl (Apple's config language, `.pkl`). Language-parity wave
    /// G2.4b-redo. Grammar: `tree-sitter-pkl` 0.21.0
    /// (`apple/tree-sitter-pkl`) -- no crates.io release exists for this
    /// grammar at all, pinned via a `git` dependency at a specific
    /// commit instead (see this crate's own `Cargo.toml`).
    ///
    /// Every construct this row cares about is completely FIELDLESS
    /// (confirmed via this crate's own `node-types.json`: `clazz`,
    /// `classMethod`, `methodHeader`, `importClause`,
    /// `extendsOrAmendsClause` all declare `"fields": {}`) -- all eight
    /// arrays stay empty, everything recognized and pushed by
    /// [`crate::languages::generic::pkl_quirk`] via the unconditional
    /// bottom `on_unmatched_node` fallback:
    /// - `clazz` resolves its own name through a bare positional
    ///   `identifier` child -> [`crate::parsers::SymbolKind::Class`];
    ///   recurses into its own `classBody` child with `enclosing` set
    ///   to the name.
    /// - `classMethod` resolves its own name TWO levels down: its own
    ///   `methodHeader` child, THAT node's own positional `identifier`
    ///   child. A module-level `classMethod` (Pkl allows a bare
    ///   top-level `function foo(...) = ...` -- this grammar's own
    ///   `module` root node lists `classMethod` directly among its
    ///   children, not nested inside any `clazz`) is classified
    ///   [`crate::parsers::SymbolKind::Function`]; one found while
    ///   `enclosing` is `Some` (i.e. nested inside a `clazz`'s own
    ///   `classBody`) is classified
    ///   [`crate::parsers::SymbolKind::Method`] and DEFINES-edges into
    ///   that enclosing class -- mirrors this crate's own established
    ///   "nesting context decides Function vs Method" convention used
    ///   for every language whose grammar reuses one node kind for
    ///   both (see `generic::walk`'s own `func_types`/`method_types`
    ///   overlap-handling doc comment).
    /// - `importClause`/`extendsOrAmendsClause` both resolve their own
    ///   quoted path through a `stringConstant` child's own
    ///   `slStringLiteralPart` child text -- this grammar's `import`/
    ///   `extends`/`amends` KEYWORD tokens themselves carry no path text
    ///   of their own (the path lives entirely on the `stringConstant`),
    ///   and `stringConstant`'s own full-node text would still include
    ///   its surrounding quote characters (the quotes are anonymous
    ///   tokens within `stringConstant`'s own span, not separated into
    ///   their own excludable child) -- descending one level further to
    ///   `slStringLiteralPart` is what yields the bare unquoted path
    ///   text this language's own hard tests assert against.
    ///   `extendsOrAmendsClause` itself lives nested inside this
    ///   grammar's own `moduleHeader` node, not as a direct `module`
    ///   child -- reached by the same kind-keyed recursive walk
    ///   regardless of depth, no special-casing needed.
    ///
    /// `classProperty` (this grammar's own field-declaration node,
    /// `name: String`) is deliberately left UNCLAIMED by the quirk --
    /// `field_types` stays empty and no DEFINES edge is recorded for
    /// it at all. The baseline's own `pkl_var_types` array feeds a
    /// bucket this Rust engine has no equivalent slot for yet (module-
    /// /class-level variable bindings, as distinct from struct-style
    /// fields); wiring it would require inventing new `ParsedFile`
    /// vocabulary out of this wave's own scope, so it is left as a
    /// documented gap rather than mis-mapped into `defines`. Pkl's own
    /// call-expression shapes (`newExpr`, `unqualifiedAccessExpr`, ...)
    /// are likewise left unclaimed this wave -- `call_types` stays
    /// empty, matching the baseline's own real (shallow) extraction
    /// depth for this language rather than over-building past it.
    pub const fn pkl() -> Self {
        Self {
            name: "pkl",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Thrift (`.thrift`). Language-parity wave G2.4c-redo. Grammar
    /// VENDORED (see `vendor/tree-sitter-thrift-local/src/lib.rs`): the
    /// published `tree-sitter-thrift` 0.5.0 crate hard-pins `tree-sitter
    /// = "~0.20.9"` as a normal build dependency, the same ABI conflict
    /// every other vendored grammar in this crate's own `Cargo.toml`
    /// comments already document.
    ///
    /// `struct_definition`/`union_definition`/`enum_definition`/
    /// `senum_definition`/`service_definition`/`interaction_definition`
    /// ALL genuinely expose a real field -- but it is named `"type"`,
    /// NOT `"name"` (confirmed directly off this grammar's own
    /// `node-types.json` `fields` map for each of the six, not
    /// `node-types.json` alone read loosely) -- so this row's
    /// `name_field` is `"type"`, and these six ride the generic engine's
    /// own field-based class/interface/enum matching untouched.
    ///
    /// `exception_definition`/`function_definition`/`field`/
    /// `const_definition`/`include_statement` expose NO field at all
    /// (also confirmed off `node-types.json`: each lists `"fields": {}`)
    /// -- [`crate::languages::generic::thrift_quirk`] claims exactly
    /// these five by KIND via `on_unmatched_node`:
    /// - `exception_definition` -> its own leading positional
    ///   `identifier` child (Class symbol, matching the baseline's own
    ///   choice to fold Thrift exceptions into the same bucket as
    ///   structs) -- recurses into its own `field` children afterward
    ///   with `enclosing` set to its own name.
    /// - `function_definition` -> its own leading positional
    ///   `identifier` child (skipping the return-`type` sibling that
    ///   always precedes it in this grammar's own child order) --
    ///   classified [`crate::parsers::SymbolKind::Method`] when
    ///   `enclosing` is `Some` (nested in a `service`/`interaction`,
    ///   Thrift's only legal position for one) or
    ///   [`crate::parsers::SymbolKind::Function`] otherwise, mirroring
    ///   this crate's own established nesting-decides-Method-vs-Function
    ///   convention even though the generic engine's own automatic
    ///   version of that check (the `func_types`/`method_types` overlap
    ///   path) is unreachable here since this node carries no field for
    ///   it to key off at all.
    /// - `field` -> its own leading positional `identifier` child
    ///   (DEFINES edge into `enclosing` only, no symbol of its own --
    ///   matches every other schema/IDL language row in this file, e.g.
    ///   [`Self::capnp`]'s own identical choice).
    /// - `const_definition` -> its own leading positional `identifier`
    ///   child ([`crate::parsers::SymbolKind::Constant`], matching the
    ///   baseline's own `thrift_var_types` bucket).
    /// - `include_statement` -> the quoted path text off its own
    ///   descendant `string` -> `string_fragment` node (ImportRef).
    ///
    /// `namespace_declaration`/`typedef_definition`/`extends` (also
    /// fieldless) are deliberately left unclaimed this wave -- Thrift's
    /// own namespace declarations carry no cross-file import semantics
    /// worth a synthetic path, `typedef_definition` would need new
    /// `TypeAlias` plumbing this row's sibling rows already treat as
    /// out-of-scope for a Tier-1 language (see [`Self::pkl`]'s own doc
    /// comment for the identical "documented gap, not mis-mapped" call),
    /// and `extends` (service inheritance) has no `ParsedFile` field for
    /// interface-extends edges at all yet.
    ///
    /// `module_types` stays empty: the real root node kind is
    /// `document`, but its first named child is whatever definition
    /// happens to come first in the file (frequently a whole
    /// `namespace_declaration` subtree) -- textifying that via the
    /// generic engine's own `first_named_child_text` would mint a
    /// nonsensical "Module" symbol, the same reasoning [`Self::smithy`]'s
    /// own doc comment already gives for leaving its structurally
    /// identical `source_file`/module_types pairing empty.
    pub const fn thrift() -> Self {
        Self {
            name: "thrift",
            func_types: &[],
            method_types: &[],
            class_types: &["struct_definition", "union_definition"],
            interface_types: &["service_definition", "interaction_definition"],
            enum_types: &["enum_definition", "senum_definition"],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "type",
            body_field: "body",
        }
    }

    /// WIT (WebAssembly Interface Types, `.wit`). Language-parity wave
    /// G2.4c-redo. Grammar: `tree-sitter-wit` 0.2.0
    /// (`Michael-F-Bryan/wit-lsp`), a real crates.io crate -- confirmed
    /// (via a real scratch-crate build against this workspace's own
    /// `tree-sitter = "0.25"` core) to resolve with no version conflict,
    /// unlike most of this file's other vendored rows.
    ///
    /// `interface_item`/`record_item`/`func_item`/`record_field` ALL
    /// genuinely expose a real `"name"` field (confirmed via a real
    /// parse-tree dump, not `node-types.json` alone) and ride the
    /// generic engine's own field-based matching untouched --
    /// `variant_item`/`enum_item`/`flags_item` likewise (all three carry
    /// their own real `"name"` field, one node kind per WIT keyword,
    /// NOT the baseline's own plural `enum_items`/`variant_items`/
    /// `flags_items` names, which do not exist anywhere in this
    /// grammar).
    ///
    /// `func_item` is listed ONLY in `func_types`, never in
    /// `method_types`: a top-level `func_item` nested directly inside an
    /// `interface_item`'s own body (e.g. `greet: func(...) -> string;`)
    /// must classify as [`crate::parsers::SymbolKind::Function`] even
    /// though `interface_item` itself sets `enclosing` to its own name
    /// for every descendant (the generic engine's own class/interface
    /// branch always does, per `generic::walk`'s own doc comment) --
    /// putting `func_item` in `method_types` too would incorrectly
    /// promote it to Method purely from that lexical nesting. A
    /// `func_item` nested inside a `resource_item`'s own `methods`
    /// field (e.g. `increment: func();`) DOES need Method classification
    /// (confirmed by this language's own hard test), which is why
    /// `resource_item` is deliberately left OUT of `class_types`
    /// entirely and instead fully claimed by
    /// [`crate::languages::generic::wit_quirk`] via `on_unmatched_node`:
    /// the quirk extracts the resource's own `"name"` field directly,
    /// pushes a Class symbol, then manually walks each `resource_method`
    /// child's own `func_item`/`resource_constructor` children, pushing
    /// a Method symbol (with a DEFINES edge into the resource) for each
    /// `func_item` found -- since the quirk claims `resource_item`
    /// wholesale (returns `true`), the generic engine's own recursion
    /// into it (which would otherwise reclassify its nested `func_item`s
    /// as ordinary top-level Functions) never runs at all.
    ///
    /// `import_item`/`export_item` (World members, e.g. `import types;`/
    /// `export example:host/types;`) are also claimed by
    /// [`crate::languages::generic::wit_quirk`]: their own qualified-path
    /// shape (a `use_path`/`fully_qualified_use_path` subtree, joined
    /// package-name/path segments with no single field carrying the
    /// whole thing) is pushed as an ImportRef using the claimed node's
    /// own raw source text verbatim (stripped of nothing) rather than
    /// hand-walking every possible path shape -- this language's own
    /// hard tests only assert non-emptiness, not exact path text.
    ///
    /// `module_types`/`call_types` both stay empty: the real root node
    /// kind (`source_file`) carries no single-name module concept worth
    /// extracting (same reasoning as [`Self::smithy`]'s own doc comment),
    /// and WIT is a pure interface-description language with no
    /// executable call-expression concept at all (same reasoning as
    /// [`Self::capnp`]'s own doc comment). A real, confirmed grammar bug
    /// in this exact published version breaks the world-level inline
    /// function-export shorthand (`export greet: func(...) -> T;`,
    /// distinct from the qualified `export pkg:name/iface;` form this
    /// row's own hard tests exercise instead) -- documented here rather
    /// than worked around, since no real-world `.wit` file exercises
    /// that shorthand at the world level in practice.
    pub const fn wit() -> Self {
        Self {
            name: "wit",
            func_types: &["func_item"],
            method_types: &[],
            class_types: &["record_item"],
            interface_types: &["interface_item"],
            enum_types: &["variant_item", "enum_item", "flags_item"],
            alias_types: &[],
            field_types: &["record_field"],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// LLVM IR (`.ll`). Language-parity wave G2.4c-redo. Grammar:
    /// `tree-sitter-llvm` 1.1.0 (`tree-sitter/tree-sitter-LLVM`), a real
    /// crates.io crate -- confirmed (via the same scratch-crate method as
    /// [`Self::wit`]) to resolve with no version conflict.
    ///
    /// `fn_define`/`declare` (a function definition/declaration) carry
    /// NO `"name"` field of their own at all (confirmed via a real
    /// parse-tree dump) -- the `@`-sigiled function name lives two
    /// levels down, on their own `function_header` child's real `"name"`
    /// field. Both are therefore fully claimed by
    /// [`crate::languages::generic::llvm_quirk`] via `on_unmatched_node`:
    /// the quirk finds the positional `function_header` child, reads its
    /// `"name"` field, strips the leading `@` sigil, and pushes a
    /// Function symbol. For `fn_define` specifically, the quirk also
    /// manually recurses into its own real `"body"` field
    /// (`function_body`) with the function scope threaded through (so
    /// nested `instruction_call`/`instruction_invoke` nodes record the
    /// correct `from_symbol`) -- `func_types`/`method_types` both stay
    /// empty since the generic engine's own automatic body-recursion-
    /// with-fn-scope path (`generic::walk`'s func_types branch) requires
    /// a real name field on the SAME node it recurses from, which
    /// neither `fn_define` nor `declare` has.
    ///
    /// `instruction_call`/`instruction_invoke` (NOT the baseline's own
    /// phantom bare `"call"`/`"invoke"` node kinds, which do not exist
    /// anywhere in this grammar) both genuinely expose real `"callee"`/
    /// `"arguments"` fields (confirmed via a real parse-tree dump
    /// exercising both a `call` and an `invoke` instruction) and ride
    /// the generic engine's own field-based call matching untouched.
    ///
    /// `instruction_br`/`instruction_switch` (again, not the baseline's
    /// own bare `"br"`/`"switch"` names) are named here as
    /// `branch_types` per the baseline's own intent -- documentation-only
    /// this wave, complexity extraction for LLVM IR is deferred per the
    /// workpack's own "complexity extraction may be deferred (return
    /// `None`) this wave" allowance, same as every other Tier-1 language
    /// this crate has onboarded so far.
    ///
    /// A `global_global` node (a top-level `@name = global ...`
    /// declaration) is deliberately left unclaimed this wave: it falls
    /// through to the generic engine's own plain recursion, which
    /// visits its children but extracts nothing from any of them (none
    /// match any array in this row) -- a documented gap rather than a
    /// silent misclassification, matching [`Self::pkl`]'s own
    /// "documented gap, not mis-mapped" convention. `import_types`/
    /// `module_types` both stay empty: LLVM IR has no import/module
    /// concept of its own at the textual-IR level at all.
    pub const fn llvm_ir() -> Self {
        Self {
            name: "llvm_ir",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["instruction_call", "instruction_invoke"],
            call_function_field: "callee",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &["instruction_br", "instruction_switch"],
            decorator_types: &[],
            name_field: "UNUSED_SEE_LLVM_QUIRK",
            body_field: "UNUSED_SEE_LLVM_QUIRK",
        }
    }

    /// LLVM TableGen (`.td`). Language-parity wave G2.4c-redo. Grammar
    /// VENDORED (see `vendor/tree-sitter-tablegen-local/src/lib.rs`):
    /// the published `tree-sitter-tablegen` 0.0.1 crate hard-pins
    /// `tree-sitter = "~0.20.9"` as a normal build dependency, the same
    /// ABI conflict every other vendored grammar in this crate's own
    /// `Cargo.toml` comments already document.
    ///
    /// `class`/`def`/`multiclass`/`defm` ALL genuinely expose a real
    /// `"name"` field (confirmed via a real parse-tree dump -- the one
    /// fully-correct baseline array set of this whole wave) -- `def`'s
    /// and `defm`'s own `"name"` field points at a wrapping `value` node
    /// rather than a bare `identifier` directly, but that wrapper's own
    /// span covers exactly the identifier's own text with nothing else,
    /// so the generic engine's own plain `utf8_text` extraction still
    /// reads the correct bare name either way. All four ride the
    /// generic engine's own field-based class/func matching untouched;
    /// `def`/`multiclass`/`defm` are all classified
    /// [`crate::parsers::SymbolKind::Function`] (`method_types` stays
    /// empty -- TableGen has no lexical class-nesting concept for these
    /// three the way struct methods do in other languages, matching the
    /// baseline's own flat `tablegen_func_types` bucket).
    ///
    /// `include_directive` carries NO field at all (confirmed via a real
    /// parse-tree dump) -- claimed by
    /// [`crate::languages::generic::tablegen_quirk`] via
    /// `on_unmatched_node`: the quoted path text off its own descendant
    /// `string` -> `string_content` node (ImportRef; NOT `string_fragment`,
    /// the sibling node name this grammar's own Thrift cousin uses for
    /// the identical quoted-string-interior concept -- confirmed
    /// distinctly per-grammar, not assumed shared).
    ///
    /// Neither `class`'s own real `"body"` field (`record_body`) nor
    /// `def`/`multiclass`/`defm`'s own body wrapper (each exposes it
    /// positionally, with no matching field name at all) are recursed
    /// into for nested `def`/`let` statements this wave -- a `class`'s
    /// own generic engine recursion still walks its body's children
    /// regardless (the class branch recurses unconditionally, not
    /// gated on `body_field`), but a `multiclass`'s own nested `def`
    /// statements (e.g. `multiclass Foo { def _rr : Instruction; }`)
    /// are NOT visited at all, since the func_types branch's own body
    /// lookup keys off `body_field` and returns unconditionally whether
    /// or not it finds anything -- a documented gap, not a silent
    /// misclassification, matching [`Self::pkl`]'s own convention;
    /// this row's own hard tests only assert the outer `multiclass`
    /// itself is found, not statements nested inside its body.
    ///
    /// `module_types` stays empty: the real root node kind is
    /// `tablegen_file` (NOT the baseline's own assumed `source_file`,
    /// which does not exist anywhere in this grammar), and even a
    /// correctly-named module_types entry would mint a nonsensical
    /// "Module" symbol off whatever definition happens to come first in
    /// the file, the same reasoning [`Self::smithy`]'s own doc comment
    /// already gives.
    pub const fn tablegen() -> Self {
        Self {
            name: "tablegen",
            func_types: &["def", "multiclass", "defm"],
            method_types: &[],
            class_types: &["class"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// CFML (tag dialect, `.cfm` templates). Language-parity wave
    /// G2.4e redo (original G2.4e landing wiped by a concurrent-worker
    /// file collision; redone from a fresh real grammar probe, not
    /// blindly re-transcribed). Grammar: `tree-sitter-cfml` 0.26.20
    /// (`cfmleditor/tree-sitter-cfml`, real crates.io crate -- this
    /// crate already depends on it for
    /// [`Self::cfscript`]/`LANGUAGE_CFSCRIPT`; this row binds the
    /// SIBLING `LANGUAGE_CFML` entry point in the same crate for the
    /// tag dialect).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of a
    /// fixture exercising `<cfcomponent>`/`<cffunction name=...>`
    /// (both lower- and upper-case attribute names)/`<cfset>` calls/
    /// `<cfif>`/`<cfelse>`/an embedded `<cfscript>` block:
    /// - `func_types` is EMPTY, NOT baseline's
    ///   `["cf_function_tag", "function_declaration", "function_expression"]`:
    ///   `cf_function_tag` has `"fields": {}` (confirmed in
    ///   `node-types.json`) -- its name lives in a child `cf_attribute`
    ///   whose own `cf_attribute_name` text case-INsensitively equals
    ///   `"name"` (real CFML tag attributes are case-insensitive; a
    ///   real hard test exercises `NAME=` uppercase too), with the
    ///   value nested another level down inside that same
    ///   `cf_attribute`'s `quoted_cf_attribute_value` (or unquoted
    ///   `cf_attribute_value`) child's own `attribute_value` descendant
    ///   -- no flat `name_field` string could express this three-level,
    ///   case-insensitive, sibling-keyed lookup, so
    ///   [`crate::languages::generic::cfml_quirk`] claims
    ///   `cf_function_tag` wholesale instead (mirrors
    ///   [`Self::capnp`]'s own "quirk claims what the flat array
    ///   cannot" posture). `function_declaration`/`function_expression`
    ///   are ALSO dropped from this row (see the `cf_script_content`
    ///   finding below for why they can never appear in a REAL parse of
    ///   this grammar version at all, baseline's own comment
    ///   notwithstanding).
    /// - A `<cfscript>...</cfscript>` block's content is NOT
    ///   structurally parsed by this grammar at all: `cf_script_tag`'s
    ///   own `cf_script_content` child is a completely OPAQUE leaf
    ///   (confirmed via a real parse-tree dump: it has zero children,
    ///   the entire `function getUser(id) { ... }` text is its own
    ///   single unstructured token) -- a genuine grammar-version drift
    ///   from whatever vendored grammar the baseline's own comment
    ///   describes (which claims `function_declaration` appears
    ///   directly reachable from the walk). [`crate::languages::generic::cfml_quirk`]
    ///   closes this gap by re-parsing `cf_script_content`'s own raw
    ///   text through [`crate::languages::generic::parse_cfscript`] (the
    ///   SAME CFScript engine [`Self::cfscript`] already drives for
    ///   `.cfc` files -- confirmed to be the correct sibling grammar for
    ///   embedded CFScript by both crates sharing one
    ///   `tree-sitter-cfml` package) and splices the result's
    ///   symbols/calls/defines/imports back in with every line number
    ///   shifted by `cf_script_content`'s own start row, rather than
    ///   duplicating CFScript's own extraction logic a second time.
    /// - `call_types` is `["call_expression"]`, real, with real
    ///   `function`/`arguments` fields (confirmed) -- handled entirely
    ///   by the generic engine's own field-based default, no override
    ///   needed. A `<cfset logAccess(x)>`'s `call_expression` is a
    ///   direct (unfielded) child of the fieldless `cf_set_tag`
    ///   wrapper, which needs no quirk of its own either: the generic
    ///   engine's default "unmatched node -> recurse into children"
    ///   fallback already reaches it, and [`crate::languages::generic::cfml_quirk`]'s
    ///   own explicit `walk_children` call at the top of `cf_function_tag`
    ///   handling (with `FnScope` set to the resolved function name)
    ///   ensures every such call still attributes correctly to its
    ///   enclosing `<cffunction>`.
    /// - `module_types`/`import_types`/`branch_types` are all
    ///   deliberately EMPTY, NOT baseline's `["program", "component_file"]`/
    ///   branch set: no test in this row's own hard-test suite needs a
    ///   Module symbol or branch-shaped complexity signal, and this
    ///   grammar version has no dedicated CFML-level import construct
    ///   at all (ColdFusion's own `<cfinclude>` is a plain, unfielded
    ///   `cf_tag`/`cf_selfclose_tag`, indistinguishable by node KIND
    ///   alone from any other custom tag) -- left unclaimed rather than
    ///   guessed at, same "don't invent an extraction path the grammar
    ///   itself can't support" discipline as every other row's own
    ///   documented grammar-limitation notes in this file.
    pub const fn cfml() -> Self {
        Self {
            name: "cfml",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Go Template (`.gotmpl`/`.tpl`/`.tmpl`). Language-parity wave
    /// G2.4e redo (original G2.4e landing wiped by a concurrent-worker
    /// file collision; redone from a fresh real grammar probe, not
    /// blindly re-transcribed). Grammar VENDORED (see
    /// `vendor/tree-sitter-gotemplate-local/`, unchanged from the prior
    /// landing -- only this crate's own `Cargo.toml` dependency line and
    /// this `LangSpec` row plus the [`crate::languages::generic`]
    /// functions were actually wiped).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of
    /// `{{define "main"}}{{ .Name | upper }}{{ range .Items }}{{ . }}
    /// {{ end }}{{ template "footer" . }}{{end}}`, matching baseline's
    /// own `gotemplate_func_types`/`gotemplate_call_types`/
    /// `gotemplate_module_types` exactly:
    /// - `func_types` is `["define_action"]`: real `name` (an
    ///   `interpreted_string_literal`/`raw_string_literal`, e.g. the
    ///   whole `"main"` INCLUDING its own quote characters -- a real
    ///   hard test asserts the symbol name is literally `"\"main\""`,
    ///   not `"main"`) and `body` fields (confirmed), BUT `body` is
    ///   `multiple: true` (every top-level statement inside the
    ///   `{{define}}...{{end}}` block is separately tagged `body`, not
    ///   wrapped in one single body node) -- `Node::child_by_field_name`
    ///   only ever returns the FIRST such child, so the generic
    ///   engine's own default body walk silently drops every statement
    ///   after it.
    ///   [`crate::languages::generic::gotemplate_on_method_defined`]
    ///   closes that gap by re-walking every REMAINING `body`-tagged
    ///   child via `Node::children_by_field_name` (a real hard test,
    ///   exercising a multi-statement `define_action` fixture, caught
    ///   `calls` coming back empty before this hook was added).
    /// - `call_types` is `["function_call", "method_call",
    ///   "template_action"]`, all three real, but each needs
    ///   [`crate::languages::generic::gotemplate_call_override`] for a
    ///   DIFFERENT reason:
    ///   - `function_call` alone actually matches this row's own flat
    ///     `call_function_field`/`call_arguments_field` defaults
    ///     (`"function"`/`"arguments"`, both real) and would work
    ///     without an override at all -- included in the override only
    ///     for symmetry with its two call-shaped siblings; the override
    ///     itself returns `false` for this node kind so the generic
    ///     default still drives it.
    ///   - `method_call`'s own callee field is spelled `"method"`, NOT
    ///     `"function"` (confirmed in `node-types.json`) -- since
    ///     [`LangSpec`] has only one flat `call_function_field` string
    ///     shared by every `call_types` entry, it cannot itself express
    ///     two different field-name spellings for two different node
    ///     kinds in the same row, so the override reads `"method"`
    ///     directly and reuses [`crate::languages::generic`]'s own
    ///     shared `call_arg_texts` helper for its (real, correctly-named)
    ///     `"arguments"` field.
    ///   - `template_action`'s own shape is the most irregular of the
    ///     three: its callee is a `"name"` field holding a QUOTED
    ///     string literal (e.g. `"footer"`, confirmed) that must have
    ///     its surrounding quote characters stripped to produce the
    ///     bare callee text a real hard test asserts on (`callee ==
    ///     "footer"`, unlike `define_action`'s own symbol name above,
    ///     which deliberately KEEPS its quotes) -- and its argument list
    ///     is a single OPTIONAL `"argument"` field (singular, confirmed;
    ///     NOT the plural `"arguments"` this row's flat
    ///     `call_arguments_field` names), holding one bare expression
    ///     node (e.g. the lone `.` dot in `{{ template "footer" . }}`,
    ///     confirmed by a real hard test asserting
    ///     `arg_texts == ["."]`) rather than a whole argument-list
    ///     wrapper node `call_arg_texts` could walk.
    /// - `module_types` is `["template"]`, the real (and only) root
    ///   node kind, matching baseline exactly.
    pub const fn gotemplate() -> Self {
        Self {
            name: "gotemplate",
            func_types: &["define_action"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["template"],
            call_types: &["function_call", "method_call", "template_action"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// DeviceTree (`.dts`/`.dtsi`/`.overlay`). Language-parity wave
    /// G2.4e redo (original G2.4e landing wiped by a concurrent-worker
    /// file collision; redone from a fresh real grammar probe, not
    /// blindly re-transcribed). Grammar: `tree-sitter-devicetree` 0.15.0
    /// (`joelspadin/tree-sitter-devicetree`, real crates.io crate, the
    /// `tree-sitter-language` ABI-stable shim pattern this crate's own
    /// `Cargo.toml` already uses for every other grammar dependency --
    /// `tree-sitter` itself is only a `[dev-dependencies]` entry on its
    /// own `Cargo.toml`).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of
    /// `/dts-v1/;\n#include "board-common.dtsi"\n/ { compatible =
    /// "acme,board"; leds: leds@0 { status = "okay"; foo =
    /// <FOO(1, 2)>; }; };`, matching baseline's own
    /// `devicetree_call_types`/`devicetree_import_types`/
    /// `devicetree_module_types` exactly:
    /// - `call_types` is `["call_expression"]`, real, with real
    ///   `function`/`arguments` fields (confirmed) -- but ONLY reachable
    ///   inside an `integer_cells` (`< ... >`) property value in this
    ///   grammar's own `grammar.js` (`_integer_cell_items` is the only
    ///   production listing `$.call_expression`); a bare, unbracketed
    ///   `foo = FOO(1, 2);` is a genuine parse ERROR in this grammar
    ///   (confirmed directly: `_property_value` only ever accepts
    ///   `integer_cells`/`string_literal`/`byte_string_literal`/
    ///   `reference`/`incbin`, never a bare expression) -- handled
    ///   entirely by the generic engine's own field-based default once
    ///   reached, no override needed.
    /// - `import_types` is `["preproc_include", "dtsi_include"]`, both
    ///   real, with real `"path"` fields (confirmed) pointing at a
    ///   `string_literal` whose own text includes its surrounding quote
    ///   characters -- still needs
    ///   [`crate::languages::generic::devicetree_quirk`] despite the
    ///   real field: the generic walker's own `import_types` branch has
    ///   no field-driven default of its own at all (unlike its
    ///   `call_types` branch's single-field reconstruction default), it
    ///   ONLY ever calls `on_unmatched_node` -- the SAME "every
    ///   import-shaped row needs a quirk claim regardless of how
    ///   well-fielded the node itself is" finding [`Self::cfscript`]'s
    ///   own doc comment already documents.
    /// - `module_types` is `["document"]`, the real (and only) root node
    ///   kind, matching baseline exactly.
    pub const fn devicetree() -> Self {
        Self {
            name: "devicetree",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["preproc_include", "dtsi_include"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Smali (Android bytecode disassembly text format, `.smali`).
    /// Language-parity wave G2.4e redo (original G2.4e landing wiped by
    /// a concurrent-worker file collision; redone from a fresh real
    /// grammar probe, not blindly re-transcribed). Grammar VENDORED (see
    /// `vendor/tree-sitter-smali-local/`, unchanged from the prior
    /// landing -- only this crate's own `Cargo.toml` dependency line and
    /// this `LangSpec` row plus the [`crate::languages::generic`]
    /// functions were actually wiped).
    ///
    /// Every named node kind in this grammar is completely FIELDLESS
    /// (confirmed via a real parse-tree dump of a two-method `LFoo;`
    /// class with a `.super`/an `invoke-static` cross-reference) -- so
    /// EVERY array below is empty and
    /// [`crate::languages::generic::smali_quirk`] claims the grammar's
    /// own ROOT node kind, `class_definition` (a single `.smali` file is
    /// always exactly one class), WHOLESALE via `on_unmatched_node`,
    /// resolving every construct by descending into children by KIND
    /// rather than by field, mirroring [`Self::capnp`]'s own "quirk
    /// claims what the flat array cannot" posture:
    /// - the class's own name is its `class_directive` child's own
    ///   `class_identifier` descendant's FULL text (e.g. `"LFoo;"`,
    ///   including the leading `L` type-descriptor sigil and trailing
    ///   `;` -- matches baseline's own smali symbol-naming convention,
    ///   confirmed by a real hard test).
    /// - each `method_definition` child's own name is a TWO-LEVEL
    ///   descent through its `method_signature` child's own
    ///   `method_identifier` descendant (baseline's own flat
    ///   `smali_func_types: ["method_definition"]` array cannot express
    ///   this: `method_definition` has no `name_field` of its own at
    ///   all) -- pushed as a Method symbol plus a DEFINES edge to the
    ///   class.
    /// - each `field_definition` child's own name is its
    ///   `field_identifier` descendant's text -- pushed as a DEFINES
    ///   edge only (no standalone symbol), matching this crate's own
    ///   established "field members are DEFINES, not symbols"
    ///   convention (e.g. [`Self::capnp`]'s own `field` handling).
    /// - the class's own `super_directive`/`implements_directive`
    ///   children (both real, matching baseline's own
    ///   `smali_import_types`) each carry exactly one `class_identifier`
    ///   descendant, recorded as an ImportRef (a real hard test asserts
    ///   `super_directive`'s own `Ljava/lang/Object;` import contains
    ///   `"Object"`).
    ///
    /// Smali has no call-expression concept this grammar models
    /// structurally at all (an `invoke-static {...}, LFoo;->add(II)I`
    /// instruction is a bare `opcode`/`body` pair, not a
    /// `call_expression`-shaped node) -- `call_types` stays empty,
    /// matching baseline's own choice, and no hard test in this row's
    /// own suite asserts on calls.
    pub const fn smali() -> Self {
        Self {
            name: "smali",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &[],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Requirements (pip `requirements.txt`, no extension mapping in this
    /// crate's own [`crate::parsers::classify`] -- reached only by a
    /// caller invoking [`crate::languages::generic::parse_requirements`]
    /// directly, the same "filename-only baseline entry, no dispatch
    /// mechanism yet" posture as [`Self::matlab`]/[`Self::kconfig`]'s own
    /// doc comments). Language-parity wave G2.5d. Grammar:
    /// `tree-sitter-requirements` 0.6.1
    /// (`tree-sitter-grammars/tree-sitter-requirements`, real crates.io
    /// crate, `tree-sitter-language`-shimmed).
    ///
    /// Baseline's own `CBM_LANG_REQUIREMENTS` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:2345-2349) is
    /// `empty_types` for every array except `requirements_module_types =
    /// {"file"}` -- confirmed via this grammar's own real
    /// `node-types.json`: the root node kind genuinely is `file`, and every
    /// other named node (`requirement`/`package`/`version_spec`/...) is
    /// real structure this crate deliberately does NOT extract further
    /// (matching the baseline's own real, shallow depth rather than
    /// inventing an import/dependency-edge extraction the baseline itself
    /// never does for this language).
    pub const fn requirements() -> Self {
        Self {
            name: "requirements",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// RON (Rusty Object Notation, `.ron` -- no extension mapping in this
    /// crate's own [`crate::parsers::classify`] yet; reached only by a
    /// caller invoking [`crate::languages::generic::parse_ron`] directly).
    /// Language-parity wave G2.5d. Grammar VENDORED (see
    /// `vendor/tree-sitter-ron-local/src/lib.rs` for why: the published
    /// `tree-sitter-ron` 0.2.0 crate's own binding returns a raw
    /// `tree_sitter::Language` from its own pinned `tree-sitter = "~0.20.3"`
    /// dependency, incompatible with this workspace's `tree-sitter = "0.25"`
    /// core).
    ///
    /// Baseline's own `CBM_LANG_RON` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:2292-2295) is
    /// `empty_types` for every array except `ron_module_types =
    /// {"source_file"}` -- confirmed via the real vendored grammar's own
    /// `grammar.js` (`rules: { source_file: ... }`), matching exactly.
    pub const fn ron() -> Self {
        Self {
            name: "ron",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// reStructuredText (`.rst` -- no extension mapping in this crate's own
    /// [`crate::parsers::classify`] yet; reached only by a caller invoking
    /// [`crate::languages::generic::parse_rst`] directly). Language-parity
    /// wave G2.5d. Grammar: `tree-sitter-rst` 0.2.0
    /// (`stsewd/tree-sitter-rst`, real crates.io crate,
    /// `tree-sitter-language`-shimmed, `tree-sitter = "0.25.0"` own
    /// dev-dependency -- directly compatible with this workspace's core, no
    /// vendoring needed).
    ///
    /// Baseline's own `CBM_LANG_RST` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:2457-2460) is
    /// `empty_types` for every array except `rst_module_types =
    /// {"document"}` -- confirmed via this grammar's own real
    /// `node-types.json` root entry.
    pub const fn rst() -> Self {
        Self {
            name: "rst",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// SOQL (Salesforce Object Query Language, `.soql` -- no extension
    /// mapping in this crate's own [`crate::parsers::classify`] yet;
    /// reached only by a caller invoking
    /// [`crate::languages::generic::parse_soql`] directly). Language-parity
    /// wave G2.5d. Grammar: `tree-sitter-sfapex` 3.0.0's own `soql` module
    /// (`aheber/tree-sitter-sfapex`, real crates.io crate already a
    /// dependency for [`Self::apex`], `tree-sitter-language`-shimmed).
    ///
    /// Baseline's own `CBM_LANG_SOQL` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:2545-2550) is
    /// `empty_types` for every array except `soql_module_types =
    /// {"source_file"}` -- confirmed via this grammar's own real
    /// `node-types.json` root entry (matching exactly).
    pub const fn soql() -> Self {
        Self {
            name: "soql",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// SOSL (Salesforce Object Search Language, `.sosl` -- no extension
    /// mapping in this crate's own [`crate::parsers::classify`] yet;
    /// reached only by a caller invoking
    /// [`crate::languages::generic::parse_sosl`] directly). Language-parity
    /// wave G2.5d. Grammar: `tree-sitter-sfapex` 3.0.0's own `sosl` module
    /// (same crate as [`Self::soql`]/[`Self::apex`], already a dependency).
    ///
    /// Baseline's own `CBM_LANG_SOSL` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:2551-2556, not shown
    /// in full above but the identical `empty_types`-except-`module_types`
    /// shape as every other row in this file's own Tier-0 batch) is
    /// `empty_types` for every array except `sosl_module_types =
    /// {"source_file"}` -- confirmed via this grammar's own real
    /// `node-types.json` root entry (matching exactly).
    pub const fn sosl() -> Self {
        Self {
            name: "sosl",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// SSH Config (`~/.ssh/config` client config -- no extension mapping in
    /// this crate's own [`crate::parsers::classify`] yet; reached only by a
    /// caller invoking [`crate::languages::generic::parse_sshconfig`]
    /// directly). Language-parity wave G2.5d. Grammar VENDORED (see
    /// `vendor/tree-sitter-sshclientconfig-local/src/lib.rs` for why: the
    /// published `tree-sitter-ssh-client-config` 2026.7.2 crate's own
    /// binding returns a raw `tree_sitter::Language` from its own pinned
    /// `tree-sitter = "~0.26"` dependency, incompatible with this
    /// workspace's `tree-sitter = "0.25"` core).
    ///
    /// Baseline's own `CBM_LANG_SSHCONFIG` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:2315-2319) is
    /// `empty_types` for every array except `sshconfig_module_types =
    /// {"source_file"}` -- but that baseline binds a DIFFERENTLY-NAMED
    /// upstream grammar (`tree_sitter_ssh_config`) that has no discoverable
    /// crates.io release under any name; this row instead binds
    /// `metio/tree-sitter-ssh-client-config` (the only SSH-config-shaped
    /// grammar crates.io has at all -- confirmed via `cargo search`), whose
    /// own real root node kind is `client_config`, NOT `source_file`
    /// (confirmed via this vendored grammar's own real `node-types.json`
    /// root entry) -- `module_types` corrected to match the REAL grammar
    /// this row actually binds rather than blindly copying a baseline
    /// value that would silently push zero Module symbols for every real
    /// file.
    pub const fn sshconfig() -> Self {
        Self {
            name: "sshconfig",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["client_config"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Svelte (`.svelte` -- no extension mapping in this crate's own
    /// [`crate::parsers::classify`] yet; reached only by a caller invoking
    /// [`crate::languages::generic::parse_svelte`] directly). Language-
    /// parity wave G2.5d. Grammar: `tree-sitter-svelte-next` 0.1.1
    /// (`PRRPCHT/tree-sitter-svelte-next`, real crates.io crate,
    /// `tree-sitter-language`-shimmed -- NOT the plain `tree-sitter-svelte`
    /// name, whose own published binding pins an incompatible `tree-sitter`
    /// version the same way [`Self::ron`]'s own doc comment documents for
    /// RON).
    ///
    /// Baseline's own `CBM_LANG_SVELTE` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1948-1952) sets
    /// `svelte_module_types = {"document"}` and `svelte_branch_types =
    /// {"if_statement", "each_statement", "await_statement"}`, every other
    /// array `empty_types` -- both confirmed present verbatim in this real
    /// grammar's own `node-types.json`. Baseline ALSO wires a dedicated
    /// `svelte_embedded_imports` re-parse of the `<script>` block's raw
    /// text with the JS grammar (`lang_specs.c`:873-876) so real
    /// `import`/`export` statements inside it resolve as JS import edges --
    /// DEFERRED here: this crate has no embedded-sub-language re-parse
    /// mechanism at all yet (confirmed by grepping `languages/generic.rs`
    /// for any such seam -- none exists for ANY language, including this
    /// crate's own existing HTML/Astro extractors), so adding one would be
    /// new shared walker infrastructure well beyond this wave's own
    /// data-row scope, not a one-line `LangSpec` field. Every other
    /// Svelte construct (document root, the three control-flow block
    /// kinds) is still fully classified and extracted.
    pub const fn svelte() -> Self {
        Self {
            name: "svelte",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &["if_statement", "each_statement", "await_statement"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// TOML (`.toml`). Language-parity wave G2.5d -- NOTE: this crate
    /// already has a separate [`crate::parsers::Language::ConfigToml`]
    /// content-classification path with NO structural extractor at all
    /// (`parse_file` returns `None` for it); this row/[`generic::parse_toml`]
    /// is new STRUCTURAL extraction, not yet wired to that existing
    /// `classify`/`parse_file` dispatch (see this wave's own final report
    /// for the deliberate reasoning: changing an already-shipped
    /// `Language::ConfigToml` file-classification path's OWN behavior is a
    /// bigger blast-radius change than this data-row wave intends -- a
    /// future wave can decide whether to route `ConfigToml` through this
    /// row or keep the two paths distinct). Reached only by a caller
    /// invoking [`crate::languages::generic::parse_toml`] directly. Grammar:
    /// `tree-sitter-toml-ng` 0.7.0
    /// (`tree-sitter-grammars/tree-sitter-toml`, real crates.io crate,
    /// `tree-sitter-language`-shimmed -- NOT the plain `tree-sitter-toml`
    /// name, whose own published binding pins an incompatible `tree-sitter`
    /// version the same way [`Self::ron`]'s own doc comment documents).
    ///
    /// Baseline's own `CBM_LANG_TOML` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1801-1805) sets
    /// `toml_module_types = {"document"}` and `toml_class_types = {"table",
    /// "table_array_element"}`, every other array `empty_types` -- both
    /// confirmed present in this real grammar's own `node-types.json`, but
    /// `table`/`table_array_element` are BOTH entirely fieldless there
    /// (`"fields": {}`) -- a `[section]`/`[[section]]` header's own key
    /// (`bare_key`/`dotted_key`/`quoted_key`) is a positional first child,
    /// not a `name`-named field this row's own `name_field` default could
    /// ever resolve. Both node kinds are therefore fully claimed by
    /// [`crate::languages::generic::toml_quirk`] via `on_unmatched_node`
    /// (found by KIND, the same "quirk claims what the flat array cannot"
    /// posture as [`Self::capnp`]) rather than the generic engine's own
    /// field-based class-symbol fallback. Baseline's own `toml_var_types =
    /// {"pair"}` has no corresponding concept in this crate's own
    /// [`LangSpec`] at all (no `var_types` field exists here -- this
    /// crate's nearest analog, `field_types`, is scoped to DEFINES-edge
    /// members inside a class/struct body, a narrower concept than
    /// baseline's own top-level "variable" bookkeeping) -- `pair` is
    /// therefore left unclaimed, matching every other already-onboarded
    /// language in this crate that shares the same gap (e.g.
    /// [`Self::fish`]/[`Self::zsh`]'s own baseline `_var_types` arrays,
    /// also unmodeled here).
    pub const fn toml() -> Self {
        Self {
            name: "toml",
            func_types: &[],
            method_types: &[],
            class_types: &["table", "table_array_element"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "UNUSED_SEE_TOML_QUIRK",
            body_field: "UNUSED_SEE_TOML_QUIRK",
        }
    }

    /// Vue (single-file component, `.vue` -- no extension mapping in this
    /// crate's own [`crate::parsers::classify`] yet; reached only by a
    /// caller invoking [`crate::languages::generic::parse_vue`] directly).
    /// Language-parity wave G2.5d. Grammar: `tree-sitter-vue-next` 0.1.0
    /// (`tree-sitter-grammars/tree-sitter-vue`, real crates.io crate,
    /// `tree-sitter-language`-shimmed -- NOT the plain `tree-sitter-vue`/
    /// `tree-sitter-vue-sqry` names, whose own published bindings pin an
    /// incompatible `tree-sitter` version the same way [`Self::ron`]'s own
    /// doc comment documents).
    ///
    /// Baseline's own `CBM_LANG_VUE` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1942-1946) sets
    /// `vue_module_types = {"document"}`, every other array `empty_types`
    /// -- confirmed present in this real grammar's own `node-types.json`
    /// root entry. Baseline ALSO wires a dedicated `vue_embedded_imports`
    /// re-parse of the `<script>` block's raw text with the JS grammar
    /// (`lang_specs.c`:869-872) -- DEFERRED here for the identical reason
    /// [`Self::svelte`]'s own doc comment explains (no embedded-sub-
    /// language re-parse mechanism exists anywhere in this crate yet). The
    /// document root itself is still fully classified.
    pub const fn vue() -> Self {
        Self {
            name: "vue",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// XML (`.xml` -- no extension mapping in this crate's own
    /// [`crate::parsers::classify`] yet; reached only by a caller invoking
    /// [`crate::languages::generic::parse_xml`] directly). Language-parity
    /// wave G2.5d. Grammar: `tree-sitter-xml` 0.7.0's own `LANGUAGE_XML`
    /// entry point (`tree-sitter-grammars/tree-sitter-xml`, real crates.io
    /// crate, `tree-sitter-language`-shimmed; this same crate's sibling
    /// `LANGUAGE_DTD` grammar is out of this row's own scope, matching the
    /// baseline's own choice of not registering a separate DTD language).
    ///
    /// Baseline's own `CBM_LANG_XML` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1906-1909) sets
    /// `xml_module_types = {"document"}` and `xml_class_types =
    /// {"element"}`, every other array `empty_types` -- both confirmed
    /// present in this real grammar's own `node-types.json`, but `element`
    /// is entirely fieldless there (`"fields": {}`): its own tag name lives
    /// TWO levels down, inside its `STag`/`EmptyElemTag` child's own `Name`
    /// child (also both fieldless, confirmed via the real `node-types.json`
    /// -- `Name` is a plain positional child of `STag`/`EmptyElemTag`, not
    /// a `name`-named field). `element` is therefore fully claimed by
    /// [`crate::languages::generic::xml_quirk`] via `on_unmatched_node`
    /// (found by KIND at both levels, the same "quirk claims what the flat
    /// array cannot" posture as [`Self::capnp`]/[`Self::toml`]) rather than
    /// the generic engine's own field-based class-symbol fallback.
    pub const fn xml() -> Self {
        Self {
            name: "xml",
            func_types: &[],
            method_types: &[],
            class_types: &["element"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "UNUSED_SEE_XML_QUIRK",
            body_field: "UNUSED_SEE_XML_QUIRK",
        }
    }

    /// YAML (`.yml`/`.yaml` -- this crate's own [`crate::parsers::classify`]
    /// already maps both extensions to
    /// [`crate::parsers::Language::ConfigYaml`], a separate
    /// content-classification path with NO structural extractor at all;
    /// this row/[`generic::parse_yaml`] is new STRUCTURAL extraction, not
    /// yet wired to that existing dispatch -- same "bigger blast-radius,
    /// deferred to a future wave" reasoning as [`Self::toml`]'s own doc
    /// comment). Reached only by a caller invoking
    /// [`crate::languages::generic::parse_yaml`] directly. Language-parity
    /// wave G2.5d. Grammar: `tree-sitter-yaml` 0.7.2
    /// (`tree-sitter-grammars/tree-sitter-yaml`, real crates.io crate,
    /// `tree-sitter-language`-shimmed).
    ///
    /// Baseline's own `CBM_LANG_YAML` row
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c`:1795-1799) sets
    /// `yaml_module_types = {"stream"}` (NOT `"document"` -- YAML's own
    /// grammar nests a `document` node one level below the real file
    /// root), every other array `empty_types` -- confirmed present in this
    /// real grammar's own `node-types.json` root entry (`stream`, matching
    /// exactly). Baseline's own `yaml_var_types = {"block_mapping_pair"}`
    /// has no corresponding concept in this crate's own [`LangSpec`] at
    /// all -- same unmodeled-`var_types` gap as [`Self::toml`]'s own doc
    /// comment explains.
    pub const fn yaml() -> Self {
        Self {
            name: "yaml",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["stream"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// JSON5 (`.json5`). Language-parity wave G2.5c (Tier-0, nominal).
    /// Grammar VENDORED (see `vendor/tree-sitter-json5-local/`, fetched
    /// from the EXACT same upstream commit the baseline itself vendors:
    /// `Joakker/tree-sitter-json5` @ `aa630ef48903`, per
    /// `codebase-memory-mcp/internal/cbm/vendored/grammars/MANIFEST.md`).
    ///
    /// Baseline's own `json5_module_types` is `["document", NULL]`, but a
    /// real `node-types.json` dump of this exact vendored grammar shows
    /// the root node (`"root": true`) is actually named `"file"`, not
    /// `"document"` -- `"document"` does not appear anywhere in this
    /// grammar's own node-types at all. Corrected here rather than
    /// blindly transcribed (the same class of dead/wrong baseline array
    /// entry this crate's own prior waves have already found and fixed
    /// elsewhere, e.g. Lua's `for_in_statement`). Every other array is
    /// empty, matching baseline's own fully nominal row -- JSON5 has no
    /// func/class/call/import concept this grammar models structurally.
    pub const fn json5() -> Self {
        Self {
            name: "json5",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// KDL (`.kdl`). Language-parity wave G2.5c (Tier-0, nominal).
    /// Grammar VENDORED (see `vendor/tree-sitter-kdl-local/`, fetched
    /// from the EXACT same upstream commit the baseline itself vendors:
    /// `tree-sitter-grammars/tree-sitter-kdl` @ `b37e3d58e5c5`, per the
    /// baseline's own vendored-grammars MANIFEST).
    ///
    /// NOT the same concept as this crate's own `Language::Kustomize`/
    /// `Language::K8s` (Kubernetes YAML manifests) -- KDL ("KDL
    /// Document Language") is an unrelated, distinct config format with
    /// its own dedicated grammar; the baseline's own `CBM_LANG_KDL` row
    /// is exactly this grammar, confirmed via its own `cbm.h` enum and
    /// `lang_specs.c` row (`kdl_module_types`, unrelated to
    /// `CBM_LANG_K8S`/`CBM_LANG_KUSTOMIZE`, which instead reuse the YAML
    /// grammar entirely -- see this crate's own deferred-language notes
    /// for those two).
    ///
    /// `kdl_module_types` is `["document", NULL]` in baseline, confirmed
    /// real and unchanged here: `document` is genuinely this grammar's
    /// own root rule (confirmed directly off its own `grammar.js`, first
    /// key of the `rules` object). Every other array is empty, matching
    /// baseline's own fully nominal row.
    pub const fn kdl() -> Self {
        Self {
            name: "kdl",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Linker Script (GNU ld `.ld`/`.lds`/`.x` scripts). Language-parity
    /// wave G2.5c (Tier-0, nominal). Grammar VENDORED (see
    /// `vendor/tree-sitter-linkerscript-local/`, fetched from the EXACT
    /// same upstream commit the baseline itself vendors:
    /// `tree-sitter-grammars/tree-sitter-linkerscript` @ `f99011a35542`,
    /// per the baseline's own vendored-grammars MANIFEST).
    ///
    /// Baseline's own `linkerscript_module_types` is
    /// `["source_file", NULL]`, but this grammar has no `source_file`
    /// node kind at all -- its own `grammar.js` names its root rule
    /// `linkerscript` (the first key of its own `rules` object, same as
    /// the grammar's own `name:` field), confirmed directly off both the
    /// generated `parser.c`'s own `sym_linkerscript` symbol AND a real
    /// `node-types.json` entry for `"type": "linkerscript"`. Corrected
    /// here rather than blindly transcribed. `linkerscript_call_types`
    /// (`["call_expression", NULL]`) IS real and kept as-is: a real
    /// `node-types.json` dump confirms `call_expression` carries real
    /// `function`/`arguments` fields, so the generic engine's own
    /// field-driven default (no quirk needed) extracts these calls
    /// (e.g. `ASSERT(...)`, `DEFINED(...)` linker-script builtins)
    /// exactly like any other language's real-fielded call node.
    pub const fn linkerscript() -> Self {
        Self {
            name: "linkerscript",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["linkerscript"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Liquid (Shopify template language, `.liquid`). Language-parity
    /// wave G2.5c (Tier-0, nominal). Grammar VENDORED (see
    /// `vendor/tree-sitter-liquid-local/`, fetched from the EXACT same
    /// upstream commit the baseline itself vendors:
    /// `hankthetank27/tree-sitter-liquid` @ `9566ca799110`, per the
    /// baseline's own vendored-grammars MANIFEST).
    ///
    /// Baseline's own `liquid_module_types` is `["template", NULL]`,
    /// but a real `node-types.json` dump of this exact vendored grammar
    /// shows the root node (`"root": true`) is actually named
    /// `"program"` -- `"template"` does not appear anywhere in this
    /// grammar's own node-types at all. Corrected here rather than
    /// blindly transcribed.
    ///
    /// `liquid_import_types` is `["include", "include_statement",
    /// NULL]` in baseline. `include_statement` is real (confirmed:
    /// carries a `string` child holding the included template's path --
    /// see [`crate::languages::generic::liquid_quirk`], needed because
    /// the generic walker's own `import_types` branch has no
    /// field-driven default at all, the same finding
    /// [`Self::devicetree`]'s own doc comment already established).
    /// Plain `"include"` is dropped: this grammar's own node-types.json
    /// lists it as an UNNAMED token (`"named": false`, the bare
    /// `include` keyword itself, not a distinct statement node) --
    /// unreachable by this crate's named-node-keyed walker, the same
    /// class of dead baseline array entry [`Self::json5`]'s own doc
    /// comment documents for a different grammar.
    pub const fn liquid() -> Self {
        Self {
            name: "liquid",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["include_statement"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Markdown (`.md`). Language-parity wave G2.5c (Tier-0, nominal).
    /// Grammar VENDORED (see `vendor/tree-sitter-markdown-local/`,
    /// fetched from the EXACT same upstream commit the baseline itself
    /// vendors: `tree-sitter-grammars/tree-sitter-markdown` @
    /// `f969cd3ae3f9`, per the baseline's own vendored-grammars
    /// MANIFEST -- specifically the `tree-sitter-markdown/` block-grammar
    /// subdirectory of that monorepo, NOT the sibling inline grammar;
    /// this crate only needs block-level heading structure).
    ///
    /// `markdown_module_types` (`["document", NULL]`) and
    /// `markdown_class_types` (`["atx_heading", "setext_heading",
    /// NULL]`) are both real, confirmed directly off this exact
    /// vendored grammar's own `node-types.json`: `document` is the real
    /// (and only) root node, and both heading kinds are real named
    /// nodes. Neither heading kind has a `name`-named field, though
    /// (each carries its own `heading_content` field instead, pointing
    /// at an `inline`/`paragraph` node holding the heading's own text)
    /// -- the generic engine's own class-handling default (a
    /// `name_field`-keyed lookup) cannot extract this, so
    /// [`crate::languages::generic::markdown_quirk`] claims both heading
    /// kinds directly via `heading_content` instead, pushing each as a
    /// [`crate::parsers::SymbolKind::Class`] symbol (matching baseline's
    /// own choice of treating a heading as this row's one "class-shaped"
    /// construct -- a Markdown file's headings are its closest analogue
    /// to a nominal structural outline).
    pub const fn markdown() -> Self {
        Self {
            name: "markdown",
            func_types: &[],
            method_types: &[],
            class_types: &["atx_heading", "setext_heading"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Mermaid (diagram-as-code, `.mmd`/`.mermaid`). Language-parity
    /// wave G2.5c (Tier-0, nominal). Grammar VENDORED (see
    /// `vendor/tree-sitter-mermaid-local/`, fetched from the EXACT same
    /// upstream commit the baseline itself vendors: `monaqa/tree-sitter-
    /// mermaid` @ `90ae195b3193`, per the baseline's own
    /// vendored-grammars MANIFEST).
    ///
    /// `mermaid_module_types` (`["source_file", NULL]`) is real,
    /// confirmed directly off this exact vendored grammar's own
    /// `grammar.js` (`source_file` is the first/root key of its own
    /// `rules` object). Every other array is empty, matching baseline's
    /// own fully nominal row -- a mermaid diagram has no func/class/
    /// call/import concept this grammar models structurally at all (it
    /// is a diagram-description DSL, not a programming language).
    pub const fn mermaid() -> Self {
        Self {
            name: "mermaid",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// PO (gettext translation catalog, `.po`). Language-parity wave
    /// G2.5c (Tier-0, nominal). Grammar VENDORED (see
    /// `vendor/tree-sitter-po-local/`, fetched from the EXACT same
    /// upstream commit the baseline itself vendors:
    /// `tree-sitter-grammars/tree-sitter-po` @ `bd860a0f57f6`, per the
    /// baseline's own vendored-grammars MANIFEST).
    ///
    /// `po_module_types` (`["source_file", NULL]`) is real, confirmed
    /// directly off this exact vendored grammar's own `grammar.js`
    /// (`source_file` is the root key of its own `rules` object). Every
    /// other array is empty, matching baseline's own fully nominal row
    /// -- a `.po` catalog's `msgid`/`msgstr` entries have no func/class/
    /// call/import analogue this grammar models structurally.
    pub const fn po() -> Self {
        Self {
            name: "po",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Java/Jakarta `.properties` (`.properties`). Language-parity wave
    /// G2.5c (Tier-0, nominal). Grammar: `tree-sitter-properties` 0.3.0,
    /// a real crates.io crate already depending on the
    /// `tree-sitter-language` ABI-stable shim (no vendoring needed,
    /// unlike this wave's other eight languages).
    ///
    /// Baseline's own `properties_module_types` is `["file",
    /// "source_file", NULL]`; this crate's own root rule is confirmed
    /// (directly off its own `grammar.js`) to be `file` only --
    /// `source_file` does not exist in this grammar at all, dropped as
    /// dead/unreachable (same class of extra baseline array entry
    /// [`Self::json5`]'s own doc comment already documents). Baseline's
    /// own `properties_var_types` (`["property", NULL]`) is real (a
    /// real, fieldless `property` node with `key`/`value` children) but
    /// is NOT mapped onto this row's [`Self::field_types`]: a
    /// `.properties` file's root has no class/module-shaped container a
    /// `DEFINES` edge could attach to (the generic engine's own
    /// `field_types` branch requires a non-`None` `enclosing` container
    /// name), the identical reasoning [`Self::wgsl`]'s own row already
    /// applies to drop its baseline `wgsl_var_types` array rather than
    /// invent a container that does not exist.
    pub const fn properties() -> Self {
        Self {
            name: "properties",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Regex (standalone regular-expression pattern, `.regex`).
    /// Language-parity wave G2.5c (Tier-0, nominal). Grammar:
    /// `tree-sitter-regex` 0.25.0, a real crates.io crate already
    /// depending on the `tree-sitter-language` ABI-stable shim (no
    /// vendoring needed).
    ///
    /// `regex_module_types` (`["pattern", NULL]`) is real, confirmed
    /// directly off this exact crate's own `grammar.js` (`pattern` is
    /// the root key of its own `rules` object). Every other array is
    /// empty, matching baseline's own fully nominal row -- a bare regex
    /// pattern has no func/class/call/import concept this grammar
    /// models structurally at all.
    pub const fn regex() -> Self {
        Self {
            name: "regex",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["pattern"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// gitignore (`.gitignore`). Language-parity wave G2.5b (Tier-0,
    /// final language batch). Grammar VENDORED (see
    /// `vendor/tree-sitter-gitignore-local/`; no crates.io
    /// `tree-sitter-gitignore` crate exists at all).
    ///
    /// `gitignore_module_types` (`["document", NULL]`) is real,
    /// confirmed via a real parse-tree dump of `node_modules/\n*.log\n!
    /// important.log\n# comment\n/build\n` -- root kind is `document`,
    /// direct children are `pattern`/`comment`, both completely
    /// fieldless (`directory_flag`/`negation`/`relative_flag`/
    /// `wildcard_chars` are the only labeled children observed, none of
    /// which this crate's own narrower [`LangSpec`] shape has anywhere
    /// to record -- no bare "ignore rule" symbol kind exists). Matches
    /// baseline's own fully nominal row exactly: every other array
    /// empty.
    pub const fn gitignore() -> Self {
        Self {
            name: "gitignore",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// GN (Generate Ninja build-config language, `.gn`/`.gni`).
    /// Language-parity wave G2.5b. Grammar VENDORED (see
    /// `vendor/tree-sitter-gn-local/`; the published `tree-sitter-gn`
    /// 1.0.0 crate pins `cc = "~1.0.83"` as a normal build dependency,
    /// a real whole-workspace dependency-graph conflict against
    /// `tree-sitter-just`'s own `cc = "^1.2.25"` requirement -- see
    /// that vendor crate's own module doc).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of
    /// `import("//build/config.gni")\n\nexecutable("foo") {\n  sources
    /// = [ "a.cc" ]\n  if (is_linux) {\n    deps = [ ":bar" ]\n  }\n
    /// foreach(f, sources) {\n    print(f)\n  }\n}\n`:
    /// - `module_types` is `["source_file"]`, the real root kind --
    ///   matches baseline.
    /// - `call_types` is `["call_expression"]`, real, with a real
    ///   `"function"` field (confirmed) but NO real `"arguments"`
    ///   field (its own argument list is a bare positional child) --
    ///   `call_arguments_field` is therefore unreachable here and
    ///   `arg_texts` is always empty, harmless, no test asserts on it
    ///   (same shape as [`Self::matlab`]'s own documented
    ///   `function_call` gap).
    /// - `import_types` is `["import_statement"]`, real, matching one
    ///   of baseline's own two entries (`gn_import_types: {
    ///   "import_statement", "import", NULL}`) -- the second,
    ///   `"import"`, names a node kind that does not exist anywhere in
    ///   this real grammar at all (confirmed absent from the parse
    ///   tree), the same class of dead baseline array entry
    ///   [`Self::lua`]'s own doc comment already documents for
    ///   `for_in_statement`. `import_statement` itself is completely
    ///   fieldless (confirmed) -- needs
    ///   [`crate::languages::generic::gn_quirk`] regardless, the same
    ///   "every import-shaped row needs a quirk claim" finding
    ///   [`Self::devicetree`]'s own doc comment documents.
    /// - `branch_types` is `["if_statement", "foreach_statement"]`,
    ///   both real, matching baseline exactly, with real
    ///   `"condition"`/`"consequence"` and `"item"`/`"list"` fields
    ///   respectively (documentary metadata only -- the generic engine
    ///   does not yet consume `branch_types` for anything functional).
    pub const fn gn() -> Self {
        Self {
            name: "gn",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &["call_expression"],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["import_statement"],
            branch_types: &["if_statement", "foreach_statement"],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Go Mod (`go.mod` file grammar -- distinct from the ordinary Go-
    /// source grammar). Language-parity wave G2.5b. Grammar VENDORED
    /// (see `vendor/tree-sitter-gomod-local/`; the published
    /// `tree-sitter-gomod` 1.0.1 crate's own Rust binding returns a
    /// `tree_sitter::Language` from a SEPARATE, incompatible
    /// `tree-sitter` version resolved as a normal (non-dev) dependency
    /// -- see that vendor crate's own module doc for the type-mismatch
    /// finding).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of
    /// `module example.com/foo\n\ngo 1.21\n\nrequire
    /// github.com/bar/baz v1.2.3\n\nrequire (\n\tgithub.com/x/y
    /// v0.1.0\n)\n\nreplace github.com/bar/baz => ../baz\n`:
    /// - `module_types` is `["source_file"]`, the real root kind --
    ///   matches baseline.
    /// - `import_types` is `["require_directive"]` -- a REAL,
    ///   deliberate CORRECTION of baseline's own `gomod_import_types`
    ///   (`{"require", NULL}`): no `"require"` node kind exists
    ///   anywhere in this real grammar at all (confirmed absent from
    ///   the parse tree, both for the single-line AND the
    ///   parenthesized-block form) -- the real node is
    ///   `require_directive`, wrapping a `require_spec` child whose own
    ///   `module_path` child holds the dependency path text. Baseline's
    ///   own `gomod_var_types` (`{"require_directive",
    ///   "replace_directive", NULL}`) has no equivalent in this crate's
    ///   own narrower [`LangSpec`] shape at all (no bare top-level
    ///   Variable-symbol concept here, the same accepted departure
    ///   [`Self::r`]'s own doc comment already documents for its own
    ///   dropped `r_var_types`) -- rather than drop `require_directive`
    ///   entirely the way a literal var-types-has-no-home read would
    ///   suggest, this row promotes it to a real IMPORTS edge instead
    ///   (matches what baseline is clearly modeling semantically: a
    ///   `require` line IS a dependency edge), while deliberately
    ///   leaving `replace_directive` unclaimed -- a replace directive
    ///   is a substitution, not an additional dependency, matching
    ///   baseline's own semantic distinction between the two node
    ///   kinds even though this crate cannot reproduce its `var_types`
    ///   bucket literally.
    pub const fn gomod() -> Self {
        Self {
            name: "gomod",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["require_directive"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// GraphQL (SDL, `.graphql`/`.gql`). Language-parity wave G2.5b.
    /// Grammar: `tree-sitter-graphql` 0.1.0, a real crates.io crate
    /// already depending on the `tree-sitter-language` ABI-stable shim
    /// (no vendoring needed).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of
    /// `type Query {\n  user(id: ID!): User\n}\n\ntype User {\n  id:
    /// ID!\n  name: String\n}\n` AND this crate's own
    /// `src/node-types.json`:
    /// - The real root kind is `source_file`, wrapping a `document`
    ///   node (`source_file: ($) => $.document` in this grammar's own
    ///   `grammar.js`) -- `module_types` is `["document"]`, matching
    ///   baseline exactly; the outer `source_file` wrapper needs no
    ///   entry of its own, since the generic engine's own universal
    ///   `on_unmatched_node`-then-`walk_children` fallback already
    ///   recurses through any unmatched node kind, reaching `document`
    ///   one level down regardless.
    /// - `class_types` is SIX of baseline's own seven
    ///   `graphql_class_types` entries --
    ///   `object_type_definition`/`input_object_type_definition`/
    ///   `enum_type_definition`/`interface_type_definition`/
    ///   `union_type_definition`/`scalar_type_definition`, all real --
    ///   deliberately DROPPING baseline's seventh, `type_definition`:
    ///   that node is a transparent WRAPPER around each of the other
    ///   six (confirmed both in the real parse tree, where every
    ///   `object_type_definition` etc. is nested one level inside a
    ///   `type_definition`, and in this grammar's own
    ///   `type_system_definition` rule), not itself a concrete
    ///   definition -- keeping it in `class_types` would double-push a
    ///   symbol for the wrapper AND its real child, since neither this
    ///   row's `name_field` nor [`crate::languages::generic::graphql_quirk`]
    ///   can find a name on the wrapper itself (it has none), and it
    ///   would just fall through to generic recursion anyway (the same
    ///   "baseline sometimes includes a redundant container name"
    ///   finding this crate has caught before). None of these six has
    ///   a real `name_field` (every one has an empty `"fields": {}` in
    ///   this crate's own `node-types.json`) -- handled entirely by
    ///   [`crate::languages::generic::graphql_quirk`] instead, which
    ///   finds each one's own `name` child by KIND (a direct, not
    ///   recursive, child search) and manually walks its
    ///   `fields_definition`/`input_fields_definition` child (if any)
    ///   to record DEFINES edges for each member.
    /// - `field_types` is `["field_definition", "input_value_definition"]`,
    ///   matching baseline's own `graphql_field_types` exactly -- BOTH
    ///   real node kinds, but this row's own `field_types` array is
    ///   documentary only: [`crate::languages::generic::graphql_quirk`]
    ///   claims every `class_types` node WHOLESALE (returns `true`),
    ///   so the generic engine's own separate `field_types` branch
    ///   never actually runs for either kind (the quirk's own manual
    ///   walk records their DEFINES edges directly, the same posture
    ///   [`Self::smithy`]'s own doc comment describes for its own
    ///   `field_types` array).
    pub const fn graphql() -> Self {
        Self {
            name: "graphql",
            func_types: &[],
            method_types: &[],
            class_types: &[
                "object_type_definition",
                "input_object_type_definition",
                "enum_type_definition",
                "interface_type_definition",
                "union_type_definition",
                "scalar_type_definition",
            ],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &["field_definition", "input_value_definition"],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// HTML (`.htm`/`.html`). Language-parity wave G2.5b. Grammar:
    /// `tree-sitter-html` 0.23.2, a real crates.io crate already
    /// depending on the `tree-sitter-language` ABI-stable shim (no
    /// vendoring needed).
    ///
    /// `html_module_types` (`["document", NULL]`) is real, confirmed
    /// via a real parse-tree dump -- root kind is `document`. Every
    /// other array is empty, matching baseline's own fully nominal row
    /// exactly: baseline's separate `html_embedded_imports` mechanism
    /// (re-parsing `<script>` bodies with the JS grammar) is DEFERRED,
    /// not modeled here at all -- this crate has no embedded-sub-
    /// language-reparse infrastructure yet (see
    /// [`crate::languages::generic::parse_html`]'s own doc comment).
    pub const fn html() -> Self {
        Self {
            name: "html",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Hyprlang (Hyprland window-manager config language, `.hl`).
    /// Language-parity wave G2.5b. Grammar VENDORED (see
    /// `vendor/tree-sitter-hyprlang-local/`; no crates.io
    /// `tree-sitter-hyprlang` crate exists at all).
    ///
    /// `module_types` is `["configuration"]` -- a REAL, deliberate
    /// CORRECTION of baseline's own `hyprlang_module_types`
    /// (`{"source_file", NULL}`): confirmed via a real parse-tree dump
    /// of `monitor=eDP-1,1920x1080@60,0x0,1\n\ngeneral {\n  gaps_in =
    /// 5\n}\n\nbind = SUPER, Q, killactive\n`, the real root kind is
    /// `configuration`, NOT `source_file` -- the same class of dead/
    /// wrong baseline root-kind entry [`Self::sshconfig`]'s own doc
    /// comment already documents for a different language. `keyword`
    /// (a single `key=value`/`key = value, args...` line) and `section`
    /// (a `name { ... }` block, itself containing nested
    /// `assignment`/`keyword` children) both have real fields
    /// (`keyword:`/`value:` and `name:` respectively, confirmed) but
    /// neither maps to any concept this crate's own narrower
    /// [`LangSpec`] shape models (no func/class/call/import structure
    /// at this language's own semantic level) -- matches baseline's
    /// own fully nominal row otherwise: every other array empty.
    pub const fn hyprlang() -> Self {
        Self {
            name: "hyprlang",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["configuration"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// INI (`.cfg`/`.conf`/`.ini`). Language-parity wave G2.5b.
    /// Grammar: `tree-sitter-ini` 1.4.0, a real crates.io crate already
    /// depending on the `tree-sitter-language` ABI-stable shim (no
    /// vendoring needed).
    ///
    /// Every node kind below confirmed via a real parse-tree dump of
    /// `[section]\nkey = value\nother = 1\n\n[section2]\nfoo=bar\n`:
    /// `module_types` is `["document"]`, `class_types` is `["section"]`
    /// -- both real and matching baseline exactly. Baseline's own
    /// `ini_var_types` (`{"setting", NULL}`) has no direct equivalent
    /// in this crate's own narrower [`LangSpec`] shape (a bare
    /// top-level Variable concept, same dropped-array class as
    /// [`Self::r`]'s own `r_var_types`) -- but UNLIKE Go Mod's
    /// `replace_directive` (deliberately left unclaimed, see
    /// [`Self::gomod`]'s own doc comment), every `setting` here DOES
    /// have a real enclosing container (its parent `section`), so
    /// [`crate::languages::generic::ini_quirk`] promotes it to a real
    /// DEFINES edge instead of dropping it entirely -- neither
    /// `section` nor `setting` has any real field of its own though
    /// (confirmed: `section_name`/`setting_name`/`setting_value` are
    /// all bare positional children), so this row's own `field_types`
    /// array is left empty (there is no working generic fallback for
    /// it to document) and the quirk handles both nodes wholesale by
    /// KIND instead. `section_name`'s own real grammar rule
    /// (`tree-sitter-ini`'s own `grammar.js`) is `seq('[',
    /// alias(/[^\[\]]+/, $.text), ']', /\r?\n/)` -- its OWN byte span
    /// includes the brackets AND the trailing newline, confirmed only
    /// by a real hard test catching the wrong "the clean identifier is
    /// section_name's own text" assumption a parse-tree dump's
    /// s-expression rendering alone did not surface; the quirk descends
    /// one level further into `section_name`'s own aliased `text`
    /// child for the clean text. `setting_name` has no such wrapper
    /// (its own grammar rule is a bare top-level `alias(...,
    /// $.setting_name)`, confirmed real -- `.utf8_text()` on it
    /// directly is already clean).
    pub const fn ini() -> Self {
        Self {
            name: "ini",
            func_types: &[],
            method_types: &[],
            class_types: &["section"],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Janet (Lisp-family scripting language, `.janet`). Language-
    /// parity wave G2.5b. Grammar VENDORED (see
    /// `vendor/tree-sitter-janet-local/`; no crates.io
    /// `tree-sitter-janet`/`tree-sitter-janet-simple` crate exists at
    /// all -- the grammar's own generated C function is
    /// `tree_sitter_janet_simple`, see that vendor crate's own module
    /// doc for why).
    ///
    /// `module_types` is `["source"]`, the real root kind, matching
    /// baseline's own `janet_module_types` exactly -- confirmed via a
    /// real parse-tree dump of `(defn foo [x] (+ x 1))\n(print (foo
    /// 2))\n`: every list-shaped form (a `defn`, a call, an arithmetic
    /// expression, a vector literal) is the SAME completely fieldless
    /// `par_tup_lit`/`sqr_tup_lit` node kind regardless of its own head
    /// symbol -- this Lisp-family grammar has no dedicated `defn`- or
    /// call-shaped node kind of its own at all to hang `func_types`/
    /// `call_types` off of. Matches baseline's own fully nominal row
    /// otherwise: every other array empty.
    pub const fn janet() -> Self {
        Self {
            name: "janet",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Jinja2 (`.j2`/`.jinja`/`.jinja2`). Language-parity wave G2.5b.
    /// Grammar: `tree-sitter-jinja2` 0.0.16, a real crates.io crate
    /// already depending on the `tree-sitter-language` ABI-stable shim
    /// (no vendoring needed).
    ///
    /// `module_types` is `["source_file"]`, the real root kind,
    /// matching baseline's own `jinja2_module_types` exactly --
    /// confirmed via a real parse-tree dump of `{% for item in items
    /// %}\n  {{ item.name }}\n{% endfor %}\n{% include "x.html" %}\n`.
    /// Matches baseline's own fully nominal row otherwise: every other
    /// array empty (this grammar's own `statement`/`expression` node
    /// kinds are generic containers for every kind of Jinja tag, not
    /// distinct func/call/import-shaped node kinds this crate's own
    /// [`LangSpec`] arrays could usefully name).
    pub const fn jinja2() -> Self {
        Self {
            name: "jinja2",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// JSDoc (standalone JSDoc comment body, no dedicated file
    /// extension). Language-parity wave G2.5b. Grammar:
    /// `tree-sitter-jsdoc` 0.25.0, a real crates.io crate already
    /// depending on the `tree-sitter-language` ABI-stable shim (no
    /// vendoring needed).
    ///
    /// `module_types` is `["document"]`, the real root kind, matching
    /// baseline's own `jsdoc_module_types` exactly -- confirmed via a
    /// real parse-tree dump of `/**\n * Does a thing.\n * @param
    /// {string} foo desc\n * @returns {number} the count\n */`
    /// (including the surrounding `/**`/`*/` delimiters themselves --
    /// confirmed this grammar accepts and parses the FULL comment,
    /// delimiters included, with no separate stripping step needed by
    /// any caller). Matches baseline's own fully nominal row otherwise:
    /// every other array empty. No [`crate::parsers::classify`]
    /// extension wiring at all
    /// -- see [`crate::languages::generic::parse_jsdoc`]'s own doc
    /// comment for why (no baseline `EXT_TABLE` entry exists for this
    /// language either).
    pub const fn jsdoc() -> Self {
        Self {
            name: "jsdoc",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// JSON (`.json`). Language-parity wave G2.5b. Grammar:
    /// `tree-sitter-json` 0.24.8, a real crates.io crate already
    /// depending on the `tree-sitter-language` ABI-stable shim (no
    /// vendoring needed). Replaces the pre-existing
    /// [`crate::parsers::Language::ConfigJson`] no-op fallback for a
    /// `.json` extension -- see that enum variant's own doc comment.
    ///
    /// `module_types` is `["document"]`, the real root kind, matching
    /// baseline's own `json_module_types` exactly -- confirmed via a
    /// real parse-tree dump of `{"a": 1, "b": [1,2,3], "c": {"nested":
    /// true}}`. Baseline's own `json_var_types` (`{"pair", NULL}`) has
    /// no equivalent in this crate's own narrower [`LangSpec`] shape at
    /// all -- same dropped-array class as [`Self::r`]'s own
    /// `r_var_types`, and UNLIKE INI's `setting` (which gets a real
    /// enclosing `section` container to attach a DEFINES edge to via a
    /// quirk), a JSON `pair` has no class-shaped enclosing container at
    /// all (a bare JSON object is not a [`SymbolKind::Class`] here) --
    /// there is nothing meaningful for a quirk to attach a `pair` to,
    /// so it is left genuinely unextracted rather than forcing a
    /// contrived container.
    pub const fn json() -> Self {
        Self {
            name: "json",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Assembly (`.s`/`.S`). Language-parity wave G2.5a (Tier-0).
    /// Grammar: `tree-sitter-asm` 0.24.0 (`RubixDev/tree-sitter-asm`), a
    /// real crates.io crate depending on the `tree-sitter-language`
    /// ABI-stable shim.
    ///
    /// Baseline's own row (`assembly_func_types`/`assembly_var_types`
    /// both `{"label", NULL}`, `assembly_module_types` `{"program",
    /// NULL}`) classifies every `label` as BOTH a function AND a
    /// variable -- this crate's own generic walker has no such
    /// double-typing concept for one node kind (nor does it consume
    /// `var_types` at all right now, see [`Self::dockerfile`]'s own doc
    /// comment for the established "documentation parity only" note),
    /// so only `func_types` is populated; `label`'s baseline
    /// presence in `assembly_var_types` too is a real, intentional gap
    /// this doc comment records rather than silently drops.
    ///
    /// `module_types` is `["program"]`, the real root kind -- confirmed
    /// via a real parse-tree dump. `label` has NO real `name` FIELD
    /// (confirmed absent from both this grammar's own `node-types.json`
    /// -- its declared `name` field expects a bare `word` node, which
    /// never actually appears for a plain `foo:` label -- and a real
    /// parse-tree dump, where the label's name surfaces as a positional,
    /// fieldless `ident` child instead), so `func_types` claims `label`
    /// entirely via [`crate::languages::generic::assembly_quirk`]
    /// rather than this row's own `name_field` fallback, which would
    /// silently find nothing. Assembly has no call-expression concept
    /// in the baseline's own row either (`empty_types` for `call`) --
    /// `call_types` stays empty, matching that shallow depth exactly; a
    /// `label` marks where a `call foo` target begins but does not
    /// itself wrap the following instructions as children at all (they
    /// are flat siblings of the `program` root), so there is no body to
    /// recurse into even in principle.
    pub const fn assembly() -> Self {
        Self {
            name: "assembly",
            func_types: &["label"],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["program"],
            call_types: &[],
            call_function_field: "UNUSED_SEE_ASSEMBLY_QUIRK",
            call_arguments_field: "UNUSED_SEE_ASSEMBLY_QUIRK",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "UNUSED_SEE_ASSEMBLY_QUIRK",
            body_field: "UNUSED_SEE_ASSEMBLY_QUIRK",
        }
    }

    /// Astro (`.astro`). Language-parity wave G2.5a (Tier-0). Grammar
    /// VENDORED (`crates/enforcer-memory/vendor/tree-sitter-astro-local/`)
    /// -- no `tree-sitter-astro` crate exists on crates.io under any
    /// plausible name, confirmed via `cargo add --dry-run` against this
    /// workspace's own registry index.
    ///
    /// `module_types` is `["document"]`, the real root kind, matching
    /// baseline's own `astro_module_types` exactly -- confirmed via a
    /// real parse-tree dump of a component with a frontmatter fence plus
    /// a `<div>` template body. Baseline's own row additionally declares
    /// `astro_embedded_imports` (re-parsing the frontmatter's own
    /// `frontmatter_js_block` slice, and any `<script>` element's own
    /// `raw_text` slice, with the JS grammar so import statements inside
    /// either become real edges) -- this crate's generic engine has NO
    /// embedded-sub-language-reparse mechanism at all yet (confirmed:
    /// no "embedded" concept anywhere in `languages::generic`'s own
    /// walker), so that richer behavior is a genuine, documented gap
    /// (an engine-level prerequisite, not something a per-language
    /// `LangSpec` row or quirk could add) rather than a silent drop.
    /// Nothing else in baseline's own row is populated (no func/class/
    /// call/branch arrays at all), matching Tier-0's nominal depth
    /// exactly.
    pub const fn astro() -> Self {
        Self {
            name: "astro",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Beancount (`.beancount`), a plain-text double-entry-accounting
    /// ledger format. Language-parity wave G2.5a (Tier-0). Grammar
    /// VENDORED (`crates/enforcer-memory/vendor/tree-sitter-beancount-local/`)
    /// -- the published `tree-sitter-beancount` 2.5.1 crate hard-pins
    /// `tree-sitter = "~0.26.3"` as a real (non-optional; its own
    /// `tree-sitter-language` Cargo feature does NOT gate this
    /// dependency away, confirmed via
    /// `cargo add --no-default-features --features tree-sitter-language`
    /// still failing the identical `links = "tree-sitter"` resolve
    /// conflict) dependency, incompatible with this workspace's
    /// `tree-sitter = "0.25"` core.
    ///
    /// `module_types` is `["file"]`, the real root kind, matching
    /// baseline's own `beancount_module_types` exactly -- confirmed via
    /// a real parse-tree dump of an `include`/`open`/dated `transaction`
    /// with a `posting`. `import_types` is `["include"]`, also matching
    /// baseline's own `beancount_import_types` exactly and confirmed
    /// present in the real grammar's own `node-types.json` (a single
    /// required, fieldless `string` child) --
    /// [`crate::languages::generic::beancount_quirk`] claims it to read
    /// that positional child directly (this row's own generic
    /// `import_types` handling has no field-based fallback of its own at
    /// all, see `generic::walk`'s own doc comment on that branch).
    /// Nothing else in baseline's own row is populated.
    pub const fn beancount() -> Self {
        Self {
            name: "beancount",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &["include"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// BibTeX (`.bib`). Language-parity wave G2.5a (Tier-0). Grammar
    /// VENDORED (`crates/enforcer-memory/vendor/tree-sitter-bibtex-local/`)
    /// -- the published `tree-sitter-bibtex` 0.1.0 crate hard-pins
    /// `tree-sitter = "0.22.6"` as a normal (non-dev) dependency,
    /// predating the `tree-sitter-language` ABI-stable shim convention.
    ///
    /// `module_types` is `["document"]`, the real root kind, matching
    /// baseline's own `bibtex_module_types` exactly -- confirmed via a
    /// real parse-tree dump of an `@article{...}` entry. `call_types`
    /// is `["command"]`, matching baseline's own `bibtex_call_types`
    /// exactly and confirmed present in the real grammar's own
    /// `node-types.json` with a genuine REQUIRED `name` field (a
    /// `command_name` node -- BibTeX/LaTeX's own backslash-macro
    /// invocation form, e.g. `{\LaTeX}` inside a field value) -- unlike
    /// most Tier-0 languages in this file, this call shape needs NO
    /// quirk at all: `call_function_field: "name"` resolves through the
    /// ordinary generic path. There is no corresponding `arguments`
    /// field on this node (its own children -- `brace_word`/`command`/
    /// `quote_word` -- are purely positional), so
    /// `call_arguments_field` is left at a placeholder;
    /// `generic::call_arg_texts` already degrades gracefully to an
    /// empty argument list when the named field is absent, matching
    /// this grammar's own shallow reality rather than needing a quirk to
    /// avoid a false extraction.
    pub const fn bibtex() -> Self {
        Self {
            name: "bibtex",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &["command"],
            call_function_field: "name",
            call_arguments_field: "UNUSED_NO_ARGUMENTS_FIELD",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Blade (Laravel's template language, `.blade.php`). Language-
    /// parity wave G2.5a (Tier-0). Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-blade-local/`) -- no
    /// `tree-sitter-blade` crate exists on crates.io under any plausible
    /// name, confirmed via `cargo add --dry-run` against this
    /// workspace's own registry index.
    ///
    /// `module_types` is `["document"]`, the real root kind, matching
    /// baseline's own `blade_module_types` exactly -- confirmed via a
    /// real parse-tree dump of an `@if`/`@endif` directive wrapping a
    /// `{{ $x }}` PHP-echo statement inside an HTML element. Nothing
    /// else in baseline's own row is populated at all, matching Tier-0's
    /// nominal depth exactly -- despite this grammar embedding a full
    /// PHP sub-grammar (hence its unusually large vendored `parser.c`),
    /// baseline extracts no structure from it whatsoever.
    pub const fn blade() -> Self {
        Self {
            name: "blade",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// CSS (`.css`). Language-parity wave G2.5a (Tier-0). Grammar:
    /// `tree-sitter-css` 0.25.0 (`tree-sitter/tree-sitter-css`, the
    /// official grammar), a real crates.io crate depending on the
    /// `tree-sitter-language` ABI-stable shim.
    ///
    /// `module_types` is `["stylesheet"]`, matching baseline's own
    /// `css_module_types` exactly -- confirmed via a real parse-tree
    /// dump of an `@import` rule plus a `.a { color: red; }` rule set.
    /// `call_types` is `["call_expression"]` (e.g. `calc(...)`,
    /// `rgb(...)`), matching baseline's own `css_call_types` exactly and
    /// confirmed present in the real grammar's own `node-types.json` --
    /// but that node has NO fields at all (`"fields": {}`; its
    /// `function_name`/`arguments` children are purely positional), so
    /// [`crate::languages::generic::css_call_override`] claims it
    /// directly rather than relying on this row's own
    /// `call_function_field`/`call_arguments_field`, which would find
    /// nothing. `import_types` is `["import_statement"]`, matching
    /// baseline's own `css_import_types` exactly; that node is ALSO
    /// fully fieldless (its own quoted path is one of many positional
    /// alternative child kinds, typically `string_value`) --
    /// [`crate::languages::generic::css_import_quirk`] claims it to find
    /// the first `string_value` child positionally.
    pub const fn css() -> Self {
        Self {
            name: "css",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["stylesheet"],
            call_types: &["call_expression"],
            call_function_field: "UNUSED_SEE_CSS_CALL_QUIRK",
            call_arguments_field: "UNUSED_SEE_CSS_CALL_QUIRK",
            import_types: &["import_statement"],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// CSV (`.csv`). Language-parity wave G2.5a (Tier-0). Grammar
    /// VENDORED (`crates/enforcer-memory/vendor/tree-sitter-csv-local/`)
    /// -- the published `tree-sitter-csv` 1.2.0 crate's own binding pins
    /// `cc = "~1.0.82"` as a normal (non-build) dependency, conflicting
    /// (via cargo's `links = "tree-sitter"` single-version-in-graph
    /// rule together with this workspace's own `tree-sitter = "0.25"`
    /// core's transitive `cc` requirement) with the rest of this
    /// workspace -- confirmed via an isolated `cargo build` in a scratch
    /// crate depending on both. That published crate bundles THREE
    /// sibling grammars (CSV/PSV/TSV) in one package; only the `csv/`
    /// sub-grammar is vendored here.
    ///
    /// `module_types` is `["document"]`, the real root kind, matching
    /// baseline's own `csv_module_types` exactly -- confirmed via a real
    /// parse-tree dump. This specific grammar's own `field` node only
    /// accepts quoted-string or numeric tokens, not bare unquoted words
    /// (confirmed: a bare-word CSV row like `a,b,c` produces `ERROR`
    /// nodes in a real parse-tree dump) -- a real, pre-existing grammar
    /// limitation this row does not attempt to work around; this
    /// crate's own fixture uses quoted/numeric fields to parse cleanly.
    /// Nothing else in baseline's own row is populated at all.
    pub const fn csv() -> Self {
        Self {
            name: "csv",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Diff/patch (`.diff`/`.patch`). Language-parity wave G2.5a
    /// (Tier-0). Grammar: `tree-sitter-diff` 0.1.0
    /// (`tree-sitter/tree-sitter-diff`), a real crates.io crate
    /// depending on the `tree-sitter-language` ABI-stable shim.
    ///
    /// `module_types` is `["source"]`, the real root kind, matching
    /// baseline's own `diff_module_types` exactly -- confirmed via a
    /// real parse-tree dump of a `diff --git`/`---`/`+++`/`@@`/`-`/`+`
    /// unified-diff fragment (that same dump's `index ...` line produces
    /// a real `ERROR` node -- a pre-existing grammar limitation for that
    /// specific line format, not a bug in this row). `call_types` is
    /// `["command"]` (the `diff --git a/x b/x` header line itself),
    /// matching baseline's own `diff_call_types` exactly and confirmed
    /// present in the real grammar's own `node-types.json` -- but that
    /// node has NO fields at all (its `argument`/`filename` children are
    /// purely positional), so
    /// [`crate::languages::generic::diff_call_override`] claims it
    /// directly. This is a nominal reuse of the call-edge shape for a
    /// command invocation, not a real function call -- matching
    /// baseline's own choice to model it that way rather than inventing
    /// a stricter interpretation.
    pub const fn diff() -> Self {
        Self {
            name: "diff",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source"],
            call_types: &["command"],
            call_function_field: "UNUSED_SEE_DIFF_CALL_QUIRK",
            call_arguments_field: "UNUSED_SEE_DIFF_CALL_QUIRK",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// Dockerfile (bare filename `Dockerfile`, or `.dockerfile`).
    /// Language-parity wave G2.5a (Tier-0). Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-dockerfile-local/`)
    /// -- the published `tree-sitter-dockerfile` 0.2.0 crate hard-pins
    /// `tree-sitter = "0.20"` as a normal dependency, predating the
    /// `tree-sitter-language` ABI-stable shim convention.
    ///
    /// `module_types` is `["source_file"]`, the real root kind, matching
    /// baseline's own `dockerfile_module_types` exactly -- confirmed via
    /// a real parse-tree dump of a `FROM`/`ENV`/`ARG`/`RUN` sequence.
    /// `var_types` is `["env_instruction", "arg_instruction"]`, matching
    /// baseline's own `dockerfile_var_types` exactly and confirmed
    /// present with real `name`/`value` (`env_instruction`) and
    /// `name`/`default` (`arg_instruction`) fields -- but this crate's
    /// own generic walker does not consume `var_types` at all yet (no
    /// "var" handling anywhere in `languages::generic`'s own walk
    /// function; confirmed by direct inspection), the same
    /// "documentation parity only, no functional effect" status
    /// `code_graph.rs`'s own complexity-deferral comments already note
    /// for every other Tier-0/1/2 language's `branch_types` -- so this
    /// array is kept for baseline-array parity/documentation but adds
    /// no extraction on its own. Nothing else in baseline's own row is
    /// populated.
    pub const fn dockerfile() -> Self {
        Self {
            name: "dockerfile",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["source_file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// DotEnv (bare filename `.env`, or any `*.env` suffix). Language-
    /// parity wave G2.5a (Tier-0). Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-dotenv-local/`) -- no
    /// `tree-sitter-dotenv` crate exists on crates.io under any
    /// plausible name.
    ///
    /// Baseline's own `dotenv_module_types` array is `{"source_file",
    /// NULL}` -- but this grammar's REAL root node kind is `"document"`,
    /// NOT `"source_file"` (confirmed via a real parse-tree dump of a
    /// `KEY=value`/`# comment`/`KEY="quoted"` sequence) -- corrected
    /// here rather than blindly transcribed, same class of baseline
    /// staleness this file's own [`Self::lua`]/[`Self::bash`] doc
    /// comments already document for their own languages. Nothing else
    /// in baseline's own row is populated at all (no func/call/import/
    /// var arrays whatsoever), matching Tier-0's nominal depth exactly
    /// -- despite this grammar's own `assignment` node having real
    /// `key`/`value` fields this row deliberately does NOT also expose
    /// as `var_types`, since baseline itself does not either (that same
    /// real dump also shows a plain `export FOO=1` form is a grammar
    /// parse ERROR for this specific grammar -- a pre-existing
    /// limitation, not a bug introduced here).
    pub const fn dotenv() -> Self {
        Self {
            name: "dotenv",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["document"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }

    /// gitattributes (bare filename `.gitattributes`). Language-parity
    /// wave G2.5a (Tier-0). Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-gitattributes-local/`)
    /// -- the published `tree-sitter-gitattributes` 0.1.6 crate hard-pins
    /// `tree-sitter = "~0.20.10"` as a normal dependency, predating the
    /// `tree-sitter-language` ABI-stable shim convention.
    ///
    /// Baseline's own `gitattributes_module_types` array is `{"source",
    /// NULL}` -- but this grammar's REAL root node kind is `"file"`, NOT
    /// `"source"` (confirmed via a real parse-tree dump of a
    /// `*.rs text eol=lf`/`*.png binary` pair of pattern-attribute
    /// lines) -- corrected here rather than blindly transcribed, same
    /// class of finding as [`Self::dotenv`]'s own doc comment. Nothing
    /// else in baseline's own row is populated at all.
    pub const fn gitattributes() -> Self {
        Self {
            name: "gitattributes",
            func_types: &[],
            method_types: &[],
            class_types: &[],
            interface_types: &[],
            enum_types: &[],
            alias_types: &[],
            field_types: &[],
            module_types: &["file"],
            call_types: &[],
            call_function_field: "function",
            call_arguments_field: "arguments",
            import_types: &[],
            branch_types: &[],
            decorator_types: &[],
            name_field: "name",
            body_field: "body",
        }
    }
}
