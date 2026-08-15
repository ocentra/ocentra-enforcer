//! Vendored Thrift tree-sitter grammar binding (language-parity wave
//! G2.4c). WHY VENDORED rather than depending on the published
//! `tree-sitter-thrift` 0.5.0 crate directly: that crate's own
//! `Cargo.toml` hard-pins `tree-sitter = "~0.20.9"` as a normal
//! (non-dev) dependency and its own `bindings/rust/lib.rs` returns a
//! bare `tree_sitter::Language` typed against THAT version -- the same
//! ABI/dependency-graph incompatibility every other vendored grammar
//! in this workspace's own `vendor/` directory already documents (see
//! e.g. `tree-sitter-squirrel-local`/`tree-sitter-capnp-local`). This
//! crate re-pins the SAME grammar (the vendored `src/parser.c` in this
//! crate is an unmodified copy of that same upstream crate's own
//! generated grammar source, ABI 14 -- confirmed via `parser.c`'s own
//! `LANGUAGE_VERSION 14` `#define`, compatible with this workspace's
//! `tree-sitter = "0.25"` core's supported 13-15 ABI range) to
//! `tree-sitter-language = "0.1"` instead. MIT-licensed, matching the
//! upstream crate's own license; `LICENSE` in this crate's own
//! directory is a standard MIT template with the upstream authors'
//! names from that crate's own `Cargo.toml` `authors` field, since the
//! published crates.io package ships no `LICENSE` file to copy
//! verbatim. `publish = false` since this is a local path-dependency
//! only.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_thrift() -> *const ();
}

/// Returns the Thrift tree-sitter [`LanguageFn`]. Same calling
/// convention as every other vendored grammar in this workspace.
pub fn language() -> LanguageFn {
    // SAFETY: `tree_sitter_thrift` is the unmodified generated parser
    // entry point from the upstream `tree-sitter-thrift` 0.5.0 crate's
    // own `src/parser.c`, ABI 14.
    unsafe { LanguageFn::from_raw(tree_sitter_thrift) }
}

/// The content of the vendored `node-types.json` for this grammar.
pub const NODE_TYPES: &str = include_str!("node-types.json");
