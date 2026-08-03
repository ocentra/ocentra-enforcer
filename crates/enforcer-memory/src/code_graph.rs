//! X06.2: the code KG indexer.
//!
//! Indexes a repository's current HEAD into a [`CodeGraph`] of files,
//! symbols (functions/types/tests), routes, imports, and calls, using
//! [`crate::parsers`]/[`crate::languages`] (tree-sitter) for structural
//! extraction and [`crate::git`] for commit metadata. Every file gets a
//! node -- languages with no extractor still get a
//! [`CodeNode::TextOnly`] node (the workpack's "never silent skip" hard
//! requirement) rather than being dropped from the graph.
//!
//! # Incremental indexing (owner-set: "indexes are disposable,
//! knowledge is not")
//!
//! [`CodeGraph::index_repository`] takes the *previous* run's
//! [`Manifest`] (or `Manifest::default()` for a first run) and returns
//! an updated [`Manifest`] plus an [`IndexReport`] describing exactly
//! what changed:
//!
//! - unchanged files (content hash matches the stored manifest entry)
//!   are skipped entirely -- no re-parse, no new nodes;
//! - changed and new files are (re)parsed and their old nodes (if any)
//!   are superseded;
//! - files present in the previous manifest but absent from the current
//!   walk are deleted: their old node is replaced with a
//!   [`CodeNode::Tombstone`] carrying the file's last-known chunk ids
//!   and history summary, never silently forgotten.
//!
//! This module defines a narrow storage seam
//! ([`CodeGraph::nodes`]/[`CodeGraph::manifest`] plus [`Manifest`]'s
//! plain-data shape) rather than a persistence trait: the graph and
//! manifest are both `Serialize`/`Deserialize`-free plain structs a
//! caller (or X06.1's SQLite store, once landed) can snapshot to/from
//! disk however it likes -- this crate does not open a database itself
//! per the file-ownership split with X06.1.

use crate::git::GitMetadata;
pub mod fingerprint;

use enforcer_domain::memory_types::{
    ComplexityLanguage, ComplexitySourceBytes, ComplexitySymbolLocation, GraphChangeCount,
    GraphSourceLine, GraphSymbolKindSnapshot, IndexMode, LanguageTag, OperationalGraphEdgeRow,
    OperationalGraphNodeRow, ParserSourceText, ReceiverHint,
};
use enforcer_syntax::parsers::{self, Language, ParsedFile};
use fingerprint::{hash_bytes, source_body_fingerprints_for_symbols, SourceBodyFingerprint};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One node in the code graph. Deliberately flat (no separate edge
/// list type per node) -- edges below ([`ImportEdge`], [`CallEdge`],
/// [`RouteEdge`]) reference node ids by string, resolved lazily by
/// callers that need traversal (X06.3's concern), not by this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeNode {
    File(FileNode),
    Function(SymbolNode),
    Type(SymbolNode),
    Test(SymbolNode),
    /// A file whose extension has no structural extractor. Still a
    /// first-class node -- see module docs, "never silent skip".
    TextOnly(FileNode),
    /// A file that existed in a previous index run and is gone now.
    /// Retains the file-history summary the workpack requires callers
    /// keep even after deletion.
    Tombstone(TombstoneNode),
    /// X06 rich vocabulary (additive): a function defined inside an
    /// `impl`/class body, as opposed to a free-standing [`Function`].
    Method(SymbolNode),
    /// A `class` product type (TS/Python).
    Class(SymbolNode),
    /// A Rust `struct` specifically.
    Struct(SymbolNode),
    /// An interface/trait declaration.
    Interface(SymbolNode),
    /// An enum declaration.
    Enum(SymbolNode),
    /// A type alias declaration.
    TypeAlias(SymbolNode),
    /// A module/namespace container.
    Module(SymbolNode),
    /// A named, assigned lambda/closure.
    Lambda(SymbolNode),
    /// A top-level variable binding.
    Variable(SymbolNode),
    /// A top-level constant binding.
    Constant(SymbolNode),
}

impl CodeNode {
    /// Return the stable identifier carried by this graph node.
    pub fn id(&self) -> &str {
        match self {
            CodeNode::File(node) | CodeNode::TextOnly(node) => &node.id,
            CodeNode::Function(node)
            | CodeNode::Type(node)
            | CodeNode::Test(node)
            | CodeNode::Method(node)
            | CodeNode::Class(node)
            | CodeNode::Struct(node)
            | CodeNode::Interface(node)
            | CodeNode::Enum(node)
            | CodeNode::TypeAlias(node)
            | CodeNode::Module(node)
            | CodeNode::Lambda(node)
            | CodeNode::Variable(node)
            | CodeNode::Constant(node) => &node.id,
            CodeNode::Tombstone(node) => &node.id,
        }
    }
}

/// A file node: one indexed source/config/text file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNode {
    /// Stable id: `file:<rel_path>` (forward-slash-normalized).
    pub id: String,
    pub rel_path: String,
    pub language: LanguageTag,
    pub content_hash: String,
    /// Last commit that touched this path, if the repo has git history
    /// and this path has been committed at least once.
    pub last_commit: Option<String>,
    /// How many commits in HEAD's history have touched this path.
    pub change_count: GraphChangeCount,
    /// Ids of every symbol/route/import/call chunk this file produced
    /// in the current index run -- the "previous chunk ids" the
    /// workpack's file-history summary requires be retained across
    /// reindexing (and, for a later delete, into the resulting
    /// [`TombstoneNode`]).
    pub chunk_ids: Vec<String>,
}

/// A function/type/test symbol found inside a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolNode {
    /// Stable id: `sym:<rel_path>:<line>:<name>`.
    pub id: String,
    pub name: String,
    pub file_id: String,
    pub line: GraphSourceLine,
    /// X06 core parity: Tier A complexity metrics
    /// ([`crate::complexity::compute`]) -- `Some` for callable symbols
    /// ([`CodeNode::Function`]/[`CodeNode::Method`]/[`CodeNode::Test`]/
    /// [`CodeNode::Lambda`]), matching the baseline's "Function/Method
    /// only" property scope (`docs/plans/enforcer-selfhost-plan/refs/
    /// x06-baseline-tool-schemas.md` §4.5); `None` on every other
    /// [`CodeNode`] variant. Additive field -- every existing
    /// `SymbolNode { .. }` literal in this crate needed exactly one
    /// more field, no signature change.
    pub metrics: Option<crate::complexity::ComplexityMetrics>,
    /// X06 core parity Tier B: interprocedural metrics
    /// ([`crate::complexity::propagate_transitive_loop_depth`]),
    /// populated by a post-index pass over the whole repo's call graph
    /// once every symbol id is known. `None` until that pass runs.
    pub transitive_metrics: Option<enforcer_domain::memory_types::ComplexityTransitiveMetrics>,
    /// Baseline-compatible source/body fingerprint evidence for
    /// `SIMILAR_TO`: deterministic shingles over the callable body text
    /// recovered during indexing. `None` when no source body can be
    /// recovered, such as Store projections without source text.
    pub source_body_fingerprint: Option<SourceBodyFingerprint>,
}

/// A file that was indexed in a previous run and is no longer present.
/// Never removed from the graph outright -- see module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TombstoneNode {
    pub id: String,
    pub rel_path: String,
    pub last_commit: Option<String>,
    pub change_count: GraphChangeCount,
    /// Chunk ids the file had at the time it was deleted (its last
    /// [`FileNode::chunk_ids`] before deletion).
    pub prior_chunk_ids: Vec<String>,
}

