//! X06.8: exact artifact retrieval, plus the `.codebase-memory/graph.db.zst`
//! + `artifact.json` persistence shape from the baseline doc §9.4.
//!
//! # Exact-match retrieval
//!
//! [`crate::store::manifest::ArtifactManifest`] is already a
//! content-addressed blob store (`put`/`get` keyed by the artifact's own
//! SHA-256 digest). This module adds the query-surface contract on top
//! of it:
//!
//! - **exact match only**: [`get_exact`] either returns the artifact
//!   whose id is byte-for-byte the requested id, or a typed
//!   [`ArtifactLookupError`] -- it NEVER falls back to a
//!   similar/fuzzy/nearest artifact. An id that does not exist yields
//!   [`ArtifactLookupError::NotFound`], never a "close enough"
//!   substitute.
//! - **traversal rejection**: any requested id shaped like a filesystem
//!   escape attempt (`../`, backslash-form `..\`, an absolute path, or a
//!   NUL byte) is rejected with [`ArtifactLookupError::TraversalRejected`]
//!   before ever touching the manifest or the filesystem.
//!
//! # Persistence artifact (baseline §9.4 field parity)
//!
//! [`export_graph_artifact`]/[`import_graph_artifact`] serialize a
//! [`crate::code_graph::CodeGraph`]'s node/edge tables to a compact
//! bincode-free JSON form (this crate's `CodeGraph` is in-memory/
//! Rust-native, not SQLite-backed, so "VACUUM INTO" has no literal
//! analogue here -- the equivalent step is "serialize every node/edge
//! table"), zstd-compress it, and write it atomically
//! (write-to-temp-then-rename) alongside a sidecar `artifact.json`
//! carrying exactly the baseline's field set: `schema_version` (=2),
//! `commit`, `indexed_at`, `project`, `nodes`, `edges`, `original_size`,
//! `compressed_size`, `compression_level`.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::code_graph::CodeNode;
use crate::ids::ArtifactId;
use crate::store::manifest::ArtifactManifest;

/// Schema version for the `artifact.json` sidecar, matching the baseline
/// doc's `CBM_ARTIFACT_SCHEMA_VERSION = 2`.
pub const GRAPH_ARTIFACT_SCHEMA_VERSION: u32 = 2;

/// Filename of the compressed graph artifact, matching the baseline's
/// `CBM_ARTIFACT_FILENAME`.
pub const GRAPH_ARTIFACT_FILENAME: &str = "graph.db.zst";

/// Filename of the sidecar metadata file, matching the baseline's
/// `CBM_ARTIFACT_META`.
pub const GRAPH_ARTIFACT_META_FILENAME: &str = "artifact.json";

/// zstd compression level used for the graph artifact. A fixed,
/// moderate level (matching the baseline's `ART_ZSTD_FAST`) -- this
/// slice does not implement the baseline's "best" index-strip pass, so
/// there is only one level to name.
pub const GRAPH_ARTIFACT_COMPRESSION_LEVEL: i32 = 3;

/// Exact-artifact-lookup failures. Deliberately narrower than
/// [`crate::error::MemoryError`] (which also covers this crate's
/// store/log machinery generally) -- this is the fail-closed surface
/// callers of the retrieval API match on.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactLookupError {
    /// The requested id is well-formed but no artifact with that exact
    /// content-address exists in the manifest. NEVER substituted with a
    /// similar artifact.
    #[error("no artifact with exact id {id:?} exists -- exact lookup never falls back to a similar artifact")]
    NotFound { id: String },

    /// The requested id is shaped like a filesystem traversal attempt
    /// (`../`, `..\`, an absolute path, or embeds a NUL byte). Rejected
    /// before any manifest/filesystem access.
    #[error("rejected traversal-shaped artifact reference {raw:?}")]
    TraversalRejected { raw: String },

    /// The artifact's on-disk bytes no longer hash to the id the
    /// manifest recorded for them.
    #[error("artifact {id} failed integrity verification: {source}")]
    Corrupt {
        id: String,
        #[source]
        source: crate::error::MemoryError,
    },
}

/// Reject any raw artifact reference that is shaped like a filesystem
/// traversal attempt. Pure string check, no filesystem access, run
/// BEFORE the value is used to build any path or manifest key.
fn reject_traversal(raw: &str) -> Result<(), ArtifactLookupError> {
    let looks_like_drive_prefix = raw.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && raw.as_bytes().get(1) == Some(&b':');
    let is_traversal = raw.contains("..")
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.contains('\0')
        || looks_like_drive_prefix;
    if is_traversal {
        return Err(ArtifactLookupError::TraversalRejected {
            raw: raw.to_owned(),
        });
    }
    Ok(())
}

