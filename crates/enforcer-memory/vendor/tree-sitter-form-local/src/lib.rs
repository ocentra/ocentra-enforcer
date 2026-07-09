//! Vendored FORM (symbolic-manipulation language) tree-sitter grammar
//! binding (language-parity wave G2.6). WHY VENDORED rather than a
//! published crate: no maintained `tree-sitter-form` crate exists on
//! crates.io. This crate's own vendored `src/parser.c` is an unmodified
//! copy of the baseline `codebase-memory-mcp` repo's own
//! `internal/cbm/vendored/grammars/form/` (MIT-licensed -- this crate's
//! own `LICENSE` copied verbatim from that same baseline vendor
//! directory), bound against `tree-sitter-language = "0.1"` to match the
//! exact `LanguageFn`-returning shape every other grammar dependency in
//! this workspace already uses. `LANGUAGE_VERSION 15` (confirmed in
//! `src/parser.c`), within this workspace's `tree-sitter = "0.25"` core's
//! 13-15 ABI range. `publish = false` in this crate's own `Cargo.toml`
//! since it is a local path-dependency only, never intended for
//! crates.io.
//!
//! `node-types.json` copied from the baseline repo's own
//! `tools/tree-sitter-form/src/node-types.json` (the full upstream
//! grammar-repo checkout kept alongside the compiled vendor directory) --
//! the baseline's own `internal/cbm/vendored/grammars/form/` doesn't ship
//! one, this workspace's convention keeps it for reference even though
//! nothing here actually consumes the `NODE_TYPES` const at runtime.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_form() -> *const ();
}

/// Returns the FORM tree-sitter [`LanguageFn`]. Same calling convention as
/// every other grammar dependency in this workspace:
/// `let language: tree_sitter::Language = tree_sitter_form_local::language().into();`
pub fn language() -> LanguageFn {
    // SAFETY: `tree_sitter_form` is the same generated parser entry point
    // the baseline vendor directory's own `parser.c` exposes, unmodified --
    // the same `LanguageFn::from_raw` contract every other grammar binding
    // in this workspace already relies on.
    unsafe { LanguageFn::from_raw(tree_sitter_form) }
}

/// The content of the vendored `node-types.json` for this grammar -- same
/// convention every crates.io grammar dependency in this workspace already
/// exposes.
pub const NODE_TYPES: &str = include_str!("node-types.json");
