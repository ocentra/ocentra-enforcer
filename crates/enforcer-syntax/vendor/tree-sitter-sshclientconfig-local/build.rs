//! Compiles the vendored SSH client config grammar's generated `parser.c`
//! (copied unmodified from the published `tree-sitter-ssh-client-config`
//! 2026.7.2 crate's own `src/` -- this grammar has no external scanner, so
//! no `scanner.c` exists to compile -- see this crate's own `Cargo.toml`
//! description and top-level `LICENSE` for provenance).

fn main() {
    let src_dir = "src";
    println!("cargo:rerun-if-changed={src_dir}/parser.c");
    cc::Build::new()
        .include(src_dir)
        .file(format!("{src_dir}/parser.c"))
        .warnings(false)
        .compile("tree-sitter-sshclientconfig-local");
}