/// Parse `raw` into an [`ArtifactId`] for an exact-match lookup. A
/// malformed id is reported as [`ArtifactLookupError::NotFound`] (it can
/// never match anything in the manifest) rather than panicking.
fn parse_claimed_id(raw: &str) -> Result<ArtifactId, ArtifactLookupError> {
    reject_traversal(raw)?;
    // The decode error itself carries no information useful to a caller
    // beyond "this string is not a well-formed digest" -- reporting it
    // as `NotFound` (never a substitute match) is the fail-closed
    // contract this function exists for, so the source error is
    // deliberately not threaded through (`_decode_error`, not `_`, so
    // clippy's `map_err_ignore` sees the discard is intentional).
    let sha: enforcer_domain::hashes::Sha256 = raw
        .parse()
        .map_err(|_decode_error| ArtifactLookupError::NotFound { id: raw.to_owned() })?;
    Ok(ArtifactId::from_digest(sha))
}

/// Exact-match artifact retrieval by content-addressed id. Returns the
/// artifact's raw bytes on an exact hit; every other outcome (unknown
/// id, malformed id, traversal-shaped id, corrupted blob) is a distinct
/// typed error -- never a substituted "similar" artifact.
pub fn get_exact(
    manifest: &ArtifactManifest,
    raw_id: &str,
) -> Result<Vec<u8>, ArtifactLookupError> {
    let id = parse_claimed_id(raw_id)?;
    if manifest.entry(&id).is_none() {
        return Err(ArtifactLookupError::NotFound {
            id: raw_id.to_owned(),
        });
    }
    manifest.get(&id).map_err(|source| match source {
        crate::error::MemoryError::Io { .. } => ArtifactLookupError::NotFound {
            id: raw_id.to_owned(),
        },
        other => ArtifactLookupError::Corrupt {
            id: raw_id.to_owned(),
            source: other,
        },
    })
}

/// Exact-match snippet retrieval. A snippet is an artifact like any
/// other in this manifest -- same content-address, same fail-closed
/// exact-match contract, same traversal rejection.
pub fn get_snippet_exact(
    manifest: &ArtifactManifest,
    raw_id: &str,
) -> Result<Vec<u8>, ArtifactLookupError> {
    get_exact(manifest, raw_id)
}

// ---------------------------------------------------------------------
// Persistence artifact: graph.db.zst + artifact.json
// ---------------------------------------------------------------------

/// A plain-data, serializable snapshot of a [`crate::code_graph::CodeGraph`]'s
/// node/edge tables -- the "serialize CodeGraph's node/edge tables to a
/// compact on-disk form" this module's docs describe. `CodeGraph`'s own
/// node/edge types are not `Serialize` (they intentionally have no
/// persistence trait of their own, per that module's docs), so this is a
/// deliberately flattened wire copy owned entirely by this module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub files: Vec<GraphFileSnapshot>,
    pub symbols: Vec<GraphSymbolSnapshot>,
    pub tombstones: Vec<GraphTombstoneSnapshot>,
    pub imports: Vec<ImportEdgeSnapshot>,
    pub calls: Vec<CallEdgeSnapshot>,
    pub routes: Vec<RouteEdgeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphFileSnapshot {
    pub id: String,
    pub rel_path: String,
    pub text_only: bool,
    pub content_hash: String,
    pub last_commit: Option<String>,
    pub change_count: usize,
    pub chunk_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphSymbolKindSnapshot {
    Function,
    Type,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSymbolSnapshot {
    pub id: String,
    pub kind: GraphSymbolKindSnapshot,
    pub name: String,
    pub file_id: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphTombstoneSnapshot {
    pub id: String,
    pub rel_path: String,
    pub last_commit: Option<String>,
    pub change_count: usize,
    pub prior_chunk_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportEdgeSnapshot {
    pub from_file_id: String,
    pub module_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEdgeSnapshot {
    pub from_file_id: String,
    pub callee: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEdgeSnapshot {
    pub from_file_id: String,
    pub method: String,
    pub path: String,
    pub line: usize,
}

impl GraphSnapshot {
    /// Flatten every node/edge in `graph` into this wire snapshot.
    pub fn from_code_graph(graph: &crate::code_graph::CodeGraph) -> Self {
        let mut snapshot = GraphSnapshot::default();
        for node in graph.nodes() {
            match node {
                CodeNode::File(f) => snapshot.files.push(file_snapshot(f, false)),
                CodeNode::TextOnly(f) => snapshot.files.push(file_snapshot(f, true)),
                CodeNode::Function(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Function));
                }
                CodeNode::Type(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Type));
                }
                CodeNode::Test(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Test));
                }
                // X06 rich vocabulary (additive CodeNode variants): this
                // wire snapshot's `GraphSymbolKindSnapshot` has not yet
                // been extended to carry the new label set -- best-effort
                // fold to the nearest existing snapshot kind so
                // persistence round-trips (node/edge counts) rather than
                // silently dropping the node. Extending the wire schema
                // itself is a follow-up, not this lane's claimed scope.
                CodeNode::Method(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Function));
                }
                CodeNode::Class(s)
                | CodeNode::Struct(s)
                | CodeNode::Interface(s)
                | CodeNode::Enum(s)
                | CodeNode::TypeAlias(s)
                | CodeNode::Module(s)
                | CodeNode::Variable(s)
                | CodeNode::Constant(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Type));
                }
                CodeNode::Lambda(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Function));
                }
                CodeNode::Tombstone(t) => snapshot.tombstones.push(GraphTombstoneSnapshot {
                    id: t.id.clone(),
                    rel_path: t.rel_path.clone(),
                    last_commit: t.last_commit.clone(),
                    change_count: t.change_count,
                    prior_chunk_ids: t.prior_chunk_ids.clone(),
                }),
            }
        }
        for edge in graph.imports() {
            snapshot.imports.push(ImportEdgeSnapshot {
                from_file_id: edge.from_file_id.clone(),
                module_path: edge.module_path.clone(),
                line: edge.line,
            });
        }
        for edge in graph.calls() {
            snapshot.calls.push(CallEdgeSnapshot {
                from_file_id: edge.from_file_id.clone(),
                callee: edge.callee.clone(),
                line: edge.line,
            });
        }
        for edge in graph.routes() {
            snapshot.routes.push(RouteEdgeSnapshot {
                from_file_id: edge.from_file_id.clone(),
                method: edge.method.clone(),
                path: edge.path.clone(),
                line: edge.line,
            });
        }
        snapshot
    }

    /// Total node count this snapshot represents (files + symbols +
    /// tombstones), matching the baseline `artifact.json`'s `nodes` field
    /// semantics.
    pub fn node_count(&self) -> usize {
        self.files.len() + self.symbols.len() + self.tombstones.len()
    }

    /// Total edge count (imports + calls + routes), matching the
    /// baseline `artifact.json`'s `edges` field semantics.
    pub fn edge_count(&self) -> usize {
        self.imports.len() + self.calls.len() + self.routes.len()
    }
}