fn language_tag(language: Language) -> LanguageTag {
    match language {
        Language::Rust => LanguageTag::Rust,
        Language::TypeScript => LanguageTag::TypeScript,
        Language::JavaScript => LanguageTag::JavaScript,
        Language::Python => LanguageTag::Python,
        Language::Go => LanguageTag::Go,
        Language::Java => LanguageTag::Java,
        Language::C => LanguageTag::C,
        Language::Cpp => LanguageTag::Cpp,
        Language::CSharp => LanguageTag::CSharp,
        Language::Php => LanguageTag::Php,
        Language::Kotlin => LanguageTag::Kotlin,
        Language::Swift => LanguageTag::Swift,
        Language::Tsx => LanguageTag::Tsx,
        Language::Solidity => LanguageTag::Solidity,
        Language::Gdscript => LanguageTag::Gdscript,
        Language::Dart => LanguageTag::Dart,
        Language::Scala => LanguageTag::Scala,
        Language::Groovy => LanguageTag::Groovy,
        Language::Ruby => LanguageTag::Ruby,
        Language::Zig => LanguageTag::Zig,
        Language::ObjectiveC => LanguageTag::ObjectiveC,
        Language::Bash => LanguageTag::Bash,
        Language::Lua => LanguageTag::Lua,
        Language::Elixir => LanguageTag::Elixir,
        Language::Haskell => LanguageTag::Haskell,
        Language::OCaml => LanguageTag::OCaml,
        Language::Erlang => LanguageTag::Erlang,
        Language::Cuda => LanguageTag::Cuda,
        Language::D => LanguageTag::D,
        Language::PowerShell => LanguageTag::PowerShell,
        Language::Fsharp => LanguageTag::Fsharp,
        Language::Gleam => LanguageTag::Gleam,
        Language::Glsl => LanguageTag::Glsl,
        Language::Ada => LanguageTag::Ada,
        Language::Apex => LanguageTag::Apex,
        Language::Crystal => LanguageTag::Crystal,
        Language::R => LanguageTag::R,
        Language::Perl => LanguageTag::Perl,
        Language::Clojure => LanguageTag::Clojure,
        Language::Julia => LanguageTag::Julia,
        Language::Odin => LanguageTag::Odin,
        Language::Pascal => LanguageTag::Pascal,
        Language::Qml => LanguageTag::Qml,
        Language::Rescript => LanguageTag::Rescript,
        Language::Squirrel => LanguageTag::Squirrel,
        Language::Sway => LanguageTag::Sway,
        Language::Starlark => LanguageTag::Starlark,
        Language::Templ => LanguageTag::Templ,
        Language::Typst => LanguageTag::Typst,
        Language::Wgsl => LanguageTag::Wgsl,
        Language::Wolfram => LanguageTag::Wolfram,
        Language::Slang => LanguageTag::Slang,
        Language::Scss => LanguageTag::Scss,
        Language::Cmake => LanguageTag::Cmake,
        Language::Makefile => LanguageTag::Makefile,
        Language::Fortran => LanguageTag::Fortran,
        Language::Vimscript => LanguageTag::Vimscript,
        Language::Puppet => LanguageTag::Puppet,
        Language::Elm => LanguageTag::Elm,
        Language::Bicep => LanguageTag::Bicep,
        Language::Bitbake => LanguageTag::Bitbake,
        Language::Cairo => LanguageTag::Cairo,
        Language::Cfscript => LanguageTag::Cfscript,
        Language::Func => LanguageTag::Func,
        Language::Move => LanguageTag::Move,
        Language::Nickel => LanguageTag::Nickel,
        Language::Jsonnet => LanguageTag::Jsonnet,
        Language::Just => LanguageTag::Just,
        Language::Hlsl => LanguageTag::Hlsl,
        Language::Ispc => LanguageTag::Ispc,
        Language::Purescript => LanguageTag::Purescript,
        Language::Magma => LanguageTag::Magma,
        Language::Hare => LanguageTag::Hare,
        Language::Pony => LanguageTag::Pony,
        Language::Nasm => LanguageTag::Nasm,
        Language::Cobol => LanguageTag::Cobol,
        Language::Commonlisp => LanguageTag::Commonlisp,
        Language::Lean => LanguageTag::Lean,
        Language::Tlaplus => LanguageTag::Tlaplus,
        Language::Verilog => LanguageTag::Verilog,
        Language::Vhdl => LanguageTag::Vhdl,
        Language::Systemverilog => LanguageTag::Systemverilog,
        Language::Capnp => LanguageTag::Capnp,
        Language::EmacsLisp => LanguageTag::EmacsLisp,
        Language::Agda => LanguageTag::Agda,
        Language::Form => LanguageTag::Form,
        Language::Awk => LanguageTag::Awk,
        Language::Fish => LanguageTag::Fish,
        Language::Zsh => LanguageTag::Zsh,
        Language::Tcl => LanguageTag::Tcl,
        Language::Scheme => LanguageTag::Scheme,
        Language::Racket => LanguageTag::Racket,
        Language::Smithy => LanguageTag::Smithy,
        Language::Pine => LanguageTag::Pine,
        Language::Matlab => LanguageTag::Matlab,
        Language::Luau => LanguageTag::Luau,
        Language::Teal => LanguageTag::Teal,
        Language::Fennel => LanguageTag::Fennel,
        Language::Meson => LanguageTag::Meson,
        Language::Kconfig => LanguageTag::Kconfig,
        Language::Hcl => LanguageTag::Hcl,
        Language::Nix => LanguageTag::Nix,
        Language::Sql => LanguageTag::Sql,
        Language::Protobuf => LanguageTag::Protobuf,
        Language::Prisma => LanguageTag::Prisma,
        Language::Pkl => LanguageTag::Pkl,
        Language::Thrift => LanguageTag::Thrift,
        Language::Wit => LanguageTag::Wit,
        Language::LlvmIr => LanguageTag::LlvmIr,
        Language::TableGen => LanguageTag::TableGen,
        Language::Cfml => LanguageTag::Cfml,
        Language::Gotemplate => LanguageTag::Gotemplate,
        Language::Devicetree => LanguageTag::Devicetree,
        Language::Smali => LanguageTag::Smali,
        Language::Json5 => LanguageTag::Json5,
        Language::Kdl => LanguageTag::Kdl,
        Language::LinkerScript => LanguageTag::LinkerScript,
        Language::Liquid => LanguageTag::Liquid,
        Language::Markdown => LanguageTag::Markdown,
        Language::Mermaid => LanguageTag::Mermaid,
        Language::Po => LanguageTag::Po,
        Language::Properties => LanguageTag::Properties,
        Language::Regex => LanguageTag::Regex,
        Language::Assembly => LanguageTag::Assembly,
        Language::Astro => LanguageTag::Astro,
        Language::Beancount => LanguageTag::Beancount,
        Language::Bibtex => LanguageTag::Bibtex,
        Language::Blade => LanguageTag::Blade,
        Language::Css => LanguageTag::Css,
        Language::Csv => LanguageTag::Csv,
        Language::Diff => LanguageTag::Diff,
        Language::Dockerfile => LanguageTag::Dockerfile,
        Language::Dotenv => LanguageTag::Dotenv,
        Language::Gitattributes => LanguageTag::Gitattributes,
        Language::Gitignore => LanguageTag::Gitignore,
        Language::Gn => LanguageTag::Gn,
        Language::GoMod => LanguageTag::GoMod,
        Language::Graphql => LanguageTag::Graphql,
        Language::Html => LanguageTag::Html,
        Language::Hyprlang => LanguageTag::Hyprlang,
        Language::Ini => LanguageTag::Ini,
        Language::Janet => LanguageTag::Janet,
        Language::Jinja2 => LanguageTag::Jinja2,
        Language::Jsdoc => LanguageTag::Jsdoc,
        Language::Json => LanguageTag::Json,
        Language::Requirements => LanguageTag::Requirements,
        Language::Ron => LanguageTag::Ron,
        Language::Rst => LanguageTag::Rst,
        Language::Soql => LanguageTag::Soql,
        Language::Sosl => LanguageTag::Sosl,
        Language::Sshconfig => LanguageTag::Sshconfig,
        Language::Svelte => LanguageTag::Svelte,
        Language::Toml => LanguageTag::Toml,
        Language::Vue => LanguageTag::Vue,
        Language::Xml => LanguageTag::Xml,
        Language::Yaml => LanguageTag::Yaml,
        Language::ConfigToml => LanguageTag::ConfigToml,
        Language::ConfigJson => LanguageTag::ConfigJson,
        Language::ConfigYaml => LanguageTag::ConfigYaml,
        Language::TextOnly => LanguageTag::TextOnly,
    }
}

/// An import edge: `from_file_id` imports `module_path` as written in
/// source (unresolved to a target node id -- resolving import paths to
/// concrete file/module nodes across an arbitrary build system is out
/// of scope for this slice; the edge still records everything a later
/// resolution pass would need).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEdge {
    pub from_file_id: String,
    pub module_path: String,
    pub line: GraphSourceLine,
}

/// A call edge: `from_file_id` calls `callee` (as written) at `line`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallEdge {
    pub from_file_id: String,
    pub callee: String,
    pub line: GraphSourceLine,
    /// X06 core parity (cross-repo-intelligence): each call argument's
    /// raw source text, in written order, carried straight through from
    /// [`crate::parsers::CallRef::arg_texts`] -- additive, defaults to
    /// empty for any construction site that predates this field (there
    /// is exactly one, [`CodeGraph::insert_file_and_chunks`], updated
    /// below). [`crate::cross_repo`] scans this for a URL/path literal
    /// on an outbound HTTP-client-shaped callee (`fetch`, `axios.get`,
    /// `requests.get`, `reqwest::get`, ...) to match against another
    /// project's declared [`RouteEdge`]s -- see that module's doc
    /// comment for the exact heuristic and its honest limitations.
    pub arg_texts: Vec<String>,
    /// X06 type-aware resolution (additive): the name of the function/
    /// method this call is lexically inside of, carried straight
    /// through from [`crate::parsers::CallRef::from_symbol`]. `None`
    /// for any construction site/extractor that predates or does not
    /// populate this field, or for a module-scope call with no
    /// enclosing symbol -- [`crate::resolution`] falls back to its
    /// unique-name matching pass when absent.
    pub from_symbol: Option<String>,
    /// The enclosing symbol's own start line, paired with
    /// [`Self::from_symbol`] -- lets [`crate::resolution`] rebuild that
    /// symbol's stable `sym:` id without a separate name+line lookup.
    pub from_symbol_line: Option<GraphSourceLine>,
    /// The method-call receiver's own text (`x` in `x.foo()`), carried
    /// through from [`crate::parsers::CallRef::receiver_text`].
    pub receiver_text: Option<String>,
    /// Cheap syntactic classification of [`Self::receiver_text`],
    /// carried through from [`crate::parsers::CallRef::receiver_hint`].
    pub receiver_hint: Option<ReceiverHint>,
}

/// A route/endpoint declared in `from_file_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEdge {
    pub from_file_id: String,
    pub method: String,
    pub path: String,
    pub line: GraphSourceLine,
}

/// X06 rich vocabulary (additive): an INHERITS edge -- `sub_id` extends
/// or is a subtrait of a symbol named `super_name` (unresolved to a
/// node id; best-effort name resolution is [`crate::analysis`]'s job,
/// matching every other edge kind here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritsEdge {
    pub sub_id: String,
    pub super_name: String,
    pub line: GraphSourceLine,
}

/// An IMPLEMENTS edge -- `type_id` implements a trait/interface named
/// `trait_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementsEdge {
    pub type_id: String,
    pub trait_name: String,
    pub line: GraphSourceLine,
}

/// A DECORATES edge -- `target_id` is decorated by `decorator_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoratesEdge {
    pub target_id: String,
    pub decorator_name: String,
    pub line: GraphSourceLine,
}

/// A TYPE_REF edge -- `from_id`'s signature references a type named
/// `type_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRefEdge {
    pub from_id: String,
    pub type_name: String,
    pub line: GraphSourceLine,
}

/// A DEFINES edge -- `container_id` defines member symbol `member_id`
/// (both already-resolved node ids, since both ends are symbols
/// extracted from the same file in the same pass).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinesEdge {
    pub container_id: String,
    pub member_id: String,
    pub line: GraphSourceLine,
}

/// One file's entry in the manifest carried between index runs: the
/// content hash used for the unchanged-file skip, plus enough of the
/// file-history summary to reconstruct a [`TombstoneNode`] if the file
/// is later deleted without needing to re-walk git history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub content_hash: String,
    pub last_commit: Option<String>,
    pub change_count: GraphChangeCount,
    pub chunk_ids: Vec<String>,
}

/// The previous run's per-file manifest, keyed by repo-relative,
/// forward-slash-normalized path. Passed into
/// [`CodeGraph::index_repository`] and returned updated so the caller
/// can persist it (to disk, to X06.1's store once landed, wherever) for
/// the next run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    pub entries: HashMap<String, ManifestEntry>,
}

/// What changed in one [`CodeGraph::index_repository`] call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexReport {
    pub unchanged: Vec<String>,
    pub changed: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
}

/// X06.8: indexing depth, mirroring the baseline doc's `full`/`moderate`/
/// `fast` mode semantics (see `docs/plans/enforcer-selfhost-plan/refs/
/// x06-baseline-tool-schemas.md` §9.2). [`crate::similarity`]'s
/// `SIMILAR_TO`/`SEMANTICALLY_RELATED` edges are a caller-invoked
/// post-index pass over a finished [`CodeGraph`] snapshot (like
/// [`crate::resolution::resolve`]), not gated by [`IndexMode`] here --
/// this crate has no macro-extraction pass to gate either, so the one
/// gate [`IndexMode`] currently controls is git-history computation,
/// exactly matching the baseline's one universally-confirmed mode
/// distinction: "`full` and `moderate` both compute git history; only
/// `fast` omits it." Adding a future gated pass should extend this
/// enum's match arms, not add a parallel boolean flag.
/// X06.8: bundled options for [`CodeGraph::index_repository_with_options`],
/// the extended entry point that adds indexing-depth gating and the
/// optional `.codebase-memory/graph.db.zst` persistence artifact on top
/// of the original [`CodeGraph::index_repository`] (which stays
/// unchanged -- all of its existing call sites keep compiling against
/// the original 3-argument signature).
#[derive(Debug, Clone, Copy, Default)]
pub struct IndexOptions<'a> {
    pub mode: IndexMode,
    /// When `true`, a successful index run additionally exports a
    /// `.codebase-memory/graph.db.zst` + `artifact.json` persistence
    /// artifact for `repo_root` via [`crate::artifacts`].
    pub persistence: bool,
    /// Project name recorded in `artifact.json` when `persistence` is
    /// enabled. Required (as `Some`) whenever `persistence` is `true`;
    /// ignored otherwise.
    pub project_name: Option<&'a str>,
    /// Timestamp recorded in `artifact.json` when `persistence` is
    /// enabled (RFC3339-ish string, caller-supplied so this module has
    /// no wall-clock dependency). Required (as `Some`) whenever
    /// `persistence` is `true`; ignored otherwise.
    pub indexed_at: Option<&'a str>,
}

