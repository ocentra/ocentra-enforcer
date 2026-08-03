//! Vendored RON (Rusty Object Notation) tree-sitter grammar binding
//! (language-parity wave G2.5d). WHY VENDORED rather than depending on the
//! published `tree-sitter-ron` crate directly: that crate's own
//! `[dependencies] tree-sitter = "~0.20.3"` pin is incompatible with this
//! workspace's `tree-sitter = "0.25"` core (confirmed via a real
//! `cargo check`: the published crate's own binding
//! (`bindings/rust/lib.rs`) exposes `pub fn language() -> tree_sitter::Language`
//! using ITS OWN pinned `tree_sitter` crate version, not the
//! `tree-sitter-language` shim every other grammar dependency in this
//! workspace already uses -- same class of incompatibility
//! `tree-sitter-capnp-local`/`tree-sitter-squirrel-local` already document).
//!
//! This crate re-compiles the SAME grammar (the vendored `src/parser.c` +
//! `src/scanner.c` are unmodified copies of the published `tree-sitter-ron`
//! 0.2.0 crate's own generated parser + external scanner) against
//! `tree-sitter-language = "0.1"` instead, mirroring the exact
//! `LanguageFn`-returning shape every other grammar dependency in this
//! workspace already uses. MIT-licensed, matching the upstream crate's own
//! license (see this crate's own `LICENSE`). `publish = false` since this
//! is a local path-dependency only.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_ron() -> *const ();
}

/// Returns the RON tree-sitter [`LanguageFn`]. Same calling convention as
/// every other grammar dependency in this workspace:
/// `let language: tree_sitter::Language = tree_sitter_ron_local::language().into();`
pub fn language() -> LanguageFn {
    // SAFETY: `tree_sitter_ron` is the same generated parser entry point the
    // published `tree-sitter-ron` 0.2.0 crate itself exposes, unmodified --
    // the same `LanguageFn::from_raw` contract every other grammar binding
    // in this workspace already relies on.
    unsafe { LanguageFn::from_raw(tree_sitter_ron) }
}

/// The content of the vendored `node-types.json` for this grammar -- same
/// convention every crates.io grammar dependency in this workspace already
/// exposes.
pub const NODE_TYPES: &str = include_str!("node-types.json");
