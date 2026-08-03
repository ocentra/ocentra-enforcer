//! Compiles the vendored Pine grammar's generated `parser.c` PLUS its
//! hand-written `scanner.c` (both copied unmodified from the upstream
//! `kvarenzn/tree-sitter-pine` source at commit `b9e8bd4` -- see this
//! crate's own `Cargo.toml` description and top-level `LICENSE` for
//! provenance). Upstream's own `bindings/rust/build.rs` leaves the
//! `scanner.c` compile step commented out despite the grammar genuinely
//! using an external scanner (confirmed: `src/scanner.c` exists and
//! `parser.c` references `tree_sitter_pine_external_scanner_*` symbols)
//! -- a real upstream packaging omission this crate's own `build.rs`
//! does not reproduce.

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    println!("cargo:rerun-if-changed={src_dir}/scanner.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .file(format!("{src_dir}/scanner.c"))
        .warnings(false)
        .compile("tree-sitter-pine-local");
}
