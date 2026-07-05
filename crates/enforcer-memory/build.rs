//! `libgit2-sys`'s Windows link-lib list (winhttp/rpcrt4/ole32/
//! crypt32/secur32) is missing `advapi32`, which is where the SID and
//! registry-key Win32 APIs `libgit2`'s `fs_path.c`/`sysdir.c`/`rand.c`
//! actually live (`OpenProcessToken`, `CheckTokenMembership`, `CopySid`,
//! `RegOpenKeyExW`, `CryptAcquireContextA`, ...). Without this, linking
//! any binary that depends on `git2` fails with ~19 unresolved
//! externals on the MSVC toolchain. This is a known gap in the
//! upstream `libgit2-sys` crate's Windows build script, not something
//! fixable from this crate's own `Cargo.toml` -- so we supply the
//! missing link directive ourselves.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
