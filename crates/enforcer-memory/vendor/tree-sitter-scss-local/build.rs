//! Compiles the vendored SCSS grammar's generated `parser.c` +
//! hand-written `scanner.c` (both copied unmodified from the upstream
//! `tree-sitter-scss` 1.0.0 crate -- see this crate's own `Cargo.toml`
//! description and top-level `LICENSE` for provenance). Deliberately does
//! NOT pass `-Wno-unused-parameter` the way that upstream crate's own
//! `bindings/rust/build.rs` does -- that unconditional (no MSVC guard)
//! flag is exactly the upstream packaging bug this vendoring works around
//! (see `src/lib.rs`'s own doc comment); `.warnings(false)` suppresses the
//! same warnings portably instead, matching every other vendored grammar
//! in this workspace (e.g. `vendor/tree-sitter-squirrel-local/build.rs`).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-scss-local");
}
