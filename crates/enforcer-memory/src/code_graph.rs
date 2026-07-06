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
use crate::parsers::{self, Language, ParsedFile};
use sha2::{Digest, Sha256};
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
    pub change_count: usize,
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
    pub line: usize,
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
    pub transitive_metrics: Option<crate::complexity::TransitiveMetrics>,
}

/// A file that was indexed in a previous run and is no longer present.
/// Never removed from the graph outright -- see module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TombstoneNode {
    pub id: String,
    pub rel_path: String,
    pub last_commit: Option<String>,
    pub change_count: usize,
    /// Chunk ids the file had at the time it was deleted (its last
    /// [`FileNode::chunk_ids`] before deletion).
    pub prior_chunk_ids: Vec<String>,
}

/// The subset of [`Language`] worth keeping on the node itself (so
/// callers querying the graph don't need to re-derive it from the
/// path's extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageTag {
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
    TextOnly,
}

impl From<Language> for LanguageTag {
    fn from(language: Language) -> Self {
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
            Language::ConfigToml => LanguageTag::ConfigToml,
            Language::ConfigJson => LanguageTag::ConfigJson,
            Language::ConfigYaml => LanguageTag::ConfigYaml,
            Language::TextOnly => LanguageTag::TextOnly,
        }
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
    pub line: usize,
}

/// A call edge: `from_file_id` calls `callee` (as written) at `line`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallEdge {
    pub from_file_id: String,
    pub callee: String,
    pub line: usize,
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
    pub from_symbol_line: Option<usize>,
    /// The method-call receiver's own text (`x` in `x.foo()`), carried
    /// through from [`crate::parsers::CallRef::receiver_text`].
    pub receiver_text: Option<String>,
    /// Cheap syntactic classification of [`Self::receiver_text`],
    /// carried through from [`crate::parsers::CallRef::receiver_hint`].
    pub receiver_hint: Option<crate::parsers::ReceiverHint>,
}

/// A route/endpoint declared in `from_file_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEdge {
    pub from_file_id: String,
    pub method: String,
    pub path: String,
    pub line: usize,
}

/// X06 rich vocabulary (additive): an INHERITS edge -- `sub_id` extends
/// or is a subtrait of a symbol named `super_name` (unresolved to a
/// node id; best-effort name resolution is [`crate::analysis`]'s job,
/// matching every other edge kind here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritsEdge {
    pub sub_id: String,
    pub super_name: String,
    pub line: usize,
}

/// An IMPLEMENTS edge -- `type_id` implements a trait/interface named
/// `trait_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementsEdge {
    pub type_id: String,
    pub trait_name: String,
    pub line: usize,
}

/// A DECORATES edge -- `target_id` is decorated by `decorator_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecoratesEdge {
    pub target_id: String,
    pub decorator_name: String,
    pub line: usize,
}

/// A TYPE_REF edge -- `from_id`'s signature references a type named
/// `type_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRefEdge {
    pub from_id: String,
    pub type_name: String,
    pub line: usize,
}

/// A DEFINES edge -- `container_id` defines member symbol `member_id`
/// (both already-resolved node ids, since both ends are symbols
/// extracted from the same file in the same pass).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinesEdge {
    pub container_id: String,
    pub member_id: String,
    pub line: usize,
}

/// One file's entry in the manifest carried between index runs: the
/// content hash used for the unchanged-file skip, plus enough of the
/// file-history summary to reconstruct a [`TombstoneNode`] if the file
/// is later deleted without needing to re-walk git history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub content_hash: String,
    pub last_commit: Option<String>,
    pub change_count: usize,
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
/// x06-baseline-tool-schemas.md` §9.2). This crate's [`CodeGraph`] has no
/// SIMILAR_TO/SEMANTICALLY_RELATED edges or macro-extraction pass to gate
/// (those are out of this slice's scope -- see `src/lib.rs` module
/// docs), so the one gate [`IndexMode`] currently controls is git-history
/// computation, exactly matching the baseline's one universally-confirmed
/// mode distinction: "`full` and `moderate` both compute git history;
/// only `fast` omits it." Adding a future gated pass (e.g. a real
/// similarity edge) should extend this enum's match arms, not add a
/// parallel boolean flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndexMode {
    /// Everything this indexer currently supports, including git
    /// history. The default -- back-compat convenience constructors
    /// ([`CodeGraph::index_repository`]) always run at this mode.
    #[default]
    Full,
    /// Same file-history computation as [`IndexMode::Full`] in this
    /// slice (the baseline's moderate/full split is driven entirely by
    /// the macro-extraction/similarity passes this crate does not
    /// implement); kept as a distinct variant so a future gated pass has
    /// somewhere to plug in without another signature change.
    Moderate,
    /// Skips git-history computation (`last_commit`/`change_count` stay
    /// at their defaults: `None`/`0`) for the fastest indexing pass.
    Fast,
}

