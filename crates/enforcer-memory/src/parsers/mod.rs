//! Language-agnostic parse result shape, plus the extension-based
//! dispatch that routes a file's content to the right
//! [`crate::languages`] extractor (or the `TextOnly` fallback).
//!
//! [`crate::code_graph`] never touches a tree-sitter tree directly: it
//! only ever sees a [`ParsedFile`], so adding a new language is a
//! change fully contained to `languages/` + the one dispatch arm in
//! [`parse_file`] below.

use crate::languages::generic;

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
    /// Kotlin (`.kt`/`.kts`). Language-parity wave G2.1a.
    Kotlin,
    /// Swift (`.swift`). Language-parity wave G2.1a.
    Swift,
    /// TSX (TypeScript-JSX, `.tsx`) -- a DISTINCT language from plain
    /// [`Language::TypeScript`], matching the baseline's own
    /// `CBM_LANG_TSX`/`CBM_LANG_TYPESCRIPT` split
    /// (`codebase-memory-mcp/internal/cbm/lang_specs.c` :1622-1636).
    /// Every `.mts`/`.cts` file still routes to `TypeScript` (neither
    /// is a JSX-capable extension in the baseline's own
    /// `src/discover/language.c` `EXT_TABLE`) -- only `.tsx` itself
    /// moves here. Language-parity wave G2.1a.
    Tsx,
    /// Solidity (`.sol`). Language-parity wave G2.1d.
    Solidity,
    /// GDScript (`.gd`). Language-parity wave G2.1d.
    Gdscript,
    /// Dart (`.dart`). Language-parity wave G2.1b.
    Dart,
    /// Scala (`.sc`/`.scala`). Language-parity wave G2.1b.
    Scala,
    /// Groovy (`.gradle`/`.groovy`). Language-parity wave G2.1b.
    Groovy,
    /// Ruby (`.rb`/`.gemspec`/`.rake`). Language-parity wave G2.1c.
    Ruby,
    /// Zig (`.zig`). Language-parity wave G2.1c.
    Zig,
    /// Objective-C (`.m`). Language-parity wave G2.1c. Unconditional,
    /// not content-sniffed the way the baseline's own
    /// `cbm_disambiguate_m` (`src/discover/language.c`:997) is -- the
    /// baseline only needs that sniff because MATLAB is ALSO a
    /// registered baseline language sharing the `.m` extension; this
    /// crate has no MATLAB extractor at all yet, so there is nothing
    /// for a `.m` file to be ambiguous *with* here. No `.mm`: the
    /// baseline's own `EXT_TABLE` (`src/discover/language.c`:30-621)
    /// has no `.mm` entry either -- Objective-C++ is out of scope for
    /// both.
    ObjectiveC,
    /// Bash (`.bash`/`.sh`). Language-parity wave G2.2f.
    Bash,
    /// Lua (`.lua`). Language-parity wave G2.2f. No `.luau`: the
    /// baseline's own `EXT_TABLE` maps that extension to a DIFFERENT
    /// registered language, `CBM_LANG_LUAU` (Tier 1, out of this wave's
    /// scope) -- Luau is a distinct dialect/grammar from plain Lua, not
    /// an alternate extension for the same language the way `.sc`/
    /// `.scala` are for Scala.
    Lua,
    /// Elixir (`.ex`/`.exs`). Language-parity wave G2.2f.
    Elixir,
    /// Haskell (`.hs`). Language-parity wave G2.2g. No `.lhs`: the
    /// baseline's own `EXT_TABLE` (`src/discover/language.c`:131) has no
    /// literate-Haskell entry either -- `.lhs` is out of scope for both.
    Haskell,
    /// OCaml (`.ml`/`.mli`). Language-parity wave G2.2g. One `Language`
    /// variant (and one [`crate::languages::spec::LangSpec::ocaml`] row)
    /// covers both -- see that row's own doc comment for why this
    /// crate, like the baseline, binds only the one implementation
    /// grammar (`tree_sitter_ocaml::LANGUAGE_OCAML`) for `.mli` content
    /// too rather than the grammar crate's separate
    /// `LANGUAGE_OCAML_INTERFACE` entry point.
    OCaml,
    /// Erlang (`.erl`). Language-parity wave G2.2g. No `.hrl`: the
    /// baseline's own `EXT_TABLE` (`src/discover/language.c`:97) has no
    /// header-file entry either -- `.hrl` is out of scope for both.
    Erlang,
    /// CUDA (`.cu`/`.cuh`). Language-parity wave G2.2b. Reuses
    /// [`crate::languages::spec::LangSpec::cpp`]/
    /// [`crate::languages::generic::cpp_quirks`] verbatim -- the
    /// baseline's own `lang_specs.c` table literally reuses C++'s node-
    /// type arrays for `CBM_LANG_CUDA` too (see
    /// [`crate::languages::spec::LangSpec::cuda`]'s own doc comment for
    /// the empirically-confirmed grammar-superset finding this relies
    /// on) -- a genuinely dedicated `tree-sitter-cuda` grammar, not a
    /// relabeled tree-sitter-cpp, but one that happens to be a strict
    /// syntactic superset of C++'s grammar for every construct this
    /// crate's extractor matches.
    Cuda,
    /// D (`.d`). Language-parity wave G2.2b. No `.di` (D interface
    /// files): the baseline's own `EXT_TABLE`
    /// (`src/discover/language.c`:343) has no `.di` entry either --
    /// `.di` is out of scope for both.
    D,
    /// PowerShell (`.ps1`/`.psm1`/`.psd1`). Language-parity wave G2.2b.
    PowerShell,
    /// F# (`.fs`/`.fsx`/`.fsi`). Language-parity wave G2.2c. One
    /// `Language` variant (and one
    /// [`crate::languages::spec::LangSpec::fsharp`] row) covers all
    /// three extensions -- the baseline's own `EXT_TABLE`
    /// (`src/discover/language.c`:100-102) maps all three to the SAME
    /// `CBM_LANG_FSHARP`, and that baseline's own `extern` declaration
    /// only ever names ONE grammar function pointer (`tree_sitter_fsharp`,
    /// no `_signature` variant referenced anywhere in that codebase) even
    /// though the real `tree-sitter-fsharp` crate this row binds actually
    /// ships a SEPARATE `fsharp_signature` grammar for `.fsi` files --
    /// this crate matches the baseline's own single-grammar-for-all-three
    /// choice (binding only `LANGUAGE_FSHARP`, never the crate's own
    /// `LANGUAGE_SIGNATURE`) rather than improving on it, per the
    /// "baseline's real depth, not an idealized one" instruction.
    Fsharp,
    /// Gleam (`.gleam`). Language-parity wave G2.2c.
    Gleam,
    /// GLSL (`.frag`/`.glsl`/`.vert`). Language-parity wave G2.2c. Reuses
    /// [`crate::languages::spec::LangSpec::cpp`]-style verbatim reuse,
    /// but of [`crate::languages::spec::LangSpec::c`] instead: the
    /// baseline's own `lang_specs.c` table literally reuses C's node-type
    /// arrays for `CBM_LANG_GLSL` too (see
    /// [`crate::languages::spec::LangSpec::glsl`]'s own doc comment for
    /// the verified-via-real-parse-tree finding this relies on, including
    /// the shader-storage-qualifier parse-error boundary that is
    /// confirmed harmless for this crate's own extraction scope). No
    /// `.comp`/`.geom`/`.tesc`/`.tese`: the baseline's own `EXT_TABLE`
    /// (`src/discover/language.c`:115-117) registers only these exact
    /// three extensions for `CBM_LANG_GLSL` -- the other shader-stage
    /// extensions are out of scope for both.
    Glsl,
    /// Ada (`.adb`/`.ads`). Language-parity wave G2.2a.
    Ada,
    /// Apex (`.cls`/`.trigger`). Language-parity wave G2.2a.
    Apex,
    /// Crystal (`.cr`). Language-parity wave G2.2a.
    Crystal,
    /// R (`.r`/`.R`). Language-parity wave G2.2h. Baseline's own
    /// `EXT_TABLE` maps both cases of the extension to the same
    /// `CBM_LANG_R` (this crate's own [`classify`] is already
    /// case-insensitive for every extension, so no special-casing is
    /// needed here beyond the ordinary lowercase match arm).
    R,
    /// Perl (`.pl`/`.pm`). Language-parity wave G2.2h. No `.t` (Perl test
    /// scripts): the baseline's own `EXT_TABLE`
    /// (`src/discover/language.c`) has no dedicated `.t` entry either --
    /// Perl test files use the plain `.pl`/`.pm` extensions there too.
    Perl,
    /// Clojure (`.clj`/`.cljc`/`.cljs`). Language-parity wave G2.2h.
    /// Baseline's own `EXT_TABLE` maps all three to the same
    /// `CBM_LANG_CLOJURE` (ClojureScript's `.cljs` and the cross-platform
    /// `.cljc` are still ordinary Clojure-grammar source from this
    /// tree-sitter grammar's own point of view -- neither is a distinct
    /// registered baseline language the way e.g. TSX is from TypeScript).
    Clojure,
    ConfigToml,
    ConfigJson,
    ConfigYaml,
    /// Julia (`.jl`). Language-parity wave G2.2d. This grammar's
    /// function/struct/call-shaped nodes are entirely unfielded --
    /// see [`crate::languages::spec::LangSpec::julia`]'s own doc
    /// comment.
    Julia,
    /// Odin (`.odin`). Language-parity wave G2.2d. No dedicated
    /// baseline `extract_base_classes` walker (Odin has no classical
    /// inheritance syntax), but this row wires its real `using`
    /// composition idiom as an INHERITS edge -- see
    /// [`crate::languages::spec::LangSpec::odin`]'s own doc comment.
    Odin,
    /// Pascal (`.pas`/`.dpr`/`.lpr`). Language-parity wave G2.2d,
    /// extension set CORRECTED in wave G2.3a: the ORIGINAL G2.2d doc
    /// comment here claimed a "`.pas`/`.pp`/`.dpr`/`.dpk`/`.inc`" extension
    /// set attributed to the baseline's own `EXT_TABLE` -- re-verified
    /// directly against that real table (`src/discover/language.c`) during
    /// G2.3a and found wrong on three of five entries: `.pp` is the
    /// baseline's OWN `CBM_LANG_PUPPET` extension (:499, not Pascal's --
    /// this crate's own [`Language::Puppet`] now claims it instead), `.inc`
    /// is the baseline's own `CBM_LANG_BITBAKE` extension (:269, not
    /// Pascal's), and `.dpk` has no baseline `EXT_TABLE` entry for ANY
    /// language at all. The real baseline set is `.pas` (:478)/`.dpr`
    /// (:349)/`.lpr` (:437, a Lazarus/Free Pascal project file the
    /// original G2.2d set omitted) -- see
    /// [`crate::languages::spec::LangSpec::pascal`]'s own doc comment for
    /// the grammar-shape findings (unaffected by this extension-set-only
    /// correction).
    Pascal,
    /// QML (Qt Modeling Language, `.qml`). Language-parity wave G2.2e.
    /// A genuine TypeScript-superset grammar (`tree-sitter-qmljs`) plus
    /// declarative `ui_*` nodes layered on top -- see
    /// [`crate::languages::spec::LangSpec::qml`]'s own doc comment,
    /// including the real (non-obvious) finding that a bare top-level
    /// JS statement with no wrapping QML object at all does not parse
    /// cleanly in this grammar.
    Qml,
    /// ReScript (`.res`/`.resi`). Language-parity wave G2.2e. Grammar
    /// sourced from the `arborium-rescript` crate (crates.io has no
    /// standalone `tree-sitter-rescript` at all) -- see
    /// [`crate::languages::spec::LangSpec::rescript`]'s own doc comment,
    /// including the real name-resolution gap (`function`'s own name
    /// lives on its PARENT `let_binding`'s `pattern` field) the
    /// baseline's own `cbm_resolve_func_name` already has a dedicated
    /// case for.
    Rescript,
    /// Squirrel (`.nut`). Language-parity wave G2.2e. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-squirrel-local/`),
    /// not a crates.io dependency -- the published `tree-sitter-squirrel`
    /// 1.0.0 crate hard-pins `tree-sitter = "~0.20.9"`, incompatible with
    /// this workspace's `tree-sitter = "0.25"` core -- see
    /// [`crate::languages::spec::LangSpec::squirrel`]'s own doc comment,
    /// including the finding that this grammar is almost entirely
    /// field-free for the constructs this row cares about.
    Squirrel,
    /// Sway (Fuel Labs' smart-contract language, `.sw`). Language-parity
    /// wave G2.3e. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-sway-local/`), not a
    /// crates.io dependency -- no such release exists under any
    /// discoverable name at all -- see
    /// [`crate::languages::spec::LangSpec::sway`]'s own doc comment.
    Sway,
    /// Starlark (Bazel's Python-like config language, `.bzl`/`.star`).
    /// Language-parity wave G2.3e. The baseline's own `EXT_TABLE`
    /// additionally maps the bare FILENAMES `BUILD`/`BUILD.bazel`/
    /// `WORKSPACE`/`WORKSPACE.bazel` (no extension at all) to
    /// `CBM_LANG_STARLARK` too -- DEFERRED here, precisely: this crate's
    /// own [`classify`] is purely extension-based
    /// (`rel_path.rsplit('.')`), with no filename-only dispatch mechanism
    /// anywhere in this crate yet (confirmed by grepping this whole module
    /// for any Dockerfile/Makefile-style precedent -- none exists), so
    /// adding one would be new shared dispatch infrastructure beyond this
    /// wave's own data-row scope, not a one-line extension-table entry.
    /// Every `.bzl`/`.star` file (the overwhelming majority of real-world
    /// Starlark source) is still fully classified and extracted;
    /// extensionless `BUILD`/`WORKSPACE` files fall through to
    /// [`Language::TextOnly`] until a future wave adds filename-based
    /// dispatch generally.
    Starlark,
    /// Templ (Go HTML-templating DSL, `.templ`). Language-parity wave
    /// G2.3e.
    Templ,
    /// Typst (typesetting/markup language, `.typ`). Language-parity wave
    /// G2.3e.
    Typst,
    /// WGSL (WebGPU Shading Language, `.wgsl`). Language-parity wave
    /// G2.3e. Grammar sourced from `tree-sitter-wgsl-bevy` (a maintained
    /// fork of `szebniok/tree-sitter-wgsl`, the SAME lineage the baseline
    /// itself vendored) -- see
    /// [`crate::languages::spec::LangSpec::wgsl`]'s own doc comment,
    /// including the finding that the plain `tree-sitter-wgsl` crate name
    /// on crates.io is stale/incompatible.
    Wgsl,
    /// Wolfram Language (Mathematica, `.wl`/`.wls`). Language-parity wave
    /// G2.3e. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-wolfram-local/`), not a
    /// crates.io dependency -- no such release exists under any
    /// discoverable name at all -- see
    /// [`crate::languages::spec::LangSpec::wolfram`]'s own doc comment. No
    /// `.m`: that extension is already claimed by [`Language::ObjectiveC`]
    /// in this crate (see that variant's own doc comment) -- the
    /// baseline's own `src/discover/language.c` `cbm_disambiguate_m`
    /// content-sniff resolves this exact ambiguity for Objective-C vs.
    /// MATLAB vs. Wolfram/Mathematica `.m` files there, but this crate has
    /// no MATLAB extractor either, so there is nothing to disambiguate
    /// `.m` FOR here yet -- deferring a Wolfram `.m` mapping until that
    /// three-way ambiguity actually needs resolving, rather than silently
    /// mis-routing every `.m` file already handled as Objective-C.
    Wolfram,
    /// Slang (shader language, superset of HLSL/GLSL-family syntax,
    /// `.slang`). Language-parity wave G2.3e.
    Slang,
    /// SCSS (`.scss`). Language-parity wave G2.3a. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-scss-local/`), not a
    /// plain crates.io dependency -- the published `tree-sitter-scss`
    /// 1.0.0 crate's own `build.rs` unconditionally passes a GCC/Clang-only
    /// compiler flag with no MSVC guard, confirmed to fail a real
    /// `cargo build` against this workspace's MSVC toolchain (an upstream
    /// packaging bug, not a tree-sitter ABI incompatibility -- see
    /// [`crate::languages::spec::LangSpec::scss`]'s own doc comment).
    Scss,
    /// CMake (`.cmake`). Language-parity wave G2.3a. No bare
    /// `CMakeLists.txt` filename dispatch -- the baseline's own
    /// `FILENAME_TABLE` (`src/discover/language.c`:633) maps that exact
    /// extensionless filename to `CBM_LANG_CMAKE` too, but this crate's own
    /// [`classify`] is purely extension-based with no filename-only
    /// dispatch mechanism anywhere yet -- same deferral, same reasoning, as
    /// [`Language::Starlark`]'s own `BUILD`/`WORKSPACE` doc comment. Every
    /// `.cmake` helper-script file (a real, common convention for included
    /// CMake modules, confirmed by the baseline's own `EXT_TABLE`
    /// `{".cmake", CBM_LANG_CMAKE}` entry at `src/discover/language.c`:59)
    /// is still fully classified and extracted; extensionless
    /// `CMakeLists.txt` falls through to [`Language::TextOnly`] until a
    /// future wave adds filename-based dispatch generally.
    Cmake,
    /// Makefile (`.mk`). Language-parity wave G2.3a. No bare
    /// `Makefile`/`makefile`/`GNUmakefile` filename dispatch -- same
    /// deferral, same reasoning, as [`Language::Cmake`] above (the
    /// baseline's own `FILENAME_TABLE` maps all three extensionless
    /// filenames to `CBM_LANG_MAKEFILE` too, `src/discover/language.c`:
    /// 635-637). Every `.mk` include-fragment file (the baseline's own
    /// `EXT_TABLE` `{".mk", CBM_LANG_MAKEFILE}` entry,
    /// `src/discover/language.c`:176) is still fully classified and
    /// extracted; the extensionless primary `Makefile` itself falls
    /// through to [`Language::TextOnly`] until a future wave adds
    /// filename-based dispatch generally.
    Makefile,
    /// Fortran (`.f90`, free-form modern Fortran). Language-parity wave
    /// G2.3a. No `.f`/`.f77`/`.for` (fixed-form legacy Fortran): the
    /// `tree-sitter-fortran` grammar this row binds targets free-form
    /// syntax; fixed-form column-position-sensitive source is a
    /// meaningfully different parse target this wave does not attempt --
    /// see [`crate::languages::spec::LangSpec::fortran`]'s own doc comment.
    Fortran,
    /// VimScript (`.vim`). Language-parity wave G2.3a.
    Vimscript,
    /// Puppet (`.pp`). Language-parity wave G2.3a.
    Puppet,
    /// Elm (`.elm`). Language-parity wave G2.3a.
    Elm,
    /// Bicep (Azure IaC, `.bicep`). Language-parity wave G2.3c.
    Bicep,
    /// BitBake (Yocto build recipes, `.bb`/`.bbappend`/`.bbclass`/`.inc`).
    /// Language-parity wave G2.3c. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-bitbake-local/`), not a
    /// plain crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::bitbake`]'s own doc comment.
    /// No plain `.bb` collision handling needed with any other registered
    /// language here (the baseline's own `EXT_TABLE` registers `.bb` for
    /// `CBM_LANG_BITBAKE` alone).
    Bitbake,
    /// Cairo (StarkNet smart contracts, `.cairo`). Language-parity wave
    /// G2.3c. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-cairo-local/`), not a
    /// plain crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::cairo`]'s own doc comment.
    Cairo,
    /// CFScript (ColdFusion/CFML script dialect, `.cfc`). Language-parity
    /// wave G2.3c. No `.cfml`/`.cfm` (the CFML tag dialect): the baseline's
    /// own `EXT_TABLE` maps those to a DIFFERENT registered language,
    /// `CBM_LANG_CFML` (still unclaimed in this crate) -- CFML is a
    /// distinct grammar/dialect from CFScript, not an alternate extension
    /// for the same language.
    Cfscript,
    /// FunC (TON smart contracts, `.fc`). Language-parity wave G2.3c.
    /// Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-func-local/`), not a
    /// plain crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::func`]'s own doc comment. No
    /// `.func`: the baseline's own `EXT_TABLE` has no such entry for
    /// `CBM_LANG_FUNC` either -- `.fc` is the only registered extension.
    Func,
    /// Move (Aptos/Sui smart contracts, `.move`). Language-parity wave
    /// G2.3c. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-move-local/`), not a
    /// plain crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::move_lang`]'s own doc comment.
    Move,
    /// Nickel (config language, `.ncl`). Language-parity wave G2.3c.
    Nickel,
    /// Jsonnet (JSON templating, `.jsonnet`/`.libsonnet`). Language-parity
    /// wave G2.3c.
    Jsonnet,
    /// Just (task-runner justfiles, `.just`). Language-parity wave G2.3d.
    /// No bare `justfile`/`Justfile`/`.justfile` filename dispatch -- same
    /// deferral, same reasoning, as [`Language::Cmake`]'s own
    /// `CMakeLists.txt` doc comment (the baseline's own `FILENAME_TABLE`
    /// maps all three extensionless conventions to `CBM_LANG_JUST` too,
    /// `src/discover/language.c`:652-654, in addition to its own
    /// `EXT_TABLE`'s `.just` entry at :259). Every `.just` imported-recipe-
    /// library file is still fully classified and extracted; the primary
    /// extensionless `justfile` itself falls through to
    /// [`Language::TextOnly`] until a future wave adds filename-based
    /// dispatch generally.
    Just,
    /// HLSL (High-Level Shading Language, DirectX; `.fx`/`.hlsl`/
    /// `.hlsli`). Language-parity wave G2.3d. See
    /// [`crate::languages::spec::LangSpec::hlsl`]'s own doc comment for
    /// the grammar-reuse rationale (a genuine `tree-sitter-cpp` fork) and
    /// the top-level-`cbuffer`-block parse-gap finding.
    Hlsl,
    /// ISPC (Intel Implicit SPMD Program Compiler, `.ispc`). Language-
    /// parity wave G2.3d. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-ispc-local/`), not a
    /// crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::ispc`]'s own doc comment.
    Ispc,
    /// PureScript (Haskell-like, compiles to JS; `.purs`). Language-parity
    /// wave G2.3d. Grammar sourced via a `git` dependency (crates.io has
    /// no `tree-sitter-purescript` at all) -- see
    /// [`crate::languages::spec::LangSpec::purescript`]'s own doc
    /// comment, including the real baseline `module_types` correction
    /// this row required.
    Purescript,
    /// Magma (computer algebra system scripting language, `.mag`/
    /// `.magma`). Language-parity wave G2.3d. Grammar sourced via a `git`
    /// dependency (crates.io has no `tree-sitter-magma` for this
    /// language at all) -- see
    /// [`crate::languages::spec::LangSpec::magma`]'s own doc comment,
    /// including the real `call_types`/`module_types`/`import_types`
    /// baseline corrections this row required (the baseline's own
    /// `magma_call_types` names a node kind that is entirely absent from
    /// this grammar).
    Magma,
    /// Hare (systems language, `.ha`). Language-parity wave G2.3d.
    /// Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-hare-local/`), not a
    /// crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::hare`]'s own doc comment.
    Hare,
    /// Pony (actor-model systems language, `.pony`). Language-parity wave
    /// G2.3d. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-pony-local/`), not a
    /// crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::pony`]'s own doc comment,
    /// including the real `actor ... is Base` INHERITS-edge improvement
    /// this row adds (the baseline itself has no dedicated Pony
    /// `extract_base_classes` walker at all).
    Pony,
    /// NASM (Netwide Assembler, x86 assembly; `.nasm`). Language-parity
    /// wave G2.3d. Grammar VENDORED
    /// (`crates/enforcer-memory/vendor/tree-sitter-nasm-local/`), not a
    /// crates.io dependency -- distinct from the generic, Tier-0 `.s`/`.S`
    /// assembly path (out of this wave's own scope) -- see
    /// [`crate::languages::spec::LangSpec::nasm`]'s own doc comment.
    Nasm,
    /// Anything else: still indexed as a file node, but with no
    /// structural extraction -- see the workpack's "unsupported files
    /// become TextOnly nodes, never silent skip" hard requirement.
    /// COBOL (`.cbl`/`.cob`). Language-parity wave G2.3b. Grammar
    /// VENDORED (`crates/enforcer-memory/vendor/tree-sitter-cobol-local/`),
    /// not a crates.io dependency -- see
    /// [`crate::languages::spec::LangSpec::cobol`]'s own doc comment.
    Cobol,
    /// Common Lisp (`.cl`/`.lisp`/`.lsp`). Language-parity wave G2.3b.
    Commonlisp,
    /// Lean (`.lean`). Language-parity wave G2.3b.
    Lean,
    /// TLA+ (`.tla`). Language-parity wave G2.3b.
    Tlaplus,
    /// Verilog (`.v`). Language-parity wave G2.3b. Baseline's own
    /// `EXT_TABLE` (`src/discover/language.c`:239-240) maps BOTH `.sv`
    /// AND `.v` to `CBM_LANG_VERILOG` -- `CBM_LANG_SYSTEMVERILOG` is a
    /// fully registered baseline language (its own `lang_specs.c` row,
    /// display name, and dedicated `if (lang == CBM_LANG_VERILOG || lang
    /// == CBM_LANG_SYSTEMVERILOG)` branches in `extract_defs.c`/
    /// `extract_calls.c`) that the baseline's OWN file-discovery path
    /// never actually reaches via any extension at all -- confirmed by an
    /// exhaustive `grep` of `EXT_TABLE` finding no `.sv`-to-
    /// `CBM_LANG_SYSTEMVERILOG` entry anywhere. This crate deliberately
    /// does NOT reproduce that dead-path baseline quirk (see
    /// [`Language::Systemverilog`]'s own doc comment for why `.sv` is
    /// instead routed to the real, tested SystemVerilog grammar here) --
    /// `.v` alone maps to this variant.
    Verilog,
    /// VHDL (`.vhd`/`.vhdl`). Language-parity wave G2.3b.
    Vhdl,
    /// SystemVerilog (`.sv`/`.svh`). Language-parity wave G2.3b. Baseline
    /// itself never routes any file extension to `CBM_LANG_SYSTEMVERILOG`
    /// at all (see [`Language::Verilog`]'s own doc comment for the full
    /// finding) -- `.sv`/`.svh` are routed HERE rather than reproducing
    /// that gap, a deliberate improvement: this crate's own
    /// `tree-sitter-systemverilog` grammar is confirmed (via real
    /// parse-tree dumps, see
    /// [`crate::languages::spec::LangSpec::systemverilog`]'s own doc
    /// comment) genuinely MORE complete for SystemVerilog source than the
    /// plain-Verilog grammar is (e.g. real, direct `[name]` fields on
    /// `class_declaration`/`module_declaration`, and a bare-statement
    /// function-call form that is a parse ERROR in the plain-Verilog
    /// grammar) -- routing `.sv` through the weaker grammar the way the
    /// baseline's own dead extension table would (if it routed `.sv`
    /// anywhere at all) would be strictly worse, not merely different.
    /// No baseline extension exists for `.svh` either; assigned here by
    /// the same real-file-extension convention every other language in
    /// this crate already follows for a header/include-file variant
    /// (mirrors [`Language::Cpp`]'s own multi-suffix header handling in
    /// spirit) rather than leaving `.svh` unclassified.
    Systemverilog,
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
        // `.tsx` is its own [`Language::Tsx`] (baseline's distinct
        // `CBM_LANG_TSX` row, see [`Language::Tsx`]'s own doc comment)
        // -- NOT folded into plain TypeScript the way this crate did
        // before language-parity wave G2.1a. `.mts`/`.cts` stay
        // TypeScript per the baseline's own `EXT_TABLE`.
        "ts" | "mts" | "cts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
        "py" | "pyi" => Language::Python,
        "go" => Language::Go,
        "java" => Language::Java,
        "c" => Language::C,
        "kt" | "kts" => Language::Kotlin,
        "swift" => Language::Swift,
        "sol" => Language::Solidity,
        "gd" => Language::Gdscript,
        "dart" => Language::Dart,
        // Baseline `src/discover/language.c` `EXT_TABLE` maps both `.sc`
        // and `.scala` to `CBM_LANG_SCALA` (Scala's Ammonite-script
        // extension and its ordinary source extension share one
        // language).
        "sc" | "scala" => Language::Scala,
        // Baseline maps both `.gradle` (Gradle's Groovy-DSL build-script
        // extension) and `.groovy` to `CBM_LANG_GROOVY`.
        "gradle" | "groovy" => Language::Groovy,
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
        "rb" | "gemspec" | "rake" => Language::Ruby,
        "zig" => Language::Zig,
        // See `Language::ObjectiveC`'s own doc comment for why this is
        // unconditional rather than content-sniffed.
        "m" => Language::ObjectiveC,
        "bash" | "sh" => Language::Bash,
        // No "luau" here -- see `Language::Lua`'s own doc comment.
        "lua" => Language::Lua,
        "ex" | "exs" => Language::Elixir,
        // No "lhs" -- see `Language::Haskell`'s own doc comment.
        "hs" => Language::Haskell,
        // One `Language::OCaml` for both -- see its own doc comment.
        "ml" | "mli" => Language::OCaml,
        // No "hrl" -- see `Language::Erlang`'s own doc comment.
        "erl" => Language::Erlang,
        "cu" | "cuh" => Language::Cuda,
        // No "di" -- see `Language::D`'s own doc comment.
        "d" => Language::D,
        "ps1" | "psm1" | "psd1" => Language::PowerShell,
        // One `Language::Fsharp` for all three -- see its own doc comment.
        "fs" | "fsx" | "fsi" => Language::Fsharp,
        "gleam" => Language::Gleam,
        // No "comp"/"geom"/"tesc"/"tese" -- see `Language::Glsl`'s own
        // doc comment.
        "frag" | "glsl" | "vert" => Language::Glsl,
        // No plain ".ada" -- the baseline's own `EXT_TABLE` registers
        // only the body/spec-file split (`.adb`/`.ads`) for
        // `CBM_LANG_ADA`, no bare `.ada` entry at all.
        "adb" | "ads" => Language::Ada,
        "cls" | "trigger" => Language::Apex,
        "cr" => Language::Crystal,
        "r" => Language::R,
        "pl" | "pm" => Language::Perl,
        "clj" | "cljc" | "cljs" => Language::Clojure,
        "jl" => Language::Julia,
        "odin" => Language::Odin,
        // Language-parity wave G2.3a correction: re-verified directly
        // against the baseline's own `src/discover/language.c`
        // `EXT_TABLE` (not re-transcribed from a prior wave's own doc
        // comment) -- the REAL baseline entries for `CBM_LANG_PASCAL` are
        // only `.pas` (:478), `.dpr` (:349), and `.lpr` (:437, a
        // Lazarus/Free Pascal project file this crate's own G2.2d wave
        // omitted entirely). `.pp` (:499) and `.inc` (:269) are baseline
        // entries for OTHER languages (`CBM_LANG_PUPPET`/
        // `CBM_LANG_BITBAKE` respectively, confirmed by grepping the exact
        // same table) -- a real, confirmed transcription error in that
        // prior wave's own doc comment (which claimed a "full Delphi/Free
        // Pascal extension set" including `.pp`/`.dpk`/`.inc` with no
        // baseline citation backing any of the three), discovered only
        // because it silently made this wave's own
        // `"pp" => Language::Puppet` arm below unreachable (a real
        // `cargo check` failure, not a hypothetical) -- fixed here rather
        // than left broken, per this wave's own "courtesy-fix a
        // crate-wide blocker if trivial, document it, move on" mandate.
        // `.dpk` (Delphi package) has NO entry anywhere in the baseline's
        // own `EXT_TABLE` at all -- dropped rather than guessed at.
        "pas" | "dpr" | "lpr" => Language::Pascal,
        "qml" => Language::Qml,
        "res" | "resi" => Language::Rescript,
        "nut" => Language::Squirrel,
        "sw" => Language::Sway,
        // No bare `BUILD`/`WORKSPACE` filename dispatch -- see
        // `Language::Starlark`'s own doc comment for why.
        "bzl" | "star" => Language::Starlark,
        "templ" => Language::Templ,
        "typ" => Language::Typst,
        "wgsl" => Language::Wgsl,
        // No ".m" -- see `Language::Wolfram`'s own doc comment (already
        // claimed by `Language::ObjectiveC`).
        "wl" | "wls" => Language::Wolfram,
        "slang" => Language::Slang,
        "scss" => Language::Scss,
        // No bare "CMakeLists.txt" filename dispatch -- see
        // `Language::Cmake`'s own doc comment.
        "cmake" => Language::Cmake,
        // No bare "Makefile"/"makefile"/"GNUmakefile" filename dispatch --
        // see `Language::Makefile`'s own doc comment.
        "mk" => Language::Makefile,
        // No ".f"/".f77"/".for" (fixed-form) -- see `Language::Fortran`'s
        // own doc comment.
        "f90" => Language::Fortran,
        "vim" => Language::Vimscript,
        "pp" => Language::Puppet,
        "elm" => Language::Elm,
        "bicep" => Language::Bicep,
        // Baseline `src/discover/language.c` `EXT_TABLE` registers this
        // exact four-extension set for `CBM_LANG_BITBAKE` (`.bb` recipes,
        // `.bbappend`/`.bbclass` Yocto layer-extension conventions,
        // `.inc` textually-included recipe fragment).
        "bb" | "bbappend" | "bbclass" | "inc" => Language::Bitbake,
        "cairo" => Language::Cairo,
        // `.cfc` only -- see `Language::Cfscript`'s own doc comment for
        // why `.cfm` (CFML's own tag dialect) is NOT also mapped here.
        "cfc" => Language::Cfscript,
        // `.fc` only -- the baseline's own `EXT_TABLE` has no `.func`
        // entry for `CBM_LANG_FUNC` at all.
        "fc" => Language::Func,
        "move" => Language::Move,
        "ncl" => Language::Nickel,
        "jsonnet" | "libsonnet" => Language::Jsonnet,
        // No bare "justfile"/"Justfile"/".justfile" filename dispatch --
        // see `Language::Just`'s own doc comment.
        "just" => Language::Just,
        "fx" | "hlsl" | "hlsli" => Language::Hlsl,
        "ispc" => Language::Ispc,
        "purs" => Language::Purescript,
        "mag" | "magma" => Language::Magma,
        "ha" => Language::Hare,
        "pony" => Language::Pony,
        "nasm" => Language::Nasm,
        "toml" => Language::ConfigToml,
        "json" => Language::ConfigJson,
        "yml" | "yaml" => Language::ConfigYaml,
        "cbl" | "cob" => Language::Cobol,
        "cl" | "lisp" | "lsp" => Language::Commonlisp,
        "lean" => Language::Lean,
        "tla" => Language::Tlaplus,
        // See `Language::Verilog`'s own doc comment: baseline's own
        // `.sv` -> `CBM_LANG_VERILOG` mapping is a dead-path quirk this
        // crate deliberately does not reproduce -- `.v` alone lands
        // here.
        "v" => Language::Verilog,
        "vhd" | "vhdl" => Language::Vhdl,
        // See `Language::Systemverilog`'s own doc comment: `.sv`/`.svh`
        // route to the real, more-complete SystemVerilog grammar here
        // rather than the baseline's own (never-actually-reached) plain
        // Verilog mapping.
        "sv" | "svh" => Language::Systemverilog,
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
        Language::Rust => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_rust`) rather than the
            // bespoke `languages::rust` extractor -- see
            // `tests/unit_languages_rust.rs`, run unchanged against
            // this dispatch, for the zero-regression proof.
            Some(generic::parse_rust(source))
        }
        Language::TypeScript | Language::JavaScript => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_typescript`) -- see
            // `tests/unit_languages_typescript.rs` (exercises both
            // `Language::TypeScript` and `Language::JavaScript`
            // scenarios), run unchanged against this dispatch, for the
            // zero-regression proof. Both languages share one
            // grammar/quirks row unchanged from the bespoke
            // extractor's own behavior.
            Some(generic::parse_typescript(source))
        }
        Language::Python => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_python`) -- see
            // `tests/unit_languages_python.rs`, run unchanged against
            // this dispatch, for the zero-regression proof.
            Some(generic::parse_python(source))
        }
        Language::Go => {
            // G1: routed through the generic spec-table engine
            // (`languages::generic::parse_go`) rather than the bespoke
            // `languages::go` extractor -- `tests/unit_lang_spec_engine.rs`
            // proves the two produce identical node/edge sets on every
            // existing Go fixture/scenario before this cutover.
            let is_test_file = rel_path.to_lowercase().ends_with("_test.go");
            Some(generic::parse_go(source, is_test_file))
        }
        Language::Java => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_java`) -- see
            // `tests/unit_languages_java.rs`, run unchanged against
            // this dispatch, for the zero-regression proof.
            Some(generic::parse_java(source))
        }
        Language::C => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_c`) -- see
            // `tests/unit_languages_c.rs`, run unchanged against this
            // dispatch, for the zero-regression proof.
            let is_test_file = is_c_family_test_path(rel_path, &["_test.c"]);
            Some(generic::parse_c(source, is_test_file))
        }
        Language::Cpp => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_cpp`) -- see
            // `tests/unit_languages_cpp.rs`, run unchanged against this
            // dispatch, for the zero-regression proof.
            let is_test_file =
                is_c_family_test_path(rel_path, &["_test.cpp", "_test.cc", "_test.cxx"]);
            Some(generic::parse_cpp(source, is_test_file))
        }
        Language::CSharp => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_csharp`) -- see
            // `tests/unit_languages_csharp.rs`, run unchanged against
            // this dispatch, for the zero-regression proof.
            Some(generic::parse_csharp(source))
        }
        Language::Php => {
            // G1b: routed through the generic spec-table engine
            // (`languages::generic::parse_php`) -- see
            // `tests/unit_languages_php.rs`, run unchanged against
            // this dispatch, for the zero-regression proof.
            Some(generic::parse_php(source))
        }
        Language::Kotlin => {
            // Language-parity wave G2.1a: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_kotlin`) -- there is no bespoke `languages::kotlin`
            // extractor to prove zero-regression against (Kotlin has
            // never had one in this crate); see
            // `tests/unit_languages_kotlin.rs`.
            Some(generic::parse_kotlin(source))
        }
        Language::Swift => {
            // Language-parity wave G2.1a: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_swift`) -- see `tests/unit_languages_swift.rs`.
            Some(generic::parse_swift(source))
        }
        Language::Tsx => {
            // Language-parity wave G2.1a: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_tsx`), reusing TypeScript's own quirks unchanged
            // (see `LangSpec::tsx`'s doc comment) with the `tsx()`
            // grammar entry point swapped in -- see
            // `tests/unit_languages_tsx.rs`.
            Some(generic::parse_tsx(source))
        }
        Language::Solidity => {
            // Language-parity wave G2.1d: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_solidity`) -- there is no bespoke
            // `languages::solidity` extractor to prove zero-regression
            // against (Solidity has never had one in this crate); see
            // `tests/unit_languages_solidity.rs`.
            Some(generic::parse_solidity(source))
        }
        Language::Gdscript => {
            // Language-parity wave G2.1d: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_gdscript`) -- see
            // `tests/unit_languages_gdscript.rs`.
            Some(generic::parse_gdscript(source))
        }
        Language::Dart => {
            // Language-parity wave G2.1b: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_dart`) -- there is no bespoke `languages::dart`
            // extractor to prove zero-regression against (Dart has
            // never had one in this crate); see
            // `tests/unit_languages_dart.rs`.
            Some(generic::parse_dart(source))
        }
        Language::Scala => {
            // Language-parity wave G2.1b: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_scala`) -- see `tests/unit_languages_scala.rs`.
            Some(generic::parse_scala(source))
        }
        Language::Groovy => {
            // Language-parity wave G2.1b: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_groovy`) -- see `tests/unit_languages_groovy.rs`.
            Some(generic::parse_groovy(source))
        }
        Language::Ruby => {
            // Language-parity wave G2.1c: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_ruby`) -- there is no bespoke `languages::ruby`
            // extractor to prove zero-regression against (Ruby has
            // never had one in this crate); see
            // `tests/unit_languages_ruby.rs`.
            Some(generic::parse_ruby(source))
        }
        Language::Zig => {
            // Language-parity wave G2.1c: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_zig`) -- see `tests/unit_languages_zig.rs`.
            Some(generic::parse_zig(source))
        }
        Language::ObjectiveC => {
            // Language-parity wave G2.1c: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_objc`) -- see `tests/unit_languages_objc.rs`.
            Some(generic::parse_objc(source))
        }
        Language::Bash => {
            // Language-parity wave G2.2f: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_bash`) -- see `tests/unit_languages_bash.rs`.
            Some(generic::parse_bash(source))
        }
        Language::Lua => {
            // Language-parity wave G2.2f: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_lua`) -- see `tests/unit_languages_lua.rs`.
            Some(generic::parse_lua(source))
        }
        Language::Elixir => {
            // Language-parity wave G2.2f: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_elixir`) -- see `tests/unit_languages_elixir.rs`.
            Some(generic::parse_elixir(source))
        }
        Language::Haskell => {
            // Language-parity wave G2.2g: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_haskell`) -- see `tests/unit_languages_haskell.rs`.
            Some(generic::parse_haskell(source))
        }
        Language::OCaml => {
            // Language-parity wave G2.2g: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_ocaml`) -- covers both `.ml`/`.mli` (see
            // `Language::OCaml`'s own doc comment); see
            // `tests/unit_languages_ocaml.rs`.
            Some(generic::parse_ocaml(source))
        }
        Language::Erlang => {
            // Language-parity wave G2.2g: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_erlang`) -- see `tests/unit_languages_erlang.rs`.
            Some(generic::parse_erlang(source))
        }
        Language::Cuda => {
            // Language-parity wave G2.2b: reuses the generic spec-table
            // engine's own C++ path (`languages::generic::parse_cuda`,
            // itself `parse_cpp` with the grammar entry point swapped --
            // see `LangSpec::cuda`'s own doc comment) -- same `tests/`/
            // `_test.cu`-suffix convention as every other C-family
            // language in this crate; see `tests/unit_languages_cuda.rs`.
            let is_test_file = is_c_family_test_path(rel_path, &["_test.cu", "_test.cuh"]);
            Some(generic::parse_cuda(source, is_test_file))
        }
        Language::D => {
            // Language-parity wave G2.2b: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_d`) -- there is no bespoke `languages::d` extractor
            // to prove zero-regression against (D has never had one in
            // this crate); see `tests/unit_languages_d.rs`.
            Some(generic::parse_d(source))
        }
        Language::PowerShell => {
            // Language-parity wave G2.2b: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_powershell`) -- see
            // `tests/unit_languages_powershell.rs`.
            Some(generic::parse_powershell(source))
        }
        Language::Fsharp => {
            // Language-parity wave G2.2c: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_fsharp`) -- there is no bespoke `languages::fsharp`
            // extractor to prove zero-regression against (F# has never
            // had one in this crate); see
            // `tests/unit_languages_fsharp.rs`.
            Some(generic::parse_fsharp(source))
        }
        Language::Gleam => {
            // Language-parity wave G2.2c: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_gleam`) -- see `tests/unit_languages_gleam.rs`.
            Some(generic::parse_gleam(source))
        }
        Language::Glsl => {
            // Language-parity wave G2.2c: reuses the generic spec-table
            // engine's own C path verbatim (`languages::generic::
            // parse_glsl`, itself `parse_c` with the grammar binding and
            // spec row both left as C's own except for `name` -- see
            // `LangSpec::glsl`'s own doc comment) -- see
            // `tests/unit_languages_glsl.rs`. `is_test_file` is not
            // threaded through (unlike `Language::C`/`Language::Cpp`):
            // GLSL shader source has no test-file naming convention in
            // the baseline (see `generic::parse_glsl`'s own doc comment).
            Some(generic::parse_glsl(source))
        }
        Language::Ada => {
            // Language-parity wave G2.2a: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_ada`) -- see `tests/unit_languages_ada.rs`.
            Some(generic::parse_ada(source))
        }
        Language::Apex => {
            // Language-parity wave G2.2a: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_apex`) -- see `tests/unit_languages_apex.rs`.
            Some(generic::parse_apex(source))
        }
        Language::Crystal => {
            // Language-parity wave G2.2a: onboarded directly through
            // the generic spec-table engine (`languages::generic::
            // parse_crystal`) -- see `tests/unit_languages_crystal.rs`.
            Some(generic::parse_crystal(source))
        }
        Language::R => {
            // Language-parity wave G2.2h: onboarded directly through the
            // generic spec-table engine (`languages::generic::parse_r`)
            // -- there is no bespoke `languages::r` extractor to prove
            // zero-regression against (R has never had one in this
            // crate); see `tests/unit_languages_r.rs`.
            Some(generic::parse_r(source))
        }
        Language::Perl => {
            // Language-parity wave G2.2h: onboarded directly through the
            // generic spec-table engine (`languages::generic::parse_perl`)
            // -- see `tests/unit_languages_perl.rs`.
            Some(generic::parse_perl(source))
        }
        Language::Clojure => {
            // Language-parity wave G2.2h: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_clojure`) -- see `tests/unit_languages_clojure.rs`.
            Some(generic::parse_clojure(source))
        }
        Language::Julia => {
            // Language-parity wave G2.2d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_julia`) -- there is no bespoke `languages::julia`
            // extractor to prove zero-regression against (Julia has
            // never had one in this crate); see
            // `tests/unit_languages_julia.rs`.
            Some(generic::parse_julia(source))
        }
        Language::Odin => {
            // Language-parity wave G2.2d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_odin`) -- see `tests/unit_languages_odin.rs`.
            Some(generic::parse_odin(source))
        }
        Language::Pascal => {
            // Language-parity wave G2.2d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_pascal`) -- see `tests/unit_languages_pascal.rs`.
            Some(generic::parse_pascal(source))
        }
        Language::Qml => {
            // Language-parity wave G2.2e: onboarded directly through the
            // generic spec-table engine (`languages::generic::parse_qml`)
            // -- see `tests/unit_languages_qml.rs`.
            Some(generic::parse_qml(source))
        }
        Language::Rescript => {
            // Language-parity wave G2.2e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_rescript`) -- see `tests/unit_languages_rescript.rs`.
            Some(generic::parse_rescript(source))
        }
        Language::Squirrel => {
            // Language-parity wave G2.2e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_squirrel`) -- see `tests/unit_languages_squirrel.rs`.
            Some(generic::parse_squirrel(source))
        }
        Language::Sway => {
            // Language-parity wave G2.3e: onboarded directly through the
            // generic spec-table engine (`languages::generic::parse_sway`)
            // -- see `tests/unit_languages_sway.rs`.
            Some(generic::parse_sway(source))
        }
        Language::Starlark => {
            // Language-parity wave G2.3e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_starlark`) -- see `tests/unit_languages_starlark.rs`.
            Some(generic::parse_starlark(source))
        }
        Language::Templ => {
            // Language-parity wave G2.3e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_templ`) -- see `tests/unit_languages_templ.rs`.
            Some(generic::parse_templ(source))
        }
        Language::Typst => {
            // Language-parity wave G2.3e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_typst`) -- see `tests/unit_languages_typst.rs`.
            Some(generic::parse_typst(source))
        }
        Language::Wgsl => {
            // Language-parity wave G2.3e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_wgsl`) -- see `tests/unit_languages_wgsl.rs`.
            Some(generic::parse_wgsl(source))
        }
        Language::Wolfram => {
            // Language-parity wave G2.3e: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_wolfram`) -- see `tests/unit_languages_wolfram.rs`.
            Some(generic::parse_wolfram(source))
        }
        Language::Slang => {
            // Language-parity wave G2.3e: reuses the generic spec-table
            // engine's own C++ path verbatim (`languages::generic::
            // parse_slang`, itself C++'s own `cpp_quirks` with the grammar
            // binding swapped -- see `LangSpec::slang`'s own doc comment)
            // -- see `tests/unit_languages_slang.rs`.
            Some(generic::parse_slang(source))
        }
        Language::Scss => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::parse_scss`)
            // -- grammar VENDORED (see `Language::Scss`'s own doc comment)
            // -- see `tests/unit_languages_scss.rs`.
            Some(generic::parse_scss(source))
        }
        Language::Cmake => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_cmake`) -- see `tests/unit_languages_cmake.rs`.
            Some(generic::parse_cmake(source))
        }
        Language::Makefile => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_makefile`) -- see `tests/unit_languages_makefile.rs`.
            Some(generic::parse_makefile(source))
        }
        Language::Fortran => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_fortran`) -- see `tests/unit_languages_fortran.rs`.
            Some(generic::parse_fortran(source))
        }
        Language::Vimscript => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_vimscript`) -- see `tests/unit_languages_vimscript.rs`.
            Some(generic::parse_vimscript(source))
        }
        Language::Puppet => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_puppet`) -- see `tests/unit_languages_puppet.rs`.
            Some(generic::parse_puppet(source))
        }
        Language::Elm => {
            // Language-parity wave G2.3a: onboarded directly through the
            // generic spec-table engine (`languages::generic::parse_elm`)
            // -- see `tests/unit_languages_elm.rs`.
            Some(generic::parse_elm(source))
        }
        Language::Bicep => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_bicep`) -- see `tests/unit_languages_bicep.rs`.
            Some(generic::parse_bicep(source))
        }
        Language::Bitbake => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_bitbake`), grammar VENDORED (see `Language::Bitbake`'s
            // own doc comment) -- see `tests/unit_languages_bitbake.rs`.
            Some(generic::parse_bitbake(source))
        }
        Language::Cairo => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_cairo`), grammar VENDORED (see `Language::Cairo`'s own
            // doc comment) -- see `tests/unit_languages_cairo.rs`.
            Some(generic::parse_cairo(source))
        }
        Language::Cfscript => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_cfscript`) -- see `tests/unit_languages_cfscript.rs`.
            Some(generic::parse_cfscript(source))
        }
        Language::Func => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_func`), grammar VENDORED (see `Language::Func`'s own
            // doc comment) -- see `tests/unit_languages_func.rs`.
            Some(generic::parse_func(source))
        }
        Language::Move => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_move`), grammar VENDORED (see `Language::Move`'s own
            // doc comment) -- see `tests/unit_languages_move.rs`.
            Some(generic::parse_move(source))
        }
        Language::Nickel => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_nickel`) -- see `tests/unit_languages_nickel.rs`.
            Some(generic::parse_nickel(source))
        }
        Language::Jsonnet => {
            // Language-parity wave G2.3c: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_jsonnet`) -- see `tests/unit_languages_jsonnet.rs`.
            Some(generic::parse_jsonnet(source))
        }
        Language::Just => {
            // Language-parity wave G2.3d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_just`) -- see `tests/unit_languages_just.rs`.
            Some(generic::parse_just(source))
        }
        Language::Hlsl => {
            // Language-parity wave G2.3d: reuses the generic spec-table
            // engine's own C++ path verbatim (`languages::generic::
            // parse_hlsl`, itself C++'s own `cpp_quirks` with the grammar
            // binding swapped -- see `LangSpec::hlsl`'s own doc comment)
            // -- see `tests/unit_languages_hlsl.rs`.
            Some(generic::parse_hlsl(source))
        }
        Language::Ispc => {
            // Language-parity wave G2.3d: reuses the generic spec-table
            // engine's own C path verbatim (`languages::generic::
            // parse_ispc`, itself C's own `c_quirks` with the grammar
            // binding swapped -- see `LangSpec::ispc`'s own doc comment),
            // grammar VENDORED (see `Language::Ispc`'s own doc comment)
            // -- see `tests/unit_languages_ispc.rs`.
            Some(generic::parse_ispc(source))
        }
        Language::Purescript => {
            // Language-parity wave G2.3d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_purescript`) -- see
            // `tests/unit_languages_purescript.rs`.
            Some(generic::parse_purescript(source))
        }
        Language::Magma => {
            // Language-parity wave G2.3d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_magma`) -- see `tests/unit_languages_magma.rs`.
            Some(generic::parse_magma(source))
        }
        Language::Hare => {
            // Language-parity wave G2.3d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_hare`), grammar VENDORED (see `Language::Hare`'s own
            // doc comment) -- see `tests/unit_languages_hare.rs`.
            Some(generic::parse_hare(source))
        }
        Language::Pony => {
            // Language-parity wave G2.3d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_pony`), grammar VENDORED (see `Language::Pony`'s own
            // doc comment) -- see `tests/unit_languages_pony.rs`.
            Some(generic::parse_pony(source))
        }
        Language::Nasm => {
            // Language-parity wave G2.3d: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_nasm`), grammar VENDORED (see `Language::Nasm`'s own
            // doc comment) -- see `tests/unit_languages_nasm.rs`.
            Some(generic::parse_nasm(source))
        }
        Language::Cobol => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_cobol`), grammar VENDORED (see `Language::Cobol`'s
            // own doc comment) -- see `tests/unit_languages_cobol.rs`.
            Some(generic::parse_cobol(source))
        }
        Language::Commonlisp => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_commonlisp`) -- see
            // `tests/unit_languages_commonlisp.rs`.
            Some(generic::parse_commonlisp(source))
        }
        Language::Lean => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_lean`) -- see `tests/unit_languages_lean.rs`.
            Some(generic::parse_lean(source))
        }
        Language::Tlaplus => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_tlaplus`) -- see `tests/unit_languages_tlaplus.rs`.
            Some(generic::parse_tlaplus(source))
        }
        Language::Verilog => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_verilog`) -- see `tests/unit_languages_verilog.rs`.
            Some(generic::parse_verilog(source))
        }
        Language::Vhdl => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_vhdl`) -- see `tests/unit_languages_vhdl.rs`.
            Some(generic::parse_vhdl(source))
        }
        Language::Systemverilog => {
            // Language-parity wave G2.3b: onboarded directly through the
            // generic spec-table engine (`languages::generic::
            // parse_systemverilog`) -- see
            // `tests/unit_languages_systemverilog.rs`.
            Some(generic::parse_systemverilog(source))
        }
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
