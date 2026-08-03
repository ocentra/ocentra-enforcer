//! Compiles the vendored Fennel grammar's generated `parser.c` plus its
//! hand-written external `scanner.c` (reader-macro/shebang/colon-string
//! token disambiguation) -- both copied unmodified from the upstream
//! `alexmozaidze/tree-sitter-fennel` source (see this crate's own
//! `src/lib.rs` module doc for provenance and why this grammar is
//! vendored rather than a plain crates.io/git dependency).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-fennel-local");
}
