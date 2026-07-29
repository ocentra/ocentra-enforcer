//! Compiles the vendored RON grammar's generated `parser.c` + hand-written
//! `scanner.c` (both copied unmodified from the published `tree-sitter-ron`
//! 0.2.0 crate's own `src/` -- see this crate's own `Cargo.toml` description
//! and top-level `LICENSE` for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-ron-local");
}
