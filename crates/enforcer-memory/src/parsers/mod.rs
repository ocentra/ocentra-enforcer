//! Language-agnostic parse result shape, plus the extension-based
//! dispatch that routes a file's content to the right
//! [`crate::languages`] extractor (or the `TextOnly` fallback).
//!
//! [`crate::code_graph`] never touches a tree-sitter tree directly: it
//! only ever sees a [`ParsedFile`], so adding a new language is a
//! change fully contained to `languages/` + the one dispatch arm in
//! [`parse_file`] below.

use crate::languages::{c, cpp, csharp, go, java, php, python, rust, typescript};

/// One extracted symbol: a function, type, or test found in a source
/// file. Route/import/call extraction is intentionally modeled
/// separately ([`RouteRef`], [`ImportRef`], [`CallRef`]) because those
/// are edges (relationships to other nodes), not nodes in their own
/// right.
///
/// X06 core parity note: this type deliberately does NOT carry Tier A
/// complexity metrics ([`crate::complexity::ComplexityMetrics`]) --
/// `rust.rs`/`typescript.rs`/`python.rs` are high-churn shared files
/// under concurrent lane edits, so threading a new field through every
/// `SymbolRef { .. }` construction site there would be a wide,
/// collision-prone diff for a metrics-only concern. Instead
/// [`crate::code_graph::CodeGraph::insert_file_and_chunks`] independently
/// locates each function/method's definition node (by name + start
/// line, which [`SymbolRef::line`] already gives it) via
/// [`crate::complexity::find_definition_node`] and calls
/// [`crate::complexity::compute`] directly -- metrics land on
/// [`crate::code_graph::SymbolNode`], never on this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    pub name: String,
    pub kind: SymbolKind,
    /// 1-based start line in the source file, for stable ids and for
    /// human-readable "why selected" traces.
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Type,
    Test,
    /// X06 rich vocabulary (additive): a function defined inside an
    /// `impl`/class body -- as opposed to a free-standing [`Function`].
    Method,
    /// A `class`/`struct` product type.
    Class,
    /// A Rust `struct` specifically (a [`Class`]-shaped product type in
    /// languages that distinguish the two; kept as its own variant so a
    /// caller can still query "give me all Structs" without also
    /// matching TS/Python classes).
    Struct,
    /// An interface/trait declaration (Rust `trait`, TS `interface`).
    Interface,
    /// An enum declaration.
    Enum,
    /// A type alias (`type X = ...`, Rust `type X = ...`).
    TypeAlias,
    /// A module/namespace container (Rust `mod`, TS `namespace`/module
    /// file, Python module).
    Module,
    /// A named, assigned lambda/closure (`let f = |x| ...`, `const f =
    /// () => ...`, `f = lambda x: ...`).
    Lambda,
    /// A top-level variable binding (mutable or otherwise non-const).
    Variable,
    /// A top-level constant binding (`const`/`static` in Rust,
    /// `UPPER_CASE` module-level assignment convention in Python, `const`
    /// in TS/JS).
    Constant,
}

/// An HTTP-style route/endpoint declaration found in source (e.g. an
/// Axum/Actix/Express/FastAPI decorator or macro).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRef {
    pub method: String,
    pub path: String,
    pub line: usize,
}

/// One import/use statement, module-path as written in source (not yet
/// resolved to a graph node id -- resolution is [`crate::code_graph`]'s
/// job once every file in the repo has been parsed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRef {
    pub module_path: String,
    pub line: usize,
}

