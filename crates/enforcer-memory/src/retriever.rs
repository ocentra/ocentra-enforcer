//! The federation/embedding retrieval seam.
//!
//! [`crate::recall::recall`] is the default, always-available,
//! zero-network retriever. The workpack's long-range plan calls for a
//! local embedding model (Qwen-class) and reranker on top of the graph;
//! that is explicitly NOT implemented in this slice — no model download,
//! no network call ships in the default path. What ships here is the
//! *trait seam* so a future embedding-backed retriever can be added as
//! an opt-in implementation without changing the recall contract or
//! requiring every caller (and every test) to depend on a model runtime.
//!
//! This module and its `EmbeddingRetriever` trait only compile when the
//! `embeddings` feature is enabled; the crate's default feature set
//! (used by `cargo test -p enforcer-memory` and `cargo build --workspace`)
//! never touches this file.

#![cfg(feature = "embeddings")]

use crate::graph::MemoryNode;

/// A retriever that ranks graph nodes by embedding similarity to a
/// query. Implementations of this trait are expected to live in a
/// separate, optional crate/module that owns the actual model runtime
/// (e.g. a Qwen embedding model) — `enforcer-memory` itself ships no
/// implementation, only the contract.
pub trait EmbeddingRetriever {
    /// Rank `candidates` by similarity to `query`, most similar first.
    /// Implementations MUST NOT perform any network call; any model
    /// weights they depend on must already be resolved locally (e.g.
    /// via arc-23 install) before this is called.
    fn rank<'a>(&self, query: &str, candidates: &[&'a MemoryNode]) -> Vec<&'a MemoryNode>;
}
