//! Compiles the vendored Blade grammar's generated `parser.c` +
//! hand-written `scanner.c` (both copied unmodified from the
//! baseline's own `internal/cbm/vendored/grammars/blade/` -- see this
//! crate's own `src/lib.rs` module doc for provenance). `parser.c` is
//! unusually large (~18MB generated source: Blade embeds the full PHP
//! and HTML grammars inline), so this build step is noticeably slower
//! than every other vendored grammar in this workspace -- expected,
//! not a bug.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-blade-local");
}
