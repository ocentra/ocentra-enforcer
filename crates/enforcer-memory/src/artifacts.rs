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

use crate::boundary::artifact_transport::{
    ArtifactCommitDto, ArtifactIndexedAtDto, ArtifactMetadataDto, ArtifactProjectDto,
    CallEdgeSnapshotDto, GraphFileSnapshotDto, GraphSnapshotDto,
    GraphSourceBodyFingerprintSnapshotDto, GraphSymbolSnapshotDto, GraphTombstoneSnapshotDto,
    ImportEdgeSnapshotDto, RouteEdgeSnapshotDto,
};
use crate::code_graph::CodeNode;
use crate::store::manifest::ArtifactManifest;
use enforcer_domain::memory_types::{
    ArtifactId, GraphArtifactCommit, GraphArtifactDirectory, GraphArtifactIndexedAt,
    GraphArtifactPath, GraphArtifactPresence, GraphArtifactProjectName, GraphArtifactRepoRoot,
    GraphArtifactSchemaVersion, GraphCompressionLevel, GraphEdgeCount, GraphNodeCount,
    GraphSymbolKindSnapshot, GraphTextOnly, MemoryArtifactBytes,
};

/// Schema version for the `artifact.json` sidecar, matching the baseline
/// doc's `CBM_ARTIFACT_SCHEMA_VERSION = 2`.
pub const GRAPH_ARTIFACT_SCHEMA_VERSION: GraphArtifactSchemaVersion =
    GraphArtifactSchemaVersion::CURRENT;

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
pub const GRAPH_ARTIFACT_COMPRESSION_LEVEL: GraphCompressionLevel = GraphCompressionLevel::FAST;

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
        source: Box<crate::error::MemoryError>,
    },
}

/// Exact-match artifact retrieval by a validated, content-addressed id.
/// Returns raw bytes on an exact hit; unknown ids and corrupt blobs remain
/// distinct typed errors -- never a substituted "similar" artifact.
pub fn get_exact(
    manifest: &ArtifactManifest,
    id: &ArtifactId,
) -> Result<MemoryArtifactBytes, ArtifactLookupError> {
    if manifest.entry(id).is_none() {
        // CLONE-JUSTIFICATION: the typed error owns the requested key after
        // this borrowed manifest lookup returns.
        return Err(ArtifactLookupError::NotFound { id: id.clone() });
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
            source: Box::new(other),
        },
    })
}

/// Exact-match snippet retrieval. A snippet is an artifact like any
/// other in this manifest -- same content-address, same fail-closed
/// exact-match contract.
pub fn get_snippet_exact(
    manifest: &ArtifactManifest,
    id: &ArtifactId,
) -> Result<MemoryArtifactBytes, ArtifactLookupError> {
    get_exact(manifest, id)
}

// ---------------------------------------------------------------------
// Persistence artifact: graph.db.zst + artifact.json
// ---------------------------------------------------------------------

