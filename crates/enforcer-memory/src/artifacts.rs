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
//! - **typed lookup boundary**: this module accepts only a validated
//!   [`ArtifactId`]. Callers must decode untrusted text at their own input
//!   boundary before it can reach the manifest.
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
    NotFound { id: ArtifactId },

    /// The artifact's on-disk bytes no longer hash to the id the
    /// manifest recorded for them.
    #[error("artifact {id} failed integrity verification: {source}")]
    Corrupt {
        id: ArtifactId,
        #[source]
        source: crate::error::MemoryError,
    },
}

/// Exact-match artifact retrieval by a validated, content-addressed id.
/// Returns raw bytes on an exact hit; unknown ids and corrupt blobs remain
/// distinct typed errors -- never a substituted "similar" artifact.
pub fn get_exact(
    manifest: &ArtifactManifest,
    id: &ArtifactId,
) -> Result<Vec<u8>, ArtifactLookupError> {
    if manifest.entry(id).is_none() {
        // CLONE-JUSTIFICATION: the typed error owns the requested key after
        // this borrowed manifest lookup returns.
        return Err(ArtifactLookupError::NotFound {
            id: id.clone(),
        });
    }
    manifest.get(id).map_err(|source| match source {
        // CLONE-JUSTIFICATION: each error alternative owns an independent
        // requested key while the public API accepts only a borrow.
        crate::error::MemoryError::Io { .. } => ArtifactLookupError::NotFound {
            // CLONE-JUSTIFICATION: this error alternative owns the key.
            id: id.clone(),
        },
        other => ArtifactLookupError::Corrupt {
            id: id.clone(),
            source: other,
        },
    })
}

