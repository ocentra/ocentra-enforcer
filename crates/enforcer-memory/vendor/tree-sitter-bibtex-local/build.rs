//! Compiles the vendored BibTeX grammar's generated `parser.c` (copied
//! unmodified from the published `tree-sitter-bibtex` 0.1.0 crate's
//! own `src/` -- see this crate's own `src/lib.rs` module doc for
//! provenance). This grammar has no hand-written external scanner.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-bibtex-local");
}
