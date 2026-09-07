//! Compiles the vendored FORM grammar's generated `parser.c` (no external
//! scanner -- the baseline vendor directory this was copied from doesn't
//! ship one either; see this crate's own `Cargo.toml` description and
//! top-level `LICENSE` for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-form-local");
}