/// One function-call expression's callee name, as written (unresolved
/// -- same rationale as [`ImportRef`]).
///
/// X06 type-aware resolution (additive): every field below `line` is
/// new and defaults empty/`None` for any extractor not yet updated to
/// populate it -- [`crate::resolution`]'s registry+type pass degrades
/// gracefully (falls through to unique-name matching, same as before)
/// when a field is absent, never panics or silently mis-resolves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallRef {
    pub callee: String,
    pub line: usize,
    /// The name of the function/method that lexically contains this
    /// call expression, if any (module-/file-scope calls have no
    /// enclosing symbol and leave this `None`).
    pub from_symbol: Option<String>,
    /// The 1-based start line of `from_symbol`'s own definition --
    /// paired with the name to build a stable `sym:` id the same way
    /// [`crate::code_graph`] already does (`sym:<rel_path>:<line>:<name>`),
    /// without this module needing to know the file path itself.
    pub from_symbol_line: Option<usize>,
    /// For a method-call-shaped callee (`x.foo(...)`), the receiver
    /// expression's own text (`x`, `self.inner`, `Foo::new()`, ...).
    /// `None` for a plain/unqualified call (`foo(...)`).
    pub receiver_text: Option<String>,
    /// A syntactic classification of [`Self::receiver_text`], cheap to
    /// compute at extraction time (no type information needed) and
    /// useful to [`crate::resolution`] as a fast first pass before
    /// falling back to full type lookup.
    pub receiver_hint: Option<ReceiverHint>,
    /// Each call argument's own source text, in written order --
    /// captured now (rather than re-parsing later) so the planned
    /// DATA_FLOW follow-up does not need a second pass over every
    /// extractor. Unresolved/unanalyzed here, same rationale as every
    /// other `*Ref` field in this module.
    pub arg_texts: Vec<String>,
}

/// Cheap syntactic classification of a call's receiver expression --
/// computed by each language extractor from local syntax alone (no
/// symbol table, no type inference), giving [`crate::resolution`] a
/// fast, always-available signal before it attempts full type-driven
/// resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverHint {
    /// `self.foo()` / `this.foo()`.
    SelfOrThis,
    /// `Type::new(...)` / `new Type(...)` -- a fresh instance the
    /// callee is invoked on.
    NewExpression,
    /// The receiver is a bare identifier (`x.foo()`) -- most likely a
    /// local variable or parameter; [`crate::resolution`] looks up its
    /// declared/inferred type.
    Identifier,
    /// A string/number/bool/etc. literal receiver (`"x".foo()`) --
    /// resolution has no local/param type to look up, so this is
    /// recorded purely for completeness/debugging.
    Literal,
    /// Anything else (a nested call, an index expression, ...) --
    /// still recorded as *some* receiver text, but not further
    /// classified.
    Other,
}

/// X06 rich vocabulary (additive): one INHERITS edge as written in
/// source (`class Sub extends Base`, `trait Sub: Base`) -- the
/// subtype's own name (the containing symbol) plus the supertype name
/// as written (unresolved, same rationale as [`ImportRef`]/[`CallRef`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritsRef {
    pub sub_name: String,
    pub super_name: String,
    pub line: usize,
}

/// One IMPLEMENTS edge as written in source (`impl Trait for Type`,
/// `class C implements I`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementsRef {
    pub type_name: String,
    pub trait_name: String,
    pub line: usize,
}

/// One DECORATES edge: a decorator/attribute-macro applied to a
/// symbol (`@decorator` in Python/TS, `#[attribute]` in Rust,
/// best-effort).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoratesRef {
    pub target_name: String,
    pub decorator_name: String,
    pub line: usize,
}

/// One TYPE_REF edge: a type usage in a signature (parameter type,
/// return type, field type), as written (unresolved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRefRef {
    pub from_name: String,
    pub type_name: String,
    pub line: usize,
}

/// One DEFINES edge: a container symbol (class/struct/module/impl)
/// defines a member symbol (method/field/nested type), both by name as
/// written, resolved lazily like every other edge here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinesRef {
    pub container_name: String,
    pub member_name: String,
    pub line: usize,
}

/// The language-agnostic result of parsing one source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedFile {
    pub symbols: Vec<SymbolRef>,
    pub routes: Vec<RouteRef>,
    pub imports: Vec<ImportRef>,
    pub calls: Vec<CallRef>,
    /// X06 rich vocabulary (additive, defaults empty for any extractor
    /// that has not been updated yet -- never a breaking field).
    pub inherits: Vec<InheritsRef>,
    pub implements: Vec<ImplementsRef>,
    pub decorates: Vec<DecoratesRef>,
    pub type_refs: Vec<TypeRefRef>,
    pub defines: Vec<DefinesRef>,
}

