//! Compiles the vendored COBOL grammar's generated `parser.c` +
//! hand-written `scanner.c` (copied from the upstream
//! `yutaro-sakamoto/tree-sitter-cobol` repository -- see this crate's own
//! `Cargo.toml` description and top-level `LICENSE` for provenance;
//! `scanner.c` carries one small portability fix documented inline at its
//! own edit site, everything else is unmodified).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-cobol-local");
}
