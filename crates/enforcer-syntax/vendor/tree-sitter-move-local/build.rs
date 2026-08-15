//! Compiles the vendored Move grammar's generated `parser.c` (copied
//! unmodified from the upstream `tzakian/tree-sitter-move` repo -- see
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
        .compile("tree-sitter-move-local");
}
