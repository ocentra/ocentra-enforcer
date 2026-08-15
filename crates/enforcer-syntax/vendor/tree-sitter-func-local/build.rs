//! Compiles the vendored FunC grammar's generated `parser.c` (copied
//! unmodified from the upstream `akifoq/tree-sitter-func` repo -- see
//! this crate's own `Cargo.toml` description and top-level `LICENSE`
//! for provenance). No `scanner.c`: this grammar has no external
//! scanner at all (confirmed -- the upstream repo's own `src/` has no
//! such file).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-func-local");
}
