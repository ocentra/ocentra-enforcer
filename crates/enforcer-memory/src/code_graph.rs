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
}

impl CodeNode {
    pub fn id(&self) -> &str {
        match self {
            CodeNode::File(node) | CodeNode::TextOnly(node) => &node.id,
            CodeNode::Function(node) | CodeNode::Type(node) | CodeNode::Test(node) => &node.id,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallEdge {
    pub from_file_id: String,
    pub callee: String,
    pub line: usize,
}

/// A route/endpoint declared in `from_file_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEdge {
    pub from_file_id: String,
    pub method: String,
    pub path: String,
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
}

impl CodeGraph {
    pub fn new() -> Self {
        Self::default()
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
            CodeNode::Function(s) | CodeNode::Type(s) | CodeNode::Test(s) => Some(s),
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
            let history = git
                .as_mut()
                .map(|g| g.history_for(&rel_path))
                .unwrap_or_default();
            let text = String::from_utf8_lossy(&content).into_owned();
            let language = parsers::classify(&rel_path);
            let parsed = parsers::parse_file(language, &text);

            let (file_id, chunk_ids) = self.insert_file_and_chunks(NewFileParams {
                rel_path: &rel_path,
                language,
                content_hash: &content_hash,
                history: &history,
                parsed,
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

        Ok((new_manifest, report))
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
        } = params;
        let file_id = file_id_for(rel_path);
        let mut chunk_ids = Vec::new();

        if let Some(parsed) = parsed {
            for symbol in &parsed.symbols {
                let sym_id = format!("sym:{rel_path}:{}:{}", symbol.line, symbol.name);
                let node = SymbolNode {
                    id: sym_id.clone(),
                    name: symbol.name.clone(),
                    file_id: file_id.clone(),
                    line: symbol.line,
                };
                self.nodes.push(match symbol.kind {
                    parsers::SymbolKind::Function => CodeNode::Function(node),
                    parsers::SymbolKind::Type => CodeNode::Type(node),
                    parsers::SymbolKind::Test => CodeNode::Test(node),
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
            .any(|n| matches!(n, CodeNode::Type(s) if s.name == "Foo"));
        let has_function = graph
            .nodes()
            .iter()
            .any(|n| matches!(n, CodeNode::Function(s) if s.name == "helper"));
        let has_test = graph
            .nodes()
            .iter()
            .any(|n| matches!(n, CodeNode::Test(s) if s.name == "a_test"));
        assert!(has_type, "expected a Type node for Foo");
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
}