impl IndexMode {
    /// Whether this mode computes per-file git history
    /// (`last_commit`/`change_count`). Matches the baseline's
    /// "`full`/`moderate` compute git history; only `fast` omits it."
    fn computes_git_history(self) -> bool {
        !matches!(self, IndexMode::Fast)
    }
}

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
    pub fn new() -> Self {
        Self::default()
    }

    /// Test-only seam: append one [`RouteEdge`] directly, bypassing the
    /// tree-sitter extraction pipeline entirely. Exists because
    /// [`crate::cross_repo`]'s matching-algorithm tests need small,
    /// hand-built graphs (a route on one side, a call site on the
    /// other) without depending on every language extractor already
    /// populating [`CallEdge::arg_texts`] end-to-end (as of this writing
    /// none do -- that plumbing is separate, wider-scoped follow-up
    /// work, not this constructor's job). `#[cfg(test)]` only: never
    /// compiled into a non-test build, so it cannot be misused as a
    /// production graph-mutation API.
    #[cfg(test)]
    pub(crate) fn push_route_for_test(&mut self, route: RouteEdge) {
        self.routes.push(route);
    }

    /// Test-only seam, symmetric with [`Self::push_route_for_test`]: see
    /// its doc comment.
    #[cfg(test)]
    pub(crate) fn push_call_for_test(&mut self, call: CallEdge) {
        self.calls.push(call);
    }

    pub fn nodes(&self) -> &[CodeNode] {
        &self.nodes
    }

    pub fn imports(&self) -> &[ImportEdge] {
        &self.imports
    }

    pub fn calls(&self) -> &[CallEdge] {
        &self.calls
    }

    pub fn routes(&self) -> &[RouteEdge] {
        &self.routes
    }

    pub fn inherits(&self) -> &[InheritsEdge] {
        &self.inherits
    }

    pub fn implements(&self) -> &[ImplementsEdge] {
        &self.implements
    }

    pub fn decorates(&self) -> &[DecoratesEdge] {
        &self.decorates
    }

    pub fn type_refs(&self) -> &[TypeRefEdge] {
        &self.type_refs
    }

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

    pub fn file_nodes(&self) -> impl Iterator<Item = &FileNode> {
        self.nodes.iter().filter_map(|n| match n {
            CodeNode::File(f) | CodeNode::TextOnly(f) => Some(f),
            _ => None,
        })
    }

    pub fn tombstones(&self) -> impl Iterator<Item = &TombstoneNode> {
        self.nodes.iter().filter_map(|n| match n {
            CodeNode::Tombstone(t) => Some(t),
            _ => None,
        })
    }

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
        if previous_manifest.entries.is_empty() && crate::artifacts::artifact_exists(repo_root) {
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
            let snapshot = crate::artifacts::GraphSnapshot::from_code_graph(self);
            let commit = GitMetadata::open(repo_root)
                .ok()
                .flatten()
                .and_then(|g| g.head_commit());
            crate::artifacts::export_graph_artifact(
                repo_root, &snapshot, project, commit, indexed_at,
            )?;
        }

        Ok((manifest, report))
    }

    /// Repopulate this (assumed-empty, freshly constructed) graph from a
    /// previously exported [`crate::artifacts::GraphSnapshot`] -- the
    /// "reconstruct node/edge counts" half of bootstrap-on-index. Kept
    /// deliberately additive (append-only, like every other mutation in
    /// this graph): a caller that bootstraps into a non-empty graph gets
    /// the union, not a silent overwrite.
    fn restore_from_snapshot(&mut self, snapshot: &crate::artifacts::GraphSnapshot) {
        for file in &snapshot.files {
            let node = FileNode {
                id: file.id.clone(),
                rel_path: file.rel_path.clone(),
                language: LanguageTag::TextOnly,
                content_hash: file.content_hash.clone(),
                last_commit: file.last_commit.clone(),
                change_count: file.change_count,
                chunk_ids: file.chunk_ids.clone(),
            };
            if file.text_only {
                self.nodes.push(CodeNode::TextOnly(node));
            } else {
                self.nodes.push(CodeNode::File(node));
            }
        }
        for symbol in &snapshot.symbols {
            let node = SymbolNode {
                id: symbol.id.clone(),
                name: symbol.name.clone(),
                file_id: symbol.file_id.clone(),
                line: symbol.line,
                // A snapshot round-trip carries no source text to
                // recompute complexity metrics from -- they are
                // deliberately dropped here (`None`) rather than
                // guessed. A caller needing them after a restore should
                // re-run `index_repository` against the working tree,
                // which recomputes them fresh (see `insert_file_and_chunks`).
                metrics: None,
                transitive_metrics: None,
            };
            self.nodes.push(match symbol.kind {
                crate::artifacts::GraphSymbolKindSnapshot::Function => CodeNode::Function(node),
                crate::artifacts::GraphSymbolKindSnapshot::Type => CodeNode::Type(node),
                crate::artifacts::GraphSymbolKindSnapshot::Test => CodeNode::Test(node),
            });
        }
        for tombstone in &snapshot.tombstones {
            self.nodes.push(CodeNode::Tombstone(TombstoneNode {
                id: tombstone.id.clone(),
                rel_path: tombstone.rel_path.clone(),
                last_commit: tombstone.last_commit.clone(),
                change_count: tombstone.change_count,
                prior_chunk_ids: tombstone.prior_chunk_ids.clone(),
            }));
        }
        for edge in &snapshot.imports {
            self.imports.push(ImportEdge {
                from_file_id: edge.from_file_id.clone(),
                module_path: edge.module_path.clone(),
                line: edge.line,
            });
        }
        for edge in &snapshot.calls {
            self.calls.push(CallEdge {
                from_file_id: edge.from_file_id.clone(),
                callee: edge.callee.clone(),
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
                from_file_id: edge.from_file_id.clone(),
                method: edge.method.clone(),
                path: edge.path.clone(),
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

        for path in walk_files {
            let rel_path = normalize_rel_path(repo_root, path)?;
            seen_paths.insert(rel_path.clone());

            let content = fs::read(path).map_err(|source| IndexError::ReadFile {
                path: rel_path.clone(),
                source,
            })?;
            let content_hash = hash_bytes(&content);

            let previous = previous_manifest.entries.get(&rel_path);
            let unchanged_entry = previous.filter(|p| p.content_hash == content_hash);

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
                    .map(|g| g.history_for(&rel_path))
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
                content_hash: &content_hash,
                history: &history,
                parsed,
                text: Some(&text),
            });

            new_manifest.entries.insert(
                rel_path.clone(),
                ManifestEntry {
                    content_hash,
                    last_commit: history.last_commit,
                    change_count: history.change_count,
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
        use crate::complexity::CallGraphNode;

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
            if let Some(s) = callable_symbol(&self.nodes[*idx]) {
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
            let Some(symbol) = callable_symbol(&self.nodes[*idx]) else {
                continue;
            };
            let loop_depth = symbol.metrics.map(|m| m.loop_depth).unwrap_or(0);
            let self_recursive = symbol.metrics.map(|m| m.self_recursive).unwrap_or(false);

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
                            callees.push((*candidate).to_string());
                        }
                    }
                }
            }

            graph_nodes.push(CallGraphNode {
                id: id.clone(),
                loop_depth,
                self_recursive,
                callees,
            });
        }

        let results = crate::complexity::propagate_transitive_loop_depth(&graph_nodes);

        for (id, idx) in &callable_ids {
            if let Some(metrics) = results.get(id) {
                if let Some(symbol) = callable_symbol_mut(&mut self.nodes[*idx]) {
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
            language: LanguageTag::from(language),
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
            // map) for languages [`crate::complexity::ComplexityLanguage`]
            // has no mapping for -- those symbols simply keep
            // `metrics: None`, same as any resolution miss.
            let metrics_by_symbol = text
                .zip(complexity_language(language))
                .map(|(text, lang)| {
                    let names: Vec<(String, usize)> = parsed
                        .symbols
                        .iter()
                        .map(|s| (s.name.clone(), s.line))
                        .collect();
                    crate::complexity::metrics_for_symbols(lang, text, &names)
                })
                .unwrap_or_default();

            for symbol in &parsed.symbols {
                let sym_id = format!("sym:{rel_path}:{}:{}", symbol.line, symbol.name);
                let metrics = metrics_by_symbol
                    .get(&(symbol.name.clone(), symbol.line))
                    .copied();
                let node = SymbolNode {
                    id: sym_id.clone(),
                    name: symbol.name.clone(),
                    file_id: file_id.clone(),
                    line: symbol.line,
                    metrics,
                    transitive_metrics: None,
                };
                sym_id_by_name.insert(symbol.name.clone(), sym_id.clone());
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
                    module_path: import.module_path.clone(),
                    line: import.line,
                });
            }
            for call in &parsed.calls {
                self.calls.push(CallEdge {
                    from_file_id: file_id.clone(),
                    callee: call.callee.clone(),
                    line: call.line,
                    arg_texts: call.arg_texts.clone(),
                    from_symbol: call.from_symbol.clone(),
                    from_symbol_line: call.from_symbol_line,
                    receiver_text: call.receiver_text.clone(),
                    receiver_hint: call.receiver_hint,
                });
            }
            for route in &parsed.routes {
                self.routes.push(RouteEdge {
                    from_file_id: file_id.clone(),
                    method: route.method.clone(),
                    path: route.path.clone(),
                    line: route.line,
                });
            }
            for inherits in &parsed.inherits {
                if let Some(sub_id) = sym_id_by_name.get(&inherits.sub_name) {
                    self.inherits.push(InheritsEdge {
                        sub_id: sub_id.clone(),
                        super_name: inherits.super_name.clone(),
                        line: inherits.line,
                    });
                }
            }
            for implements in &parsed.implements {
                if let Some(type_id) = sym_id_by_name.get(&implements.type_name) {
                    self.implements.push(ImplementsEdge {
                        type_id: type_id.clone(),
                        trait_name: implements.trait_name.clone(),
                        line: implements.line,
                    });
                }
            }
            for decorates in &parsed.decorates {
                if let Some(target_id) = sym_id_by_name.get(&decorates.target_name) {
                    self.decorates.push(DecoratesEdge {
                        target_id: target_id.clone(),
                        decorator_name: decorates.decorator_name.clone(),
                        line: decorates.line,
                    });
                }
            }
            for type_ref in &parsed.type_refs {
                if let Some(from_id) = sym_id_by_name.get(&type_ref.from_name) {
                    self.type_refs.push(TypeRefEdge {
                        from_id: from_id.clone(),
                        type_name: type_ref.type_name.clone(),
                        line: type_ref.line,
                    });
                }
            }
            for defines in &parsed.defines {
                if let (Some(container_id), Some(member_id)) = (
                    sym_id_by_name.get(&defines.container_name),
                    sym_id_by_name.get(&defines.member_name),
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
            language: LanguageTag::from(language),
            content_hash: content_hash.to_string(),
            last_commit: history.last_commit.clone(),
            change_count: history.change_count,
            chunk_ids: chunk_ids.clone(),
        };
        if matches!(language, Language::TextOnly) {
            self.nodes.push(CodeNode::TextOnly(file_node));
        } else {
            self.nodes.push(CodeNode::File(file_node));
        }

        (file_id, chunk_ids)
    }
}

fn file_id_for(rel_path: &str) -> String {
    format!("file:{rel_path}")
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

/// Map a [`Language`] to the [`crate::complexity::ComplexityLanguage`]
/// its Tier A metrics pass understands, or `None` for a language with
/// no structural extractor (config/text-only -- `parsed` is already
/// `None` for those, so this is belt-and-suspenders) or one not yet
/// wired into `complexity.rs` (wave-B languages: extend this match
/// alongside `ComplexityLanguage`'s own variants, not by adding a
/// second parallel mapping elsewhere).
fn complexity_language(language: Language) -> Option<crate::complexity::ComplexityLanguage> {
    match language {
        Language::Rust => Some(crate::complexity::ComplexityLanguage::Rust),
        Language::TypeScript | Language::JavaScript => {
            Some(crate::complexity::ComplexityLanguage::TypeScriptOrJavaScript)
        }
        Language::Python => Some(crate::complexity::ComplexityLanguage::Python),
        Language::Go => Some(crate::complexity::ComplexityLanguage::Go),
        Language::Java => Some(crate::complexity::ComplexityLanguage::Java),
        Language::C => Some(crate::complexity::ComplexityLanguage::C),
        Language::Cpp => Some(crate::complexity::ComplexityLanguage::Cpp),
        Language::CSharp => Some(crate::complexity::ComplexityLanguage::CSharp),
        Language::Php => Some(crate::complexity::ComplexityLanguage::Php),
        Language::ConfigToml | Language::ConfigJson | Language::ConfigYaml | Language::TextOnly => {
            None
        }
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fs;
    use std::process::Command;

    type TestResult = Result<(), Box<dyn Error>>;

    fn init_git_repo(dir: &Path) -> TestResult {
        run_git(dir, &["init", "--quiet"])?;
        run_git(dir, &["config", "user.email", "test@example.com"])?;
        run_git(dir, &["config", "user.name", "Test"])?;
        Ok(())
    }

    fn commit_all(dir: &Path, message: &str) -> TestResult {
        run_git(dir, &["add", "-A"])?;
        run_git(dir, &["commit", "--quiet", "-m", message])?;
        Ok(())
    }

    fn run_git(dir: &Path, args: &[&str]) -> TestResult {
        let status = Command::new("git").args(args).current_dir(dir).status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    }

    #[test]
    fn unchanged_file_is_skipped_on_second_run() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let (manifest_v1, report_v1) = graph.index_repository(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
        )?;
        assert_eq!(report_v1.added, vec!["a.rs".to_string()]);

        let mut graph2 = CodeGraph::new();
        let (_manifest_v2, report_v2) =
            graph2.index_repository(dir.path(), &[file_path], &manifest_v1)?;
        assert_eq!(report_v2.unchanged, vec!["a.rs".to_string()]);
        assert!(report_v2.changed.is_empty());
        assert!(report_v2.added.is_empty());
        Ok(())
    }

    #[test]
    fn changed_file_is_reindexed() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let (manifest_v1, _) = graph.index_repository(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
        )?;

        fs::write(&file_path, "fn a() {} fn b() {}")?;
        commit_all(dir.path(), "second")?;

        let mut graph2 = CodeGraph::new();
        let (_manifest_v2, report_v2) =
            graph2.index_repository(dir.path(), &[file_path], &manifest_v1)?;
        assert_eq!(report_v2.changed, vec!["a.rs".to_string()]);
        let names: Vec<&str> = graph2.symbol_nodes().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        Ok(())
    }

    #[test]
    fn deleted_file_gets_tombstone_not_silently_dropped() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let (manifest_v1, _) = graph.index_repository(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
        )?;

        fs::remove_file(&file_path)?;
        commit_all(dir.path(), "delete a.rs")?;

        let mut graph2 = CodeGraph::new();
        let (manifest_v2, report_v2) = graph2.index_repository(dir.path(), &[], &manifest_v1)?;

        assert_eq!(report_v2.deleted, vec!["a.rs".to_string()]);
        let tombstones: Vec<&TombstoneNode> = graph2.tombstones().collect();
        assert_eq!(tombstones.len(), 1);
        assert_eq!(tombstones[0].rel_path, "a.rs");
        assert!(!tombstones[0].prior_chunk_ids.is_empty());
        assert!(!manifest_v2.entries.contains_key("a.rs"));
        Ok(())
    }

    #[test]
    fn symbol_extraction_produces_function_type_test_nodes() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(
            &file_path,
            "struct Foo;\nfn helper() {}\n#[test]\nfn a_test() {}\n",
        )?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let has_type = graph
            .nodes()
            .iter()
            .any(|n| matches!(n, CodeNode::Struct(s) if s.name == "Foo"));
        let has_function = graph
            .nodes()
            .iter()
            .any(|n| matches!(n, CodeNode::Function(s) if s.name == "helper"));
        let has_test = graph
            .nodes()
            .iter()
            .any(|n| matches!(n, CodeNode::Test(s) if s.name == "a_test"));
        assert!(has_type, "expected a Struct node for Foo");
        assert!(has_function, "expected a Function node for helper");
        assert!(has_test, "expected a Test node for a_test");
        Ok(())
    }

    #[test]
    fn route_extraction_produces_route_edges() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("server.js");
        fs::write(&file_path, "app.get(\"/health\", (req, res) => {});")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        assert!(graph
            .routes()
            .iter()
            .any(|r| r.method == "GET" && r.path == "/health"));
        Ok(())
    }

    #[test]
    fn import_and_call_edges_are_recorded() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "use std::fs;\nfn f() { fs::read(\"x\"); }\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        assert!(graph.imports().iter().any(|i| i.module_path.contains("fs")));
        assert!(graph.calls().iter().any(|c| c.callee.contains("read")));
        Ok(())
    }

    #[test]
    fn unsupported_extension_becomes_text_only_node_not_skipped() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("NOTES.qux");
        fs::write(&file_path, "some free text notes")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository(dir.path(), &[file_path], &Manifest::default())?;

        let text_only = graph
            .nodes()
            .iter()
            .find(|n| matches!(n, CodeNode::TextOnly(f) if f.rel_path == "NOTES.qux"));
        assert!(
            text_only.is_some(),
            "unsupported extension must still produce a TextOnly node, never be skipped"
        );
        Ok(())
    }

    #[test]
    fn fast_mode_skips_git_history_full_mode_computes_it() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}")?;
        commit_all(dir.path(), "first")?;

        let mut fast_graph = CodeGraph::new();
        fast_graph.index_repository_with_options(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
            IndexOptions {
                mode: IndexMode::Fast,
                ..IndexOptions::default()
            },
        )?;
        let fast_file = fast_graph
            .file_nodes()
            .find(|f| f.rel_path == "a.rs")
            .ok_or("expected a.rs file node")?;
        assert_eq!(
            fast_file.last_commit, None,
            "fast mode must skip git history"
        );

        let mut full_graph = CodeGraph::new();
        full_graph.index_repository_with_options(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
            IndexOptions {
                mode: IndexMode::Full,
                ..IndexOptions::default()
            },
        )?;
        let full_file = full_graph
            .file_nodes()
            .find(|f| f.rel_path == "a.rs")
            .ok_or("expected a.rs file node")?;
        assert!(
            full_file.last_commit.is_some(),
            "full mode must compute git history"
        );
        Ok(())
    }

    #[test]
    fn persistence_true_without_project_name_is_a_typed_error() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let outcome = graph.index_repository_with_options(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
            IndexOptions {
                persistence: true,
                indexed_at: Some("2026-07-05T00:00:00Z"),
                ..IndexOptions::default()
            },
        );
        assert!(matches!(
            outcome,
            Err(IndexWithOptionsError::MissingProjectName)
        ));
        Ok(())
    }

    #[test]
    fn persistence_true_writes_artifact_and_bootstrap_reimports_same_counts() -> TestResult {
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}\nfn b() {}\n")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        graph.index_repository_with_options(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
            IndexOptions {
                mode: IndexMode::Full,
                persistence: true,
                project_name: Some("demo"),
                indexed_at: Some("2026-07-05T00:00:00Z"),
            },
        )?;
        assert!(crate::artifacts::artifact_exists(dir.path()));

        let original_node_count = graph.nodes().len();
        let original_edge_count =
            graph.imports().len() + graph.calls().len() + graph.routes().len();

        // A brand-new CodeGraph with an EMPTY previous manifest but the
        // artifact already on disk must bootstrap-import it before
        // indexing (even though this second call passes an empty file
        // list, so nothing would be found by walking otherwise).
        let mut bootstrapped = CodeGraph::new();
        bootstrapped.index_repository_with_options(
            dir.path(),
            &[],
            &Manifest::default(),
            IndexOptions::default(),
        )?;

        assert_eq!(bootstrapped.nodes().len(), original_node_count);
        let bootstrapped_edge_count =
            bootstrapped.imports().len() + bootstrapped.calls().len() + bootstrapped.routes().len();
        assert_eq!(bootstrapped_edge_count, original_edge_count);
        Ok(())
    }

    #[test]
    fn index_repository_original_signature_still_compiles_and_runs() -> TestResult {
        // Back-compat: the plain 3-arg `index_repository` (every existing
        // call site in this crate) must keep working unchanged.
        let dir = tempfile::tempdir()?;
        init_git_repo(dir.path())?;
        let file_path = dir.path().join("a.rs");
        fs::write(&file_path, "fn a() {}")?;
        commit_all(dir.path(), "first")?;

        let mut graph = CodeGraph::new();
        let (_manifest, report) = graph.index_repository(
            dir.path(),
            std::slice::from_ref(&file_path),
            &Manifest::default(),
        )?;
        assert_eq!(report.added, vec!["a.rs".to_string()]);
        Ok(())
    }
}
