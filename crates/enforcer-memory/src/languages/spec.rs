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
}
