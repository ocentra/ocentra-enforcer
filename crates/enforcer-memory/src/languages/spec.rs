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
}