/// Which extractor produced (or would produce) a [`ParsedFile`] for a
/// given path -- also doubles as the "supported language" predicate
/// [`crate::code_graph`] uses to decide symbol/route/import/call nodes
/// vs a bare `TextOnly` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Php,
    ConfigToml,
    ConfigJson,
    ConfigYaml,
    /// Anything else: still indexed as a file node, but with no
    /// structural extraction -- see the workpack's "unsupported files
    /// become TextOnly nodes, never silent skip" hard requirement.
    TextOnly,
}

/// Classify a file purely by its extension. Case-insensitive so
/// `Foo.RS`/`foo.rs` land the same way.
pub fn classify(rel_path: &str) -> Language {
    let ext = rel_path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "rs" => Language::Rust,
        "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "py" | "pyi" => Language::Python,
        "go" => Language::Go,
        "java" => Language::Java,
        "c" => Language::C,
        // ".h" ambiguity (C vs C++ header): default to C++'s grammar.
        // tree-sitter-cpp's grammar is a syntactic superset of C for
        // every construct this crate's extractor matches (function/
        // struct/enum/typedef/#define/#include/call), so a plain-C
        // header still extracts every symbol/edge a C-specific grammar
        // would; the reverse is not true (a C grammar cannot parse
        // `class`/`namespace`/templates), so routing ".h" through C++
        // strictly dominates routing it through C -- see this module's
        // doc and the workpack's "route .h by content heuristic or
        // default to C++ ... -- document the choice" instruction.
        "h" | "hh" | "hpp" | "hxx" | "h++" => Language::Cpp,
        "cpp" | "cc" | "cxx" | "c++" => Language::Cpp,
        "cs" => Language::CSharp,
        "php" => Language::Php,
        "toml" => Language::ConfigToml,
        "json" => Language::ConfigJson,
        "yml" | "yaml" => Language::ConfigYaml,
        _ => Language::TextOnly,
    }
}

/// Parse `source` per `language`. Returns `None` for languages that
/// have no structural extractor ([`Language::ConfigToml`],
/// [`Language::ConfigJson`], [`Language::ConfigYaml`],
/// [`Language::TextOnly`]) -- callers must still create a file node for
/// those, just with no symbols/routes/imports/calls attached (the
/// `TextOnly`-node fallback).
///
/// `rel_path` additionally lets the Go extractor recognize `_test.go`
/// files (Go test detection is filename-gated, not annotation-gated,
/// per Go convention -- see `languages/go.rs`'s module doc).
pub fn parse_file(language: Language, source: &str, rel_path: &str) -> Option<ParsedFile> {
    match language {
        Language::Rust => Some(rust::parse(source)),
        Language::TypeScript | Language::JavaScript => Some(typescript::parse(source, language)),
        Language::Python => Some(python::parse(source)),
        Language::Go => {
            let is_test_file = rel_path.to_lowercase().ends_with("_test.go");
            Some(go::parse(source, is_test_file))
        }
        Language::Java => Some(java::parse(source)),
        Language::C => {
            let is_test_file = is_c_family_test_path(rel_path, &["_test.c"]);
            Some(c::parse(source, is_test_file))
        }
        Language::Cpp => {
            let is_test_file =
                is_c_family_test_path(rel_path, &["_test.cpp", "_test.cc", "_test.cxx"]);
            Some(cpp::parse(source, is_test_file))
        }
        Language::CSharp => Some(csharp::parse(source)),
        Language::Php => Some(php::parse(source)),
        Language::ConfigToml | Language::ConfigJson | Language::ConfigYaml | Language::TextOnly => {
            None
        }
    }
}

/// C/C++ file-level test signal (workpack: "files under test/ or
/// *_test.c(pp)"): a path with any `test`-named path segment (`test/`,
/// `tests/`, case-insensitive) OR whose filename ends with one of
/// `suffixes` (e.g. `_test.c`, `_test.cpp`).
fn is_c_family_test_path(rel_path: &str, suffixes: &[&str]) -> bool {
    let lower = rel_path.to_lowercase().replace('\\', "/");
    let under_test_dir = lower
        .split('/')
        .any(|segment| segment == "test" || segment == "tests");
    let matches_suffix = suffixes.iter().any(|suffix| lower.ends_with(suffix));
    under_test_dir || matches_suffix
}
