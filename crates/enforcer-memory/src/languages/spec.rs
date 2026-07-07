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
//! G1 scope note: every one of our existing 10 languages has a
//! `LangSpec` row below (node-kind names read off the current bespoke
//! extractor in `languages/*.rs`). Go is fully routed through the
//! generic walker (see `generic::parse_go`, wired into
//! [`crate::parsers::parse_file`]) as this wave's zero-regression
//! proof; the other 9 rows are data-complete but not yet dispatched
//! through the generic path -- their bespoke extractors keep running
//! unchanged. Cutting them over is mechanical repetition of the same
//! Go migration (route via `generic::parse_with_spec` + a quirk
//! closure for whatever that language's rich behaviors need) but is
//! left as explicit follow-up so this wave's diff stays reviewable and
//! the zero-regression bar stays provable one language at a time, per
//! this wave's own migration strategy ("port ONE language first ...
//! then the rest").

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
            field_types: &["field_declaration"],
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
            field_types: &["public_field_definition"],
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
            method_types: &["method_declaration", "constructor_declaration"],
            class_types: &["class_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &[],
            field_types: &["field_declaration"],
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

    /// C: node kinds read off `languages/c.rs`.
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
            name_field: "declarator",
            body_field: "body",
        }
    }

    /// C++: node kinds read off `languages/cpp.rs`.
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
            name_field: "declarator",
            body_field: "body",
        }
    }

    /// C#: node kinds read off `languages/csharp.rs`.
    pub const fn csharp() -> Self {
        Self {
            name: "csharp",
            func_types: &["local_function_statement"],
            method_types: &["method_declaration", "constructor_declaration"],
            class_types: &["class_declaration", "struct_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &[],
            field_types: &["field_declaration", "property_declaration"],
            module_types: &["namespace_declaration", "file_scoped_namespace_declaration"],
            call_types: &["invocation_expression"],
            call_function_field: "function",
            call_arguments_field: "argument_list",
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

    /// PHP: node kinds read off `languages/php.rs`.
    pub const fn php() -> Self {
        Self {
            name: "php",
            func_types: &["function_definition"],
            method_types: &["method_declaration"],
            class_types: &["class_declaration"],
            interface_types: &["interface_declaration"],
            enum_types: &["enum_declaration"],
            alias_types: &[],
            field_types: &["property_declaration"],
            module_types: &["namespace_definition"],
            call_types: &["function_call_expression", "member_call_expression"],
            call_function_field: "function",
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
