//! Compiles the vendored Agda grammar's generated `parser.c` + hand-written
//! `scanner.c` (both unmodified copies of the baseline `codebase-memory-mcp`
//! repo's own `internal/cbm/vendored/grammars/agda/` -- see this crate's own
//! `Cargo.toml` description and top-level `LICENSE` for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-agda-local");
}
