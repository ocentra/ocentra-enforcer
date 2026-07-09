//! Vendored Agda tree-sitter grammar binding (language-parity wave G2.6).
//! WHY VENDORED rather than a published crate: no maintained
//! `tree-sitter-agda` crate exists on crates.io. This crate's own vendored
//! `src/parser.c` + `src/scanner.c` are unmodified copies of the baseline
//! `codebase-memory-mcp` repo's own
//! `internal/cbm/vendored/grammars/agda/` (`tree-sitter/tree-sitter-agda`
//! upstream, MIT-licensed -- this crate's own `LICENSE` copied verbatim
//! from that same baseline vendor directory), bound against
//! `tree-sitter-language = "0.1"` to match the exact `LanguageFn`-returning
//! shape every other grammar dependency in this workspace already uses.
//! `LANGUAGE_VERSION 14` (confirmed in `src/parser.c`), within this
//! workspace's `tree-sitter = "0.25"` core's 13-15 ABI range. `publish =
//! false` in this crate's own `Cargo.toml` since it is a local
//! path-dependency only, never intended for crates.io.
//!
//! No `node-types.json` is shipped -- the baseline vendor directory this
//! was copied from doesn't have one either, and nothing in this workspace
//! actually consumes the `NODE_TYPES` const other grammar bindings expose
//! (confirmed: it's dead convention, not a real dependency).

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_agda() -> *const ();
}

/// Returns the Agda tree-sitter [`LanguageFn`]. Same calling convention as
/// every other grammar dependency in this workspace:
/// `let language: tree_sitter::Language = tree_sitter_agda_local::language().into();`
pub fn language() -> LanguageFn {
    // SAFETY: `tree_sitter_agda` is the same generated parser entry point
    // the baseline vendor directory's own `parser.c` exposes, unmodified --
    // the same `LanguageFn::from_raw` contract every other grammar binding
    // in this workspace already relies on.
    unsafe { LanguageFn::from_raw(tree_sitter_agda) }
}