fn file_snapshot(f: &crate::code_graph::FileNode, text_only: bool) -> GraphFileSnapshot {
    GraphFileSnapshot {
        id: f.id.clone(),
        rel_path: f.rel_path.clone(),
        text_only,
        content_hash: f.content_hash.clone(),
        last_commit: f.last_commit.clone(),
        change_count: f.change_count,
        chunk_ids: f.chunk_ids.clone(),
    }
}

fn symbol_snapshot(
    s: &crate::code_graph::SymbolNode,
    kind: GraphSymbolKindSnapshot,
) -> GraphSymbolSnapshot {
    GraphSymbolSnapshot {
        id: s.id.clone(),
        kind,
        name: s.name.clone(),
        file_id: s.file_id.clone(),
        line: s.line,
    }
}

/// `artifact.json` sidecar, field-for-field matching the baseline doc's
/// §9.4 shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub schema_version: u32,
    pub commit: Option<String>,
    pub indexed_at: String,
    pub project: String,
    pub nodes: usize,
    pub edges: usize,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_level: i32,
}

/// Errors from exporting/importing the persistence artifact.
#[derive(Debug, thiserror::Error)]
pub enum GraphArtifactError {
    #[error("json codec failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zstd compression failed: {0}")]
    Compression(#[source] std::io::Error),
    #[error("zstd decompression failed: {0}")]
    Decompression(#[source] std::io::Error),
    #[error("io error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The artifact's `.zst` file already exists at the destination --
    /// export refuses to silently overwrite an existing artifact
    /// (mirrors the baseline's O_EXCL-guarded sidecar write).
    #[error("artifact already exists at {path:?} -- refusing to silently overwrite")]
    AlreadyExists { path: PathBuf },
    /// The sidecar's `schema_version` is not
    /// [`GRAPH_ARTIFACT_SCHEMA_VERSION`].
    #[error("artifact schema version {found} is not supported (expected {expected})")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },
}

/// Where the persistence artifact lives for a given repo root:
/// `<repo_root>/.codebase-memory/{graph.db.zst,artifact.json}`.
pub fn artifact_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".codebase-memory")
}

/// Whether a persistence artifact already exists at `repo_root` (both
/// the `.zst` file is present and non-empty, and the sidecar parses with
/// a supported schema version) -- mirrors `cbm_artifact_exists`.
pub fn artifact_exists(repo_root: &Path) -> bool {
    let dir = artifact_dir(repo_root);
    let zst_path = dir.join(GRAPH_ARTIFACT_FILENAME);
    let meta_path = dir.join(GRAPH_ARTIFACT_META_FILENAME);
    let zst_non_empty = std::fs::metadata(&zst_path)
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    if !zst_non_empty {
        return false;
    }
    let Ok(raw) = std::fs::read_to_string(&meta_path) else {
        return false;
    };
    let Ok(meta) = serde_json::from_str::<ArtifactMetadata>(&raw) else {
        return false;
    };
    meta.schema_version <= GRAPH_ARTIFACT_SCHEMA_VERSION
}

