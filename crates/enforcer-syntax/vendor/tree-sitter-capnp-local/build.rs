//! Compiles the vendored Cap'n Proto grammar's generated `parser.c`
//! (copied unmodified from the published `tree-sitter-capnp` 1.5.0
//! crate's own source -- see this crate's own `Cargo.toml` description
//! and top-level `LICENSE` for provenance). No external scanner exists
//! for this grammar (confirmed: the published crate ships no
//! `scanner.c`).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-capnp-local");
}
