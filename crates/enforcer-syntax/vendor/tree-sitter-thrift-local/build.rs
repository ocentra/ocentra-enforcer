//! Compiles the vendored Thrift grammar's generated `parser.c` (copied
//! unmodified from the published `tree-sitter-thrift` 0.5.0 crate --
//! see this crate's own `Cargo.toml` description and top-level
//! `LICENSE` for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-thrift-local");
}
