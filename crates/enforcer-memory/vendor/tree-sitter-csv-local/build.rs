//! Compiles the vendored CSV grammar's generated `parser.c` (copied
//! unmodified from the published `tree-sitter-csv` 1.2.0 crate's own
//! `csv/src/` sub-grammar -- that crate bundles three sibling
//! grammars, CSV/PSV/TSV; only the CSV one is needed here. See this
//! crate's own `src/lib.rs` module doc for provenance). This grammar
//! has no hand-written external scanner.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-csv-local");
}