impl GraphSnapshotDto {
    /// Flatten every node/edge in `graph` into this wire snapshot.
    pub fn from_code_graph(
        graph: &crate::code_graph::CodeGraph,
    ) -> Result<Self, GraphArtifactError> {
        let mut snapshot = GraphSnapshotDto::default();
        for node in graph.nodes() {
            match node {
                CodeNode::File(f) => {
                    snapshot
                        .files
                        .push(file_snapshot(f, FileSnapshotKind::Structured)?);
                }
                CodeNode::TextOnly(f) => {
                    snapshot
                        .files
                        .push(file_snapshot(f, FileSnapshotKind::TextOnly)?);
                }
                CodeNode::Function(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Function)?);
                }
                CodeNode::Type(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Type)?);
                }
                CodeNode::Test(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Test)?);
                }
                CodeNode::Method(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Method)?);
                }
                CodeNode::Class(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Class)?);
                }
                CodeNode::Struct(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Struct)?);
                }
                CodeNode::Interface(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Interface)?);
                }
                CodeNode::Enum(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Enum)?);
                }
                CodeNode::TypeAlias(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::TypeAlias)?);
                }
                CodeNode::Module(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Module)?);
                }
                CodeNode::Lambda(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Lambda)?);
                }
                CodeNode::Variable(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Variable)?);
                }
                CodeNode::Constant(s) => {
                    snapshot
                        .symbols
                        .push(symbol_snapshot(s, GraphSymbolKindSnapshot::Constant)?);
                }
                // CLONE-JUSTIFICATION: snapshots are owned persistence values
                // and must outlive the borrowed in-memory graph.
                CodeNode::Tombstone(t) => snapshot.tombstones.push(GraphTombstoneSnapshotDto {
                    // CLONE-JUSTIFICATION: this DTO owns its id.
                    id: t.id.clone().try_into()?,
                    rel_path: t.rel_path.clone().try_into()?,
                    last_commit: t.last_commit.clone().map(TryInto::try_into).transpose()?,
                    change_count: t.change_count,
                    // CLONE-JUSTIFICATION: this DTO owns its historical ids.
                    prior_chunk_ids: t
                        .prior_chunk_ids
                        .iter()
                        .cloned()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                }),
            }
        }
        for edge in graph.imports() {
            // CLONE-JUSTIFICATION: the persisted edge snapshot owns graph
            // identifiers independently of the source graph.
            snapshot.imports.push(ImportEdgeSnapshotDto {
                from_file_id: edge.from_file_id.clone().try_into()?,
                module_path: edge.module_path.clone().try_into()?,
                line: edge.line,
            });
        }
        for edge in graph.calls() {
            // CLONE-JUSTIFICATION: the persisted edge snapshot owns graph
            // identifiers independently of the source graph.
            snapshot.calls.push(CallEdgeSnapshotDto {
                from_file_id: edge.from_file_id.clone().try_into()?,
                callee: edge.callee.clone().try_into()?,
                line: edge.line,
            });
        }
        for edge in graph.routes() {
            // CLONE-JUSTIFICATION: the persisted edge snapshot owns graph
            // identifiers independently of the source graph.
            snapshot.routes.push(RouteEdgeSnapshotDto {
                from_file_id: edge.from_file_id.clone().try_into()?,
                method: edge.method.clone().try_into()?,
                // CLONE-JUSTIFICATION: the persisted route path is owned.
                path: edge.path.clone().try_into()?,
                line: edge.line,
            });
        }
        Ok(snapshot)
    }

    /// Total node count this snapshot represents (files + symbols +
    /// tombstones), matching the baseline `artifact.json`'s `nodes` field
    /// semantics.
    pub fn node_count(&self) -> GraphNodeCount {
        (self.files.len() + self.symbols.len() + self.tombstones.len()).into()
    }

    /// Total edge count (imports + calls + routes), matching the
    /// baseline `artifact.json`'s `edges` field semantics.
    pub fn edge_count(&self) -> GraphEdgeCount {
        (self.imports.len() + self.calls.len() + self.routes.len()).into()
    }
}

/// The source graph distinguishes structured files from text-only files.
/// Keeping that distinction typed avoids accidentally reversing a boolean at
/// this persistence boundary.
#[derive(Clone, Copy)]
enum FileSnapshotKind {
    Structured,
    TextOnly,
}

impl FileSnapshotKind {
    const fn text_only(&self) -> GraphTextOnly {
        match self {
            Self::Structured => GraphTextOnly::STRUCTURED,
            Self::TextOnly => GraphTextOnly::TEXT_ONLY,
        }
    }
}

fn file_snapshot(
    f: &crate::code_graph::FileNode,
    kind: FileSnapshotKind,
) -> Result<GraphFileSnapshotDto, GraphArtifactError> {
    // CLONE-JUSTIFICATION: a graph snapshot is an owned serialization
    // boundary and cannot borrow fields from the live graph node.
    Ok(GraphFileSnapshotDto {
        // CLONE-JUSTIFICATION: exported id is owned snapshot data.
        id: f.id.clone().try_into()?,
        // CLONE-JUSTIFICATION: exported path is owned snapshot data.
        rel_path: f.rel_path.clone().try_into()?,
        text_only: kind.text_only(),
        // CLONE-JUSTIFICATION: exported content digest is owned snapshot data.
        content_hash: f.content_hash.clone().try_into()?,
        last_commit: f.last_commit.clone().map(TryInto::try_into).transpose()?,
        change_count: f.change_count,
        chunk_ids: f
            .chunk_ids
            .iter()
            .cloned()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()?,
    })
}