/// X06.8: errors from [`CodeGraph::index_repository_with_options`] that
/// are specific to the options wrapper (persistence/bootstrap) rather
/// than the underlying indexing pass (those still surface as
/// [`IndexError`]).
#[derive(Debug, thiserror::Error)]
pub enum IndexWithOptionsError {
    #[error(transparent)]
    Index(#[from] IndexError),
    #[error("persistence=true requires project_name to be set")]
    MissingProjectName,
    #[error("persistence=true requires indexed_at to be set")]
    MissingIndexedAt,
    #[error(transparent)]
    Artifact(#[from] crate::artifacts::GraphArtifactError),
}

/// Bundled arguments for [`CodeGraph::insert_file_and_chunks`] --
/// grouped into one struct rather than passed positionally so adding a
/// future field (e.g. a language-server diagnostic) does not require
/// touching every call site's argument order.
struct NewFileParams<'a> {
    rel_path: &'a str,
    language: Language,
    content_hash: &'a str,
    history: &'a crate::git::PathHistory,
    parsed: Option<ParsedFile>,
    /// X06 core parity: the file's source text, re-parsed into a
    /// tree-sitter AST here (independent of [`Self::parsed`]'s own
    /// walk) purely to compute Tier A complexity metrics per callable
    /// symbol via [`crate::complexity::find_definition_node`] +
    /// [`crate::complexity::compute`]. `None` for [`Language::TextOnly`]/
    /// config languages, matching `parsed`'s own `None` case.
    text: Option<&'a str>,
}

/// The code graph: a flat node store plus typed edge lists, exactly
/// mirroring the shape of [`crate::graph::MemoryGraph`] (flat node
/// vec, no in-place mutation of existing nodes -- reindexing appends a
/// new node revision and, for deletions, a tombstone, rather than
/// editing history away).
#[derive(Debug, Clone, Default)]
pub struct CodeGraph {
    nodes: Vec<CodeNode>,
    imports: Vec<ImportEdge>,
    calls: Vec<CallEdge>,
    routes: Vec<RouteEdge>,
    inherits: Vec<InheritsEdge>,
    implements: Vec<ImplementsEdge>,
    decorates: Vec<DecoratesEdge>,
    type_refs: Vec<TypeRefEdge>,
    defines: Vec<DefinesEdge>,
    /// X06 type-aware resolution (additive): [`crate::resolution::resolve`]'s
    /// output over this graph's own [`Self::calls`], recomputed by
    /// [`Self::index_repository_at_mode`] (and therefore by both public
    /// entry points) after every index run. Index-aligned with
    /// [`Self::calls`] -- `resolved_calls()[i]` is the resolution result
    /// for `calls()[i]`. Empty (not absent) on a freshly `default()`-
    /// constructed graph that has never been indexed, same "empty, not
    /// fabricated" convention as every other derived field here.
    resolved_calls: Vec<crate::resolution::ResolvedCall>,
}

impl CodeGraph {
    /// Construct an empty graph with no fabricated nodes or edges.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild a minimal [`CodeGraph`] read model from the Store-backed
    /// operational projection. This is intentionally lossy: Store graph
    /// events are the canonical facts here (`node_id`, `node_kind`, edge
    /// endpoints, label), so fields that require source text or git
    /// history are left empty instead of guessed.
    pub fn from_store_projection(
        nodes: &[OperationalGraphNodeRow],
        edges: &[OperationalGraphEdgeRow],
    ) -> Self {
        let mut graph = Self::new();

        for row in nodes {
            graph.nodes.push(code_node_from_store_projection(
                row.node_id.as_str(),
                row.node_kind.as_str(),
            ));
        }

        for row in edges {
            let from = row.from_id.as_str();
            let to = row.to_id.as_str();
            match normalize_store_projection_kind(row.label.as_str()).as_str() {
                "calls" | "call" => graph.calls.push(CallEdge {
                    from_file_id: file_id_for_projection_edge(from),
                    callee: node_name_from_projection_id(to),
                    line: line_from_projection_id(to),
                    ..CallEdge::default()
                }),
                "imports" | "import" => graph.imports.push(ImportEdge {
                    from_file_id: file_id_for_projection_edge(from),
                    module_path: module_path_from_projection_target(to),
                    line: line_from_projection_id(to),
                }),
                "route" | "routes" => {
                    let (method, path) = route_parts_from_projection_target(to);
                    graph.routes.push(RouteEdge {
                        from_file_id: file_id_for_projection_edge(from),
                        method,
                        path,
                        line: line_from_projection_id(to),
                    });
                }
                "inherits" | "inherit" => graph.inherits.push(InheritsEdge {
                    sub_id: from.to_owned(),
                    super_name: node_name_from_projection_id(to),
                    line: line_from_projection_id(to),
                }),
                "implements" | "implement" => graph.implements.push(ImplementsEdge {
                    type_id: from.to_owned(),
                    trait_name: node_name_from_projection_id(to),
                    line: line_from_projection_id(to),
                }),
                "decorates" | "decorate" => graph.decorates.push(DecoratesEdge {
                    target_id: from.to_owned(),
                    decorator_name: node_name_from_projection_id(to),
                    line: line_from_projection_id(to),
                }),
                "typeref" | "type_ref" | "type_refs" => graph.type_refs.push(TypeRefEdge {
                    from_id: from.to_owned(),
                    type_name: node_name_from_projection_id(to),
                    line: line_from_projection_id(to),
                }),
                "defines" | "define" => graph.defines.push(DefinesEdge {
                    container_id: from.to_owned(),
                    member_id: to.to_owned(),
                    line: line_from_projection_id(to),
                }),
                // `contains` is derived by CodeAdjacency from each symbol's
                // file_id, so replaying it here would duplicate edges.
                "contains" | "contain" => {}
                _ => {}
            }
        }

        graph
    }

    /// Internal seam used by tests that need small, hand-built graphs
    /// without depending on the full tree-sitter extraction pipeline.
    /// Kept crate-private so it does not become part of the public API.
    #[doc(hidden)]
    pub fn push_route_for_test(&mut self, route: RouteEdge) {
        self.routes.push(route);
    }

    /// Symmetric internal seam for tests that need to inject call
    /// edges directly.
    #[doc(hidden)]
    pub fn push_call_for_test(&mut self, call: CallEdge) {
        self.calls.push(call);
    }

    /// Return all graph nodes in insertion order.
    pub fn nodes(&self) -> &[CodeNode] {
        &self.nodes
    }

    /// Return import edges captured during indexing.
    pub fn imports(&self) -> &[ImportEdge] {
        &self.imports
    }

    /// Return call edges captured during indexing.
    pub fn calls(&self) -> &[CallEdge] {
        &self.calls
    }

    /// Return route edges captured during indexing.
    pub fn routes(&self) -> &[RouteEdge] {
        &self.routes
    }

    /// Return inheritance edges captured during indexing.
    pub fn inherits(&self) -> &[InheritsEdge] {
        &self.inherits
    }

    /// Return interface-implementation edges captured during indexing.
    pub fn implements(&self) -> &[ImplementsEdge] {
        &self.implements
    }

    /// Return decorator edges captured during indexing.
    pub fn decorates(&self) -> &[DecoratesEdge] {
        &self.decorates
    }

    /// Return type-reference edges captured during indexing.
    pub fn type_refs(&self) -> &[TypeRefEdge] {
        &self.type_refs
    }

    /// Return definition edges captured during indexing.
    pub fn defines(&self) -> &[DefinesEdge] {
        &self.defines
    }

    /// X06 type-aware resolution (additive): the resolved form of
    /// [`Self::calls`], one [`crate::resolution::ResolvedCall`] per
    /// entry in [`Self::calls`] at the same index -- see
    /// [`crate::resolution`]'s module docs for the resolution ladder.
    /// Recomputed on every [`Self::index_repository`]/
    /// [`Self::index_repository_with_options`] call; empty on a graph
    /// that has never been indexed (e.g. restored only via
    /// [`Self::restore_from_snapshot`], which -- like every other
    /// derived-at-index-time field -- does not recompute this).
    pub fn resolved_calls(&self) -> &[crate::resolution::ResolvedCall] {
        &self.resolved_calls
    }

    /// Iterate over file and text-only nodes.
    pub fn file_nodes(&self) -> impl Iterator<Item = &FileNode> {
        self.nodes.iter().filter_map(|n| match n {
            CodeNode::File(f) | CodeNode::TextOnly(f) => Some(f),
            _ => None,
        })
    }

    /// Iterate over deleted-file tombstones.
    pub fn tombstones(&self) -> impl Iterator<Item = &TombstoneNode> {
        self.nodes.iter().filter_map(|n| match n {
            CodeNode::Tombstone(t) => Some(t),
            _ => None,
        })
    }

    /// Iterate over structural symbol nodes.
    pub fn symbol_nodes(&self) -> impl Iterator<Item = &SymbolNode> {
        self.nodes.iter().filter_map(|n| match n {
            CodeNode::Function(s)
            | CodeNode::Type(s)
            | CodeNode::Test(s)
            | CodeNode::Method(s)
            | CodeNode::Class(s)
            | CodeNode::Struct(s)
            | CodeNode::Interface(s)
            | CodeNode::Enum(s)
            | CodeNode::TypeAlias(s)
            | CodeNode::Module(s)
            | CodeNode::Lambda(s)
            | CodeNode::Variable(s)
            | CodeNode::Constant(s) => Some(s),
            _ => None,
        })
    }

    /// Index `repo_root` at its current working-tree state (HEAD +
    /// uncommitted edits -- this walks the filesystem, not a git tree
    /// object) against `previous_manifest`. Returns the updated
    /// manifest (for the caller to persist for the next run) and an
    /// [`IndexReport`] of what changed.
    ///
    /// `walk_files` is the list of repo-relative paths to consider,
    /// typically every non-ignored file under `repo_root` -- this
    /// function does not walk the filesystem tree itself (directory
    /// walking/`.gitignore` filtering is a caller/CLI concern per this
    /// lane's scope: X06.2 owns extraction and the manifest diff, not
    /// directory traversal policy).
    pub fn index_repository(
        &mut self,
        repo_root: &Path,
        walk_files: &[PathBuf],
        previous_manifest: &Manifest,
    ) -> Result<(Manifest, IndexReport), IndexError> {
        self.index_repository_at_mode(repo_root, walk_files, previous_manifest, IndexMode::Full)
    }

