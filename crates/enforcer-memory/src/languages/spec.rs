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
}
