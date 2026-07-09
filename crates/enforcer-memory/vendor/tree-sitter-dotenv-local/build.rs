//! Compiles the vendored DotEnv grammar's generated `parser.c` +
//! hand-written `scanner.c` (both copied unmodified from the
//! baseline's own `internal/cbm/vendored/grammars/dotenv/` -- see this
//! crate's own `src/lib.rs` module doc for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-dotenv-local");
}
