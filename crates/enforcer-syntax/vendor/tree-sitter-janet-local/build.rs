//! Compiles the vendored Janet grammar's generated `parser.c` +
//! hand-written external-scanner `scanner.c` (both copied unmodified
//! from the baseline `codebase-memory-mcp` repo's own
//! `internal/cbm/vendored/grammars/janet/` -- see this crate's own
//! `Cargo.toml` description and top-level `LICENSE` for provenance).
//! `scanner.c` is a plain C translation unit (no `extern "C++"`
//! wrapper needed, unlike some grammars' C++-flavored scanners), same
//! `cc::Build` shape as `tree-sitter-bitbake-local`'s own `build.rs`.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-janet-local");
}
