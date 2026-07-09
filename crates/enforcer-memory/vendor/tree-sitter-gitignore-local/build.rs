//! Compiles the vendored gitignore grammar's generated `parser.c`
//! (copied unmodified from the baseline `codebase-memory-mcp`
//! repo's own `internal/cbm/vendored/grammars/gitignore/` -- see this
//! crate's own `Cargo.toml` description and top-level `LICENSE` for
//! provenance). No `scanner.c`: this grammar has no external scanner.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-gitignore-local");
}