fn symbol_snapshot(
    s: &crate::code_graph::SymbolNode,
    kind: GraphSymbolKindSnapshot,
) -> Result<GraphSymbolSnapshotDto, GraphArtifactError> {
    // CLONE-JUSTIFICATION: the snapshot owns stable wire data after the
    // in-memory symbol graph may be mutated or dropped.
    Ok(GraphSymbolSnapshotDto {
        id: s.id.clone().try_into()?,
        kind,
        // CLONE-JUSTIFICATION: exported symbol name is owned snapshot data.
        name: s.name.clone().try_into()?,
        // CLONE-JUSTIFICATION: exported file id is owned snapshot data.
        file_id: s.file_id.clone().try_into()?,
        line: s.line,
        source_body_fingerprint: s
            .source_body_fingerprint
            .as_ref()
            .map(|fp| {
                // CLONE-JUSTIFICATION: fingerprint wire values are owned by the
                // exported snapshot, not borrowed from the live graph.
                Ok::<
                    GraphSourceBodyFingerprintSnapshotDto,
                    enforcer_domain::boundary::decode_error::DecodeError,
                >(GraphSourceBodyFingerprintSnapshotDto {
                    source_hash: String::from(fp.source_hash.as_str()).try_into()?,
                    fp: fp
                        .fp
                        .as_ref()
                        .map(|value| String::from(value.as_str()).try_into())
                        .transpose()?,
                    k: fp.k.map(|value| value.get().into()),
                    body_grams: fp
                        .body_grams
                        .iter()
                        .map(|gram| String::from(gram.as_str()).try_into())
                        .collect::<Result<_, _>>()?,
                })
            })
            .transpose()?,
    })
}

