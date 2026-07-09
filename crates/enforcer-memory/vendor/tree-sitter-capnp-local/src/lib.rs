//! Vendored Cap'n Proto tree-sitter grammar binding (language-parity
//! wave). WHY VENDORED rather than depending on the published
//! `tree-sitter-capnp` crate directly: that crate's own
//! `[build-dependencies] cc = "~1.0"` pin conflicts with this
//! workspace's `tree-sitter = "0.25"` core, which (transitively, via
//! other grammar dependencies already in this workspace) requires
//! `cc ^1.2.10` -- cargo cannot resolve two incompatible `cc` version
//! requirements in one dependency graph (confirmed directly: `cargo
//! check` fails with "failed to select a version for `cc`" naming
//! exactly this conflict). The published crate's own binding
//! (`bindings/rust/lib.rs`) also uses the old `extern "C" fn
//! tree_sitter_capnp() -> tree_sitter::Language` pattern rather than
//! the `tree-sitter-language` shim every other grammar dependency in
//! this workspace already uses, matching the same class of
//! incompatibility `tree-sitter-sway-local`/`tree-sitter-wolfram-local`
//! document.
//!
//! This crate re-compiles the SAME grammar (the vendored
//! `src/parser.c` is an unmodified copy of the published
//! `tree-sitter-capnp` 1.5.0 crate's own generated parser -- no
//! external scanner exists for this grammar) against
//! `tree-sitter-language = "0.1"` instead, mirroring the exact
//! `LanguageFn`-returning shape every other grammar dependency in this
//! workspace already uses. MIT-licensed, matching the upstream crate's
//! own license (copied verbatim into this crate's own `LICENSE`).
//! `publish = false` since this is a local path-dependency only.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_capnp() -> *const ();
}

/// Returns the Cap'n Proto tree-sitter [`LanguageFn`]. Same calling
/// convention as every other grammar dependency in this workspace:
/// `let language: tree_sitter::Language = tree_sitter_capnp_local::language().into();`
pub fn language() -> LanguageFn {
    // SAFETY: `tree_sitter_capnp` is the same generated parser entry
    // point the published `tree-sitter-capnp` 1.5.0 crate itself
    // exposes, unmodified -- the same `LanguageFn::from_raw` contract
    // every other grammar binding in this workspace already relies on.
    unsafe { LanguageFn::from_raw(tree_sitter_capnp) }
}

/// The content of the vendored `node-types.json` for this grammar --
/// same convention every crates.io grammar dependency in this
/// workspace already exposes.
pub const NODE_TYPES: &str = include_str!("node-types.json");
