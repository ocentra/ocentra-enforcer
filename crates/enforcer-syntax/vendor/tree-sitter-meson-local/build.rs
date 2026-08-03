//! Compiles the vendored Meson grammar's generated `parser.c` -- copied
//! unmodified from the codebase-memory-mcp C baseline's own
//! `internal/cbm/vendored/grammars/meson/` (itself a vendored copy of
//! the upstream `Decodetalkers/tree-sitter-meson` source; see this
//! crate's own `src/lib.rs` module doc for provenance and why this
//! grammar is vendored rather than a plain crates.io dependency). No
//! external scanner -- this grammar's own baseline vendor directory has
//! no `scanner.c` at all.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-meson-local");
}