/// Errors from exporting/importing the persistence artifact.
#[derive(Debug, thiserror::Error)]
pub enum GraphArtifactError {
    #[error("graph snapshot contains an invalid canonical value: {0}")]
    InvalidSnapshotValue(#[from] enforcer_domain::boundary::decode_error::DecodeError),
    #[error("json codec failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zstd compression failed: {0}")]
    Compression(#[source] std::io::Error),
    #[error("zstd decompression failed: {0}")]
    Decompression(#[source] std::io::Error),
    #[error("io error at {path:?}: {source}")]
    Io {
        path: GraphArtifactPath,
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
    AlreadyExists { path: GraphArtifactPath },
    /// The sidecar's `schema_version` is not
    /// [`GRAPH_ARTIFACT_SCHEMA_VERSION`].
    #[error("artifact schema version {found} is not supported (expected {expected})")]
    UnsupportedSchemaVersion {
        found: GraphArtifactSchemaVersion,
        expected: GraphArtifactSchemaVersion,
    },
}

/// Where the persistence artifact lives for a given repo root:
/// `<repo_root>/.codebase-memory/{graph.db.zst,artifact.json}`.
pub fn artifact_dir<'a>(repo_root: impl Into<GraphArtifactRepoRoot<'a>>) -> GraphArtifactDirectory {
    repo_root.into().as_path().join(".codebase-memory").into()
}

/// Whether a persistence artifact already exists at `repo_root` (both
/// the `.zst` file is present and non-empty, and the sidecar parses with
/// a supported schema version) -- mirrors `cbm_artifact_exists`.
pub fn artifact_exists<'a>(
    repo_root: impl Into<GraphArtifactRepoRoot<'a>>,
) -> GraphArtifactPresence {
    let dir = artifact_dir(repo_root);
    let zst_path: GraphArtifactPath = dir.join(GRAPH_ARTIFACT_FILENAME).into();
    let meta_path: GraphArtifactPath = dir.join(GRAPH_ARTIFACT_META_FILENAME).into();
    let zst_non_empty = std::fs::metadata(zst_path.as_ref())
        .map(|m| m.len() > 0)
        .unwrap_or(false);
    if !zst_non_empty {
        return false.into();
    }
    let Ok(meta) = crate::boundary::artifact_transport::read_metadata(meta_path.as_ref()) else {
        return false.into();
    };
    (meta.schema_version <= GRAPH_ARTIFACT_SCHEMA_VERSION).into()
}

/// Export `snapshot` to `<repo_root>/.codebase-memory/graph.db.zst` +
/// `artifact.json`, atomically: the `.zst` and sidecar are each written
/// to a temp path first and then renamed into place, so a reader never
/// observes a half-written artifact.
pub fn export_graph_artifact<'a>(
    repo_root: impl Into<GraphArtifactRepoRoot<'a>>,
    snapshot: &GraphSnapshotDto,
    project: impl Into<GraphArtifactProjectName>,
    commit: Option<GraphArtifactCommit>,
    indexed_at: impl Into<GraphArtifactIndexedAt>,
) -> Result<ArtifactMetadataDto, GraphArtifactError> {
    let dir = artifact_dir(repo_root);
    std::fs::create_dir_all(&dir).map_err(|source| GraphArtifactError::Io {
        // CLONE-JUSTIFICATION: the typed error must retain the failed path.
        path: dir.as_path().to_path_buf().into(),
        source,
    })?;

    let compressed = crate::boundary::artifact_transport::encode(snapshot)?;
    let original_size = crate::boundary::artifact_transport::encoded_snapshot_size(snapshot)?;
    let compressed_size = compressed
        .len()
        .try_into()
        .map_err(|_size_error| GraphArtifactError::ArtifactTooLarge)?;

    let zst_path: GraphArtifactPath = dir.join(GRAPH_ARTIFACT_FILENAME).into();
    let meta_path: GraphArtifactPath = dir.join(GRAPH_ARTIFACT_META_FILENAME).into();
    // An artifact is a stable snapshot. Refuse partial prior exports as well
    // as complete ones rather than silently replacing either component.
    if zst_path.as_ref().exists() {
        return Err(GraphArtifactError::AlreadyExists { path: zst_path });
    }
    if meta_path.as_ref().exists() {
        return Err(GraphArtifactError::AlreadyExists { path: meta_path });
    }
    write_atomic(&zst_path, &compressed.into())?;

    let metadata = ArtifactMetadataDto {
        schema_version: GRAPH_ARTIFACT_SCHEMA_VERSION,
        commit: ArtifactCommitDto(commit.map(Into::into)),
        indexed_at: ArtifactIndexedAtDto(indexed_at.into().into()),
        project: ArtifactProjectDto(project.into().into()),
        nodes: snapshot.node_count(),
        edges: snapshot.edge_count(),
        original_size,
        compressed_size,
        compression_level: GRAPH_ARTIFACT_COMPRESSION_LEVEL,
    };
    let meta_json = crate::boundary::artifact_transport::encode_metadata(&metadata)?;
    write_atomic(&meta_path, &meta_json.into())?;

    Ok(metadata)
}

/// Import a previously exported artifact from `repo_root`, returning the
/// reconstructed [`GraphSnapshotDto`] and its sidecar [`ArtifactMetadataDto`].
/// Fails closed on a missing artifact, a corrupt zstd stream, or an
/// unsupported schema version -- never a partial/best-effort import.
pub fn import_graph_artifact<'a>(
    repo_root: impl Into<GraphArtifactRepoRoot<'a>>,
) -> Result<(GraphSnapshotDto, ArtifactMetadataDto), GraphArtifactError> {
    let dir = artifact_dir(repo_root);
    let zst_path = dir.join(GRAPH_ARTIFACT_FILENAME);
    let meta_path = dir.join(GRAPH_ARTIFACT_META_FILENAME);

    let metadata = crate::boundary::artifact_transport::read_metadata(&meta_path)?;
    if metadata.schema_version > GRAPH_ARTIFACT_SCHEMA_VERSION {
        return Err(GraphArtifactError::UnsupportedSchemaVersion {
            found: metadata.schema_version,
            expected: GRAPH_ARTIFACT_SCHEMA_VERSION,
        });
    }

    let compressed = std::fs::read(&zst_path).map_err(|source| GraphArtifactError::Io {
        path: zst_path.into(),
        source,
    })?;
    let snapshot = crate::boundary::artifact_transport::decode(&compressed)?;

    Ok((snapshot, metadata))
}

/// Write `bytes` to `path` atomically: write to a sibling temp file
/// first, then rename over the destination. The temp file is created
/// with a unique name derived from the process id and a monotonic
/// counter-like nanosecond timestamp so concurrent exports never
/// collide on the same temp path.
fn write_atomic(
    path: &GraphArtifactPath,
    bytes: &MemoryArtifactBytes,
) -> Result<(), GraphArtifactError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(GraphArtifactError::Clock)?
        .as_nanos();
    let unique = format!("{}.{}.tmp", std::process::id(), timestamp);
    let tmp_path = path.as_ref().with_extension(unique);
    std::fs::write(&tmp_path, bytes.as_ref()).map_err(|source| GraphArtifactError::Io {
        // CLONE-JUSTIFICATION: the typed error must retain the failed temp path.
        path: tmp_path.clone().into(),
        source,
    })?;
    std::fs::rename(&tmp_path, path.as_ref()).map_err(|source| GraphArtifactError::Io {
        path: path.as_ref().to_path_buf().into(),
        source,
    })
}