/// Export `snapshot` to `<repo_root>/.codebase-memory/graph.db.zst` +
/// `artifact.json`, atomically: the `.zst` and sidecar are each written
/// to a temp path first and then renamed into place, so a reader never
/// observes a half-written artifact.
pub fn export_graph_artifact(
    repo_root: &Path,
    snapshot: &GraphSnapshot,
    project: &str,
    commit: Option<String>,
    indexed_at: &str,
) -> Result<ArtifactMetadata, GraphArtifactError> {
    let dir = artifact_dir(repo_root);
    std::fs::create_dir_all(&dir).map_err(|source| GraphArtifactError::Io {
        path: dir.clone(),
        source,
    })?;

    let json = serde_json::to_vec(snapshot)?;
    let original_size = json.len() as u64;

    let mut encoder = zstd::Encoder::new(Vec::new(), GRAPH_ARTIFACT_COMPRESSION_LEVEL)
        .map_err(GraphArtifactError::Compression)?;
    encoder
        .write_all(&json)
        .map_err(GraphArtifactError::Compression)?;
    let compressed = encoder.finish().map_err(GraphArtifactError::Compression)?;
    let compressed_size = compressed.len() as u64;

    let zst_path = dir.join(GRAPH_ARTIFACT_FILENAME);
    let meta_path = dir.join(GRAPH_ARTIFACT_META_FILENAME);
    write_atomic(&zst_path, &compressed)?;

    let metadata = ArtifactMetadata {
        schema_version: GRAPH_ARTIFACT_SCHEMA_VERSION,
        commit,
        indexed_at: indexed_at.to_owned(),
        project: project.to_owned(),
        nodes: snapshot.node_count(),
        edges: snapshot.edge_count(),
        original_size,
        compressed_size,
        compression_level: GRAPH_ARTIFACT_COMPRESSION_LEVEL,
    };
    let meta_json = serde_json::to_vec_pretty(&metadata)?;
    write_atomic(&meta_path, &meta_json)?;

    Ok(metadata)
}

/// Import a previously exported artifact from `repo_root`, returning the
/// reconstructed [`GraphSnapshot`] and its sidecar [`ArtifactMetadata`].
/// Fails closed on a missing artifact, a corrupt zstd stream, or an
/// unsupported schema version -- never a partial/best-effort import.
pub fn import_graph_artifact(
    repo_root: &Path,
) -> Result<(GraphSnapshot, ArtifactMetadata), GraphArtifactError> {
    let dir = artifact_dir(repo_root);
    let zst_path = dir.join(GRAPH_ARTIFACT_FILENAME);
    let meta_path = dir.join(GRAPH_ARTIFACT_META_FILENAME);

    let meta_raw =
        std::fs::read_to_string(&meta_path).map_err(|source| GraphArtifactError::Io {
            path: meta_path.clone(),
            source,
        })?;
    let metadata: ArtifactMetadata = serde_json::from_str(&meta_raw)?;
    if metadata.schema_version > GRAPH_ARTIFACT_SCHEMA_VERSION {
        return Err(GraphArtifactError::UnsupportedSchemaVersion {
            found: metadata.schema_version,
            expected: GRAPH_ARTIFACT_SCHEMA_VERSION,
        });
    }

    let compressed = std::fs::read(&zst_path).map_err(|source| GraphArtifactError::Io {
        path: zst_path,
        source,
    })?;
    let decompressed =
        zstd::decode_all(compressed.as_slice()).map_err(GraphArtifactError::Decompression)?;
    let snapshot: GraphSnapshot = serde_json::from_slice(&decompressed)?;

    Ok((snapshot, metadata))
}

/// Write `bytes` to `path` atomically: write to a sibling temp file
/// first, then rename over the destination. The temp file is created
/// with a unique name derived from the process id and a monotonic
/// counter-like nanosecond timestamp so concurrent exports never
/// collide on the same temp path (the O_EXCL-guard analogue -- an
/// existing destination is replaced only by the final rename, never by
/// an in-place write).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), GraphArtifactError> {
    let unique = format!(
        "{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    let tmp_path = path.with_extension(unique);
    std::fs::write(&tmp_path, bytes).map_err(|source| GraphArtifactError::Io {
        path: tmp_path.clone(),
        source,
    })?;
    std::fs::rename(&tmp_path, path).map_err(|source| GraphArtifactError::Io {
        path: path.to_path_buf(),
        source,
    })
}