    /// X06.8: [`Self::index_repository`] extended with indexing-depth
    /// gating ([`IndexMode`]) and the optional persistence artifact
    /// (`.codebase-memory/graph.db.zst` + `artifact.json`, via
    /// [`crate::artifacts`]) plus bootstrap-on-index: if `previous_manifest`
    /// is empty (no local store for this project yet) AND a persistence
    /// artifact already exists at `repo_root`, this imports that artifact
    /// FIRST (regardless of `options.persistence` on this specific call --
    /// matching the baseline's "triggers purely on artifact exists but no
    /// local db yet") before running the fresh index pass, so a teammate
    /// bootstraps from a committed artifact instead of a full re-index.
    ///
    /// [`Self::index_repository`] itself is left completely unchanged
    /// (still the plain 3-argument call every existing call site in this
    /// crate uses) -- this is an additive entry point, not a signature
    /// change.
    pub fn index_repository_with_options(
        &mut self,
        repo_root: &Path,
        walk_files: &[PathBuf],
        previous_manifest: &Manifest,
        options: IndexOptions<'_>,
    ) -> Result<(Manifest, IndexReport), IndexWithOptionsError> {
        if options.persistence {
            if options.project_name.is_none() {
                return Err(IndexWithOptionsError::MissingProjectName);
            }
            if options.indexed_at.is_none() {
                return Err(IndexWithOptionsError::MissingIndexedAt);
            }
        }

        // Bootstrap-on-index: no local manifest entries yet, but an
        // artifact already exists on disk -- import it before indexing.
        if previous_manifest.entries.is_empty()
            && crate::artifacts::artifact_exists(repo_root).is_present()
        {
            let (snapshot, _meta) = crate::artifacts::import_graph_artifact(repo_root)?;
            self.restore_from_snapshot(&snapshot);
        }

        let (manifest, report) =
            self.index_repository_at_mode(repo_root, walk_files, previous_manifest, options.mode)?;

        if options.persistence {
            let project = options
                .project_name
                .ok_or(IndexWithOptionsError::MissingProjectName)?;
            let indexed_at = options
                .indexed_at
                .ok_or(IndexWithOptionsError::MissingIndexedAt)?;
            let snapshot =
                crate::boundary::artifact_transport::GraphSnapshotDto::from_code_graph(self)?;
            let commit = GitMetadata::open(repo_root)
                .ok()
                .flatten()
                .and_then(|g| g.head_commit());
            crate::artifacts::export_graph_artifact(
                repo_root,
                &snapshot,
                project,
                commit.map(|commit| {
                    enforcer_domain::memory_types::GraphArtifactCommit::from(commit.as_str())
                }),
                indexed_at,
            )?;
        }

        Ok((manifest, report))
    }

    /// Append the current graph snapshot into `store` as graph events,
    /// one `NodeAdded` per node plus the derived projection edges needed
    /// by the Store-backed operational graph. This is additive-only: it
    /// records the graph as it exists now, but it does not attempt to
    /// delete or supersede an older persisted snapshot.
    pub fn append_store_projection_events(
        &self,
        store: &mut crate::store::Store,
        ts: &str,
    ) -> crate::error::Result<StoreProjectionPersistReport> {
        let mut node_events = 0_u64;
        for node in &self.nodes {
            store.append_graph_event(
                enforcer_domain::memory_types::GraphEventKind::NodeAdded {
                    node_id: node.id().into(),
                    node_kind: store_projection_node_kind(node).into(),
                },
                ts.to_owned(),
            )?;
            node_events += 1;
        }

        let mut edge_events = 0_u64;
        for (from, to, label) in self.store_projection_edges() {
            store.append_graph_event(
                enforcer_domain::memory_types::GraphEventKind::EdgeAdded {
                    from: from.into(),
                    to: to.into(),
                    label: label.into(),
                },
                ts.to_owned(),
            )?;
            edge_events += 1;
        }

        Ok(StoreProjectionPersistReport {
            node_events,
            edge_events,
        })
    }

    /// Repopulate this (assumed-empty, freshly constructed) graph from a
    /// previously exported [`crate::boundary::artifact_transport::GraphSnapshotDto`] -- the
    /// "reconstruct node/edge counts" half of bootstrap-on-index. Kept
    /// deliberately additive (append-only, like every other mutation in
    /// this graph): a caller that bootstraps into a non-empty graph gets
    /// the union, not a silent overwrite.
    fn restore_from_snapshot(
        &mut self,
        snapshot: &crate::boundary::artifact_transport::GraphSnapshotDto,
    ) {
        for file in &snapshot.files {
            let node = FileNode {
                id: file.id.as_str().to_owned(),
                rel_path: file.rel_path.as_str().to_owned(),
                language: LanguageTag::TextOnly,
                content_hash: file.content_hash.as_str().to_owned(),
                last_commit: file
                    .last_commit
                    .as_ref()
                    .map(|commit| commit.as_str().to_owned()),
                change_count: file.change_count,
                chunk_ids: file
                    .chunk_ids
                    .iter()
                    .map(|chunk| chunk.as_str().to_owned())
                    .collect(),
            };
            if file.text_only.is_text_only() {
                self.nodes.push(CodeNode::TextOnly(node));
            } else {
                self.nodes.push(CodeNode::File(node));
            }
        }
        for symbol in &snapshot.symbols {
            let node = SymbolNode {
                id: symbol.id.as_str().to_owned(),
                name: symbol.name.as_str().to_owned(),
                file_id: symbol.file_id.as_str().to_owned(),
                line: symbol.line,
                // A snapshot round-trip carries no source text to
                // recompute complexity metrics from -- they are
                // deliberately dropped here (`None`) rather than
                // guessed. A caller needing them after a restore should
                // re-run `index_repository` against the working tree,
                // which recomputes them fresh (see `insert_file_and_chunks`).
                metrics: None,
                transitive_metrics: None,
                source_body_fingerprint: symbol.source_body_fingerprint.as_ref().map(|fp| {
                    SourceBodyFingerprint {
                        source_hash: fp.source_hash.as_str().into(),
                        fp: fp
                            .fp
                            .as_ref()
                            .map(|fingerprint| fingerprint.as_str().into()),
                        k: fp.k.map(|value| usize::from(value).into()),
                        body_grams: fp
                            .body_grams
                            .iter()
                            .map(|gram| gram.as_str().into())
                            .collect(),
                    }
                }),
            };
            self.nodes.push(match symbol.kind {
                GraphSymbolKindSnapshot::Function => CodeNode::Function(node),
                GraphSymbolKindSnapshot::Type => CodeNode::Type(node),
                GraphSymbolKindSnapshot::Test => CodeNode::Test(node),
                GraphSymbolKindSnapshot::Method => CodeNode::Method(node),
                GraphSymbolKindSnapshot::Class => CodeNode::Class(node),
                GraphSymbolKindSnapshot::Struct => CodeNode::Struct(node),
                GraphSymbolKindSnapshot::Interface => CodeNode::Interface(node),
                GraphSymbolKindSnapshot::Enum => CodeNode::Enum(node),
                GraphSymbolKindSnapshot::TypeAlias => CodeNode::TypeAlias(node),
                GraphSymbolKindSnapshot::Module => CodeNode::Module(node),
                GraphSymbolKindSnapshot::Lambda => CodeNode::Lambda(node),
                GraphSymbolKindSnapshot::Variable => CodeNode::Variable(node),
                GraphSymbolKindSnapshot::Constant => CodeNode::Constant(node),
            });
        }
        for tombstone in &snapshot.tombstones {
            self.nodes.push(CodeNode::Tombstone(TombstoneNode {
                id: tombstone.id.as_str().to_owned(),
                rel_path: tombstone.rel_path.as_str().to_owned(),
                last_commit: tombstone
                    .last_commit
                    .as_ref()
                    .map(|commit| commit.as_str().to_owned()),
                change_count: tombstone.change_count,
                prior_chunk_ids: tombstone
                    .prior_chunk_ids
                    .iter()
                    .map(|chunk| chunk.as_str().to_owned())
                    .collect(),
            }));
        }
        for edge in &snapshot.imports {
            self.imports.push(ImportEdge {
                from_file_id: edge.from_file_id.as_str().to_owned(),
                module_path: edge.module_path.as_str().to_owned(),
                line: edge.line,
            });
        }
        for edge in &snapshot.calls {
            self.calls.push(CallEdge {
                from_file_id: edge.from_file_id.as_str().to_owned(),
                callee: edge.callee.as_str().to_owned(),
                line: edge.line,
                // A snapshot round-trip carries no argument source text
                // (same "dropped, not guessed" rationale as the symbol
                // metrics fields above) -- empty, not fabricated.
                arg_texts: Vec::new(),
                // Same "dropped, not guessed" rationale: a snapshot
                // carries no from_symbol/receiver info either.
                from_symbol: None,
                from_symbol_line: None,
                receiver_text: None,
                receiver_hint: None,
            });
        }
        for edge in &snapshot.routes {
            self.routes.push(RouteEdge {
                from_file_id: edge.from_file_id.as_str().to_owned(),
                method: edge.method.as_str().to_owned(),
                path: edge.path.as_str().to_owned(),
                line: edge.line,
            });
        }
    }

    fn index_repository_at_mode(
        &mut self,
        repo_root: &Path,
        walk_files: &[PathBuf],
        previous_manifest: &Manifest,
        mode: IndexMode,
    ) -> Result<(Manifest, IndexReport), IndexError> {
        let mut git = GitMetadata::open(repo_root).map_err(IndexError::Git)?;
        let mut new_manifest = Manifest::default();
        let mut report = IndexReport::default();
        let mut seen_paths = std::collections::HashSet::new();

        // Filesystem enumeration order is platform-dependent.  Normalize and
        // sort the input once so graph insertion and later duplicate-symbol
        // resolution do not vary between Windows, macOS, and Linux.
        let mut ordered_files: Vec<(String, &Path)> = walk_files
            .iter()
            .map(|path| Ok((normalize_rel_path(repo_root, path)?, path.as_path())))
            .collect::<Result<_, IndexError>>()?;
        ordered_files.sort_by(|(left, _), (right, _)| left.cmp(right));

        for (rel_path, path) in ordered_files {
            seen_paths.insert(rel_path.clone());

            let content = fs::read(path).map_err(|source| IndexError::ReadFile {
                path: rel_path.clone(),
                source,
            })?;
            let content_hash = hash_bytes(ComplexitySourceBytes::from(content.as_slice()));
            let content_hash_text = content_hash.as_str().to_owned();

            let previous = previous_manifest.entries.get(&rel_path);
            let unchanged_entry = previous.filter(|p| p.content_hash == content_hash_text);

            if let Some(entry) = unchanged_entry {
                let entry = entry.clone();
                report.unchanged.push(rel_path.clone());
                self.reinsert_unchanged(&rel_path, &entry);
                new_manifest.entries.insert(rel_path, entry);
                continue;
            }

            let is_new = previous.is_none();
            let history = if mode.computes_git_history() {
                git.as_mut()
                    .map(|g| {
                        let rel_path =
                            enforcer_domain::memory_types::MemoryGitRelativePath::from(&rel_path);
                        g.history_for(&rel_path)
                    })
                    .unwrap_or_default()
            } else {
                crate::git::PathHistory::default()
            };
            let text = String::from_utf8_lossy(&content).into_owned();
            let language = parsers::classify(&rel_path);
            let parsed = parsers::parse_file(language, &text, &rel_path);

            let (file_id, chunk_ids) = self.insert_file_and_chunks(NewFileParams {
                rel_path: &rel_path,
                language,
                content_hash: &content_hash_text,
                history: &history,
                parsed,
                text: Some(&text),
            });

            new_manifest.entries.insert(
                rel_path.clone(),
                ManifestEntry {
                    content_hash: content_hash_text,
                    last_commit: history.last_commit.map(Into::into),
                    change_count: history.change_count.get().into(),
                    chunk_ids,
                },
            );

            if is_new {
                report.added.push(rel_path);
            } else {
                report.changed.push(rel_path);
            }
            debug_assert!(!file_id.is_empty());
        }

        for (rel_path, entry) in &previous_manifest.entries {
            if !seen_paths.contains(rel_path) {
                self.nodes.push(CodeNode::Tombstone(TombstoneNode {
                    id: format!("tomb:{rel_path}"),
                    rel_path: rel_path.clone(),
                    last_commit: entry.last_commit.clone(),
                    change_count: entry.change_count,
                    prior_chunk_ids: entry.chunk_ids.clone(),
                }));
                report.deleted.push(rel_path.clone());
            }
        }

        self.propagate_complexity();
        self.resolved_calls = crate::resolution::resolve(self);

        Ok((new_manifest, report))
    }