/// Exact-match snippet retrieval. A snippet is an artifact like any
/// other in this manifest -- same content-address, same fail-closed
/// exact-match contract.
pub fn get_snippet_exact(
    manifest: &ArtifactManifest,
    id: &ArtifactId,
) -> Result<Vec<u8>, ArtifactLookupError> {
    get_exact(manifest, id)
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
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    TypeAlias,
    Module,
    Lambda,
    Variable,
    Constant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSymbolSnapshot {
    pub id: String,
    pub kind: GraphSymbolKindSnapshot,
    pub name: String,
    pub file_id: String,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_body_fingerprint: Option<GraphSourceBodyFingerprintSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSourceBodyFingerprintSnapshot {
    pub source_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<usize>,
    pub body_grams: Vec<String>,
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
                CodeNode::File(f) => {
                    snapshot
                        .files
                        .push(file_snapshot(f, FileSnapshotKind::Structured));
                }
                CodeNode::TextOnly(f) => {
                    snapshot
                        .files
                        .push(file_snapshot(f, FileSnapshotKind::TextOnly));
                }
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
                CodeNode::Method(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Method));
                }
                CodeNode::Class(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Class));
                }
                CodeNode::Struct(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Struct));
                }
                CodeNode::Interface(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Interface));
                }
                CodeNode::Enum(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Enum));
                }
                CodeNode::TypeAlias(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::TypeAlias));
                }
                CodeNode::Module(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Module));
                }
                CodeNode::Lambda(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Lambda));
                }
                CodeNode::Variable(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Variable));
                }
                CodeNode::Constant(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Constant));
                }
                // CLONE-JUSTIFICATION: snapshots are owned persistence values
                // and must outlive the borrowed in-memory graph.
                CodeNode::Tombstone(t) => snapshot.tombstones.push(GraphTombstoneSnapshot {
                    // CLONE-JUSTIFICATION: this DTO owns its id.
                    id: t.id.clone(),
                    rel_path: t.rel_path.clone(),
                    last_commit: t.last_commit.clone(),
                    change_count: t.change_count,
                    // CLONE-JUSTIFICATION: this DTO owns its historical ids.
                    prior_chunk_ids: t.prior_chunk_ids.clone(),
                }),
            }
        }
        for edge in graph.imports() {
            // CLONE-JUSTIFICATION: the persisted edge snapshot owns graph
            // identifiers independently of the source graph.
            snapshot.imports.push(ImportEdgeSnapshot {
                from_file_id: edge.from_file_id.clone(),
                module_path: edge.module_path.clone(),
                line: edge.line,
            });
        }
        for edge in graph.calls() {
            // CLONE-JUSTIFICATION: the persisted edge snapshot owns graph
            // identifiers independently of the source graph.
            snapshot.calls.push(CallEdgeSnapshot {
                from_file_id: edge.from_file_id.clone(),
                callee: edge.callee.clone(),
                line: edge.line,
            });
        }
        for edge in graph.routes() {
            // CLONE-JUSTIFICATION: the persisted edge snapshot owns graph
            // identifiers independently of the source graph.
            snapshot.routes.push(RouteEdgeSnapshot {
                from_file_id: edge.from_file_id.clone(),
                method: edge.method.clone(),
                // CLONE-JUSTIFICATION: the persisted route path is owned.
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

/// The source graph distinguishes structured files from text-only files.
/// Keeping that distinction typed avoids accidentally reversing a boolean at
/// this persistence boundary.
enum FileSnapshotKind {
    Structured,
    TextOnly,
}

impl FileSnapshotKind {
    const fn is_text_only(&self) -> bool {
        matches!(self, Self::TextOnly)
    }
}

fn file_snapshot(f: &crate::code_graph::FileNode, kind: FileSnapshotKind) -> GraphFileSnapshot {
    // CLONE-JUSTIFICATION: a graph snapshot is an owned serialization
    // boundary and cannot borrow fields from the live graph node.
    GraphFileSnapshot {
        // CLONE-JUSTIFICATION: exported id is owned snapshot data.
        id: f.id.clone(),
        // CLONE-JUSTIFICATION: exported path is owned snapshot data.
        rel_path: f.rel_path.clone(),
        text_only: kind.is_text_only(),
        // CLONE-JUSTIFICATION: exported content digest is owned snapshot data.
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
    // CLONE-JUSTIFICATION: the snapshot owns stable wire data after the
    // in-memory symbol graph may be mutated or dropped.
    GraphSymbolSnapshot {
        id: s.id.clone(),
        kind,
        // CLONE-JUSTIFICATION: exported symbol name is owned snapshot data.
        name: s.name.clone(),
        // CLONE-JUSTIFICATION: exported file id is owned snapshot data.
        file_id: s.file_id.clone(),
        line: s.line,
        source_body_fingerprint: s.source_body_fingerprint.as_ref().map(|fp| {
            // CLONE-JUSTIFICATION: fingerprint wire values are owned by the
            // exported snapshot, not borrowed from the live graph.
            GraphSourceBodyFingerprintSnapshot {
                // CLONE-JUSTIFICATION: exported fingerprint hash is owned.
                source_hash: fp.source_hash.clone(),
                // CLONE-JUSTIFICATION: exported optional fingerprint is owned.
                fp: fp.fp.clone(),
                k: fp.k,
                body_grams: fp.body_grams.iter().cloned().collect(),
            }
        }),
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
    #[error("system clock is before the Unix epoch: {0}")]
    Clock(#[source] std::time::SystemTimeError),
    #[error("artifact byte length exceeds the u64 persistence format")]
    ArtifactTooLarge,
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
        // CLONE-JUSTIFICATION: the typed error must retain the failed path.
        path: dir.clone(),
        source,
    })?;

    let json = serde_json::to_vec(snapshot)?;
    let original_size = json
        .len()
        .try_into()
        .map_err(|_| GraphArtifactError::ArtifactTooLarge)?;

    let mut encoder = zstd::Encoder::new(Vec::new(), GRAPH_ARTIFACT_COMPRESSION_LEVEL)
        .map_err(GraphArtifactError::Compression)?;
    encoder
        .write_all(&json)
        .map_err(GraphArtifactError::Compression)?;
    let compressed = encoder.finish().map_err(GraphArtifactError::Compression)?;
    let compressed_size = compressed
        .len()
        .try_into()
        .map_err(|_| GraphArtifactError::ArtifactTooLarge)?;

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
            // CLONE-JUSTIFICATION: the typed error must retain the failed path.
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
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(GraphArtifactError::Clock)?
        .as_nanos();
    let unique = format!(
        "{}.{}.tmp",
        std::process::id(),
        timestamp
    );
    let tmp_path = path.with_extension(unique);
    std::fs::write(&tmp_path, bytes).map_err(|source| GraphArtifactError::Io {
        // CLONE-JUSTIFICATION: the typed error must retain the failed temp path.
        path: tmp_path.clone(),
        source,
    })?;
    std::fs::rename(&tmp_path, path).map_err(|source| GraphArtifactError::Io {
        path: path.to_path_buf(),
        source,
    })
}
