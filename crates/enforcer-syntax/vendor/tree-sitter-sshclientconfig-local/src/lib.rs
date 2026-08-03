//! Vendored SSH client config tree-sitter grammar binding (language-parity
//! wave G2.5d). WHY VENDORED rather than depending on the published
//! `tree-sitter-ssh-client-config` crate directly: that crate's own
//! `[dependencies] tree-sitter = "~0.26"` pin is incompatible with this
//! workspace's `tree-sitter = "0.25"` core (confirmed via a real
//! `cargo check`: the published crate's own binding
//! (`bindings/rust/lib.rs`) exposes `pub fn language() -> tree_sitter::Language`
//! using ITS OWN pinned `tree_sitter` crate version, not the
//! `tree-sitter-language` shim every other grammar dependency in this
//! workspace already uses -- same class of incompatibility
//! `tree-sitter-capnp-local`/`tree-sitter-ron-local` already document).
//!
//! This crate re-compiles the SAME grammar (the vendored `src/parser.c` is
//! an unmodified copy of the published `tree-sitter-ssh-client-config`
//! 2026.7.2 crate's own generated parser -- no external scanner exists for
//! this grammar) against `tree-sitter-language = "0.1"` instead, mirroring
//! the exact `LanguageFn`-returning shape every other grammar dependency in
//! this workspace already uses. CC0-1.0-licensed (public domain
//! dedication), matching the upstream crate's own license (see this
//! crate's own `LICENSE`). `publish = false` since this is a local
//! path-dependency only.
//!
//! Baseline note: the C baseline's own `CBM_LANG_SSHCONFIG` row binds a
//! DIFFERENTLY-NAMED upstream grammar (`tree_sitter_ssh_config`, root node
//! kind `source_file`) -- no crate under that exact name (or any
//! discoverable alias) exists on crates.io. This crate binds
//! `metio/tree-sitter-ssh-client-config` instead (the only SSH-config-shaped
//! grammar crates.io has at all), a functionally equivalent but distinct
//! grammar whose own root node kind is `client_config`, not `source_file`
//! -- see [`crate::languages::spec::LangSpec::sshconfig`]'s own doc comment
//! for the resulting `module_types` correction.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_ssh_client_config() -> *const ();
}

/// Returns the SSH client config tree-sitter [`LanguageFn`]. Same calling
/// convention as every other grammar dependency in this workspace:
/// `let language: tree_sitter::Language = tree_sitter_sshclientconfig_local::language().into();`
pub fn language() -> LanguageFn {
    // SAFETY: `tree_sitter_ssh_client_config` is the same generated parser
    // entry point the published `tree-sitter-ssh-client-config` 2026.7.2
    // crate itself exposes, unmodified -- the same `LanguageFn::from_raw`
    // contract every other grammar binding in this workspace already
    // relies on.
    unsafe { LanguageFn::from_raw(tree_sitter_ssh_client_config) }
}

/// The content of the vendored `node-types.json` for this grammar -- same
/// convention every crates.io grammar dependency in this workspace already
/// exposes.
pub const NODE_TYPES: &str = include_str!("node-types.json");