    /// X06 core parity Tier B: run [`crate::complexity::propagate_transitive_loop_depth`]
    /// over every callable symbol in this graph and write the result
    /// back onto each [`SymbolNode::transitive_metrics`].
    ///
    /// # Call-graph resolution caveat
    ///
    /// [`CallEdge`] is file-scoped (`from_file_id` + callee text), not
    /// symbol-scoped -- this crate's extractors do not currently track
    /// *which function* a call expression sits inside, only which
    /// *file*. Building a precise per-function call graph would need
    /// that extra field threaded through every language extractor.
    /// Absent it, this pass resolves a call edge to a callee by
    /// matching the callee's last name segment against every callable
    /// symbol name in the graph (any file, not just the caller's own)
    /// and, from every file that both calls something AND defines a
    /// same-named symbol, treats every callable symbol in the calling
    /// file as a potential caller of every matching callee -- an
    /// over-approximation (a false-positive CALLS edge is possible when
    /// two unrelated functions in the same file happen to share a
    /// callee name with a same-file symbol) that is safe for this
    /// metric's stated purpose: `transitive_loop_depth` is documented
    /// as "a bottleneck *candidate* signal, not a proof" (see this
    /// module's `complexity.rs` sibling's doc comment and the baseline
    /// doc's own "upper-bound heuristic, not a proof" wording) --
    /// over-approximating candidates is the correct failure direction
    /// for a signal callers use to decide where to *look*, not a
    /// compiler-grade call graph.
    fn propagate_complexity(&mut self) {
        use enforcer_domain::memory_types::{ComplexityCallGraph, ComplexityCallGraphNode};

        let callable_ids: Vec<(String, usize)> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| callable_symbol(node).map(|s| (s.id.clone(), idx)))
            .collect();

        // file_id -> callee names as written in that file's CallEdges.
        let mut callees_by_file: HashMap<&str, Vec<&str>> = HashMap::new();
        for call in &self.calls {
            callees_by_file
                .entry(call.from_file_id.as_str())
                .or_default()
                .push(call.callee.as_str());
        }

        // callee last-name-segment -> ids of callable symbols with that name.
        let mut ids_by_name: HashMap<&str, Vec<&str>> = HashMap::new();
        for (id, idx) in &callable_ids {
            if let Some(s) = self.nodes.get(*idx).and_then(callable_symbol) {
                ids_by_name
                    .entry(s.name.as_str())
                    .or_default()
                    .push(id.as_str());
            }
        }

        let mut graph_nodes = Vec::with_capacity(callable_ids.len());
        for (id, idx) in &callable_ids {
            // `callable_ids` was built from `callable_symbol(..).is_some()`
            // above, so this is never `None` in practice -- but rather
            // than `.expect()` (no unwrap/expect/panic in this crate's
            // style), a resolution miss simply skips the node: it is
            // still safe (just conservatively drops one node's Tier B
            // metrics) if that invariant is ever violated by a future
            // edit.
            let Some(symbol) = self.nodes.get(*idx).and_then(callable_symbol) else {
                continue;
            };
            let loop_depth = symbol
                .metrics
                .map(|m| m.loop_depth)
                .unwrap_or(enforcer_domain::memory_types::ComplexityMeasure::ZERO);
            let self_recursive = symbol.metrics.map(|m| m.self_recursive).unwrap_or_default();

            let mut callees = Vec::new();
            if let Some(names) = callees_by_file.get(symbol.file_id.as_str()) {
                for callee_text in names {
                    let last = callee_text
                        .rsplit(['.', ':'])
                        .next()
                        .unwrap_or(callee_text)
                        .trim_end_matches(['(', ')']);
                    if let Some(matches) = ids_by_name.get(last) {
                        for candidate in matches {
                            callees.push((*candidate).to_string().into());
                        }
                    }
                }
            }

            graph_nodes.push(ComplexityCallGraphNode {
                id: id.clone().into(),
                loop_depth,
                self_recursive,
                callees,
            });
        }

        let results = crate::complexity::propagate_transitive_loop_depth(ComplexityCallGraph::new(
            &graph_nodes,
        ));

        for (id, idx) in &callable_ids {
            if let Some(metrics) = results.get(id) {
                if let Some(symbol) = self.nodes.get_mut(*idx).and_then(callable_symbol_mut) {
                    symbol.transitive_metrics = Some(*metrics);
                }
            }
        }
    }

    /// An unchanged file still needs a live [`FileNode`] in the fresh
    /// graph instance the caller is building this run (this module
    /// does not persist [`CodeGraph`] across runs itself -- see module
    /// docs on the storage seam) -- but with zero re-parsing: the node
    /// is reconstructed from the manifest entry alone, no file read
    /// beyond the hash comparison already done, no tree-sitter call.
    fn reinsert_unchanged(&mut self, rel_path: &str, entry: &ManifestEntry) {
        let language = parsers::classify(rel_path);
        let node = FileNode {
            id: file_id_for(rel_path),
            rel_path: rel_path.to_string(),
            language: language_tag(language),
            content_hash: entry.content_hash.clone(),
            last_commit: entry.last_commit.clone(),
            change_count: entry.change_count,
            chunk_ids: entry.chunk_ids.clone(),
        };
        if matches!(language, Language::TextOnly) {
            self.nodes.push(CodeNode::TextOnly(node));
        } else {
            self.nodes.push(CodeNode::File(node));
        }
    }

    fn insert_file_and_chunks(&mut self, params: NewFileParams<'_>) -> (String, Vec<String>) {
        let NewFileParams {
            rel_path,
            language,
            content_hash,
            history,
            parsed,
            text,
        } = params;
        let file_id = file_id_for(rel_path);
        let mut chunk_ids = Vec::new();

        if let Some(parsed) = parsed {
            // name -> sym id, for this file's own symbols only -- used
            // to resolve the new rich-vocabulary edges (INHERITS/
            // IMPLEMENTS/DECORATES/TYPE_REF/DEFINES all reference a
            // container/member/decorated symbol by name as written)
            // on a best-effort last-write-wins basis, matching every
            // other best-effort name resolution in this crate.
            let mut sym_id_by_name: HashMap<String, String> = HashMap::new();

            // X06 core parity: Tier A complexity metrics, computed once
            // per file for every callable symbol name/line pair, keyed
            // by `(name, line)` so each symbol below can look its own
            // metrics up without re-parsing per-symbol. `None` (empty
            // map) for languages [`ComplexityLanguage`]
            // has no mapping for -- those symbols simply keep
            // `metrics: None`, same as any resolution miss.
            let metrics_by_symbol = text
                .zip(complexity_language(language))
                .map(|(text, lang)| {
                    let names: Vec<ComplexitySymbolLocation> = parsed
                        .symbols
                        .iter()
                        .map(|s| ComplexitySymbolLocation::new(s.name.clone(), s.line))
                        .collect();
                    crate::complexity::metrics_for_symbols(lang, text, &names)
                })
                .unwrap_or_default();
            let fingerprints_by_symbol = text
                .map(|text| {
                    source_body_fingerprints_for_symbols(
                        ParserSourceText::from(text),
                        &parsed.symbols,
                    )
                })
                .unwrap_or_default();

            for symbol in &parsed.symbols {
                let sym_id = format!("sym:{rel_path}:{}:{}", symbol.line, symbol.name);
                let metrics = metrics_by_symbol
                    .get(&ComplexitySymbolLocation::new(
                        symbol.name.clone(),
                        symbol.line,
                    ))
                    .copied();
                let node = SymbolNode {
                    id: sym_id.clone(),
                    name: symbol.name.clone().into(),
                    file_id: file_id.clone(),
                    line: symbol.line,
                    metrics,
                    transitive_metrics: None,
                    source_body_fingerprint: fingerprints_by_symbol
                        .get(&(symbol.name.clone(), symbol.line))
                        .cloned(),
                };
                sym_id_by_name.insert(symbol.name.clone().into(), sym_id.clone());
                self.nodes.push(match symbol.kind {
                    parsers::SymbolKind::Function => CodeNode::Function(node),
                    parsers::SymbolKind::Type => CodeNode::Type(node),
                    parsers::SymbolKind::Test => CodeNode::Test(node),
                    parsers::SymbolKind::Method => CodeNode::Method(node),
                    parsers::SymbolKind::Class => CodeNode::Class(node),
                    parsers::SymbolKind::Struct => CodeNode::Struct(node),
                    parsers::SymbolKind::Interface => CodeNode::Interface(node),
                    parsers::SymbolKind::Enum => CodeNode::Enum(node),
                    parsers::SymbolKind::TypeAlias => CodeNode::TypeAlias(node),
                    parsers::SymbolKind::Module => CodeNode::Module(node),
                    parsers::SymbolKind::Lambda => CodeNode::Lambda(node),
                    parsers::SymbolKind::Variable => CodeNode::Variable(node),
                    parsers::SymbolKind::Constant => CodeNode::Constant(node),
                });
                chunk_ids.push(sym_id);
            }
            for import in &parsed.imports {
                self.imports.push(ImportEdge {
                    from_file_id: file_id.clone(),
                    module_path: import.module_path.clone().into(),
                    line: import.line,
                });
            }
            for call in &parsed.calls {
                self.calls.push(CallEdge {
                    from_file_id: file_id.clone(),
                    callee: call.callee.clone().into(),
                    line: call.line,
                    arg_texts: call.arg_texts.iter().cloned().map(Into::into).collect(),
                    from_symbol: call.from_symbol.clone().map(Into::into),
                    from_symbol_line: call.from_symbol_line,
                    receiver_text: call.receiver_text.clone().map(Into::into),
                    receiver_hint: call.receiver_hint,
                });
            }
            for route in &parsed.routes {
                self.routes.push(RouteEdge {
                    from_file_id: file_id.clone(),
                    method: route.method.clone().into(),
                    path: route.path.clone().into(),
                    line: route.line,
                });
            }
            for inherits in &parsed.inherits {
                if let Some(sub_id) = sym_id_by_name.get(inherits.sub_name.as_str()) {
                    self.inherits.push(InheritsEdge {
                        sub_id: sub_id.clone(),
                        super_name: inherits.super_name.clone().into(),
                        line: inherits.line,
                    });
                }
            }
            for implements in &parsed.implements {
                if let Some(type_id) = sym_id_by_name.get(implements.type_name.as_str()) {
                    self.implements.push(ImplementsEdge {
                        type_id: type_id.clone(),
                        trait_name: implements.trait_name.clone().into(),
                        line: implements.line,
                    });
                }
            }
            for decorates in &parsed.decorates {
                if let Some(target_id) = sym_id_by_name.get(decorates.target_name.as_str()) {
                    self.decorates.push(DecoratesEdge {
                        target_id: target_id.clone(),
                        decorator_name: decorates.decorator_name.clone().into(),
                        line: decorates.line,
                    });
                }
            }
            for type_ref in &parsed.type_refs {
                if let Some(from_id) = sym_id_by_name.get(type_ref.from_name.as_str()) {
                    self.type_refs.push(TypeRefEdge {
                        from_id: from_id.clone(),
                        type_name: type_ref.type_name.clone().into(),
                        line: type_ref.line,
                    });
                }
            }
            for defines in &parsed.defines {
                if let (Some(container_id), Some(member_id)) = (
                    sym_id_by_name.get(defines.container_name.as_str()),
                    sym_id_by_name.get(defines.member_name.as_str()),
                ) {
                    self.defines.push(DefinesEdge {
                        container_id: container_id.clone(),
                        member_id: member_id.clone(),
                        line: defines.line,
                    });
                }
            }
        }

        let file_node = FileNode {
            id: file_id.clone(),
            rel_path: rel_path.to_string(),
            language: language_tag(language),
            content_hash: content_hash.to_string(),
            last_commit: history.last_commit.as_ref().map(ToString::to_string),
            change_count: history.change_count.get().into(),
            chunk_ids: chunk_ids.clone(),
        };
        if matches!(language, Language::TextOnly) {
            self.nodes.push(CodeNode::TextOnly(file_node));
        } else {
            self.nodes.push(CodeNode::File(file_node));
        }

        (file_id, chunk_ids)
    }

