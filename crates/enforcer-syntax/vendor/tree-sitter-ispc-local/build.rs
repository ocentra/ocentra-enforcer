//! Compiles the vendored ISPC grammar's generated `parser.c` (no external
//! scanner for this grammar -- copied unmodified from the upstream
//! `fab4100/tree-sitter-ispc` source -- see this crate's own `Cargo.toml`
//! description and top-level `LICENSE` for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-ispc-local");
}
