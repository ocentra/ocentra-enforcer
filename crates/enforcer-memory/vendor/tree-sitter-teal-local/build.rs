//! Compiles the vendored Teal grammar's generated `parser.c` plus its
//! hand-written external `scanner.c` -- both copied unmodified from the
//! codebase-memory-mcp C baseline's own
//! `internal/cbm/vendored/grammars/teal/` (itself a vendored copy of
//! the upstream `teal-language/tree-sitter-teal` source; see this
//! crate's own `src/lib.rs` module doc for provenance and why this
//! grammar is vendored rather than a plain crates.io dependency).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-teal-local");
}