    fn store_projection_edges(&self) -> Vec<(String, String, String)> {
        let mut edges = Vec::new();

        for symbol in self.symbol_nodes() {
            edges.push((
                symbol.file_id.clone(),
                symbol.id.clone(),
                "contains".to_owned(),
            ));
        }

        for import in &self.imports {
            edges.push((
                import.from_file_id.clone(),
                import.module_path.clone(),
                "imports".to_owned(),
            ));
        }

        for (call, resolved) in self.calls.iter().zip(self.resolved_calls.iter()) {
            let from = resolved
                .from_symbol_id
                .clone()
                .map(|id| id.to_string())
                .unwrap_or_else(|| call.from_file_id.clone());
            for target in &resolved.candidates {
                edges.push((from.clone(), target.to_string(), "calls".to_owned()));
            }
        }

        for route in &self.routes {
            edges.push((
                route.from_file_id.clone(),
                format!("route:{} {}", route.method, route.path),
                "routes".to_owned(),
            ));
        }

        for inherits in &self.inherits {
            edges.push((
                inherits.sub_id.clone(),
                inherits.super_name.clone(),
                "inherits".to_owned(),
            ));
        }

        for implements in &self.implements {
            edges.push((
                implements.type_id.clone(),
                implements.trait_name.clone(),
                "implements".to_owned(),
            ));
        }

        for decorates in &self.decorates {
            edges.push((
                decorates.target_id.clone(),
                decorates.decorator_name.clone(),
                "decorates".to_owned(),
            ));
        }

        for type_ref in &self.type_refs {
            edges.push((
                type_ref.from_id.clone(),
                type_ref.type_name.clone(),
                "type_refs".to_owned(),
            ));
        }

        for defines in &self.defines {
            edges.push((
                defines.container_id.clone(),
                defines.member_id.clone(),
                "defines".to_owned(),
            ));
        }

        edges
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Counts emitted while persisting a graph projection into the store.
pub struct StoreProjectionPersistReport {
    pub node_events: u64,
    pub edge_events: u64,
}

fn file_id_for(rel_path: &str) -> String {
    format!("file:{rel_path}")
}

fn store_projection_node_kind(node: &CodeNode) -> &'static str {
    match node {
        CodeNode::File(_) => "File",
        CodeNode::Function(_) => "Function",
        CodeNode::Type(_) => "Type",
        CodeNode::Test(_) => "Test",
        CodeNode::TextOnly(_) => "TextOnly",
        CodeNode::Tombstone(_) => "Tombstone",
        CodeNode::Method(_) => "Method",
        CodeNode::Class(_) => "Class",
        CodeNode::Struct(_) => "Struct",
        CodeNode::Interface(_) => "Interface",
        CodeNode::Enum(_) => "Enum",
        CodeNode::TypeAlias(_) => "TypeAlias",
        CodeNode::Module(_) => "Module",
        CodeNode::Lambda(_) => "Lambda",
        CodeNode::Variable(_) => "Variable",
        CodeNode::Constant(_) => "Constant",
    }
}

fn code_node_from_store_projection(node_id: &str, node_kind: &str) -> CodeNode {
    match normalize_store_projection_kind(node_kind).as_str() {
        "file" => CodeNode::File(file_node_from_projection_id(node_id, false)),
        "textonly" | "text_only" => CodeNode::TextOnly(file_node_from_projection_id(node_id, true)),
        "tombstone" => CodeNode::Tombstone(tombstone_from_projection_id(node_id)),
        "type" => CodeNode::Type(symbol_node_from_projection_id(node_id)),
        "test" => CodeNode::Test(symbol_node_from_projection_id(node_id)),
        "method" => CodeNode::Method(symbol_node_from_projection_id(node_id)),
        "class" => CodeNode::Class(symbol_node_from_projection_id(node_id)),
        "struct" => CodeNode::Struct(symbol_node_from_projection_id(node_id)),
        "interface" => CodeNode::Interface(symbol_node_from_projection_id(node_id)),
        "enum" => CodeNode::Enum(symbol_node_from_projection_id(node_id)),
        "typealias" | "type_alias" => CodeNode::TypeAlias(symbol_node_from_projection_id(node_id)),
        "module" => CodeNode::Module(symbol_node_from_projection_id(node_id)),
        "lambda" => CodeNode::Lambda(symbol_node_from_projection_id(node_id)),
        "variable" => CodeNode::Variable(symbol_node_from_projection_id(node_id)),
        "constant" => CodeNode::Constant(symbol_node_from_projection_id(node_id)),
        "function" => CodeNode::Function(symbol_node_from_projection_id(node_id)),
        _ if node_id.starts_with("sym:") => {
            CodeNode::Function(symbol_node_from_projection_id(node_id))
        }
        _ => CodeNode::File(file_node_from_projection_id(node_id, false)),
    }
}

fn normalize_store_projection_kind(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn file_node_from_projection_id(node_id: &str, _text_only: bool) -> FileNode {
    let rel_path = node_id.strip_prefix("file:").unwrap_or(node_id).to_owned();
    FileNode {
        id: node_id.to_owned(),
        rel_path,
        language: LanguageTag::TextOnly,
        content_hash: String::new(),
        last_commit: None,
        change_count: GraphChangeCount::ZERO,
        chunk_ids: Vec::new(),
    }
}

fn tombstone_from_projection_id(node_id: &str) -> TombstoneNode {
    let rel_path = node_id
        .strip_prefix("file:")
        .or_else(|| node_id.strip_prefix("tombstone:"))
        .unwrap_or(node_id)
        .to_owned();
    TombstoneNode {
        id: node_id.to_owned(),
        rel_path,
        last_commit: None,
        change_count: GraphChangeCount::ZERO,
        prior_chunk_ids: Vec::new(),
    }
}

fn symbol_node_from_projection_id(node_id: &str) -> SymbolNode {
    let (file_id, line, name) = symbol_parts_from_projection_id(node_id);
    SymbolNode {
        id: node_id.to_owned(),
        name,
        file_id,
        line: line.into(),
        metrics: None,
        transitive_metrics: None,
        source_body_fingerprint: None,
    }
}

fn symbol_parts_from_projection_id(node_id: &str) -> (String, usize, String) {
    let Some(rest) = node_id.strip_prefix("sym:") else {
        return (String::new(), 0, node_id.to_owned());
    };
    let mut parts = rest.rsplitn(3, ':');
    let name = parts.next().unwrap_or(rest).to_owned();
    let line = parts
        .next()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(0);
    let rel_path = parts.next().unwrap_or_default();
    let file_id = if rel_path.is_empty() {
        String::new()
    } else {
        file_id_for(rel_path)
    };
    (file_id, line, name)
}

fn node_name_from_projection_id(node_id: &str) -> String {
    if node_id.starts_with("sym:") {
        return symbol_parts_from_projection_id(node_id).2;
    }
    node_id
        .strip_prefix("file:")
        .or_else(|| node_id.strip_prefix("route:"))
        .unwrap_or(node_id)
        .to_owned()
}

fn line_from_projection_id(node_id: &str) -> GraphSourceLine {
    if node_id.starts_with("sym:") {
        return symbol_parts_from_projection_id(node_id).1.into();
    }
    GraphSourceLine::UNKNOWN
}

fn file_id_for_projection_edge(node_id: &str) -> String {
    if node_id.starts_with("file:") {
        return node_id.to_owned();
    }
    let file_id = symbol_parts_from_projection_id(node_id).0;
    if file_id.is_empty() {
        node_id.to_owned()
    } else {
        file_id
    }
}

fn module_path_from_projection_target(node_id: &str) -> String {
    node_id.strip_prefix("file:").unwrap_or(node_id).to_owned()
}

fn route_parts_from_projection_target(node_id: &str) -> (String, String) {
    let raw = node_id.strip_prefix("route:").unwrap_or(node_id);
    let mut parts = raw.splitn(2, ' ');
    let first = parts.next().unwrap_or_default();
    let Some(path) = parts.next() else {
        return ("GET".to_owned(), raw.to_owned());
    };
    (first.to_owned(), path.to_owned())
}

/// The [`SymbolNode`] inside a [`CodeNode`] if -- and only if -- that
/// variant is "callable" in the baseline's "Function/Method only"
/// sense (see `complexity.rs`'s module doc): [`CodeNode::Function`],
/// [`CodeNode::Method`], [`CodeNode::Test`] (a test is still a runnable
/// function), and [`CodeNode::Lambda`] (a named, assigned
/// closure -- callable the same way). Every other variant (types,
/// files, tombstones) returns `None`, which is this crate's signal
/// that complexity metrics simply do not apply.
fn callable_symbol(node: &CodeNode) -> Option<&SymbolNode> {
    match node {
        CodeNode::Function(s) | CodeNode::Method(s) | CodeNode::Test(s) | CodeNode::Lambda(s) => {
            Some(s)
        }
        _ => None,
    }
}

fn callable_symbol_mut(node: &mut CodeNode) -> Option<&mut SymbolNode> {
    match node {
        CodeNode::Function(s) | CodeNode::Method(s) | CodeNode::Test(s) | CodeNode::Lambda(s) => {
            Some(s)
        }
        _ => None,
    }
}

/// Map a [`Language`] to the canonical [`ComplexityLanguage`]
/// its Tier A metrics pass understands, or `None` for a language with
/// no structural extractor (config/text-only -- `parsed` is already
/// `None` for those, so this is belt-and-suspenders) or one not yet
/// wired into `complexity.rs` (wave-B languages: extend this match
/// alongside `ComplexityLanguage`'s own variants, not by adding a
/// second parallel mapping elsewhere).
fn complexity_language(language: Language) -> Option<ComplexityLanguage> {
    match language {
        Language::Rust => Some(ComplexityLanguage::Rust),
        Language::TypeScript | Language::JavaScript => {
            Some(ComplexityLanguage::TypeScriptOrJavaScript)
        }
        Language::Python => Some(ComplexityLanguage::Python),
        Language::Go => Some(ComplexityLanguage::Go),
        Language::Java => Some(ComplexityLanguage::Java),
        Language::C => Some(ComplexityLanguage::C),
        Language::Cpp => Some(ComplexityLanguage::Cpp),
        Language::CSharp => Some(ComplexityLanguage::CSharp),
        Language::Php => Some(ComplexityLanguage::Php),
        // Language-parity wave G2.1a: Kotlin/Swift/TSX are onboarded
        // for structural extraction (symbols/calls/imports/DEFINES/
        // INHERITS) but NOT yet wired into `complexity.rs`'s own
        // `ComplexityLanguage`/`NodeKindTable` (out of this wave's
        // explicit file-ownership scope) -- `None` here is this
        // function's own documented convention for exactly that case
        // ("one not yet wired into `complexity.rs`"), not a silent
        // gap: `parsed` (symbols/calls/imports/...) is populated in
        // full for all three, only the separate Tier-A complexity-
        // metrics pass is deferred.
        // Language-parity wave G2.1d: same "not yet wired into
        // `complexity.rs`" convention as the G2.1a trio above, for the
        // same reason (out of that wave's own explicit file-ownership
        // scope too).
        // Language-parity wave G2.1b: same "not yet wired into
        // `complexity.rs`" convention as G2.1a/G2.1d above, for the
        // same reason.
        // Language-parity wave G2.1c: same "not yet wired into
        // `complexity.rs`" convention as every sibling G2.1 batch above
        // -- kept consistent across the whole wave rather than wiring
        // Ruby/Zig alone (both COULD resolve through
        // `find_definition_node`'s existing plain-`name_field` path;
        // Objective-C's `method_definition`/`method_declaration`
        // genuinely could not without a third declarator-unwrapping
        // fallback shape added to that shared function, well beyond
        // this batch's own scope -- see this worker's own final report
        // for the full finding). A single future pass wiring
        // `ComplexityLanguage`/`NodeKindTable` for every G2.1 language
        // together (once ObjC's own path is decided) is more coherent
        // than each batch independently deciding its own subset's
        // complexity depth.
        // Language-parity wave G2.2f: same "not yet wired into
        // `complexity.rs`" convention as every sibling G2.1/G2.2 batch
        // above -- Bash/Lua both have real `branch_types` arrays (a
        // future pass COULD resolve their complexity depth through
        // `find_definition_node`'s existing plain-`name_field` path the
        // same way Ruby/Zig could), but Elixir's own defs are entirely
        // quirk-claimed `call`-shaped nodes with no `name_field`
        // `find_definition_node` could resolve at all without a
        // dedicated fallback shape, well beyond this batch's own scope --
        // kept consistent with every language above rather than wiring
        // Bash/Lua alone.
        Language::Kotlin
        | Language::Swift
        | Language::Tsx
        | Language::Solidity
        | Language::Gdscript
        | Language::Dart
        | Language::Scala
        | Language::Groovy
        | Language::Ruby
        | Language::Zig
        | Language::ObjectiveC
        | Language::Bash
        | Language::Lua
        | Language::Elixir
        // Courtesy addition by the G2.2f/Lua-Elixir-Bash worker while
        // compile-checking its own claim (Haskell/OCaml/Erlang are a
        // sibling G2.2 batch's own languages, not this worker's) --
        // same "not yet wired into `complexity.rs`" convention as every
        // language above.
        | Language::Haskell
        | Language::OCaml
        | Language::Erlang
        // Language-parity wave G2.2b: same "not yet wired into
        // `complexity.rs`" convention as every language above -- CUDA
        // reuses C++'s `LangSpec`/quirks in full (see `LangSpec::cuda`'s
        // own doc comment) but this crate's separate
        // `ComplexityLanguage`/`NodeKindTable` pass has no `Cuda` arm of
        // its own yet either, so it stays grouped with the rest of this
        // "structural extraction done, complexity deferred" cohort
        // rather than silently piggy-backing on `ComplexityLanguage::Cpp`
        // (which this function's own signature -- one `Language` maps to
        // AT MOST one `ComplexityLanguage` -- has no way to express
        // "same complexity rules as a DIFFERENT language" without a
        // dedicated arm this wave does not add). D/PowerShell have no
        // complexity wiring of their own for the identical reason.
        | Language::Cuda
        | Language::D
        | Language::PowerShell
        // Language-parity wave G2.2c: same "not yet wired into
        // `complexity.rs`" convention as every language above -- courtesy
        // addition while compile-checking this worker's own F#/Gleam/GLSL
        // claim (GLSL reuses C's own `LangSpec`/quirks in full, same
        // "one `Language` maps to at most one `ComplexityLanguage`" limit
        // this function's own doc comment already gives for CUDA's
        // identical C++-reuse case above -- it cannot silently piggy-back
        // on `ComplexityLanguage::C` without a dedicated arm this wave
        // does not add).
        | Language::Fsharp
        | Language::Gleam
        | Language::Glsl
        // Language-parity wave G2.2a: same "not yet wired into
        // `complexity.rs`" convention as every language above -- Ada/
        // Apex/Crystal are all onboarded for structural extraction
        // (symbols/calls/imports/DEFINES/INHERITS) but this wave's own
        // explicit file-ownership scope does not extend to
        // `complexity.rs`'s own `ComplexityLanguage`/`NodeKindTable`.
        | Language::Ada
        | Language::Apex
        | Language::Crystal
        // Language-parity wave G2.2h: same "not yet wired into
        // `complexity.rs`" convention as every language above -- R/Perl
        // both have real `branch_types` arrays a future pass COULD
        // resolve through `find_definition_node`'s existing plain-
        // `name_field` path (Perl's `subroutine_declaration_statement`
        // genuinely has one; R's `function_definition` does not, see
        // `LangSpec::r`'s own doc comment), but Clojure's own defs are
        // entirely quirk-claimed `list_lit`-shaped nodes with no
        // `name_field` `find_definition_node` could resolve at all
        // without a dedicated fallback shape, well beyond this wave's
        // own scope -- kept consistent with every language above rather
        // than wiring R/Perl alone.
        | Language::R
        | Language::Perl
        | Language::Clojure
        // Language-parity wave G2.2d: same "not yet wired into
        // `complexity.rs`" convention as every language above --
        // Julia/Odin both have real `branch_types` arrays (a future pass
        // COULD resolve their complexity depth), but Julia's own defs
        // are entirely quirk-claimed unfielded nodes (`function_definition`
        // has no `name_field` `find_definition_node` could resolve at
        // all without a dedicated fallback shape) and Pascal's `declProc`
        // is genuinely `find_definition_node`-resolvable but its
        // out-of-line `defProc` implementation is not, well beyond this
        // batch's own scope -- kept consistent with every language above
        // rather than wiring any one of the three alone.
        | Language::Julia
        | Language::Odin
        | Language::Pascal
        // Language-parity wave G2.2e: same "not yet wired into
        // `complexity.rs`" convention as every language above -- QML
        // reuses TypeScript's own `branch_types` array in full (a future
        // pass COULD resolve its complexity depth the same way plain TS
        // eventually would), but Squirrel's own defs are entirely
        // quirk-claimed unfielded nodes (`function_declaration` has no
        // `name_field` `find_definition_node` could resolve at all
        // without a dedicated fallback shape) and ReScript's `function`
        // needs its own parent-`let_binding` name climb
        // `find_definition_node` has no equivalent for either, well
        // beyond this wave's own scope -- kept consistent with every
        // language above rather than wiring QML alone.
        | Language::Qml
        | Language::Rescript
        | Language::Squirrel
        // Language-parity wave G2.3e: same "not yet wired into
        // `complexity.rs`" convention as every language above -- Sway/
        // Starlark/Templ have real `branch_types` arrays and mostly
        // ordinary `find_definition_node`-resolvable fields (a future pass
        // COULD resolve their complexity depth), but Typst/WGSL/Wolfram's
        // own defs are entirely quirk-claimed with no `name_field`
        // `find_definition_node` could resolve at all without a dedicated
        // fallback shape (Typst's `let`-nested-call-pattern name, WGSL's
        // unfielded call-callee descent, Wolfram's fully positional,
        // zero-field grammar), and Slang's `LangSpec` row itself declares
        // its `name_field`/`body_field` unused (fully reusing `cpp_quirk`,
        // whose own out-of-line-declarator resolution
        // `find_definition_node` has no equivalent for) -- kept consistent
        // with every language above rather than wiring the three simpler
        // ones alone.
        | Language::Sway
        | Language::Starlark
        | Language::Templ
        | Language::Typst
        | Language::Wgsl
        | Language::Wolfram
        | Language::Slang
        // Courtesy addition (not this wave's own language assignment --
        // added only to keep this match exhaustive after these `Language`
        // variants landed from sibling workers): deferred to `None` for
        // the same "not yet wired into `complexity.rs`" reason as every
        // language above, pending whoever owns Scss/Cmake/Makefile/
        // Fortran/Vimscript/Puppet/Elm to revisit if any warrants real
        // complexity depth.
        | Language::Scss
        | Language::Cmake
        | Language::Makefile
        | Language::Fortran
        | Language::Vimscript
        | Language::Puppet
        | Language::Elm
        // Language-parity wave G2.3c: same "not yet wired into
        // `complexity.rs`" convention as every language above -- deferred
        // per this wave's own explicit "complexity extraction may be
        // deferred (return None) this wave" allowance. Bicep has no
        // branch-shaped nodes at all in its own grammar (a purely
        // declarative IaC language, see `LangSpec::bicep`'s own doc
        // comment); BitBake/FunC keep `branch_types` empty matching the
        // C baseline's own choice for both; Cairo/CFScript/Move/Nickel/
        // Jsonnet each have real `branch_types` entries a future pass
        // COULD resolve, but every one of the eight also needs at least
        // one dedicated quirk-claimed, unfielded-name definition shape
        // (`function_definition`'s positional identifier for Cairo/
        // BitBake, `fun_expr`'s parent-`let_binding` climb for Nickel,
        // `bind`/`field`'s function-vs-value disambiguation for Jsonnet,
        // ...) `find_definition_node` has no equivalent fallback for,
        // well beyond this wave's own scope -- kept consistent with every
        // language above rather than wiring any subset alone.
        | Language::Bicep
        | Language::Bitbake
        | Language::Cairo
        | Language::Cfscript
        | Language::Func
        | Language::Move
        | Language::Nickel
        | Language::Jsonnet
        // Language-parity wave G2.3d: same "not yet wired into
        // `complexity.rs`" convention as every language above -- deferred
        // per this wave's own explicit "complexity extraction may be
        // deferred (return None) this wave" allowance. Just/Magma both
        // have real, `find_definition_node`-resolvable definition shapes
        // (Magma's `function_definition`/`procedure_definition`/
        // `intrinsic_definition` all have plain `"name"` fields; a future
        // pass COULD wire either), but Just's own `recipe` needs its
        // nested-`recipe_header` name climb, HLSL/ISPC both reuse C++/C's
        // own out-of-line-declarator name resolution (the same "no
        // `find_definition_node` equivalent" limit CUDA/GLSL/Slang's own
        // doc comments above already establish for that identical reuse
        // shape), PureScript's `function` needs its own `"rhs"`-not-
        // `"body"` field name plus multi-child `"name"` filtering, Hare's
        // `type_declaration`/Pony's every one of `method`/`constructor`/
        // `actor_definition`/etc are entirely positional/unfielded, and
        // NASM's own `label`/`preproc_def` have no branch-shaped body
        // concept at all to begin with -- kept consistent with every
        // language above rather than wiring Just/Magma alone.
        | Language::Just
        | Language::Hlsl
        | Language::Ispc
        | Language::Purescript
        | Language::Magma
        | Language::Hare
        | Language::Pony
        | Language::Nasm
        // Language-parity wave G2.3b: same "not yet wired into
        // `complexity.rs`" convention as every language above -- deferred
        // per this wave's own explicit "complexity extraction may be
        // deferred (return None) this wave" allowance. Every one of these
        // seven has real, `branch_types`-array-driven decision points a
        // future pass COULD wire (COBOL's `if_header`/`evaluate_header`/
        // `perform_statement_call_proc`, Lean's `if`/`match`/`do`, TLA+'s
        // `if_then_else`/`case`, Verilog/VHDL/SystemVerilog's own
        // `conditional_statement`/`case_statement`/`loop_statement`
        // families), but every one ALSO needs at least one dedicated
        // quirk-claimed, unfielded/multiply-nested definition-name shape
        // `find_definition_node` has no equivalent fallback for (COBOL's
        // `program_definition` -> `identification_division` ->
        // `program_name` two-level walk, Verilog's doubly-nested
        // `function_identifier` wrapper, VHDL's declaration-keyword-to-
        // field-name map, SystemVerilog's own real direct `[name]` field
        // being the one partial exception -- but consistency with its own
        // Verilog sibling matters more than wiring it alone) -- kept
        // consistent with every language above rather than wiring any
        // subset alone. Common Lisp's own `defun` likewise needs its own
        // two-level `defun_header`/`function_name` walk this file's
        // generic fallback cannot express either.
        | Language::Cobol
        | Language::Commonlisp
        | Language::Lean
        | Language::Tlaplus
        | Language::Verilog
        | Language::Vhdl
        | Language::Systemverilog
        // Courtesy additions (found by the G2.4a worker while
        // compile-checking its own claim -- Capnp/EmacsLisp are a
        // sibling G2.4c/G2.4e batch's own languages, not this worker's;
        // the `complexity_language`/`LanguageTag` matches are
        // exhaustive, so any language variant left unlisted here breaks
        // every concurrent worker's own `cargo check`, the same
        // "courtesy-fix a crate-wide blocker, document it, move on"
        // situation `Language::Qml`'s own `LanguageTag` doc comment
        // already documents for an earlier wave): same "not yet wired
        // into `complexity.rs`" convention as every language above.
        | Language::Capnp
        | Language::EmacsLisp
        // Language-parity wave G2.6 (found missing during G2.5
        // closeout): same "not yet wired into complexity.rs" convention
        // as every language above.
        | Language::Agda
        | Language::Form
        // Language-parity wave G2.4a: same "not yet wired into
        // `complexity.rs`" convention as every language above -- all six
        // have real `branch_types` arrays this wave already ported (see
        // `LangSpec::awk`/`fish`/`zsh`/`tcl`'s own doc comments), but
        // wiring a dedicated `ComplexityLanguage`/`NodeKindTable` arm for
        // any of them is out of this wave's own explicit scope (defs
        // extraction may be deferred to complexity per the workpack's own
        // "complexity extraction may be deferred (return `None`) this
        // wave" allowance) -- Scheme/Racket's own defs are additionally
        // entirely quirk-claimed unfielded `list` nodes, the identical
        // "no `name_field` `find_definition_node` could resolve without a
        // dedicated fallback shape" gap [`Language::Clojure`]'s own
        // arm above already documents.
        | Language::Awk
        | Language::Fish
        | Language::Zsh
        | Language::Tcl
        | Language::Scheme
        | Language::Racket
        // Smithy/Pine (orchestrator completion pass): same "not yet
        // wired into complexity.rs" convention as every language above.
        | Language::Smithy
        | Language::Pine
        // Language-parity wave G2.4d redo: MATLAB/Luau/Teal/Fennel/
        // Meson/Kconfig. Same "not yet wired into complexity.rs"
        // convention as every language above -- every one has real
        // `branch_types` this wave already ported (see each
        // `LangSpec::matlab`/`luau`/`teal`/`fennel`/`meson`/`kconfig`'s
        // own doc comment), but wiring a dedicated
        // `ComplexityLanguage`/`NodeKindTable` arm for any of them is
        // out of this wave's own explicit scope (defs extraction may be
        // deferred to complexity per the workpack's own "complexity
        // extraction may be deferred (return `None`) this wave"
        // allowance).
        | Language::Matlab
        | Language::Luau
        | Language::Teal
        | Language::Fennel
        | Language::Meson
        | Language::Kconfig
        // Language-parity wave G2.4b-redo: HCL/Nix/SQL/Protobuf/Prisma/
        // Pkl. Same "not yet wired into complexity.rs" convention as
        // every language above -- Nix/SQL both have real `branch_types`
        // arrays this wave already ported (`if_expression`/`case`), but
        // wiring a dedicated `ComplexityLanguage`/`NodeKindTable` arm
        // for either is out of this wave's own explicit scope (defs
        // extraction may be deferred to complexity per the workpack's
        // own "complexity extraction may be deferred (return `None`)
        // this wave" allowance).
        | Language::Hcl
        | Language::Nix
        | Language::Sql
        | Language::Protobuf
        | Language::Prisma
        | Language::Pkl
        // Language-parity wave G2.4c-redo: Thrift/WIT/LLVM IR/TableGen.
        // Same "not yet wired into complexity.rs" convention as every
        // language above -- LLVM IR has a real `branch_types` array
        // this wave already ported (`instruction_br`/
        // `instruction_switch`), but wiring a dedicated
        // `ComplexityLanguage`/`NodeKindTable` arm for it is out of this
        // wave's own explicit scope (same "may be deferred" allowance
        // as every prior wave above).
        | Language::Thrift
        | Language::Wit
        | Language::LlvmIr
        | Language::TableGen
        // Language-parity wave G2.4e redo: CFML/Go Template/DeviceTree/
        // Smali. Same "not yet wired into complexity.rs" convention as
        // every language above.
        | Language::Cfml
        | Language::Gotemplate
        | Language::Devicetree
        | Language::Smali
        // Language-parity wave G2.5c: JSON5/KDL/Linker Script/Liquid/
        // Markdown/Mermaid/PO/Properties/Regex. Every one of these is a
        // Tier-0 nominal language with `branch_types: &[]` in its own
        // `LangSpec` row (no decision-point vocabulary at all) -- there
        // is nothing for a `ComplexityLanguage`/`NodeKindTable` arm to
        // compute here even in principle, unlike the "deferred, could be
        // wired later" languages above.
        | Language::Json5
        | Language::Kdl
        | Language::LinkerScript
        | Language::Liquid
        | Language::Markdown
        | Language::Mermaid
        | Language::Po
        | Language::Properties
        | Language::Regex
        // Language-parity wave G2.5b: gitignore/GN/Go Mod/GraphQL/HTML/
        // Hyprlang/INI/Janet/Jinja2/JSDoc/JSON, the final Tier-0
        // language batch. Same "no decision-point vocabulary at all"
        // reasoning as the G2.5c batch above -- every one of these
        // `LangSpec` rows has `branch_types: &[]` except GN's own
        // (`if_statement`/`foreach_statement`), which is documentary
        // metadata only (the generic engine does not yet consume
        // `branch_types` for anything functional, so there is still
        // nothing for a `ComplexityLanguage`/`NodeKindTable` arm to
        // compute here even for GN).
        | Language::Gitignore
        | Language::Gn
        | Language::GoMod
        | Language::Graphql
        | Language::Html
        | Language::Hyprlang
        | Language::Ini
        | Language::Janet
        | Language::Jinja2
        | Language::Jsdoc
        | Language::Json
        // Language-parity wave G2.5a: Assembly/Astro/Beancount/Bibtex/
        // Blade/Css/Csv/Diff/Dockerfile/Dotenv/Gitattributes, the
        // Tier-0 batch preceding rich-tier wave G3. Same "no
        // decision-point vocabulary at all" reasoning as the G2.5b/c
        // batches above -- every one of these `LangSpec` rows has
        // `branch_types: &[]`, so there is nothing for a
        // `ComplexityLanguage`/`NodeKindTable` arm to compute here even
        // in principle.
        | Language::Assembly
        | Language::Astro
        | Language::Beancount
        | Language::Bibtex
        | Language::Blade
        | Language::Css
        | Language::Csv
        | Language::Diff
        | Language::Dockerfile
        | Language::Dotenv
        | Language::Gitattributes
        // Language-parity wave G2.5d/orchestrator completion pass:
        // Requirements/RON/RST/SOQL/SOSL/SSHConfig/Svelte/TOML/Vue/
        // XML/YAML, closing out Tier-0. Same "no decision-point
        // vocabulary at all" reasoning as every batch above.
        | Language::Requirements
        | Language::Ron
        | Language::Rst
        | Language::Soql
        | Language::Sosl
        | Language::Sshconfig
        | Language::Svelte
        | Language::Toml
        | Language::Vue
        | Language::Xml
        | Language::Yaml
        | Language::ConfigToml
        | Language::ConfigJson
        | Language::ConfigYaml
        | Language::TextOnly => None,
    }
}

fn normalize_rel_path(repo_root: &Path, path: &Path) -> Result<String, IndexError> {
    let rel = path
        .strip_prefix(repo_root)
        .map_err(|source| IndexError::PathNotUnderRoot {
            path: path.to_path_buf(),
            root: repo_root.to_path_buf(),
            source,
        })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

/// Errors from [`CodeGraph::index_repository`].
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("failed to open git metadata: {0}")]
    Git(git2::Error),
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("path {path} is not under repo root {root}")]
    PathNotUnderRoot {
        path: PathBuf,
        root: PathBuf,
        #[source]
        source: std::path::StripPrefixError,
    },
}
